use crate::gui::types::*;
use crate::gui::GuiClient;
use crate::mcp::handlers::json_pretty;
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
    Ok(ToolResult::text(json_pretty(&tree)?))
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
            "Found {} element(s). Use cx/cy with mouse_click.\n{}",
            elements.len(),
            json_pretty(&elements)?
        )))
    }
}

/// Resolve a single element by the id handed out by `accessibility_tree` /
/// `accessibility_find` and return its current properties.
pub(crate) async fn handle_element_properties(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let id = super::str_arg(args, "element_id")?;
    let node = client
        .get_element_properties(id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ToolResult::text(json_pretty(&node)?))
}

/// Invoke a named action on an element resolved by id (e.g. "invoke"/"press"
/// on a button). The element id must come from the most recent tree walk.
pub(crate) async fn handle_element_action(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let id = super::str_arg(args, "element_id")?;
    let action = super::str_arg(args, "action")?;
    let value = action_value(args)?;

    // The value and the verb have to agree before anything is invoked. A value
    // the backend cannot use is refused rather than dropped — discarding one of
    // the caller's arguments and reporting success is the failure class recorded
    // in `docs/ISSUES.md` #40 — and a write with nothing to write is refused
    // here so the answer is the same on every platform.
    if value.is_some() && !takes_value(action) {
        return Err(format!(
            "action '{action}' does not take a value; remove the value argument (only {} accept one)",
            VALUE_VERBS.join(", ")
        ));
    }
    if value.is_none() && takes_value(action) {
        return Err(format!(
            "action '{action}' writes a value and needs a value argument; pass value: \"\u{2026}\""
        ));
    }

    client
        .invoke_element_action(id, action, value.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    if value.is_some() {
        // The written value is deliberately not echoed back. It is whatever the
        // caller meant to put in the field — a password among them — and the
        // caller already knows it.
        Ok(ToolResult::text(format!(
            "Invoked '{action}' on element {id} with the given value."
        )))
    } else {
        Ok(ToolResult::text(format!(
            "Invoked '{action}' on element {id}."
        )))
    }
}

/// The action verbs that carry an operand, spelled the way a caller would write
/// them after the platform dispatchers normalize (`-` read as `_`,
/// case-insensitive). Only `set_value` today; a list because the refusal message
/// has to name what is accepted.
const VALUE_VERBS: [&str; 3] = ["set_value", "setvalue", "set_text"];

fn takes_value(action: &str) -> bool {
    let verb = action.trim().to_ascii_lowercase().replace('-', "_");
    VALUE_VERBS.contains(&verb.as_str())
}

/// Read the optional `value` argument.
///
/// A number is accepted as well as a string, because `{"value": 42}` is the
/// natural way to address a spinner or slider. An empty string is a real value:
/// clearing a field is a legitimate edit, so it must not read as absent. `null`
/// does mean absent.
fn action_value(args: &Value) -> Result<Option<String>, String> {
    let Some(raw) = args.get("value") else {
        return Ok(None);
    };
    match raw {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s.clone())),
        Value::Number(n) => Ok(Some(n.to_string())),
        other => Err(format!("value must be a string or a number, got {other}")),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// This module is accessibility only; the vision / template-matching handlers
// live in `vision.rs`.
// ═══════════════════════════════════════════════════════════════════════════
