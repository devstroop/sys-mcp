#![allow(dead_code)]
#[cfg(feature = "ocr")]
use crate::gui::ocr;
use crate::gui::types::*;
use crate::gui::GuiClient;
use crate::mcp::handlers::{opt_u32, parse_button, str_arg, u32_arg};
use crate::protocol::mcp::{ContentItem, ToolResult};
use serde_json::Value;

pub(crate) async fn handle_screenshot(client: &GuiClient) -> Result<ToolResult, String> {
    let shot = client.screenshot().await.map_err(|e| e.to_string())?;

    #[cfg(feature = "ocr")]
    {
        match ocr::compress_screenshot(&shot, 60, 0.5) {
            Ok((jpeg_b64, w, h)) => Ok(ToolResult {
                content: vec![
                    ContentItem::image_base64("image/jpeg", &jpeg_b64),
                    ContentItem::text(format!(
                        "{}x{} JPEG (compressed from {}x{}). Use gui_read_screen for text content.",
                        w, h, shot.width, shot.height
                    )),
                ],
                is_error: None,
            }),
            Err(_) => {
                let b64 =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &shot.data);
                Ok(ToolResult {
                    content: vec![
                        ContentItem::image_base64("image/png", &b64),
                        ContentItem::text(format!("{}x{} PNG", shot.width, shot.height)),
                    ],
                    is_error: None,
                })
            }
        }
    }

    #[cfg(not(feature = "ocr"))]
    {
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &shot.data);
        Ok(ToolResult {
            content: vec![
                ContentItem::image_base64("image/png", &b64),
                ContentItem::text(format!("{}x{} PNG", shot.width, shot.height)),
            ],
            is_error: None,
        })
    }
}

pub(crate) async fn handle_screenshot_region(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
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

    #[cfg(feature = "ocr")]
    {
        match ocr::compress_screenshot(&shot, 70, 1.0) {
            Ok((jpeg_b64, w, h)) => Ok(ToolResult {
                content: vec![
                    ContentItem::image_base64("image/jpeg", &jpeg_b64),
                    ContentItem::text(format!("{}x{} JPEG region", w, h)),
                ],
                is_error: None,
            }),
            Err(_) => {
                let b64 =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &shot.data);
                Ok(ToolResult {
                    content: vec![ContentItem::image_base64("image/png", &b64)],
                    is_error: None,
                })
            }
        }
    }

    #[cfg(not(feature = "ocr"))]
    {
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &shot.data);
        Ok(ToolResult {
            content: vec![ContentItem::image_base64("image/png", &b64)],
            is_error: None,
        })
    }
}

pub(crate) async fn handle_get_screen_size(client: &GuiClient) -> Result<ToolResult, String> {
    let r = client.get_screen_size().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("{}x{}", r.width, r.height)))
}

pub(crate) async fn handle_list_monitors(client: &GuiClient) -> Result<ToolResult, String> {
    let monitors = client.list_monitors().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&monitors).unwrap_or_default(),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// OCR handlers
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_click(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let x = u32_arg(args, "x")?;
    let y = u32_arg(args, "y")?;
    let button = parse_button(args);
    client
        .click(x, y, button)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("Clicked at ({x}, {y}).")))
}

pub(crate) async fn handle_double_click(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let x = u32_arg(args, "x")?;
    let y = u32_arg(args, "y")?;
    let button = parse_button(args);
    client
        .double_click(x, y, button)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("Double-clicked at ({x}, {y}).")))
}

pub(crate) async fn handle_mouse_move(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let x = u32_arg(args, "x")?;
    let y = u32_arg(args, "y")?;
    client.mouse_move(x, y).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("Moved cursor to ({x}, {y}).")))
}

pub(crate) async fn handle_mouse_position(client: &GuiClient) -> Result<ToolResult, String> {
    let pos = client.mouse_position().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("({}, {})", pos.x, pos.y)))
}

pub(crate) async fn handle_drag(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let from = Point {
        x: u32_arg(args, "from_x")?,
        y: u32_arg(args, "from_y")?,
    };
    let to = Point {
        x: u32_arg(args, "to_x")?,
        y: u32_arg(args, "to_y")?,
    };
    let button = parse_button(args);
    client
        .drag(from, to, button)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text("Drag complete."))
}

pub(crate) async fn handle_scroll(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let x = u32_arg(args, "x")?;
    let y = u32_arg(args, "y")?;
    let direction = match str_arg(args, "direction")? {
        "up" => ScrollDirection::Up,
        "down" => ScrollDirection::Down,
        "left" => ScrollDirection::Left,
        "right" => ScrollDirection::Right,
        d => return Err(format!("unknown direction: {d}")),
    };
    let amount = opt_u32(args, "amount", 3) as i32;
    client
        .scroll(x, y, direction, amount)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text("Scrolled."))
}

// ═══════════════════════════════════════════════════════════════════════════
// Keyboard handlers
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_type_text(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let text = str_arg(args, "text")?;
    client.type_text(text).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!(
        "Typed {} characters.",
        text.len()
    )))
}

pub(crate) async fn handle_press_key(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let raw_key = str_arg(args, "key")?;
    // Normalize key names to lowercase for rustautogui compatibility
    let key = raw_key.to_lowercase();
    if key.contains('+') {
        let keys: Vec<String> = key.split('+').map(|s| s.trim().to_string()).collect();
        client.key_combo(&keys).await.map_err(|e| e.to_string())?;
    } else {
        client.press_key(&key).await.map_err(|e| e.to_string())?;
    }
    Ok(ToolResult::text(format!("Pressed {raw_key}.")))
}

// ═══════════════════════════════════════════════════════════════════════════
// Clipboard handlers
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_get_clipboard(client: &GuiClient) -> Result<ToolResult, String> {
    let text = client
        .get_clipboard_text()
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text(text))
}

pub(crate) async fn handle_set_clipboard(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let text = str_arg(args, "text")?;
    client
        .set_clipboard_text(text)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text("Clipboard set."))
}

// ═══════════════════════════════════════════════════════════════════════════
// Window Management handlers
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(data)
}

pub(crate) fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD
        .decode(input)
        .map_err(|e| format!("Invalid base64: {}", e))
}
