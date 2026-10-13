//! JEV (TypeSafe System One) decision layer.
//!
//! JEV is a *decision* model: you submit a `state` plus typed `questions`
//! and it returns typed answers with calibrated probabilities. It never
//! generates text. See `JEV_PLAN.md` for the full design.
//!
//! Module layout:
//! - [`types`] — serde types for the `/v1/systemone` request/response.
//! - [`redact`] — best-effort secret scrubbing applied to outbound state.
//! - [`guard`] — opt-in risk guard for destructive tools. The denylist and
//!   verdict types compile without any feature flag; only the model-backed
//!   `assess` needs `jev`.
//! - `client` — async HTTP client (requires the `jev` cargo feature).

pub mod guard;
pub mod redact;
pub mod types;

#[cfg(feature = "jev")]
pub mod client;

use std::sync::OnceLock;

pub use redact::{redact, redact_value};
pub use types::{
    parse_and_validate, Answer, Criteria, DecisionError, DecisionErrorEnvelope, DecisionRequest,
    DecisionResponse, Question, QuestionType, Usage, ValidatedDecision,
};

use crate::config::JevConfig;

static JEV_CONFIG: OnceLock<JevConfig> = OnceLock::new();

/// Install the process-wide JEV configuration.
///
/// Called once at startup from `main` with the parsed `ServerConfig`.
/// If never called, [`config`] falls back to env/CLI defaults.
pub fn init(config: JevConfig) {
    let _ = JEV_CONFIG.set(config);
}

/// The active JEV configuration: the value installed via [`init`], or the
/// clap-resolved default (honors `JEV_*` env vars).
pub fn config() -> JevConfig {
    JEV_CONFIG.get_or_init(JevConfig::default).clone()
}
