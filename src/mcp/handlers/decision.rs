//! `decision` tool handler — evaluate a state against typed JEV questions.
//!
//! The live HTTP path requires the `jev` cargo feature. Without it the tool
//! is still advertised (see `tools.rs`) but calls fail with a clear error.

use serde_json::Value;

use crate::gui::GuiClient;
use crate::protocol::mcp::ToolResult;

#[cfg(feature = "jev")]
pub(crate) async fn handle_decision(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    use crate::decision::client::DecisionClient;
    use crate::decision::types::parse_and_validate;

    let validated = parse_and_validate(args)?;
    let config = crate::decision::config();
    let client = DecisionClient::new(&config)?;

    let response = client
        .decide(
            validated.state,
            validated.questions,
            validated.model.as_deref(),
        )
        .await?;

    // Preserve full distributions/confidence so the calling agent can
    // branch on values instead of receiving a flattened verdict.
    let output = serde_json::json!({
        "answers": response.answers,
        "model": response
            .model
            .or_else(|| validated.model.clone())
            .unwrap_or_else(|| client.default_model().to_string()),
        "usage": response.usage,
        "redacted": validated.redacted,
    });

    Ok(ToolResult::text(
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| output.to_string()),
    ))
}

#[cfg(not(feature = "jev"))]
pub(crate) async fn handle_decision(
    _client: &GuiClient,
    _args: &Value,
) -> Result<ToolResult, String> {
    Err(
        "decision tool is not compiled in: rebuild sys-mcp with the `jev` cargo feature (--features jev)."
            .to_string(),
    )
}
