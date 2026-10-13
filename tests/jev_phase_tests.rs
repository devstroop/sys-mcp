//! Tests for JEV plan Phases 3 (ranking) and 5 (cost ledger).
//!
//! These are offline: `split_blocks`/`render` are pure, `stats` is pure
//! bookkeeping, and `rank`'s small-input path returns before any model call,
//! so the whole file runs without network or an API key.
//!
//! Gated on the `jev` feature because `decision::rank` and `decision::client`
//! only exist with it (Phase 5's `stats` ledger compiles everywhere, but it
//! is exercised here alongside the ranking it exists to audit).

#![cfg(feature = "jev")]

use std::time::Duration;

use sys_mcp::decision::rank::{
    render, split_blocks, RankOutput, RankedBlock, DEFAULT_BLOCK_CHARS, FLOOR_CHARS,
};
use sys_mcp::decision::stats;

// ─── Phase 3: block splitting ──────────────────────────────────────────────

#[test]
fn split_preserves_all_content() {
    // The one invariant that must never break: splitting and rejoining is
    // lossless. Losing log lines would make ranking worse than useless.
    let text = "line one\nline two\nline three\n";
    let blocks = split_blocks(text, 12);
    assert_eq!(
        blocks.concat(),
        text,
        "splitting must not lose or add bytes"
    );
}

#[test]
fn split_respects_the_char_cap_on_normal_input() {
    let text = "aaa\nbbb\nccc\nddd\neee\n";
    for b in split_blocks(text, 6) {
        assert!(b.len() <= 6, "block exceeds cap: {b:?}");
    }
}

#[test]
fn split_hard_splits_an_over_long_line() {
    // Logs and minified output routinely contain single enormous lines; the
    // cap must hold rather than being blown past.
    let long = "x".repeat(500);
    let blocks = split_blocks(&long, 100);
    assert!(blocks.len() >= 5, "expected hard splits, got {blocks:?}");
    assert_eq!(blocks.concat(), long, "hard split must be lossless");
    assert!(blocks.iter().all(|b| b.len() <= 100));
}

#[test]
fn split_handles_empty_input() {
    assert!(split_blocks("", 100).is_empty());
}

// ─── Phase 3: ranking without a model ──────────────────────────────────────

fn sample_blocks(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("block {i} with some filler text\n"))
        .collect()
}

#[test]
fn small_input_is_returned_whole_without_a_model_call() {
    // The floor exists because ranking a small input costs more than it saves.
    let text = sample_blocks(3).concat();
    assert!(text.len() < FLOOR_CHARS);

    // A client pointing at a dead port proves no request was attempted: if
    // one were, this would error rather than succeed.
    let cfg = sys_mcp::config::JevConfig {
        endpoint: "http://127.0.0.1:9/v1/systemone".to_string(),
        api_key: "unused".to_string(),
        ..sys_mcp::config::JevConfig::default()
    };
    let client = sys_mcp::decision::client::DecisionClient::new(&cfg).expect("client");

    let out = tokio_test::block_on(sys_mcp::decision::rank::rank(
        &client,
        "anything",
        &text,
        2,
        DEFAULT_BLOCK_CHARS,
        40,
    ))
    .expect("small input must not require the model");

    assert!(out.skipped_small);
    assert_eq!(out.batches, 0, "no model call should have been made");
    assert_eq!(out.kept, out.total);
    assert_eq!(out.dropped, 0);
    // Every block the splitter produced comes back, kept — nothing is lost by
    // skipping the model call.
    let expected = split_blocks(&text, DEFAULT_BLOCK_CHARS).len();
    assert_eq!(out.blocks.len(), expected);
    assert!(out.blocks.iter().all(|b| b.kept));
    let rejoined: String = out.blocks.iter().map(|b| b.text.as_str()).collect();
    assert_eq!(rejoined, text, "skipping must still be lossless");
}

#[test]
fn render_marks_dropped_blocks_and_never_silently_cuts() {
    let out = RankOutput {
        total: 3,
        kept: 1,
        dropped: 2,
        skipped_small: false,
        batches: 1,
        failed_batches: 0,
        blocks: vec![
            RankedBlock {
                index: 0,
                text: "keep me".into(),
                score: 0.9,
                kept: true,
                unranked: false,
            },
            RankedBlock {
                index: 1,
                text: "drop me".into(),
                score: 0.1,
                kept: false,
                unranked: false,
            },
            RankedBlock {
                index: 2,
                text: "drop me too".into(),
                score: 0.05,
                kept: false,
                unranked: false,
            },
        ],
    };

    let rendered = render(&out, 10_000);
    assert!(rendered.contains("keep me"));
    assert!(
        !rendered.contains("drop me"),
        "dropped text must not leak through"
    );
    // …but the caller must be told something was dropped.
    assert!(
        rendered.contains("2 of 3 blocks dropped"),
        "got: {rendered}"
    );
}

#[test]
fn render_reports_failed_batches_so_unknown_is_not_confused_with_irrelevant() {
    let out = RankOutput {
        total: 2,
        kept: 2,
        dropped: 0,
        skipped_small: false,
        batches: 2,
        failed_batches: 1,
        blocks: vec![
            RankedBlock {
                index: 0,
                text: "a".into(),
                score: 1.0,
                kept: true,
                unranked: true,
            },
            RankedBlock {
                index: 1,
                text: "b".into(),
                score: 0.4,
                kept: true,
                unranked: false,
            },
        ],
    };
    let rendered = render(&out, 10_000);
    assert!(rendered.contains("could not be ranked"), "got: {rendered}");
}

#[test]
fn render_enforces_the_char_budget_and_says_so() {
    let blocks: Vec<RankedBlock> = (0..10)
        .map(|i| RankedBlock {
            index: i,
            text: "x".repeat(100),
            score: 1.0,
            kept: true,
            unranked: false,
        })
        .collect();
    let out = RankOutput {
        total: 10,
        kept: 10,
        dropped: 0,
        skipped_small: false,
        batches: 1,
        failed_batches: 0,
        blocks,
    };
    let rendered = render(&out, 250);
    assert!(rendered.contains("truncated"), "got: {rendered}");
}

// ─── Phase 5: cost ledger ──────────────────────────────────────────────────

// One test, not three: the ledger is process-global and cargo runs tests in
// parallel, so separate functions calling reset() would race and flake.
#[test]
fn ledger_lifecycle_accumulates_reports_and_resets() {
    stats::reset();

    // Empty ledger reports zeroes rather than dividing by zero.
    let empty = stats::summary();
    assert_eq!(empty.total_calls, 0);
    assert_eq!(empty.latency_ms.p50, 0);
    assert_eq!(empty.by_tag.len(), 0);

    stats::record(
        "decision",
        "jev-1.13-free",
        100,
        10,
        Duration::from_millis(10),
        true,
    );
    stats::record(
        "rank",
        "jev-1.13-free",
        500,
        50,
        Duration::from_millis(100),
        true,
    );
    stats::record(
        "guard",
        "jev-1.13-free",
        50,
        5,
        Duration::from_millis(200),
        false,
    );

    let s = stats::summary();
    assert_eq!(s.total_calls, 3);
    assert_eq!(s.failed_calls, 1);
    assert_eq!(s.input_tokens, 650);
    assert_eq!(s.output_tokens, 65);
    assert_eq!(s.total_tokens, 715);
    assert_eq!(s.latency_ms.min, 10);
    assert_eq!(s.latency_ms.max, 200);

    // Failures are visible per tag, not only in the total.
    assert_eq!(s.by_tag["guard"].calls, 1);
    assert_eq!(s.by_tag["guard"].failed, 1);
    assert_eq!(s.by_tag["rank"].input_tokens, 500);

    stats::reset();
    assert_eq!(stats::summary().total_calls, 0);
}
