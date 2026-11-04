#![allow(dead_code)]
#[cfg(feature = "ocr")]
use crate::gui::ocr;
#[cfg(feature = "ocr")]
use crate::gui::types::*;
use crate::gui::GuiClient;
#[cfg(feature = "ocr")]
use crate::mcp::handlers::{json_compact, json_pretty, str_arg, u32_arg};
use crate::protocol::mcp::ToolResult;
use serde_json::Value;

pub(crate) async fn handle_read_screen(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    #[cfg(feature = "ocr")]
    {
        // Check if a region is specified. OCR runs on the cropped image, so its
        // coordinates come back relative to that crop; remember the origin to
        // translate them into screen space, which is what `mouse_click` takes.
        let (shot, region_origin) = if args.get("x").is_some() {
            let region = Region {
                x: u32_arg(args, "x")?,
                y: u32_arg(args, "y")?,
                width: u32_arg(args, "width")?,
                height: u32_arg(args, "height")?,
            };
            let shot = client
                .screenshot_region(region)
                .await
                .map_err(|e| e.to_string())?;
            (shot, Some((region.x as i32, region.y as i32)))
        } else {
            (client.screenshot().await.map_err(|e| e.to_string())?, None)
        };

        let mut result = tokio::task::spawn_blocking(move || ocr::read_screen(&shot))
            .await
            .map_err(|e| format!("OCR task failed: {e}"))?
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

        let detail = args.get("detail").and_then(Value::as_str).unwrap_or("full");

        let summary = match detail {
            "text" => format!(
                "{scope}, {} lines of text detected.\n\n{}",
                result.lines.len(),
                result.text,
            ),
            "lines" => {
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
            _ => {
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
        let q = query.clone();
        let matches = tokio::task::spawn_blocking(move || ocr::find_text(&shot, &q))
            .await
            .map_err(|e| format!("OCR task failed: {e}"))?
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
            let q = query.clone();
            let matches = tokio::task::spawn_blocking(move || ocr::find_text(&shot, &q))
                .await
                .map_err(|e| format!("OCR task failed: {e}"))?
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
