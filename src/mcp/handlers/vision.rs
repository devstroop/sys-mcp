#![allow(dead_code)]
use crate::gui::GuiClient;
use crate::protocol::mcp::ToolResult;
use serde_json::Value;

pub(crate) async fn handle_find_image(
    _client: &GuiClient,
    _args: &Value,
) -> Result<ToolResult, String> {
    // Phase 7 — Template matching via rustautogui
    Err(
        "Template matching not yet implemented. This is a permanent error — do not retry."
            .to_string(),
    )
}

pub(crate) async fn handle_wait_for_image(
    _client: &GuiClient,
    _args: &Value,
) -> Result<ToolResult, String> {
    Err(
        "Template matching not yet implemented. This is a permanent error — do not retry."
            .to_string(),
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// Utility handlers
// ═══════════════════════════════════════════════════════════════════════════
