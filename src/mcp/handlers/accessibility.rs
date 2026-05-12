#![allow(dead_code)]
use crate::gui::types::*;
use crate::gui::GuiClient;
use crate::protocol::mcp::ToolResult;
use serde_json::Value;

pub(crate) async fn handle_accessibility_tree(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let window_id = args.get("window_id").and_then(Value::as_u64);
    let max_depth = args
        .get("max_depth")
        .and_then(Value::as_u64)
        .map(|v| v as u32);
    let tree = client
        .get_accessibility_tree(window_id, max_depth)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&tree).unwrap_or_default(),
    ))
}

pub(crate) async fn handle_find_ui_element(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let query = AccessibilityQuery {
        name: args.get("query").and_then(Value::as_str).map(String::from),
        role: args.get("role").and_then(Value::as_str).map(String::from),
        window_id: args.get("window_id").and_then(Value::as_u64),
        max_depth: Some(10),
    };
    let elements = client
        .find_ui_elements(query)
        .await
        .map_err(|e| e.to_string())?;
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
