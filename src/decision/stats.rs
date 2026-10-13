//! Phase 5: per-decision observability and a cost ledger.
//!
//! Every JEV call is recorded here — tokens in and out, latency, and whether
//! it succeeded — so the operator can answer the question the plan cares
//! about: *is this saving tokens or spending them?* The `jev-cost` lesson
//! from the ecosystem is that a decision layer can be net-negative on small
//! inputs, so the ledger reports spend plainly rather than assuming a win.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;

/// One recorded JEV call.
#[derive(Debug, Clone, Serialize)]
pub struct CallRecord {
    pub tag: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub latency_ms: u64,
    pub ok: bool,
}

#[derive(Debug, Default)]
struct Ledger {
    calls: Vec<CallRecord>,
    /// Bounded so a long-running server cannot grow this without limit.
    max_records: usize,
}

/// Summary of recorded calls, ready to serialise into a tool response.
#[derive(Debug, Serialize)]
pub struct StatsSummary {
    pub total_calls: u64,
    pub failed_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub latency_ms: LatencySummary,
    pub by_tag: HashMap<String, TagSummary>,
}

#[derive(Debug, Serialize)]
pub struct LatencySummary {
    pub min: u64,
    pub p50: u64,
    pub p95: u64,
    pub max: u64,
    pub mean: u64,
}

#[derive(Debug, Default, Serialize)]
pub struct TagSummary {
    pub calls: u64,
    pub failed: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Retain at most this many records. Enough for meaningful percentiles on a
/// busy server without unbounded growth.
const MAX_RECORDS: usize = 4096;

static LEDGER: Mutex<Option<Ledger>> = Mutex::new(None);

/// Record one completed JEV call.
///
/// `ok` should be false for transport, auth, rate-limit and parse failures so
/// error pressure is visible alongside token spend.
pub fn record(
    tag: &str,
    model: &str,
    usage_input: u64,
    usage_output: u64,
    latency: Duration,
    ok: bool,
) {
    let record = CallRecord {
        tag: tag.to_string(),
        model: model.to_string(),
        input_tokens: usage_input,
        output_tokens: usage_output,
        latency_ms: latency.as_millis() as u64,
        ok,
    };

    let Ok(mut guard) = LEDGER.lock() else {
        // A poisoned lock must never take the server down over bookkeeping.
        log::warn!("decision stats ledger lock poisoned; call not recorded");
        return;
    };
    let ledger = guard.get_or_insert_with(|| Ledger {
        calls: Vec::new(),
        max_records: MAX_RECORDS,
    });
    if ledger.calls.len() >= ledger.max_records {
        // Drop the oldest quarter in one go rather than reallocating per call.
        let drop_n = ledger.max_records / 4;
        ledger.calls.drain(0..drop_n);
    }
    ledger.calls.push(record);
}

/// Summarise everything recorded so far.
pub fn summary() -> StatsSummary {
    let calls = LEDGER
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|l| l.calls.clone()))
        .unwrap_or_default();

    let mut by_tag: HashMap<String, TagSummary> = HashMap::new();
    let mut total_input = 0u64;
    let mut total_output = 0u64;
    let mut failed = 0u64;
    let mut latencies: Vec<u64> = Vec::with_capacity(calls.len());

    for c in &calls {
        total_input += c.input_tokens;
        total_output += c.output_tokens;
        latencies.push(c.latency_ms);
        if !c.ok {
            failed += 1;
        }
        let entry = by_tag.entry(c.tag.clone()).or_default();
        entry.calls += 1;
        entry.input_tokens += c.input_tokens;
        entry.output_tokens += c.output_tokens;
        if !c.ok {
            entry.failed += 1;
        }
    }

    latencies.sort_unstable();
    let latency_ms = if latencies.is_empty() {
        LatencySummary {
            min: 0,
            p50: 0,
            p95: 0,
            max: 0,
            mean: 0,
        }
    } else {
        let n = latencies.len();
        let sum: u64 = latencies.iter().sum();
        LatencySummary {
            min: latencies[0],
            p50: latencies[n * 50 / 100],
            p95: latencies[(n * 95 / 100).min(n - 1)],
            max: latencies[n - 1],
            mean: sum / n as u64,
        }
    };

    StatsSummary {
        total_calls: calls.len() as u64,
        failed_calls: failed,
        input_tokens: total_input,
        output_tokens: total_output,
        total_tokens: total_input + total_output,
        latency_ms,
        by_tag,
    }
}

/// Clear the ledger. Exposed for tests and for an operator who wants to reset
/// the window without restarting the server.
pub fn reset() {
    if let Ok(mut guard) = LEDGER.lock() {
        *guard = None;
    }
}
