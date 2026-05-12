#![allow(dead_code)]
use crate::gui::ocr;
use crate::gui::types::*;
use crate::gui::GuiClient;
use crate::mcp::handlers::{str_arg, u32_arg};
use crate::protocol::mcp::ToolResult;
use serde_json::Value;

pub(crate) async fn handle_read_screen(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    #[cfg(feature = "ocr")]
    {
        // Check if a region is specified
        let shot = if args.get("x").is_some() {
            let region = Region {
                x: u32_arg(args, "x")?,
                y: u32_arg(args, "y")?,
                width: u32_arg(args, "width")?,
                height: u32_arg(args, "height")?,
            };
            client
                .screenshot_region(region)
                .await
                .map_err(|e| e.to_string())?
        } else {
            client.screenshot().await.map_err(|e| e.to_string())?
        };

        let result = tokio::task::spawn_blocking(move || ocr::read_screen(&shot))
            .await
            .map_err(|e| format!("OCR task failed: {e}"))?
            .map_err(|e| e.to_string())?;

        let detail = args.get("detail").and_then(Value::as_str).unwrap_or("full");

        let summary = match detail {
            "text" => format!(
                "Screen {}x{}, {} lines of text detected.\n\n{}",
                result.screen_width,
                result.screen_height,
                result.lines.len(),
                result.text,
            ),
            "lines" => {
                // Lines with bounding boxes but no word-level detail
                let lines_data: Vec<serde_json::Value> = result.lines.iter().map(|l| {
                    serde_json::json!({
                        "text": l.text,
                        "x": l.x, "y": l.y,
                        "width": l.width, "height": l.height,
                        "cx": l.x + l.width / 2,
                        "cy": l.y + l.height / 2,
                    })
                }).collect();
                format!(
                    "Screen {}x{}, {} lines of text detected.\n\n{}\n\n---\nLine coordinates (use cx/cy for gui_click):\n{}",
                    result.screen_width,
                    result.screen_height,
                    result.lines.len(),
                    result.text,
                    serde_json::to_string(&lines_data).unwrap_or_default(),
                )
            }
            _ => format!(
                "Screen {}x{}, {} lines of text detected.\n\n{}\n\n---\nStructured data (use cx/cy from words for gui_click):\n{}",
                result.screen_width,
                result.screen_height,
                result.lines.len(),
                result.text,
                serde_json::to_string(&result.lines).unwrap_or_default(),
            ),
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
                "No text matching '{}' found on screen. Try gui_read_screen to see all visible text.",
                query
            )))
        } else {
            let summary = format!(
                "Found {} match(es). Use cx/cy coordinates directly with gui_click.\n{}",
                matches.len(),
                serde_json::to_string_pretty(&matches).unwrap_or_default()
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
                return Ok(ToolResult::text(format!(
                    "Found '{}' after {} scroll(s). Use cx/cy with gui_click.\n{}",
                    query,
                    i,
                    serde_json::to_string_pretty(&matches).unwrap_or_default()
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
            "Text '{}' not found after {} scrolls. Try gui_read_screen to see what's currently visible.",
            query, max_scrolls
        )))
    }

    #[cfg(not(feature = "ocr"))]
    {
        let _ = (client, args);
        Err("OCR not available — build with 'ocr' feature enabled.".to_string())
    }
}
