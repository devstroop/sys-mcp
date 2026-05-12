use serde_json::Value;

use crate::gui::GuiClient;
use crate::gui::types::*;
use crate::protocol::mcp::{ContentItem, ToolResult};
use crate::terminal::{PtyManager, TerminalHandle};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use uuid::Uuid;

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

        // Object detection
        #[cfg(feature = "detection")]
        "gui_detect_objects" => handle_detect_objects(client, &args).await,
        #[cfg(feature = "detection")]
        "gui_click_object" => handle_click_object(client, &args).await,

        // Utility
        "gui_wait" => handle_wait(&args).await,
        "gui_scroll_to_text" => handle_scroll_to_text(client, &args).await,

        // System
        "gui_system_info" => handle_system_info(client).await,

        // File System
        "gui_read_file" => handle_read_file(client, &args).await,
        "gui_write_file" => handle_write_file(client, &args).await,
        "gui_list_dir" => handle_list_dir(client, &args).await,
        "gui_file_exists" => handle_file_exists(client, &args).await,
        "gui_delete_file" => handle_delete_file(client, &args).await,
        "gui_create_dir" => handle_create_dir(client, &args).await,

        // Shell/Terminal
        "gui_shell_exec" => handle_shell_exec(client, &args).await,
        "gui_shell_open" => handle_shell_open(client, &args).await,
        "gui_shell_write" => handle_shell_write(client, &args).await,
        "gui_shell_read" => handle_shell_read(client, &args).await,
        "gui_shell_close" => handle_shell_close(client, &args).await,
        "gui_shell_list" => handle_shell_list(client).await,

        // MCP Hub (MCP Server Passthrough)
        "mcp_discover" => handle_mcp_discover(client).await,
        "mcp_list" => handle_mcp_list(client).await,
        "mcp_register" => handle_mcp_register(client, &args).await,
        "mcp_unregister" => handle_mcp_unregister(client, &args).await,
        "mcp_start" => handle_mcp_start(client, &args).await,
        "mcp_stop" => handle_mcp_stop(client, &args).await,
        "mcp_tools" => handle_mcp_tools(client).await,
        "mcp_tool_groups" => handle_mcp_tool_groups(client).await,
        "mcp_exec" => handle_mcp_exec(client, &args).await,

        // Process Manager
        "process_list" => handle_process_list(client, &args).await,
        "process_kill" => handle_process_kill(client, &args).await,
        "process_info" => handle_process_info(client, &args).await,
        "process_start" => handle_process_start(client, &args).await,

        // Service Manager
        "service_list" => handle_service_list(client).await,
        "service_start" => handle_service_start(client, &args).await,
        "service_stop" => handle_service_stop(client, &args).await,
        "service_status" => handle_service_status(client, &args).await,

        // Network Tools
        "network_info" => handle_network_info(client).await,
        "network_connections" => handle_network_connections(client, &args).await,

        // System Monitoring
        "system_stats" => handle_system_stats(client).await,
        "disk_usage" => handle_disk_usage(client).await,

        // Log Viewer
        "system_logs" => handle_system_logs(client, &args).await,

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
// Utility handlers
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_wait(args: &Value) -> Result<ToolResult, String> {
    let ms = args.get("ms").and_then(Value::as_u64).unwrap_or(500);
    let ms = ms.min(30000); // Cap at 30 seconds
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    Ok(ToolResult::text(format!("Waited {ms}ms.")))
}

async fn handle_scroll_to_text(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    #[cfg(feature = "ocr")]
    {
        let query = str_arg(args, "query")?.to_string();
        let direction = match args.get("direction").and_then(Value::as_str).unwrap_or("down") {
            "up" => ScrollDirection::Up,
            _ => ScrollDirection::Down,
        };
        let max_scrolls = args.get("max_scrolls").and_then(Value::as_u64).unwrap_or(10) as u32;
        let scroll_amount = args.get("scroll_amount").and_then(Value::as_u64).unwrap_or(3) as i32;

        // Default scroll position to screen center
        let screen = client.get_screen_size().await.map_err(|e| e.to_string())?;
        let sx = args.get("x").and_then(Value::as_u64).map(|v| v as u32).unwrap_or(screen.width / 2);
        let sy = args.get("y").and_then(Value::as_u64).map(|v| v as u32).unwrap_or(screen.height / 2);

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
            client.scroll(sx, sy, direction, scroll_amount).await.map_err(|e| e.to_string())?;
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

// ═══════════════════════════════════════════════════════════════════════════
// Detection handlers
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "detection")]
async fn handle_detect_objects(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let min_confidence = args.get("min_confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.3) as f32;

    let filter_labels: Option<Vec<String>> = args.get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    let result = client.detect_objects().await.map_err(|e| e.to_string())?;

    let mut detections: Vec<_> = result.detections;

    // Filter by confidence
    detections.retain(|d| d.confidence >= min_confidence);

    // Filter by labels if specified
    if let Some(labels) = &filter_labels {
        let labels_lower: Vec<String> = labels.iter().map(|l| l.to_lowercase()).collect();
        detections.retain(|d| {
            labels_lower.iter().any(|l| d.label.to_lowercase().contains(l))
        });
    }

    if detections.is_empty() {
        return Ok(ToolResult::text("No objects detected. Try lowering min_confidence or checking what's on screen."));
    }

    // Format output
    let mut output = String::from("Detected objects:\n");
    for (i, det) in detections.iter().enumerate() {
        output.push_str(&format!(
            "{}: {} (conf: {:.2}) at {},{} size {}x{}\n",
            i, det.label, det.confidence, det.x, det.y, det.width, det.height
        ));
    }
    output.push_str("\nUse gui_click_object with label and index to click.");

    Ok(ToolResult { content: vec![ContentItem::text(output)], is_error: None })
}

#[cfg(feature = "detection")]
async fn handle_click_object(client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let label = args.get("label")
        .and_then(|v| v.as_str())
        .ok_or("missing 'label' argument")?;

    let index = args.get("index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let result = client.detect_objects().await.map_err(|e| e.to_string())?;

    // Filter by label
    let matches: Vec<_> = result.detections.iter()
        .filter(|d| d.label.to_lowercase().contains(&label.to_lowercase()))
        .collect();

    if matches.is_empty() {
        return Err(format!("No objects found with label '{}'", label));
    }

    if index >= matches.len() {
        return Err(format!("Index {} out of range (found {} objects)", index, matches.len()));
    }

    let target = matches[index];
    client.click(target.cx as u32, target.cy as u32, MouseButton::Left).await
        .map_err(|e| e.to_string())?;

    Ok(ToolResult::text(format!(
        "Clicked {} at ({}, {})",
        target.label, target.cx, target.cy
    )))
}

#[cfg(not(feature = "detection"))]
mod detection_disabled {
    use super::*;
    pub async fn handle_detect_objects(_client: &GuiClient, _args: &Value) -> Result<ToolResult, String> {
        Err("Detection not available — build with 'detection' feature enabled.".to_string())
    }
    pub async fn handle_click_object(_client: &GuiClient, _args: &Value) -> Result<ToolResult, String> {
        Err("Detection not available — build with 'detection' feature enabled.".to_string())
    }
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

// ═══════════════════════════════════════════════════════════════════════════
// File System handlers
// ═══════════════════════════════════════════════════════════════════════════

use std::fs;
use std::path::Path;

async fn handle_read_file(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let path = Path::new(path);

    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.is_dir() {
        return Err(format!("Path is a directory, not a file: {}", path.display()));
    }

    // Limit file size to 10MB to prevent memory issues
    if metadata.len() > 10_000_000 {
        return Err(format!("File too large ({} bytes). Max size is 10MB.", metadata.len()));
    }

    let data = fs::read(path).map_err(|e| e.to_string())?;
    let base64 = base64_encode(&data);

    Ok(ToolResult::text(base64))
}

async fn handle_write_file(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let content = str_arg(args, "content")?;

    let path = Path::new(path);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
    }

    let data = base64_decode(content)?;
    fs::write(path, data).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(ToolResult::text(format!("Written to {}", path.display())))
}

async fn handle_list_dir(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let path = args.get("path")
        .and_then(Value::as_str)
        .unwrap_or(".");
    let path = Path::new(path);

    if !path.exists() {
        return Err(format!("Directory not found: {}", path.display()));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }

    let mut entries: Vec<serde_json::Value> = Vec::new();

    let read_dir = fs::read_dir(path).map_err(|e| e.to_string())?;

    for entry in read_dir {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata().map_err(|e| e.to_string())?;

        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };

        let modified = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        entries.push(serde_json::json!({
            "name": file_name,
            "type": file_type,
            "size": metadata.len(),
            "modified": modified
        }));
    }

    // Sort by name
    entries.sort_by(|a, b| {
        let a_is_dir = a.get("type").and_then(|t| t.as_str()) == Some("directory");
        let b_is_dir = b.get("type").and_then(|t| t.as_str()) == Some("directory");
        if a_is_dir != b_is_dir {
            b_is_dir.cmp(&a_is_dir)
        } else {
            a.get("name").and_then(|n| n.as_str()).unwrap_or("").cmp(
                b.get("name").and_then(|n| n.as_str()).unwrap_or("")
            )
        }
    });

    Ok(ToolResult::text(serde_json::to_string_pretty(&entries).unwrap_or("[]".to_string())))
}

async fn handle_file_exists(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let path = Path::new(path);

    let exists = path.exists();
    let file_type = if !exists {
        "none"
    } else if path.is_dir() {
        "directory"
    } else if path.is_file() {
        "file"
    } else {
        "other"
    };

    Ok(ToolResult::text(serde_json::json!({
        "exists": exists,
        "type": file_type
    }).to_string()))
}

async fn handle_delete_file(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let path = Path::new(path);

    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    if path.is_dir() {
        return Err("Path is a directory, not a file.".to_string());
    }

    fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))?;

    Ok(ToolResult::text(format!("Deleted {}", path.display())))
}

async fn handle_create_dir(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let path = Path::new(path);

    fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))?;

    Ok(ToolResult::text(format!("Created {}", path.display())))
}

// ═══════════════════════════════════════════════════════════════════════════
// Base64 helpers
// ═══════════════════════════════════════════════════════════════════════════

fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(data)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.decode(input).map_err(|e| format!("Invalid base64: {}", e))
}

// ═══════════════════════════════════════════════════════════════════════════
// Shell/Terminal state
// ═══════════════════════════════════════════════════════════════════════════

static SHELL_SESSIONS: std::sync::LazyLock<Arc<RwLock<HashMap<String, SessionState>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

static PTY_MANAGER: std::sync::LazyLock<Arc<PtyManager>> =
    std::sync::LazyLock::new(|| Arc::new(PtyManager::new()));

struct SessionState {
    handle: TerminalHandle,
    output_buffer: Arc<Mutex<Vec<u8>>>,
}

async fn handle_shell_exec(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let command = str_arg(args, "command")?;
    let cwd = args.get("cwd").and_then(Value::as_str);

    let session_id = format!("exec_{}", Uuid::new_v4());
    let output_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let output_buffer_clone = output_buffer.clone();

    let handle = PTY_MANAGER
        .spawn(
            session_id.clone(),
            80,
            24,
            cwd.map(String::from),
            vec![],
            move |data| {
                let mut buf = output_buffer_clone.lock().unwrap();
                buf.extend(data);
            },
        )
        .await
        .map_err(|e| format!("Failed to spawn terminal: {}", e))?;

    // Send command with newline
    handle
        .input_tx
        .send(format!("{}\n", command).into_bytes())
        .await
        .map_err(|e| format!("Failed to send command: {}", e))?;

    // Close the terminal (signals EOF to the process)
    let _ = PTY_MANAGER.close(&session_id).await;

    // Give the process a moment to finish and flush output
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Get output
    let output = spawn_blocking_get_buffer(output_buffer).await;
    let output_str = String::from_utf8(output).unwrap_or_default();

    // Clean up ANSI escape codes for display
    let cleaned = strip_ansi_codes(&output_str);

    Ok(ToolResult::text(cleaned))
}

async fn handle_shell_open(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let cwd = args.get("cwd").and_then(Value::as_str).map(String::from);
    let cols = args
        .get("cols")
        .and_then(|v| v.as_u64())
        .unwrap_or(80) as u16;
    let rows = args
        .get("rows")
        .and_then(|v| v.as_u64())
        .unwrap_or(24) as u16;

    let session_id = Uuid::new_v4().to_string();
    let output_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let output_buffer_clone = output_buffer.clone();

    let handle = PTY_MANAGER
        .spawn(
            session_id.clone(),
            cols,
            rows,
            cwd,
            vec![],
            move |data| {
                let mut buf = output_buffer_clone.lock().unwrap();
                buf.extend(data);
            },
        )
        .await
        .map_err(|e| format!("Failed to open shell: {}", e))?;

    let state = SessionState {
        handle,
        output_buffer,
    };

    SHELL_SESSIONS
        .write()
        .map_err(|e| format!("sessions poisoned: {}", e))?
        .insert(session_id.clone(), state);

    Ok(ToolResult::text(serde_json::json!({
        "session_id": session_id,
        "message": "Shell session opened. Use gui_shell_write to send commands, gui_shell_read to get output, gui_shell_close to close."
    }).to_string()))
}

async fn handle_shell_write(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let session_id = str_arg(args, "session_id")?;
    let input = str_arg(args, "input")?;

    let sessions = SHELL_SESSIONS.read().map_err(|e| format!("sessions poisoned: {}", e))?;
    let session = sessions
        .get(session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    session
        .handle
        .input_tx
        .send(input.as_bytes().to_vec())
        .await
        .map_err(|e| format!("Failed to write to shell: {}", e))?;

    Ok(ToolResult::text(format!("Sent {} bytes to session {}", input.len(), session_id)))
}

async fn handle_shell_read(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let session_id = str_arg(args, "session_id")?;

    let sessions = SHELL_SESSIONS.read().map_err(|e| format!("sessions poisoned: {}", e))?;
    let session = sessions
        .get(session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let output = spawn_blocking_get_buffer(session.output_buffer.clone()).await;
    let output_str = String::from_utf8(output).unwrap_or_default();
    let cleaned = strip_ansi_codes(&output_str);

    Ok(ToolResult::text(cleaned))
}

async fn handle_shell_close(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let session_id = str_arg(args, "session_id")?;

    PTY_MANAGER
        .close(&session_id)
        .await
        .map_err(|e| format!("Failed to close shell: {}", e))?;

    SHELL_SESSIONS.write().map_err(|e| format!("sessions poisoned: {}", e))?.remove(session_id);

    Ok(ToolResult::text(format!("Closed session {}", session_id)))
}

/// Helper to read a buffer from a std::sync::Mutex inside an async context.
async fn spawn_blocking_get_buffer(buf: Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    let buf_clone = buf.clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = buf_clone.lock().unwrap();
        std::mem::take(&mut *guard)
    })
    .await
    .unwrap_or_default()
}

async fn handle_shell_list(_client: &GuiClient) -> Result<ToolResult, String> {
    let sessions = SHELL_SESSIONS.read().map_err(|e| format!("sessions poisoned: {}", e))?;
    let ids: Vec<String> = sessions.keys().cloned().collect();

    Ok(ToolResult::text(serde_json::to_string_pretty(&ids).unwrap_or("[]".to_string())))
}

fn strip_ansi_codes(s: &str) -> String {
    // Remove ANSI escape sequences using a simple state machine.
    // Handles CSI sequences (most common), and strips everything from ESC
    // until a known terminator letter.
    let mut result = String::with_capacity(s.len());
    let mut skip = false;

    for c in s.chars() {
        if c == '\u{1B}' {
            skip = true;
        } else if skip {
            // Terminator bytes for CSI and other escape sequences
            if c.is_ascii_alphabetic() || c == '@' || c == '`' {
                if c == 'm' || c == 'H' || c == 'J' || c == 'K' || c == 'A'
                    || c == 'B' || c == 'C' || c == 'D' || c == 'P'
                    || c == 'S' || c == 'T' || c == 'f'
                {
                    skip = false;
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

// ═══════════════════════════════════════════════════════════════════════════
// MCP Hub (MCP Server Passthrough/Tunnel)
// ═══════════════════════════════════════════════════════════════════════════

use crate::mcp::hub::{McpHub, McpServerConfig, McpServerInfo};

static MCP_HUB: std::sync::LazyLock<Arc<McpHub>> =
    std::sync::LazyLock::new(|| Arc::new(McpHub::new()));

async fn handle_mcp_discover(_client: &GuiClient) -> Result<ToolResult, String> {
    let file_based = MCP_HUB.discover().await;
    let npm_based = crate::mcp::hub::discover_npm_mcp_servers().await;

    let mut all_discovered: Vec<McpServerInfo> = file_based;
    for server in npm_based {
        if !all_discovered.iter().any(|s| s.name == server.name) {
            all_discovered.push(server);
        }
    }

    Ok(ToolResult::text(serde_json::to_string_pretty(&all_discovered).unwrap_or("[]".to_string())))
}

async fn handle_mcp_list(_client: &GuiClient) -> Result<ToolResult, String> {
    let servers = MCP_HUB.list_servers().await;
    Ok(ToolResult::text(serde_json::to_string_pretty(&servers).unwrap_or("[]".to_string())))
}

async fn handle_mcp_register(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    let command = str_arg(args, "command")?;

    let args: Vec<String> = args.get("args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let config = McpServerConfig {
        command: command.to_string(),
        args,
        env: std::collections::HashMap::new(),
        transport: "stdio".to_string(),
    };

    let info = MCP_HUB.register(name.to_string(), config).await?;
    Ok(ToolResult::text(serde_json::to_string_pretty(&info).unwrap_or("{}".to_string())))
}

async fn handle_mcp_unregister(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    MCP_HUB.unregister(&name).await?;
    Ok(ToolResult::text(format!("Unregistered MCP server: {}", name)))
}

async fn handle_mcp_start(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    let tools = MCP_HUB.start_server(&name).await?;
    Ok(ToolResult::text(serde_json::json!({
        "message": format!("Started MCP server: {}", name),
        "tools_count": tools.len(),
        "tools": tools
    }).to_string()))
}

async fn handle_mcp_stop(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    MCP_HUB.stop_server(&name).await?;
    Ok(ToolResult::text(format!("Stopped MCP server: {}", name)))
}

async fn handle_mcp_tools(_client: &GuiClient) -> Result<ToolResult, String> {
    let tools = MCP_HUB.list_all_tools().await;
    Ok(ToolResult::text(serde_json::to_string_pretty(&tools).unwrap_or("[]".to_string())))
}

async fn handle_mcp_tool_groups(_client: &GuiClient) -> Result<ToolResult, String> {
    let groups = MCP_HUB.get_tool_groups().await;
    Ok(ToolResult::text(serde_json::to_string_pretty(&groups).unwrap_or("[]".to_string())))
}

async fn handle_mcp_exec(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let server = str_arg(args, "server")?;
    let tool = str_arg(args, "tool")?;
    let tool_args = args.get("args").cloned().unwrap_or(Value::Null);

    let result = MCP_HUB.execute_tool(&server, &tool, tool_args).await?;
    Ok(ToolResult::text(result.to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════
// Process Manager
// ═══════════════════════════════════════════════════════════════════════════

use crate::sysinfo::ProcessManager;

async fn handle_process_list(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let mut processes = ProcessManager::list_processes();
    processes.truncate(limit);

    Ok(ToolResult::text(serde_json::to_string_pretty(&processes).unwrap_or("[]".to_string())))
}

async fn handle_process_kill(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let pid = u64_arg(args, "pid")? as u32;

    ProcessManager::kill_process(pid).map_err(|e| e)?;

    Ok(ToolResult::text(format!("Killed process {}", pid)))
}

async fn handle_process_info(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let pid = u64_arg(args, "pid")? as u32;

    let info = ProcessManager::get_process_info(pid).map_err(|e| e)?;

    Ok(ToolResult::text(serde_json::to_string_pretty(&info).unwrap_or("{}".to_string())))
}

async fn handle_process_start(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let command = str_arg(args, "command")?;
    let args: Vec<String> = args.get("args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let pid = ProcessManager::start_process(&command, args).map_err(|e| e)?;

    Ok(ToolResult::text(serde_json::json!({
        "message": format!("Started process {}", command),
        "pid": pid
    }).to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════
// Service Manager
// ═══════════════════════════════════════════════════════════════════════════

use crate::system::{ServiceManager, NetworkManager, SystemMonitor, LogViewer};

async fn handle_service_list(_client: &GuiClient) -> Result<ToolResult, String> {
    let services = ServiceManager::list_services();
    Ok(ToolResult::text(serde_json::to_string_pretty(&services).unwrap_or("[]".to_string())))
}

async fn handle_service_start(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    ServiceManager::start_service(&name).map_err(|e| e)?;
    Ok(ToolResult::text(format!("Started service: {}", name)))
}

async fn handle_service_stop(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    ServiceManager::stop_service(&name).map_err(|e| e)?;
    Ok(ToolResult::text(format!("Stopped service: {}", name)))
}

async fn handle_service_status(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    let status = ServiceManager::service_status(&name).map_err(|e| e)?;
    Ok(ToolResult::text(serde_json::to_string_pretty(&status).unwrap_or("{}".to_string())))
}

// ═══════════════════════════════════════════════════════════════════════════
// Network Tools
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_network_info(_client: &GuiClient) -> Result<ToolResult, String> {
    let info = NetworkManager::get_info().map_err(|e| e)?;
    Ok(ToolResult::text(serde_json::to_string_pretty(&info).unwrap_or("{}".to_string())))
}

async fn handle_network_connections(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let _limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let connections = NetworkManager::list_connections();
    Ok(ToolResult::text(serde_json::to_string_pretty(&connections).unwrap_or("[]".to_string())))
}

// ═══════════════════════════════════════════════════════════════════════════
// System Monitoring
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_system_stats(_client: &GuiClient) -> Result<ToolResult, String> {
    let stats = SystemMonitor::get_stats().map_err(|e| e)?;
    Ok(ToolResult::text(serde_json::to_string_pretty(&stats).unwrap_or("{}".to_string())))
}

async fn handle_disk_usage(_client: &GuiClient) -> Result<ToolResult, String> {
    let usage = SystemMonitor::get_disk_usage();
    Ok(ToolResult::text(serde_json::to_string_pretty(&usage).unwrap_or("[]".to_string())))
}

// ═══════════════════════════════════════════════════════════════════════════
// Log Viewer
// ═══════════════════════════════════════════════════════════════════════════

async fn handle_system_logs(_client: &GuiClient, args: &Value) -> Result<ToolResult, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let level = args.get("level").and_then(|v| v.as_str()).map(String::from);

    let logs = LogViewer::get_system_logs(count, level);
    Ok(ToolResult::text(serde_json::to_string_pretty(&logs).unwrap_or("[]".to_string())))
}
