#[cfg(feature = "ocr")]
use crate::gui::ocr;
#[cfg(feature = "ocr")]
use crate::gui::types::*;
use crate::gui::GuiClient;
#[cfg(feature = "ocr")]
use crate::mcp::handlers::{json_compact, json_pretty, str_arg};
use crate::protocol::mcp::ToolResult;
use serde_json::Value;

/// How much of the OCR payload comes back.
///
/// An unrecognized `detail` used to fall through to the `_` arm, which is
/// `Full` — the heaviest branch. A caller that wrote `"txt"` or `"Lines"`
/// paid for word-level coordinates it had explicitly asked not to receive,
/// and the response gave no sign the request had been ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "ocr")]
enum Detail {
    Text,
    Lines,
    Full,
}

/// Absent or null means `Full`, matching the declared default. Anything else
/// must be one of the three advertised names.
#[cfg(feature = "ocr")]
fn parse_detail(args: &Value) -> Result<Detail, String> {
    match args.get("detail") {
        None | Some(Value::Null) => Ok(Detail::Full),
        Some(Value::String(raw)) => match raw.trim().to_ascii_lowercase().as_str() {
            "text" => Ok(Detail::Text),
            "lines" => Ok(Detail::Lines),
            "full" => Ok(Detail::Full),
            _ => Err(format!(
                "invalid argument: 'detail' = {raw:?} is not one of full, lines, text"
            )),
        },
        Some(v) => Err(format!(
            "invalid argument: 'detail' must be a string (full, lines or text), got {v}"
        )),
    }
}

/// A screen region is all four coordinates or none.
///
/// A *present* key is what selects region mode, so a present-but-null `x` must
/// not trigger it and then fail with "missing required argument: x". Half a
/// region is refused rather than guessed at: cropping by only two of the four
/// numbers would silently OCR a rectangle the caller never described.
#[cfg(feature = "ocr")]
fn parse_region(args: &Value) -> Result<Option<Region>, String> {
    const KEYS: [&str; 4] = ["x", "y", "width", "height"];
    let present = |key: &str| !matches!(args.get(key), None | Some(Value::Null));

    let missing: Vec<&str> = KEYS.iter().copied().filter(|k| !present(k)).collect();
    if missing.len() == KEYS.len() {
        return Ok(None);
    }
    if !missing.is_empty() {
        return Err(format!(
            "invalid argument: a region read needs all four of x, y, width and height (missing: {}). Omit all four to OCR the whole screen.",
            missing.join(", ")
        ));
    }

    let coord = |key: &str| {
        args.get(key)
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .ok_or_else(|| {
                format!("invalid argument: region '{key}' must be a non-negative integer")
            })
    };
    let region = Region {
        x: coord("x")?,
        y: coord("y")?,
        width: coord("width")?,
        height: coord("height")?,
    };
    if region.width == 0 || region.height == 0 {
        return Err(format!(
            "invalid argument: region must have a positive width and height, got {}x{}",
            region.width, region.height
        ));
    }
    Ok(Some(region))
}

pub(crate) async fn handle_read_screen(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    #[cfg(feature = "ocr")]
    {
        // Check if a region is specified. OCR runs on the cropped image, so its
        // coordinates come back relative to that crop; remember the origin to
        // translate them into screen space, which is what `mouse_click` takes.
        let region = parse_region(args)?;
        let detail = parse_detail(args)?;
        let (shot, region_origin) = match region {
            Some(region) => {
                let shot = client
                    .screenshot_region(region)
                    .await
                    .map_err(|e| e.to_string())?;
                (shot, Some((region.x as i32, region.y as i32)))
            }
            None => (client.screenshot().await.map_err(|e| e.to_string())?, None),
        };

        let mut result = client
            .ocr()
            .map_err(|e| e.to_string())?
            .read_screen(shot)
            .await
            .map_err(|e| e.to_string())?;

        // `screen_width`/`screen_height` on the result are the *captured image*
        // dimensions. For a region read the coordinates are about to become
        // screen-absolute, so labelling them "Screen 420x340" would put `cy`
        // outside the stated height and read as a corrupt payload. Name the
        // scope explicitly instead.
        let scope = match region_origin {
            Some((ox, oy)) => {
                ocr::translate_to_screen(&mut result, (ox, oy));
                format!(
                    "Region {}x{} at ({ox},{oy}) — coordinates are screen-space",
                    result.screen_width, result.screen_height
                )
            }
            None => format!("Screen {}x{}", result.screen_width, result.screen_height),
        };

        let summary = match detail {
            Detail::Text => format!(
                "{scope}, {} lines of text detected.\n\n{}",
                result.lines.len(),
                result.text,
            ),
            Detail::Lines => {
                // Lines with bounding boxes but no word-level detail
                let lines_data: Vec<serde_json::Value> = result
                    .lines
                    .iter()
                    .map(|l| {
                        serde_json::json!({
                            "text": l.text,
                            "x": l.x, "y": l.y,
                            "width": l.width, "height": l.height,
                            "cx": l.x + l.width / 2,
                            "cy": l.y + l.height / 2,
                        })
                    })
                    .collect();
                let coordinates = json_compact(&lines_data)?;
                format!(
                    "{scope}, {} lines of text detected.\n\n{}\n\n---\nLine coordinates (use cx/cy for mouse_click):\n{}",
                    result.lines.len(),
                    result.text,
                    coordinates,
                )
            }
            Detail::Full => {
                let structured = json_compact(&result.lines)?;
                format!(
                    "{scope}, {} lines of text detected.\n\n{}\n\n---\nStructured data (use cx/cy from words for mouse_click):\n{}",
                    result.lines.len(),
                    result.text,
                    structured,
                )
            }
        };
        Ok(ToolResult::text(summary))
    }

    #[cfg(not(feature = "ocr"))]
    {
        let _ = (client, args);
        Err("OCR not available — build with 'ocr' feature enabled.".to_string())
    }
}

pub(crate) async fn handle_find_text(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    #[cfg(feature = "ocr")]
    {
        let query = str_arg(args, "query")?.to_string();
        let shot = client.screenshot().await.map_err(|e| e.to_string())?;
        let matches = client
            .ocr()
            .map_err(|e| e.to_string())?
            .find_text(shot, query.clone())
            .await
            .map_err(|e| e.to_string())?;

        if matches.is_empty() {
            Ok(ToolResult::text(format!(
                "No text matching '{}' found on screen. Try screen_read to see all visible text.",
                query
            )))
        } else {
            let locations = json_pretty(&matches)?;
            let summary = format!(
                "Found {} match(es). Use cx/cy coordinates directly with mouse_click.\n{}",
                matches.len(),
                locations
            );
            Ok(ToolResult::text(summary))
        }
    }

    #[cfg(not(feature = "ocr"))]
    {
        let _ = (client, args);
        Err("OCR not available — build with 'ocr' feature enabled.".to_string())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Mouse handlers
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_scroll_to_text(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    #[cfg(feature = "ocr")]
    {
        let query = str_arg(args, "query")?.to_string();
        let direction = match args
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("down")
        {
            "up" => ScrollDirection::Up,
            _ => ScrollDirection::Down,
        };
        let max_scrolls = args
            .get("max_scrolls")
            .and_then(Value::as_u64)
            .unwrap_or(10) as u32;
        let scroll_amount = args
            .get("scroll_amount")
            .and_then(Value::as_u64)
            .unwrap_or(3) as i32;

        // Default scroll position to screen center
        let screen = client.get_screen_size().await.map_err(|e| e.to_string())?;
        let sx = args
            .get("x")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(screen.width / 2);
        let sy = args
            .get("y")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or(screen.height / 2);

        for i in 0..max_scrolls {
            // Take screenshot and OCR
            let shot = client.screenshot().await.map_err(|e| e.to_string())?;
            let matches = client
                .ocr()
                .map_err(|e| e.to_string())?
                .find_text(shot, query.clone())
                .await
                .map_err(|e| e.to_string())?;

            if !matches.is_empty() {
                let locations = json_pretty(&matches)?;
                return Ok(ToolResult::text(format!(
                    "Found '{}' after {} scroll(s). Use cx/cy with mouse_click.\n{}",
                    query, i, locations
                )));
            }

            // Scroll and wait for content to settle
            client
                .scroll(sx, sy, direction, scroll_amount)
                .await
                .map_err(|e| e.to_string())?;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        Ok(ToolResult::text(format!(
            "Text '{}' not found after {} scrolls. Try screen_read to see what's currently visible.",
            query, max_scrolls
        )))
    }

    #[cfg(not(feature = "ocr"))]
    {
        let _ = (client, args);
        Err("OCR not available — build with 'ocr' feature enabled.".to_string())
    }
}

#[cfg(all(test, feature = "ocr"))]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_region(got: Option<Region>, want: (u32, u32, u32, u32)) {
        let region = got.expect("expected a parsed region");
        assert_eq!(
            (region.x, region.y, region.width, region.height),
            want,
            "region fields"
        );
    }

    #[test]
    fn absent_or_null_detail_means_the_declared_default() {
        assert_eq!(parse_detail(&json!({})).unwrap(), Detail::Full);
        assert_eq!(
            parse_detail(&json!({ "detail": null })).unwrap(),
            Detail::Full
        );
        assert_eq!(
            parse_detail(&json!({ "detail": "text" })).unwrap(),
            Detail::Text
        );
        assert_eq!(
            parse_detail(&json!({ "detail": "lines" })).unwrap(),
            Detail::Lines
        );
    }

    #[test]
    fn detail_is_matched_case_insensitively_but_not_by_prefix() {
        // `"LINES"` and `" lines "` mean the same thing as `"lines"`. A typo does
        // not: the old `_` arm sent it to the heaviest branch.
        assert_eq!(
            parse_detail(&json!({ "detail": "LINES" })).unwrap(),
            Detail::Lines
        );
        assert_eq!(
            parse_detail(&json!({ "detail": " text " })).unwrap(),
            Detail::Text
        );

        for raw in ["txt", "word", "Full1", ""] {
            let err = parse_detail(&json!({ "detail": raw })).unwrap_err();
            assert!(err.contains("full, lines, text"), "{raw:?} => {err}");
        }
        for wrong_type in [json!(5), json!(true), json!(["lines"]), json!({ "a": 1 })] {
            let err = parse_detail(&json!({ "detail": wrong_type })).unwrap_err();
            assert!(err.contains("must be a string"), "{wrong_type} => {err}");
        }
    }

    #[test]
    fn a_full_region_parses_and_no_region_keys_means_whole_screen() {
        assert_region(
            parse_region(&json!({ "x": 10, "y": 20, "width": 300, "height": 40 })).unwrap(),
            (10, 20, 300, 40),
        );

        // Nothing region-shaped in the arguments: OCR the whole screen.
        assert!(parse_region(&json!({})).unwrap().is_none());
        assert!(parse_region(&json!({ "detail": "text" }))
            .unwrap()
            .is_none());

        // Present-but-null does not select region mode, so it must not then
        // complain about the keys it is not asking for.
        assert!(
            parse_region(&json!({ "x": null, "y": null, "width": null, "height": null }))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn half_a_region_is_refused_naming_the_missing_keys() {
        // The bug: mode selection keyed on `x` alone, so `{"x": 10}` cropped with
        // defaults for the other three and OCR'd a rectangle nobody described.
        let err = parse_region(&json!({ "x": 10, "y": 20 })).unwrap_err();
        assert!(err.contains("width, height"), "{err}");
        assert!(err.contains("Omit all four"), "{err}");

        let err = parse_region(&json!({ "width": 300 })).unwrap_err();
        assert!(err.contains("x, y, height"), "{err}");
    }

    #[test]
    fn region_coordinates_must_be_usable_integers() {
        for key in ["x", "y", "width", "height"] {
            for bad in [json!("10"), json!(-1), json!(1.5), json!(true)] {
                let mut args = json!({ "x": 1, "y": 2, "width": 3, "height": 4 });
                args[key] = bad.clone();
                let err = parse_region(&args).unwrap_err();
                assert!(
                    err.contains(&format!("'{key}' must be a non-negative integer")),
                    "{key}={bad} => {err}"
                );
            }
        }
    }

    #[test]
    fn a_zero_sized_region_is_refused_instead_of_cropping_nothing() {
        let err = parse_region(&json!({ "x": 0, "y": 0, "width": 0, "height": 100 })).unwrap_err();
        assert!(err.contains("positive width and height"), "{err}");
        assert!(err.contains("got 0x100"), "{err}");

        let err = parse_region(&json!({ "x": 0, "y": 0, "width": 100, "height": 0 })).unwrap_err();
        assert!(err.contains("got 100x0"), "{err}");
    }
}
