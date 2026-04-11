use serde_json::Value;

use crate::gui::GuiClient;
use crate::gui::types::*;
use crate::protocol::mcp::{ContentItem, ToolResult};

#[cfg(feature = "ocr")]
use crate::gui::ocr;

/// Dispatch a tool call to the appropriate handler.
pub async fn handle_tool_call(client: &GuiClient, tool_name: &str, args: Value) -> ToolResult {
    let result = match tool_name {
        // Display
        "gui_screenshot" => handle_screenshot(client).await,
        "gui_screenshot_region" => handle_screenshot_region(client, &args).await,
        "gui_get_screen_size" => handle_get_screen_size(client).await,
        "gui_list_monitors" => handle_list_monitors(client).await,

        // OCR
        "gui_read_screen" => handle_read_screen(client, &args).await,
        "gui_find_text" => handle_find_text(client, &args).await,

        // Mouse
        "gui_click" => handle_click(client, &args).await,
        "gui_double_click" => handle_double_click(client, &args).await,
        "gui_mouse_move" => handle_mouse_move(client, &args).await,
        "gui_mouse_position" => handle_mouse_position(client).await,
        "gui_drag" => handle_drag(client, &args).await,
        "gui_scroll" => handle_scroll(client, &args).await,

        // Keyboard
        "gui_type_text" => handle_type_text(client, &args).await,
        "gui_press_key" => handle_press_key(client, &args).await,

        // Clipboard
        "gui_get_clipboard" => handle_get_clipboard(client).await,
        "gui_set_clipboard" => handle_set_clipboard(client, &args).await,

        // Window Management
        "gui_list_windows" => handle_list_windows(client).await,
        "gui_get_active_window" => handle_get_active_window(client).await,
        "gui_focus_window" => handle_focus_window(client, &args).await,
        "gui_move_resize_window" => handle_move_resize_window(client, &args).await,
        "gui_window_action" => handle_window_action(client, &args).await,
        "gui_find_windows" => handle_find_windows(client, &args).await,

        // Accessibility
        "gui_accessibility_tree" => handle_accessibility_tree(client, &args).await,
        "gui_find_ui_element" => handle_find_ui_element(client, &args).await,

        // Template matching
        "gui_find_image" => handle_find_image(client, &args).await,
        "gui_wait_for_image" => handle_wait_for_image(client, &args).await,

        // System
        "gui_system_info" => handle_system_info(client).await,

        _ => Err(format!("unknown tool: {tool_name}")),
    };

    match result {
        Ok(tr) => tr,
        Err(e) => {
            let msg = if e.contains("not supported") || e.contains("not yet implemented") {
                format!("{e}. This is a permanent error — do not retry.")
            } else if e.contains("timed out") {
                e
            } else {
                e
            };
            ToolResult::error(msg)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Argument helpers
// ═══════════════════════════════════════════════════════════════════════════

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

fn u32_arg(args: &Value, key: &str) -> Result<u32, String> {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

fn u64_arg(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

fn opt_u32(args: &Value, key: &str, default: u32) -> u32 {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .unwrap_or(default)
}

fn parse_button(args: &Value) -> MouseButton {
    match args.get("button").and_then(Value::as_str).unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Display handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_screenshot(client: &GuiClient) -> Result<ToolResult, String> {
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
                let b64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &shot.data,
                );
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
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &shot.data,
        );
        Ok(ToolResult {
            content: vec![
                ContentItem::image_base64("image/png", &b64),
                ContentItem::text(format!("{}x{} PNG", shot.width, shot.height)),
            ],
            is_error: None,
        })
    }
}

async fn handle_screenshot_region(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
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
                let b64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &shot.data,
                );
                Ok(ToolResult {
                    content: vec![ContentItem::image_base64("image/png", &b64)],
                    is_error: None,
                })
            }
        }
    }

    #[cfg(not(feature = "ocr"))]
    {
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &shot.data,
        );
        Ok(ToolResult {
            content: vec![ContentItem::image_base64("image/png", &b64)],
            is_error: None,
        })
    }
}

async fn handle_get_screen_size(client: &GuiClient) -> Result<ToolResult, String> {
    let r = client.get_screen_size().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("{}x{}", r.width, r.height)))
}

async fn handle_list_monitors(client: &GuiClient) -> Result<ToolResult, String> {
    let monitors = client.list_monitors().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&monitors).unwrap_or_default(),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// OCR handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_read_screen(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
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
            client.screenshot_region(region).await.map_err(|e| e.to_string())?
        } else {
            client.screenshot().await.map_err(|e| e.to_string())?
        };

        let result = tokio::task::spawn_blocking(move || ocr::read_screen(&shot))
            .await
            .map_err(|e| format!("OCR task failed: {e}"))?
            .map_err(|e| e.to_string())?;

        let summary = format!(
            "Screen {}x{}, {} lines of text detected.\n\n{}\n\n---\nStructured data (use cx/cy from words for gui_click):\n{}",
            result.screen_width,
            result.screen_height,
            result.lines.len(),
            result.text,
            serde_json::to_string(&result.lines).unwrap_or_default(),
        );
        Ok(ToolResult::text(summary))
    }

    #[cfg(not(feature = "ocr"))]
    {
        let _ = (client, args);
        Err("OCR not available — build with 'ocr' feature enabled.".to_string())
    }
}

async fn handle_find_text(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
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

async fn handle_click(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let x = u32_arg(args, "x")?;
    let y = u32_arg(args, "y")?;
    let button = parse_button(args);
    client.click(x, y, button).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("Clicked at ({x}, {y}).")))
}

async fn handle_double_click(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let x = u32_arg(args, "x")?;
    let y = u32_arg(args, "y")?;
    let button = parse_button(args);
    client.double_click(x, y, button).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("Double-clicked at ({x}, {y}).")))
}

async fn handle_mouse_move(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let x = u32_arg(args, "x")?;
    let y = u32_arg(args, "y")?;
    client.mouse_move(x, y).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("Moved cursor to ({x}, {y}).")))
}

async fn handle_mouse_position(client: &GuiClient) -> Result<ToolResult, String> {
    let pos = client.mouse_position().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("({}, {})", pos.x, pos.y)))
}

async fn handle_drag(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let from = Point {
        x: u32_arg(args, "from_x")?,
        y: u32_arg(args, "from_y")?,
    };
    let to = Point {
        x: u32_arg(args, "to_x")?,
        y: u32_arg(args, "to_y")?,
    };
    let button = parse_button(args);
    client.drag(from, to, button).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text("Drag complete."))
}

async fn handle_scroll(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
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
    client.scroll(x, y, direction, amount).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text("Scrolled."))
}

// ═══════════════════════════════════════════════════════════════════════════
// Keyboard handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_type_text(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let text = str_arg(args, "text")?;
    client.type_text(text).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(format!("Typed {} characters.", text.len())))
}

async fn handle_press_key(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let key = str_arg(args, "key")?;
    if key.contains('+') {
        let keys: Vec<String> = key.split('+').map(|s| s.trim().to_string()).collect();
        client.key_combo(&keys).await.map_err(|e| e.to_string())?;
    } else {
        client.press_key(key).await.map_err(|e| e.to_string())?;
    }
    Ok(ToolResult::text(format!("Pressed {key}.")))
}

// ═══════════════════════════════════════════════════════════════════════════
// Clipboard handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_get_clipboard(client: &GuiClient) -> Result<ToolResult, String> {
    let text = client.get_clipboard_text().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(text))
}

async fn handle_set_clipboard(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let text = str_arg(args, "text")?;
    client.set_clipboard_text(text).await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text("Clipboard set."))
}

// ═══════════════════════════════════════════════════════════════════════════
// Window Management handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_list_windows(client: &GuiClient) -> Result<ToolResult, String> {
    let windows = client.list_windows().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&windows).unwrap_or_default(),
    ))
}

async fn handle_get_active_window(client: &GuiClient) -> Result<ToolResult, String> {
    let window = client.get_active_window().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&window).unwrap_or_default(),
    ))
}

async fn handle_focus_window(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    if let Some(id) = args.get("window_id").and_then(Value::as_u64) {
        client.focus_window(id).await.map_err(|e| e.to_string())?;
        Ok(ToolResult::text(format!("Focused window {id}.")))
    } else if let Some(title) = args.get("title").and_then(Value::as_str) {
        let matches = client.find_windows_by_title(title).await.map_err(|e| e.to_string())?;
        if matches.is_empty() {
            Err(format!("No window found matching '{title}'."))
        } else {
            let id = matches[0].id;
            client.focus_window(id).await.map_err(|e| e.to_string())?;
            Ok(ToolResult::text(format!(
                "Focused window '{}' (id: {id}).",
                matches[0].title
            )))
        }
    } else {
        Err("Provide window_id or title.".to_string())
    }
}

async fn handle_move_resize_window(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let wid = u64_arg(args, "window_id")?;

    if let (Some(x), Some(y)) = (
        args.get("x").and_then(Value::as_i64),
        args.get("y").and_then(Value::as_i64),
    ) {
        client.move_window(wid, x as i32, y as i32).await.map_err(|e| e.to_string())?;
    }

    if let (Some(w), Some(h)) = (
        args.get("width").and_then(Value::as_u64),
        args.get("height").and_then(Value::as_u64),
    ) {
        client.resize_window(wid, w as u32, h as u32).await.map_err(|e| e.to_string())?;
    }

    Ok(ToolResult::text("Window updated."))
}

async fn handle_window_action(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let wid = u64_arg(args, "window_id")?;
    let action = str_arg(args, "action")?;

    match action {
        "minimize" => client.minimize_window(wid).await.map_err(|e| e.to_string())?,
        "maximize" => client.maximize_window(wid).await.map_err(|e| e.to_string())?,
        "restore" => client.restore_window(wid).await.map_err(|e| e.to_string())?,
        "close" => client.close_window(wid).await.map_err(|e| e.to_string())?,
        _ => return Err(format!("unknown action: {action}")),
    }

    Ok(ToolResult::text(format!("{action} done.")))
}

async fn handle_find_windows(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let query = str_arg(args, "query")?;
    let windows = client.find_windows_by_title(query).await.map_err(|e| e.to_string())?;
    if windows.is_empty() {
        Ok(ToolResult::text(format!("No windows matching '{query}'.")))
    } else {
        Ok(ToolResult::text(
            serde_json::to_string_pretty(&windows).unwrap_or_default(),
        ))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Accessibility handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_accessibility_tree(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let window_id = args.get("window_id").and_then(Value::as_u64);
    let max_depth = args.get("max_depth").and_then(Value::as_u64).map(|v| v as u32);
    let tree = client
        .get_accessibility_tree(window_id, max_depth)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&tree).unwrap_or_default(),
    ))
}

async fn handle_find_ui_element(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let query = AccessibilityQuery {
        name: args.get("query").and_then(Value::as_str).map(String::from),
        role: args.get("role").and_then(Value::as_str).map(String::from),
        window_id: args.get("window_id").and_then(Value::as_u64),
        max_depth: Some(10),
    };
    let elements = client.find_ui_elements(query).await.map_err(|e| e.to_string())?;
    if elements.is_empty() {
        Ok(ToolResult::text("No matching UI elements found."))
    } else {
        Ok(ToolResult::text(format!(
            "Found {} element(s). Use cx/cy with gui_click.\n{}",
            elements.len(),
            serde_json::to_string_pretty(&elements).unwrap_or_default()
        )))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Template matching handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_find_image(_client: &GuiClient, _args: &Value) -> Result<ToolResult, String> {
    // Phase 7 — Template matching via rustautogui
    Err("Template matching not yet implemented. This is a permanent error — do not retry.".to_string())
}

async fn handle_wait_for_image(_client: &GuiClient, _args: &Value) -> Result<ToolResult, String> {
    Err("Template matching not yet implemented. This is a permanent error — do not retry.".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// System handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_system_info(client: &GuiClient) -> Result<ToolResult, String> {
    let info = client.system_info().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&info).unwrap_or_default(),
    ))
}
