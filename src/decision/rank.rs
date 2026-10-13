//! Phase 3: rank large text blocks with JEV to cut host-LLM token spend.
//!
//! The problem this solves: `system_logs`, large `fs_read` results and long
//! `shell_exec` output get dumped wholesale into the calling agent's context.
//! JEV cannot summarise (it never generates text), but it *can* rate blocks
//! for relevance, so the caller gets only the blocks that matter.
//!
//! Design rules taken from `JEV_PLAN.md` and the ecosystem notes:
//! - **Selection, not rewriting.** The text returned is the input's own text,
//!   verbatim, in original document order.
//! - **Never silently cut.** Every input block appears in the output with an
//!   explicit `kept` flag and a score; dropped blocks are counted and marked,
//!   and blocks whose batch failed are *kept* rather than lost.
//! - **Skip small inputs.** Below [`FLOOR_CHARS`] the latency and token cost of
//!   a JEV call outweigh any saving, so the text is returned whole.
//! - **Redact before send.** The outbound state goes through
//!   [`crate::decision::redact::redact_value`].

use serde::{Deserialize, Serialize};

use crate::decision::redact::redact_value;
use crate::decision::types::{Criteria, Question, QuestionType};
use std::collections::BTreeMap;

/// Below this many characters, ranking cannot pay for itself.
pub const FLOOR_CHARS: usize = 8 * 1024;
/// Default characters per block. Small enough to rank granularly, large
/// enough that a block is a meaningful excerpt rather than a line fragment.
pub const DEFAULT_BLOCK_CHARS: usize = 1200;
/// Default blocks per JEV request. Keeps a batch comfortably inside the 32K
/// "state + longest question" budget while amortising request overhead.
pub const DEFAULT_BATCH_SIZE: usize = 40;

/// One input block and its ranking verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedBlock {
    /// Position in the original input, 0-based. Output preserves this order.
    pub index: usize,
    pub text: String,
    /// Relevance in `0..=1`, or `1.0` when the score is unknown (batch failed
    /// or the block was returned without being ranked).
    pub score: f64,
    /// Whether the caller should keep this block.
    pub kept: bool,
    /// Set when this block could not be ranked, so a caller can tell
    /// "irrelevant" apart from "we never found out".
    pub unranked: bool,
}

/// Result of ranking one body of text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankOutput {
    pub blocks: Vec<RankedBlock>,
    pub kept: usize,
    pub dropped: usize,
    /// Total input blocks, so `kept + dropped == total` always holds.
    pub total: usize,
    /// True when ranking was skipped because the input was small enough that
    /// a JEV call would cost more than it saves.
    pub skipped_small: bool,
    /// Batches sent to the model.
    pub batches: usize,
    /// Batches that failed. Their blocks are kept, never dropped.
    pub failed_batches: usize,
}

/// Split `text` into blocks of at most `max_chars`, breaking on line
/// boundaries so a block is never a mid-line truncation.
///
/// A single line longer than `max_chars` is split hard, since logs and minified
/// output routinely contain them.
pub fn split_blocks(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut blocks = Vec::new();
    let mut current = String::new();

    for line in text.split_inclusive('\n') {
        // Hard-split an over-long single line rather than overflowing the cap.
        let mut rest = line;
        while rest.len() > max_chars {
            let (head, tail) = rest.split_at(max_chars);
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            blocks.push(head.to_string());
            rest = tail;
        }
        if current.len() + rest.len() > max_chars && !current.is_empty() {
            blocks.push(std::mem::take(&mut current));
        }
        current.push_str(rest);
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
}

/// Rank `text` against `question`, returning the most relevant `top_k` blocks
/// in original document order.
///
/// Fails safe: if a batch request errors, its blocks are kept and counted in
/// [`RankOutput::failed_batches`] rather than being dropped.
pub async fn rank(
    client: &crate::decision::client::DecisionClient,
    question: &str,
    text: &str,
    top_k: usize,
    max_block_chars: usize,
    batch_size: usize,
) -> Result<RankOutput, String> {
    let blocks = split_blocks(text, max_block_chars);

    // Small input, or already within budget: return everything, ranked or not.
    if text.len() < FLOOR_CHARS || blocks.len() <= top_k {
        let total = blocks.len();
        let ranked = blocks
            .into_iter()
            .enumerate()
            .map(|(index, text)| RankedBlock {
                index,
                text,
                score: 1.0,
                kept: true,
                unranked: false,
            })
            .collect::<Vec<_>>();
        return Ok(RankOutput {
            blocks: ranked,
            kept: total,
            dropped: 0,
            total,
            skipped_small: true,
            batches: 0,
            failed_batches: 0,
        });
    }

    let batch_size = batch_size.max(1);
    let mut scores: Vec<Option<f64>> = vec![None; blocks.len()];
    let mut batches = 0usize;
    let mut failed_batches = 0usize;

    for (batch_idx, batch) in blocks.chunks(batch_size).enumerate() {
        batches += 1;
        let offset = batch_idx * batch_size;

        // JEV evaluates each question IN ISOLATION against the shared state.
        // That means a batch of questions asking "is this block relevant?"
        // with identical wording all receive the same answer, because no
        // question says WHICH block it is about — which silently reduces
        // ranking to noise. So each question carries its own block text, and
        // the shared state carries only the goal.
        let state = serde_json::json!({ "goal": question });
        let (state, _redacted) = redact_value(&state);

        let mut questions: BTreeMap<String, Question> = BTreeMap::new();
        for (i, block) in batch.iter().enumerate() {
            questions.insert(
                format!("b{i}"),
                Question {
                    question_type: QuestionType::Noul,
                    // Plain and short, with the verb repeated: naming the noun
                    // alone makes any mention of it look relevant.
                    instructions: format!(
                        "Does this text contain information that helps answer the goal?\n\n\
                         Goal: {question}\n\nText:\n{block}"
                    ),
                    criteria: Some(Criteria::Map(
                        [
                            (
                                "true".to_string(),
                                "The text is directly relevant to answering the goal".to_string(),
                            ),
                            (
                                "false".to_string(),
                                "The text is unrelated, redundant or merely mentions the topic"
                                    .to_string(),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    )),
                },
            );
        }

        match client.decide_tag(state, questions, None, "rank").await {
            Ok(response) => {
                for (i, _) in batch.iter().enumerate() {
                    let score = response
                        .answers
                        .get(&format!("b{i}"))
                        .and_then(|a| a.noul)
                        // A missing answer is unknown, not irrelevant: leave it
                        // unranked so it is kept rather than silently dropped.
                        .filter(|s| s.is_finite());
                    scores[offset + i] = score;
                }
            }
            Err(e) => {
                // Fail safe: a failed batch never costs the caller data.
                log::warn!(
                    "decision_rank: batch {batch_idx} failed ({e}); \
                     keeping its {} block(s) unranked",
                    batch.len()
                );
                failed_batches += 1;
            }
        }
    }

    // Rank the scored blocks, then select the top_k. Blocks with no score are
    // treated as maximally relevant so they survive — we do not drop text we
    // failed to judge.
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    order.sort_by(|&a, &b| {
        let sa = scores[a].unwrap_or(1.0);
        let sb = scores[b].unwrap_or(1.0);
        sb.partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let mut keep = vec![false; blocks.len()];
    for &i in order.iter().take(top_k) {
        keep[i] = true;
    }

    let ranked = blocks
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let score = scores[index];
            RankedBlock {
                index,
                kept: keep[index],
                score: score.unwrap_or(1.0),
                unranked: score.is_none(),
                text,
            }
        })
        .collect::<Vec<_>>();

    let kept = ranked.iter().filter(|b| b.kept).count();
    Ok(RankOutput {
        total: ranked.len(),
        kept,
        dropped: ranked.len() - kept,
        skipped_small: false,
        batches,
        failed_batches,
        blocks: ranked,
    })
}

/// Convenience: render a [`RankOutput`] as the text a caller should forward,
/// with dropped blocks marked in place so nothing vanishes silently.
pub fn render(out: &RankOutput, budget_chars: usize) -> String {
    let mut kept_chars = 0usize;
    let mut shown = Vec::new();
    let mut truncated = false;

    for b in out.blocks.iter().filter(|b| b.kept) {
        if kept_chars + b.text.len() > budget_chars {
            truncated = true;
            break;
        }
        kept_chars += b.text.len();
        shown.push(b.text.clone());
    }

    let mut result = shown.join("");
    if truncated {
        result.push_str(&format!(
            "\n[… content truncated at the {budget_chars}-char budget; {} kept block(s) omitted …]\n",
            out.blocks.iter().filter(|b| b.kept).count() - shown.len()
        ));
    }
    if out.dropped > 0 {
        result.push_str(&format!(
            "\n[… {} of {} blocks dropped as irrelevant; pass a larger top_k or budget to keep more …]\n",
            out.dropped, out.total
        ));
    }
    if out.failed_batches > 0 {
        result.push_str(&format!(
            "\n[… {} batch(es) could not be ranked and were kept unranked …]\n",
            out.failed_batches
        ));
    }
    result
}
