//! `ui_decide` tool handler — Phase 4 bounded GUI action loop.
//!
//! Availability follows the `decision`/`detect_objects` pattern: the tool is
//! always advertised, and a call fails with an actionable message when the
//! `jev` feature is off **or** the `--jev-ui-loop` / `SYS_MCP_JEV_UI_LOOP`
//! opt-in flag is off (default). The loop drives real mouse/keyboard input,
//! so both gates must be explicitly opened before it runs.

use crate::gui::GuiClient;
use crate::protocol::mcp::ToolResult;
use serde_json::Value;

/// Feature off: same shape as `decision`'s fallback, so an agent sees one
/// consistent story about opt-in JEV capabilities.
#[cfg(not(feature = "jev"))]
pub(crate) async fn handle_ui_decide(
    _client: &GuiClient,
    _args: &Value,
) -> Result<ToolResult, String> {
    Err(
        "ui_decide is not compiled in: rebuild sys-mcp with the `jev` cargo feature (--features jev) and enable the loop with --jev-ui-loop / SYS_MCP_JEV_UI_LOOP=1."
            .to_string(),
    )
}

#[cfg(feature = "jev")]
pub(crate) async fn handle_ui_decide(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    use crate::decision::ui_loop;

    // Opt-in gate. Guardrails-style flags are config-only on purpose: an
    // agent must not be able to enable the input-driving loop for itself.
    let config = crate::decision::config();
    if !config.ui_loop {
        return Err(
            "ui_decide is disabled: the bounded action loop drives real mouse/keyboard input and is opt-in. Enable it with --jev-ui-loop or SYS_MCP_JEV_UI_LOOP=1 and restart the server."
                .to_string(),
        );
    }

    let goal = super::str_arg(args, "goal")?;
    if goal.trim().is_empty() {
        return Err("invalid argument: goal must be a non-empty string".to_string());
    }

    let values =
        match args.get("values") {
            None | Some(Value::Null) => Value::Object(Default::default()),
            Some(v @ Value::Object(_)) => v.clone(),
            Some(_) => return Err(
                "invalid argument: values must be an object mapping field names to literal strings"
                    .to_string(),
            ),
        };

    let max_steps = args
        .get("max_steps")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(ui_loop::DEFAULT_MAX_STEPS);
    let confirm = args
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let outcome = ui_loop::run_goal(client, &config, goal, &values, max_steps, confirm).await?;
    let payload = serde_json::to_value(&outcome)
        .unwrap_or_else(|_| serde_json::json!({ "status": "error", "goal": goal }));

    Ok(ToolResult::text(
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string()),
    ))
}
