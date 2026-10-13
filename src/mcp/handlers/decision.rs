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

// ═══════════════════════════════════════════════════════════════════════════
// Phase 3 — decision_rank: relevance-rank large text to cut context spend
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "jev")]
pub(crate) async fn handle_decision_rank(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    use crate::decision::rank::{self, DEFAULT_BATCH_SIZE, DEFAULT_BLOCK_CHARS};

    let text = super::str_arg(args, "text")?;
    let question = super::str_arg(args, "question")?;

    let top_k = args.get("top_k").and_then(Value::as_u64).unwrap_or(20) as usize;
    let max_block_chars = args
        .get("max_block_chars")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_BLOCK_CHARS as u64) as usize;
    let batch_size = args
        .get("batch_size")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_BATCH_SIZE as u64) as usize;
    let budget_chars = args
        .get("budget_chars")
        .and_then(Value::as_u64)
        .unwrap_or(8000) as usize;

    if top_k == 0 {
        return Err("invalid argument: top_k must be at least 1".to_string());
    }

    let config = crate::decision::config();
    let client = crate::decision::client::DecisionClient::new(&config)?;

    let out = rank::rank(&client, question, text, top_k, max_block_chars, batch_size).await?;
    let rendered = rank::render(&out, budget_chars);

    let payload = serde_json::json!({
        "content": rendered,
        "kept": out.kept,
        "dropped": out.dropped,
        "total_blocks": out.total,
        "skipped_small": out.skipped_small,
        "batches": out.batches,
        "failed_batches": out.failed_batches,
        "note": if out.skipped_small {
            "input was small enough that ranking would cost more than it saves; returned whole"
        } else {
            "blocks returned in original document order; dropped blocks are counted, never silently cut"
        },
    });

    Ok(ToolResult::text(
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string()),
    ))
}

#[cfg(not(feature = "jev"))]
pub(crate) async fn handle_decision_rank(
    _client: &GuiClient,
    _args: &Value,
) -> Result<ToolResult, String> {
    Err(
        "decision_rank is not compiled in: rebuild sys-mcp with the `jev` cargo feature (--features jev)."
            .to_string(),
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 5 — decision_stats: the cost ledger
// ═══════════════════════════════════════════════════════════════════════════

/// Deliberately NOT feature-gated: the ledger is empty when the `jev` feature
/// is off, and reporting "zero calls" is more honest than reporting
/// "unknown tool". It also keeps the schema stable across builds.
pub(crate) async fn handle_decision_stats(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    if args.get("reset").and_then(Value::as_bool).unwrap_or(false) {
        crate::decision::stats::reset();
    }

    let summary = crate::decision::stats::summary();
    let payload = serde_json::json!({
        "summary": summary,
        "note": "Tokens are what the JEV gateway reported. Compare 'kept' savings from \
                 decision_rank against this spend: a decision layer can be net-negative on \
                 small inputs, which is exactly what this ledger is for.",
    });

    Ok(ToolResult::text(
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string()),
    ))
}
