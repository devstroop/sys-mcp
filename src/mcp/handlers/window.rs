#![allow(dead_code)]
use crate::gui::GuiClient;
use crate::mcp::handlers::{str_arg, u64_arg};
use crate::protocol::mcp::ToolResult;
use serde_json::Value;

pub(crate) async fn handle_list_windows(client: &GuiClient) -> Result<ToolResult, String> {
    let windows = client.list_windows().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&windows).unwrap_or_default(),
    ))
}

pub(crate) async fn handle_get_active_window(client: &GuiClient) -> Result<ToolResult, String> {
    let window = client
        .get_active_window()
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&window).unwrap_or_default(),
    ))
}

pub(crate) async fn handle_focus_window(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    if let Some(id) = args.get("window_id").and_then(Value::as_u64) {
        client.focus_window(id).await.map_err(|e| e.to_string())?;
        Ok(ToolResult::text(format!("Focused window {id}.")))
    } else if let Some(title) = args.get("title").and_then(Value::as_str) {
        let matches = client
            .find_windows_by_title(title)
            .await
            .map_err(|e| e.to_string())?;
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

pub(crate) async fn handle_move_resize_window(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let wid = u64_arg(args, "window_id")?;

    if let (Some(x), Some(y)) = (
        args.get("x").and_then(Value::as_i64),
        args.get("y").and_then(Value::as_i64),
    ) {
        client
            .move_window(wid, x as i32, y as i32)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let (Some(w), Some(h)) = (
        args.get("width").and_then(Value::as_u64),
        args.get("height").and_then(Value::as_u64),
    ) {
        client
            .resize_window(wid, w as u32, h as u32)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(ToolResult::text("Window updated."))
}

pub(crate) async fn handle_window_action(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let wid = u64_arg(args, "window_id")?;
    let action = str_arg(args, "action")?;

    match action {
        "minimize" => client
            .minimize_window(wid)
            .await
            .map_err(|e| e.to_string())?,
        "maximize" => client
            .maximize_window(wid)
            .await
            .map_err(|e| e.to_string())?,
        "restore" => client
            .restore_window(wid)
            .await
            .map_err(|e| e.to_string())?,
        "close" => client.close_window(wid).await.map_err(|e| e.to_string())?,
        _ => return Err(format!("unknown action: {action}")),
    }

    Ok(ToolResult::text(format!("{action} done.")))
}

pub(crate) async fn handle_find_windows(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let query = str_arg(args, "query")?;
    let windows = client
        .find_windows_by_title(query)
        .await
        .map_err(|e| e.to_string())?;
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
