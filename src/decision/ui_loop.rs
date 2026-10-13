//! Phase 4: `ui_decide` — a bounded, JEV-guided GUI action loop.
//!
//! The design law from `JEV_PLAN.md` applies harder here than anywhere else
//! in this crate, because this is the only module that *drives input* based
//! on a model's judgment:
//!
//! ```text
//! deterministic code observes → builds a BOUNDED action space → Jev picks ONE
//! → Rust policy gates (completion, irreversibility, freshness, literals)
//! → code executes → code re-observes → compact result out
//! ```
//!
//! Non-negotiables implemented here:
//! - Status vocabulary is fixed: `done | likely_done | needs_confirmation |
//!   error | stuck | ambiguous | blocked`. The loop NEVER reports `done`:
//!   completion is model judgment and is reported as `likely_done`
//!   (JEV_PLAN.md safety rule 4).
//! - Irreversible/destructive next actions are held for confirmation
//!   (`needs_confirmation`) unless the caller passed `confirm: true`.
//! - Text to type comes ONLY from the caller-supplied `values` map. The model
//!   picks *where* to type; it never generates *what* is typed (safety
//!   rule 3).
//! - Permission/elevation observation failures map to `blocked` — an
//!   inaccessible window is never driven.
//! - Step budget (default 12, hard cap 20) with no blind retry loops: if two
//!   consecutive actions leave the observed UI unchanged, the loop is `stuck`.
//! - More than [`MAX_CANDIDATES`] actionable elements is `ambiguous`, not a
//!   silent truncation.
//!
//! The JEV call goes through [`crate::decision::client::DecisionClient::decide_tag`]
//! with the tag `ui_loop`, so every token this spends is visible in the cost
//! ledger. Outbound state is redacted via
//! [`crate::decision::redact::redact_value`].

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

use crate::config::JevConfig;
use crate::decision::redact::redact_value;
use crate::decision::types::{Criteria, DecisionResponse, Question, QuestionType};
use crate::error::GuiError;
use crate::gui::types::{AccessibilityNode, MouseButton, WindowInfo};
use crate::gui::GuiClient;

// ─── Tunables ──────────────────────────────────────────────────────────────

/// Steps before the loop gives up with `stuck` when the caller does not say.
pub const DEFAULT_MAX_STEPS: usize = 12;
/// Hard ceiling on steps, regardless of what the caller asks for.
pub const MAX_STEPS_LIMIT: usize = 20;
/// More actionable elements than this is `ambiguous`, never a silent trim.
pub const MAX_CANDIDATES: usize = 40;
/// `goal_achieved` noul at or above this reports `likely_done` (never `done`).
pub const DONE_THRESHOLD: f64 = 0.85;
/// `irreversible` noul at or above this holds the action for confirmation.
pub const IRREVERSIBLE_THRESHOLD: f64 = 0.5;
/// Accessibility depth requested from the platform layer.
const TREE_DEPTH: u32 = 10;
/// Safety valve on the tree walk, independent of the platform's own limit.
const MAX_TREE_NODES: usize = 5000;
/// A target whose centre moved further than this between observe and act is
/// stale → `stuck` rather than a click at the wrong place.
const POSITION_TOLERANCE_PX: i32 = 8;
/// How long `wait` pauses before the loop re-observes.
const WAIT_MS: u64 = 500;

/// Roles that count as actionable candidates. `"text"` is the UIA Edit role
/// (see `platform/accessibility/windows.rs`'s `control_type_to_role`) and is
/// the only role eligible for `type:` actions.
const ACTIONABLE_ROLES: &[&str] = &[
    "button",
    "menu_item",
    "checkbox",
    "radio_button",
    "combobox",
    "link",
    "tab",
    "tree_item",
    "list_item",
    "text",
];

/// The fixed key vocabulary offered alongside element actions. A closed set:
/// the model may pick a key name, never invent one.
const OFFERED_KEYS: &[&str] = &["enter", "escape", "tab", "backspace"];

// ─── Public result types ───────────────────────────────────────────────────

/// The exact status vocabulary from `JEV_PLAN.md` Phase 4. Serialized
/// lowercase so tool output matches the plan strings verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UiStatus {
    /// Reserved by the vocabulary but NEVER produced by this loop: local
    /// verification of an arbitrary GUI goal is not implemented, so the
    /// model's completion judgment is always reported as `LikelyDone`.
    Done,
    LikelyDone,
    NeedsConfirmation,
    Error,
    Stuck,
    Ambiguous,
    Blocked,
}

/// One entry of the action trace returned to the caller.
#[derive(Debug, Clone, Serialize)]
pub struct TraceEntry {
    pub step: usize,
    /// The chosen option key (`click:3`, `type:0`, `key:enter`, `wait`,
    /// `stop`) — or the gate that ended the loop.
    pub action: String,
    /// Human-readable target description, when the action had one.
    pub target: Option<String>,
    pub note: String,
}

/// The final report of one `run_goal` invocation.
#[derive(Debug, Clone, Serialize)]
pub struct UiOutcome {
    pub status: UiStatus,
    pub goal: String,
    pub steps_taken: usize,
    pub actions: Vec<TraceEntry>,
    /// One sentence explaining the final status.
    pub message: String,
    /// Last `goal_achieved` noul the model returned (why it thought done).
    pub done_score: Option<f64>,
    /// Last `irreversible` noul the model returned (why an action was held).
    pub irreversible_score: Option<f64>,
}

/// One candidate action offered to the model.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// Index into the candidate list; also the `<idx>` in `click:<idx>` /
    /// `type:<idx>`.
    pub idx: usize,
    pub id: String,
    pub role: String,
    pub name: Option<String>,
    pub cx: i32,
    pub cy: i32,
    /// True when `role` is an editable text field.
    pub editable: bool,
    /// For editable fields: whether the caller supplied a literal. Exposed to
    /// the model as a flag — the literal itself is never sent.
    pub has_literal: bool,
}

impl Candidate {
    pub fn click_key(&self) -> String {
        format!("click:{}", self.idx)
    }

    pub fn type_key(&self) -> String {
        format!("type:{}", self.idx)
    }

    pub fn label(&self) -> String {
        match &self.name {
            Some(n) if !n.trim().is_empty() => format!("'{}'", n.trim()),
            _ => "(unnamed)".to_string(),
        }
    }
}

// ─── JEV seam ──────────────────────────────────────────────────────────────

/// The single model call the loop is allowed to make. A trait so tests can
/// drive every gate deterministically without a network.
#[async_trait]
pub trait UiDecider: Send + Sync {
    async fn decide(
        &self,
        state: Value,
        questions: BTreeMap<String, Question>,
    ) -> Result<DecisionResponse, String>;
}

/// Production decider: one `decide_tag` call per step, tagged `ui_loop` so
/// spend shows up in the cost ledger under its own caller.
pub struct ClientDecider {
    client: crate::decision::client::DecisionClient,
}

impl ClientDecider {
    pub fn new(config: &JevConfig) -> Result<Self, String> {
        Ok(Self {
            client: crate::decision::client::DecisionClient::new(config)?,
        })
    }
}

#[async_trait]
impl UiDecider for ClientDecider {
    async fn decide(
        &self,
        state: Value,
        questions: BTreeMap<String, Question>,
    ) -> Result<DecisionResponse, String> {
        self.client
            .decide_tag(state, questions, None, "ui_loop")
            .await
    }
}

// ─── Entry points ──────────────────────────────────────────────────────────

/// Run `goal` against the foreground window, executing up to `max_steps`
/// (clamped to `1..=`[`MAX_STEPS_LIMIT`]) JEV-guided actions.
///
/// Returns `Ok` for every loop outcome — including `error`, `stuck`,
/// `ambiguous` and `blocked` — because those are *results*, not tool
/// failures. `Err` is reserved for setup problems (bad `values` shape,
/// unusable decision client).
pub async fn run_goal(
    client: &GuiClient,
    config: &JevConfig,
    goal: &str,
    values: &Value,
    max_steps: usize,
    confirm: bool,
) -> Result<UiOutcome, String> {
    let decider = ClientDecider::new(config)?;
    run_goal_with(&decider, client, goal, values, max_steps, confirm).await
}

/// [`run_goal`] with an injectable decider (the test seam; also useful for
/// hosts that want to wrap or log the model call).
pub async fn run_goal_with(
    decider: &dyn UiDecider,
    gui: &GuiClient,
    goal: &str,
    values: &Value,
    max_steps: usize,
    confirm: bool,
) -> Result<UiOutcome, String> {
    let max_steps = max_steps.clamp(1, MAX_STEPS_LIMIT);
    let literals = parse_values(values)?;

    let mut outcome = UiOutcome {
        status: UiStatus::Error,
        goal: goal.to_string(),
        steps_taken: 0,
        actions: Vec::new(),
        message: String::new(),
        done_score: None,
        irreversible_score: None,
    };

    // Step 0: observation. A permission/elevation failure here means the
    // foreground window is not ours to drive — refuse, never retry.
    let (mut window, mut tree) = match observe(gui).await {
        Ok(v) => v,
        Err(e) => {
            let (status, msg) = classify_observe_failure(&e);
            outcome.status = status;
            outcome.message = msg;
            return Ok(outcome);
        }
    };
    let mut unchanged_rounds = 0usize;

    for step in 1..=max_steps {
        outcome.steps_taken = step;

        // ── Bounded candidate set ─────────────────────────────────────
        let candidates = match build_candidates(&tree, &literals) {
            Ok(c) => c,
            Err(actionable_count) => {
                outcome.status = UiStatus::Ambiguous;
                outcome.message = format!(
                    "The foreground window exposes {} actionable elements, over the {}-candidate bound; refusing to act on a silently truncated list. Narrow the window's scope or drive it step-by-step with accessibility_tree, mouse_click and keyboard_type.",
                    actionable_count, MAX_CANDIDATES
                );
                return Ok(outcome);
            }
        };

        // ── One JEV call, three questions ─────────────────────────────
        let state = build_state(goal, &window, &candidates, &outcome.actions);
        let (state, _redacted) = redact_value(&state);
        let questions = build_questions(goal, &candidates);
        let response = match decider.decide(state, questions).await {
            Ok(r) => r,
            Err(e) => {
                outcome.status = UiStatus::Error;
                outcome.message = format!("JEV call failed at step {step}: {e}");
                return Ok(outcome);
            }
        };

        let choice = response
            .answers
            .get("next_action")
            .and_then(|a| a.choice.clone());
        let done_score = response
            .answers
            .get("goal_achieved")
            .and_then(|a| a.noul)
            .filter(|v| v.is_finite())
            .unwrap_or(0.0);
        // Missing/absent irreversible answer is treated as 1.0: when we
        // cannot tell whether an action is safe, we hold it. For a loop that
        // drives real input, "unknown" must lose to caution.
        let irr_score = response
            .answers
            .get("irreversible")
            .and_then(|a| a.noul)
            .filter(|v| v.is_finite());
        outcome.done_score = Some(done_score);
        outcome.irreversible_score = irr_score;

        // ── Gate 1: completion (before any action, always) ────────────
        if done_score >= DONE_THRESHOLD {
            outcome.status = UiStatus::LikelyDone;
            outcome.actions.push(TraceEntry {
                step,
                action: "goal_achieved".to_string(),
                target: None,
                note: format!("model judged the goal achieved (noul={done_score:.2})"),
            });
            outcome.message = format!(
                "The decision model judged the goal achieved (noul={done_score:.2}); reported as likely_done, not done, because this is model judgment rather than a local verification."
            );
            return Ok(outcome);
        }

        // ── Parse the chosen action ───────────────────────────────────
        let choice = match choice {
            Some(c) => c,
            None => {
                outcome.status = UiStatus::Error;
                outcome.message =
                    "The JEV response contained no choice for next_action; nothing was executed."
                        .to_string();
                return Ok(outcome);
            }
        };
        let action = match parse_action(&choice, &candidates) {
            Some(a) => a,
            None => {
                outcome.status = UiStatus::Error;
                outcome.message = format!(
                    "The model returned '{choice}', which is not one of the offered options; nothing was executed."
                );
                return Ok(outcome);
            }
        };

        // ── Gate 2: irreversible → hold for confirmation ──────────────
        let held = irr_score.unwrap_or(1.0) >= IRREVERSIBLE_THRESHOLD;
        if held && !confirm {
            let (action_key, target_desc) = action.describe(&candidates);
            outcome.status = UiStatus::NeedsConfirmation;
            outcome.actions.push(TraceEntry {
                step,
                action: action_key.clone(),
                target: target_desc.clone(),
                note: "held for confirmation: model judged the next action possibly irreversible or destructive"
                    .to_string(),
            });
            outcome.message = format!(
                "Nothing was executed: the next action ({action_key}{}) was judged possibly irreversible or destructive (noul={:.2}); re-run with confirm: true to proceed.",
                target_desc
                    .map(|t| format!(", target {t}"))
                    .unwrap_or_default(),
                irr_score.unwrap_or(1.0)
            );
            return Ok(outcome);
        }

        // ── Type safety: literals come from the caller, never the model ─
        let target: Option<&Candidate> = match &action {
            NextAction::Click(i) | NextAction::Type(i) => Some(&candidates[*i]),
            _ => None,
        };
        let literal = match &action {
            NextAction::Type(i) => match resolve_literal(&literals, &candidates[*i]) {
                Some(l) => Some(l),
                None => {
                    outcome.status = UiStatus::Error;
                    outcome.actions.push(TraceEntry {
                        step,
                        action: format!("type:{i}"),
                        target: Some(candidates[*i].label()),
                        note: "aborted: no caller-supplied literal for this field".to_string(),
                    });
                    outcome.message = format!(
                        "The model chose to type into {}, but no literal was supplied for it in 'values' (keyed by element name, 'type:{i}', or element id). Text to type must come from the caller — the model never generates it. Nothing was executed.",
                        candidates[*i].label()
                    );
                    return Ok(outcome);
                }
            },
            _ => None,
        };

        // ── Gate 3: freshness — re-read the target before acting ───────
        let (fresh_window, fresh_tree) = match observe(gui).await {
            Ok(v) => v,
            Err(e) => {
                let (status, msg) = classify_observe_failure(&e);
                outcome.status = status;
                outcome.message = format!("Freshness re-check failed: {msg}");
                return Ok(outcome);
            }
        };
        if fresh_window.id != window.id {
            outcome.status = UiStatus::Stuck;
            outcome.actions.push(TraceEntry {
                step,
                action: choice.clone(),
                target: target.map(|t| t.label()),
                note: "stale target: the foreground window changed between observation and action"
                    .to_string(),
            });
            outcome.message = format!(
                "The foreground window changed between observation and action (was '{}', now '{}'); refusing to act on a stale target.",
                window.title, fresh_window.title
            );
            return Ok(outcome);
        }
        if let Some(target) = target {
            if let Some(stale) = find_stale_reason(&fresh_tree, target) {
                outcome.status = UiStatus::Stuck;
                outcome.actions.push(TraceEntry {
                    step,
                    action: choice.clone(),
                    target: Some(target.label()),
                    note: format!("stale target: {stale}"),
                });
                outcome.message = format!(
                    "The target {} changed between observation and action ({stale}); refusing to act on a stale target.",
                    target.label()
                );
                return Ok(outcome);
            }
        }
        // The fresh observation is the pre-action baseline from here on.
        window = fresh_window;
        tree = fresh_tree;
        let pre_action_fp = fingerprint(&window, &tree);

        // ── Execute ───────────────────────────────────────────────────
        let (action_key, target_desc) = action.describe(&candidates);
        match &action {
            NextAction::Click(i) => {
                let c = &candidates[*i];
                gui.click(c.cx.max(0) as u32, c.cy.max(0) as u32, MouseButton::Left)
                    .await
                    .map_err(|e| format!("click on {} failed: {e}", c.label()))?;
            }
            NextAction::Type(_) => {
                let text = literal.expect("checked above");
                let c = target.expect("Type always carries a target");
                // Typing goes to the FOCUSED element, so focus the field the
                // model picked first — otherwise the literal lands wherever
                // focus happens to be, which is exactly the kind of
                // untargeted write this loop must not do.
                gui.click(c.cx.max(0) as u32, c.cy.max(0) as u32, MouseButton::Left)
                    .await
                    .map_err(|e| format!("focusing {} failed: {e}", c.label()))?;
                gui.type_text(&text)
                    .await
                    .map_err(|e| format!("typing into {} failed: {e}", c.label()))?;
            }
            NextAction::Key(k) => {
                gui.press_key(k)
                    .await
                    .map_err(|e| format!("pressing key '{k}' failed: {e}"))?;
            }
            NextAction::Wait => {
                tokio::time::sleep(std::time::Duration::from_millis(WAIT_MS)).await;
            }
            NextAction::Stop => {
                outcome.status = UiStatus::Stuck;
                outcome.actions.push(TraceEntry {
                    step,
                    action: "stop".to_string(),
                    target: None,
                    note: "the decision model chose to stop".to_string(),
                });
                outcome.message =
                    "The decision model chose to stop rather than take another action; the goal was not reached."
                        .to_string();
                return Ok(outcome);
            }
        }
        outcome.actions.push(TraceEntry {
            step,
            action: action_key,
            target: target_desc,
            note: if held {
                "executed with caller confirmation (confirm: true)".to_string()
            } else {
                "executed".to_string()
            },
        });

        // ── Verify something changed ──────────────────────────────────
        let (new_window, new_tree) = match observe(gui).await {
            Ok(v) => v,
            Err(e) => {
                let (status, msg) = classify_observe_failure(&e);
                outcome.status = status;
                outcome.message =
                    format!("The action executed, but the follow-up observation failed: {msg}");
                return Ok(outcome);
            }
        };
        let new_fingerprint = fingerprint(&new_window, &new_tree);
        if new_fingerprint == pre_action_fp {
            unchanged_rounds += 1;
            if unchanged_rounds >= 2 {
                outcome.status = UiStatus::Stuck;
                outcome.message = "Two consecutive actions left the observed UI unchanged; the goal is not being reached and further attempts would be a blind retry loop.".to_string();
                return Ok(outcome);
            }
        } else {
            unchanged_rounds = 0;
        }
        window = new_window;
        tree = new_tree;
    }

    // ── Budget exhausted ──────────────────────────────────────────────
    outcome.status = UiStatus::Stuck;
    outcome.message = format!(
        "The step budget ({max_steps}) was exhausted without the goal being reached; the caller should re-observe and decide whether to continue with a fresh call."
    );
    Ok(outcome)
}

/// Observe-only mode: report the foreground window, the bounded candidate
/// set, and ONE JEV decision — without executing anything. Used by live
/// verification and by callers who want to audit what the loop *would* do.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewOutcome {
    pub window_title: String,
    pub process: Option<String>,
    pub candidates: Vec<PreviewCandidate>,
    pub decision: Option<PreviewDecision>,
    pub message: String,
}

/// Public view of a candidate (no internal ids beyond what callers need).
#[derive(Debug, Clone, Serialize)]
pub struct PreviewCandidate {
    pub key: String,
    pub role: String,
    pub name: Option<String>,
    pub has_literal: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewDecision {
    pub next_action: Option<String>,
    pub goal_achieved: Option<f64>,
    pub irreversible: Option<f64>,
}

/// [`run_goal`] without the executing half: observe, build candidates, ask
/// JEV once, report. No mouse or keyboard input is ever produced.
pub async fn preview_goal(
    client: &GuiClient,
    config: &JevConfig,
    goal: &str,
    values: &Value,
) -> Result<PreviewOutcome, String> {
    let decider = ClientDecider::new(config)?;
    preview_goal_with(&decider, client, goal, values).await
}

/// [`preview_goal`] with an injectable decider (test seam).
pub async fn preview_goal_with(
    decider: &dyn UiDecider,
    gui: &GuiClient,
    goal: &str,
    values: &Value,
) -> Result<PreviewOutcome, String> {
    let literals = parse_values(values)?;
    let (window, tree) = observe(gui).await.map_err(|e| {
        let (_, msg) = classify_observe_failure(&e);
        msg
    })?;

    let candidates = match build_candidates(&tree, &literals) {
        Ok(c) => c,
        Err(actionable_count) => {
            return Ok(PreviewOutcome {
                window_title: window.title.clone(),
                process: window.process_name.clone(),
                candidates: Vec::new(),
                decision: None,
                message: format!(
                    "The foreground window exposes {} actionable elements, over the {}-candidate bound; refusing to act on a silently truncated list.",
                    actionable_count, MAX_CANDIDATES
                ),
            });
        }
    };

    let preview_candidates: Vec<PreviewCandidate> = candidates
        .iter()
        .flat_map(|c| {
            let mut v = vec![PreviewCandidate {
                key: c.click_key(),
                role: c.role.clone(),
                name: c.name.clone(),
                has_literal: c.has_literal,
            }];
            if c.editable {
                v.push(PreviewCandidate {
                    key: c.type_key(),
                    role: c.role.clone(),
                    name: c.name.clone(),
                    has_literal: c.has_literal,
                });
            }
            v
        })
        .collect();

    let state = build_state(goal, &window, &candidates, &[]);
    let (state, _redacted) = redact_value(&state);
    let questions = build_questions(goal, &candidates);
    let response = decider.decide(state, questions).await?;

    let decision = PreviewDecision {
        next_action: response
            .answers
            .get("next_action")
            .and_then(|a| a.choice.clone()),
        goal_achieved: response.answers.get("goal_achieved").and_then(|a| a.noul),
        irreversible: response.answers.get("irreversible").and_then(|a| a.noul),
    };

    Ok(PreviewOutcome {
        window_title: window.title.clone(),
        process: window.process_name.clone(),
        candidates: preview_candidates,
        decision: Some(decision),
        message: "Observation and one JEV decision only: no mouse or keyboard input was produced."
            .to_string(),
    })
}

// ─── Observation ───────────────────────────────────────────────────────────

/// Observe the foreground window and its accessibility tree — nothing else.
async fn observe(gui: &GuiClient) -> Result<(WindowInfo, AccessibilityNode), GuiError> {
    let window = gui.get_active_window().await?;
    let tree = gui
        .get_accessibility_tree(Some(window.id), Some(TREE_DEPTH))
        .await?;
    Ok((window, tree))
}

/// Distinguish "the window is not ours to drive" (→ `blocked`) from ordinary
/// observation failures (→ `error`). Matches on the error text because the
/// platform layer reports elevation failures as `PlatformError` strings.
fn classify_observe_failure(e: &GuiError) -> (UiStatus, String) {
    let s = e.to_string().to_lowercase();
    let permission = s.contains("access is denied")
        || s.contains("access_denied")
        || s.contains("denied")
        || s.contains("elevat")
        || s.contains("privilege")
        || s.contains("permission")
        || s.contains("0x80070005");
    if permission {
        (
            UiStatus::Blocked,
            format!(
                "Cannot observe the foreground window ({e}); it appears to be elevated or protected. Driving it is refused — no action was taken."
            ),
        )
    } else {
        (
            UiStatus::Error,
            format!("Failed to observe the foreground window: {e}"),
        )
    }
}

/// Stable-enough comparison key for "did the UI change?". Names, roles,
/// values and positions all participate, so a click that opens a dialog or a
/// literal that fills a field both count as change.
fn fingerprint(window: &WindowInfo, tree: &AccessibilityNode) -> String {
    match serde_json::to_string(tree) {
        Ok(t) => format!("{}|{}|{t}", window.id, window.title),
        Err(_) => format!("{}|{}", window.id, window.title),
    }
}

// ─── Candidates ────────────────────────────────────────────────────────────

/// Flatten the tree into the bounded candidate set.
///
/// `Err(n)` means more than [`MAX_CANDIDATES`] actionable elements exist —
/// the count is carried so the caller can report it instead of truncating.
pub fn build_candidates(
    root: &AccessibilityNode,
    literals: &BTreeMap<String, String>,
) -> Result<Vec<Candidate>, usize> {
    let mut nodes: Vec<&AccessibilityNode> = Vec::new();
    collect_nodes(root, &mut nodes, MAX_TREE_NODES);

    let mut candidates = Vec::new();
    let mut actionable_count = 0usize;
    for node in nodes {
        if !ACTIONABLE_ROLES.contains(&node.role.as_str()) {
            continue;
        }
        if node.states.iter().any(|s| s == "disabled") {
            continue;
        }
        let (Some(cx), Some(cy)) = (node.cx, node.cy) else {
            continue; // not clickable at a known position
        };
        actionable_count += 1;
        if actionable_count > MAX_CANDIDATES {
            return Err(actionable_count);
        }
        let editable = node.role == "text";
        let idx = candidates.len();
        let mut candidate = Candidate {
            idx,
            id: node.id.clone(),
            role: node.role.clone(),
            name: node.name.clone(),
            cx,
            cy,
            editable,
            has_literal: false,
        };
        // Same lookup the executor uses, so "has_literal" as shown to the
        // model is exactly what execution will find — no drift.
        candidate.has_literal = editable && resolve_literal(literals, &candidate).is_some();
        candidates.push(candidate);
    }
    Ok(candidates)
}

fn collect_nodes<'a>(
    node: &'a AccessibilityNode,
    out: &mut Vec<&'a AccessibilityNode>,
    budget: usize,
) {
    if out.len() >= budget {
        return;
    }
    out.push(node);
    for child in &node.children {
        collect_nodes(child, out, budget);
    }
}

/// Look up the literal for a field: by element name, then `type:<idx>`, then
/// element id. Everything the loop types passes through here — there is no
/// other source of text.
fn resolve_literal(literals: &BTreeMap<String, String>, candidate: &Candidate) -> Option<String> {
    if let Some(name) = candidate
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        if let Some(v) = literals.get(name) {
            return Some(v.clone());
        }
    }
    literals
        .get(&format!("type:{}", candidate.idx))
        .or_else(|| literals.get(&candidate.id))
        .cloned()
}

/// Validate the caller's `values`: an object of string → string.
fn parse_values(values: &Value) -> Result<BTreeMap<String, String>, String> {
    match values {
        Value::Object(map) => {
            let mut out = BTreeMap::new();
            for (k, v) in map {
                match v {
                    Value::String(s) => {
                        out.insert(k.clone(), s.clone());
                    }
                    _ => {
                        return Err(format!(
                        "invalid argument: values['{k}'] must be a string (literal text to type)"
                    ))
                    }
                }
            }
            Ok(out)
        }
        Value::Null => Ok(BTreeMap::new()),
        _ => Err(
            "invalid argument: values must be an object mapping field names to literal strings"
                .to_string(),
        ),
    }
}

// ─── Actions ───────────────────────────────────────────────────────────────

/// A parsed, validated next action. Only ever constructed from the offered
/// vocabulary — an index is range-checked and a key name is allowlisted at
/// parse time.
#[derive(Debug, Clone, PartialEq)]
pub enum NextAction {
    Click(usize),
    Type(usize),
    Key(String),
    Wait,
    Stop,
}

impl NextAction {
    fn describe(&self, candidates: &[Candidate]) -> (String, Option<String>) {
        match self {
            NextAction::Click(i) => (
                format!("click:{i}"),
                Some(format!(
                    "{} at ({}, {})",
                    candidates[*i].label(),
                    candidates[*i].cx,
                    candidates[*i].cy
                )),
            ),
            NextAction::Type(i) => (
                format!("type:{i}"),
                Some(format!("type into {}", candidates[*i].label())),
            ),
            NextAction::Key(k) => (format!("key:{k}"), None),
            NextAction::Wait => ("wait".to_string(), None),
            NextAction::Stop => ("stop".to_string(), None),
        }
    }
}

/// Parse a choice string into an action. Returns `None` for anything outside
/// the offered set — including out-of-range indices, `type:` on a
/// non-editable element, and non-allowlisted key names, so a hallucinated
/// option can never reach the input layer.
pub fn parse_action(choice: &str, candidates: &[Candidate]) -> Option<NextAction> {
    if choice == "wait" {
        return Some(NextAction::Wait);
    }
    if choice == "stop" {
        return Some(NextAction::Stop);
    }
    let idx_of = |raw: &str| -> Option<usize> {
        raw.parse::<usize>().ok().filter(|i| *i < candidates.len())
    };
    if let Some(rest) = choice.strip_prefix("click:") {
        return idx_of(rest).map(NextAction::Click);
    }
    if let Some(rest) = choice.strip_prefix("type:") {
        // Only editable candidates (role "text") may receive type actions.
        return idx_of(rest)
            .filter(|i| candidates[*i].editable)
            .map(NextAction::Type);
    }
    if let Some(name) = choice.strip_prefix("key:") {
        let name = name.trim().to_ascii_lowercase();
        if OFFERED_KEYS.contains(&name.as_str()) {
            return Some(NextAction::Key(name));
        }
        return None;
    }
    None
}

// ─── State and questions for the one JEV call ─────────────────────────────

fn build_state(
    goal: &str,
    window: &WindowInfo,
    candidates: &[Candidate],
    trace: &[TraceEntry],
) -> Value {
    let candidate_rows: Vec<Value> = candidates
        .iter()
        .flat_map(|c| {
            let mut rows = vec![serde_json::json!({
                "key": c.click_key(),
                "role": c.role,
                "name": c.name,
            })];
            if c.editable {
                rows.push(serde_json::json!({
                    "key": c.type_key(),
                    "role": c.role,
                    "name": c.name,
                    "has_literal": c.has_literal,
                }));
            }
            rows
        })
        .collect();
    serde_json::json!({
        "goal": goal,
        "window": { "title": window.title, "process": window.process_name },
        "candidates": candidate_rows,
        "other_options": [
            OFFERED_KEYS.iter().map(|k| format!("key:{k}")).collect::<Vec<_>>(),
            "wait",
            "stop",
        ],
        "previous_actions": trace.iter().take(5).map(|t| serde_json::json!({
            "step": t.step, "action": t.action, "note": t.note,
        })).collect::<Vec<_>>(),
        "rules": "Pick exactly one next action. 'type:' options send a caller-supplied literal, never model-generated text. Choose 'stop' if no offered action can safely advance the goal; choose 'wait' if the UI needs time to settle.",
    })
}

fn build_questions(goal: &str, candidates: &[Candidate]) -> BTreeMap<String, Question> {
    let mut criteria: BTreeMap<String, String> = BTreeMap::new();
    for c in candidates {
        criteria.insert(
            c.click_key(),
            format!("Click the {} {} at ({}, {})", c.role, c.label(), c.cx, c.cy),
        );
        if c.editable {
            criteria.insert(
                c.type_key(),
                format!(
                    "Type a caller-supplied literal into the {} {} ({})",
                    c.role,
                    c.label(),
                    if c.has_literal {
                        "a literal is available"
                    } else {
                        "NO literal is available — typing will be refused, prefer another action"
                    }
                ),
            );
        }
    }
    for k in OFFERED_KEYS {
        criteria.insert(format!("key:{k}"), format!("Press the {k} key"));
    }
    criteria.insert(
        "wait".to_string(),
        "Wait briefly for the UI to settle before deciding again".to_string(),
    );
    criteria.insert(
        "stop".to_string(),
        "Stop: no offered action can safely advance the goal right now".to_string(),
    );

    let mut questions = BTreeMap::new();
    questions.insert(
        "next_action".to_string(),
        Question {
            question_type: QuestionType::Choice,
            instructions: format!(
                "Which single action should be taken next to achieve the goal?\n\nGoal: {goal}"
            ),
            criteria: Some(Criteria::Map(criteria)),
        },
    );
    questions.insert(
        "goal_achieved".to_string(),
        Question {
            question_type: QuestionType::Noul,
            instructions: format!(
                "Is the goal already achieved in the current UI state?\n\nGoal: {goal}"
            ),
            criteria: Some(Criteria::Map(
                [
                    (
                        "true".to_string(),
                        "The visible UI state shows the goal fully achieved".to_string(),
                    ),
                    (
                        "false".to_string(),
                        "The goal is not yet achieved; more steps are needed".to_string(),
                    ),
                ]
                .into_iter()
                .collect(),
            )),
        },
    );
    questions.insert(
        "irreversible".to_string(),
        Question {
            question_type: QuestionType::Noul,
            instructions: "Would taking the recommended next action be irreversible or destructive — deleting data, sending a message, paying, installing something, or closing work with unsaved changes?"
                .to_string(),
            criteria: Some(Criteria::Map(
                [
                    (
                        "true".to_string(),
                        "The action deletes, sends, pays, installs, or discards unsaved work"
                            .to_string(),
                    ),
                    (
                        "false".to_string(),
                        "The action is routine and reversible (navigating, focusing, typing into a scratch field)"
                            .to_string(),
                    ),
                ]
                .into_iter()
                .collect(),
            )),
        },
    );
    questions
}

// ─── Freshness ─────────────────────────────────────────────────────────────

/// Why the target no longer matches the fresh observation, if it doesn't.
/// Matching is by (role, name) because the platform's element ids are
/// pointer-derived and unstable across tree rebuilds.
fn find_stale_reason(tree: &AccessibilityNode, target: &Candidate) -> Option<String> {
    let mut nodes: Vec<&AccessibilityNode> = Vec::new();
    collect_nodes(tree, &mut nodes, MAX_TREE_NODES);
    let target_name = target.name.as_deref().map(str::trim).unwrap_or("");
    let found = nodes
        .iter()
        .find(|n| {
            n.role == target.role && n.name.as_deref().map(str::trim).unwrap_or("") == target_name
        })
        .copied();
    match found {
        None => Some("it disappeared from the accessibility tree".to_string()),
        Some(n) => {
            let moved = match (n.cx, n.cy) {
                (Some(cx), Some(cy)) => {
                    (cx - target.cx).abs() > POSITION_TOLERANCE_PX
                        || (cy - target.cy).abs() > POSITION_TOLERANCE_PX
                }
                _ => true,
            };
            if moved {
                Some(format!(
                    "it moved from ({}, {}) to ({}, {})",
                    target.cx,
                    target.cy,
                    n.cx.map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
                    n.cy.map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
                ))
            } else {
                None
            }
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(role: &str, name: Option<&str>, cx: i32, cy: i32) -> AccessibilityNode {
        AccessibilityNode {
            id: format!("uia:test:{role}:{name:?}:{cx}:{cy}"),
            role: role.to_string(),
            name: name.map(str::to_string),
            value: None,
            description: None,
            x: Some(cx),
            y: Some(cy),
            width: Some(10),
            height: Some(10),
            cx: Some(cx),
            cy: Some(cy),
            children: vec![],
            actions: vec![],
            states: vec![],
        }
    }

    #[test]
    fn max_steps_is_clamped_to_the_safe_range() {
        assert_eq!(max_steps_for(0), 1, "zero means one step, not zero");
        assert_eq!(max_steps_for(1), 1);
        assert_eq!(max_steps_for(DEFAULT_MAX_STEPS), DEFAULT_MAX_STEPS);
        assert_eq!(max_steps_for(MAX_STEPS_LIMIT), MAX_STEPS_LIMIT);
        assert_eq!(max_steps_for(9999), MAX_STEPS_LIMIT);
    }

    fn max_steps_for(n: usize) -> usize {
        n.clamp(1, MAX_STEPS_LIMIT)
    }

    #[test]
    fn parse_action_accepts_only_the_offered_vocabulary() {
        let cands = vec![
            node_candidate("button", Some("OK"), false),
            node_candidate("text", Some("Search"), true),
            node_candidate("button", Some("Cancel"), false),
        ];
        assert_eq!(parse_action("wait", &cands), Some(NextAction::Wait));
        assert_eq!(parse_action("stop", &cands), Some(NextAction::Stop));
        assert_eq!(parse_action("click:0", &cands), Some(NextAction::Click(0)));
        assert_eq!(parse_action("click:2", &cands), Some(NextAction::Click(2)));
        assert_eq!(parse_action("type:1", &cands), Some(NextAction::Type(1)));
        assert_eq!(
            parse_action("key:enter", &cands),
            Some(NextAction::Key("enter".into()))
        );
    }

    #[test]
    fn parse_action_rejects_hallucinated_options() {
        let cands = vec![
            node_candidate("button", Some("OK"), false),
            node_candidate("text", Some("Search"), true),
            node_candidate("button", Some("Cancel"), false),
        ];
        assert_eq!(parse_action("click:3", &cands), None, "out-of-range index");
        assert_eq!(parse_action("click:x", &cands), None, "non-numeric index");
        assert_eq!(parse_action("type:99", &cands), None);
        assert_eq!(
            parse_action("type:0", &cands),
            None,
            "type on a non-editable element must be refused"
        );
        assert_eq!(
            parse_action("key:ctrl+alt+delete", &cands),
            None,
            "not in the key allowlist"
        );
        assert_eq!(parse_action("key:alt+f4", &cands), None);
        assert_eq!(parse_action("type_text:hello", &cands), None);
        assert_eq!(parse_action("shut_down_the_pc", &cands), None);
        assert_eq!(parse_action("", &cands), None);
    }

    fn node_candidate(role: &str, name: Option<&str>, editable: bool) -> Candidate {
        Candidate {
            idx: 0,
            id: format!("uia:{role}:{name:?}"),
            role: role.to_string(),
            name: name.map(str::to_string),
            cx: 10,
            cy: 10,
            editable,
            has_literal: false,
        }
    }

    #[test]
    fn candidates_are_bounded_and_editable_is_role_based() {
        let mut root = node("window", Some("App"), 0, 0);
        root.children = vec![
            node("button", Some("OK"), 10, 10),
            node("text", Some("Search"), 20, 20),
            node("image", Some("logo"), 30, 30), // not actionable
            node("button", Some("Off"), 40, 40),
        ];
        root.children[3].states = vec!["disabled".to_string()];
        let literals = BTreeMap::new();
        let out = build_candidates(&root, &literals).expect("well under the cap");
        // image skipped, disabled button skipped → OK + Search only.
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].role, "button");
        assert!(!out[0].editable);
        assert_eq!(out[1].role, "text");
        assert!(out[1].editable);
        assert!(!out[1].has_literal, "no literals supplied");
    }

    #[test]
    fn candidate_cap_returns_the_count_instead_of_truncating() {
        let mut root = node("window", Some("Busy"), 0, 0);
        for i in 0..(MAX_CANDIDATES + 1) {
            root.children
                .push(node("button", Some(&format!("B{i}")), i as i32, 0));
        }
        let literals = BTreeMap::new();
        let err = build_candidates(&root, &literals).expect_err("must exceed the cap");
        assert_eq!(err, MAX_CANDIDATES + 1);
    }

    #[test]
    fn literals_resolve_by_name_then_key_then_id() {
        let mut literals = BTreeMap::new();
        literals.insert("Search".to_string(), "invoice".to_string());
        let c = Candidate {
            idx: 0,
            id: "uia:42".into(),
            role: "text".into(),
            name: Some("Search".into()),
            cx: 0,
            cy: 0,
            editable: true,
            has_literal: true,
        };
        assert_eq!(resolve_literal(&literals, &c).as_deref(), Some("invoice"));

        let unnamed = Candidate {
            name: None,
            ..c.clone()
        };
        assert_eq!(
            resolve_literal(&literals, &unnamed),
            None,
            "no name, no key, no id"
        );

        literals.clear();
        literals.insert("type:0".to_string(), "fallback".to_string());
        assert_eq!(resolve_literal(&literals, &c).as_deref(), Some("fallback"));
    }

    #[test]
    fn permission_failures_classify_as_blocked() {
        let e = GuiError::PlatformError(
            "ElementFromHandle failed: Access is denied (0x80070005)".into(),
        );
        let (status, msg) = classify_observe_failure(&e);
        assert_eq!(status, UiStatus::Blocked);
        assert!(msg.contains("no action was taken"), "got: {msg}");

        let e = GuiError::WindowError("No active window".into());
        let (status, _) = classify_observe_failure(&e);
        assert_eq!(
            status,
            UiStatus::Error,
            "ordinary failures are errors, not blocked"
        );
    }

    #[test]
    fn status_vocabulary_serializes_to_the_plan_strings() {
        for (status, expected) in [
            (UiStatus::Done, "done"),
            (UiStatus::LikelyDone, "likely_done"),
            (UiStatus::NeedsConfirmation, "needs_confirmation"),
            (UiStatus::Error, "error"),
            (UiStatus::Stuck, "stuck"),
            (UiStatus::Ambiguous, "ambiguous"),
            (UiStatus::Blocked, "blocked"),
        ] {
            assert_eq!(
                serde_json::to_value(status).unwrap(),
                json!(expected),
                "status must serialize to the exact plan string"
            );
        }
    }

    #[test]
    fn values_must_be_an_object_of_strings() {
        assert!(parse_values(&json!({})).is_ok());
        assert!(parse_values(&json!(null)).is_ok());
        assert!(parse_values(&json!("oops")).is_err());
        assert!(parse_values(&json!({"a": 1})).is_err());
        assert!(parse_values(&json!({"a": "ok", "b": "fine"})).is_ok());
    }

    #[test]
    fn stale_detection_catches_gone_and_moved_targets() {
        let target = Candidate {
            idx: 0,
            id: "uia:x".into(),
            role: "button".into(),
            name: Some("Save".into()),
            cx: 100,
            cy: 100,
            editable: false,
            has_literal: false,
        };

        let mut gone = node("window", Some("App"), 0, 0);
        gone.children = vec![node("button", Some("Cancel"), 100, 100)];
        assert!(
            find_stale_reason(&gone, &target).is_some(),
            "gone target must be stale"
        );

        let mut moved = node("window", Some("App"), 0, 0);
        moved.children = vec![node("button", Some("Save"), 500, 100)];
        let reason = find_stale_reason(&moved, &target).expect("moved target is stale");
        assert!(reason.contains("moved"), "got: {reason}");

        let mut same = node("window", Some("App"), 0, 0);
        same.children = vec![node("button", Some("Save"), 104, 97)];
        assert_eq!(find_stale_reason(&same, &target), None, "within tolerance");
    }
}
