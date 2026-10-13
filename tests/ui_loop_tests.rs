//! Tests for Phase 4 — the `ui_decide` bounded action loop.
//!
//! Everything here is offline: a scripted GUI backend drives the observation
//! half and a scripted decider drives the judgment half, so every safety gate
//! is exercised deterministically without a network, an API key, or real
//! mouse/keyboard input. The `StubBackend`-based availability tests live at
//! the bottom and run regardless of feature flags.

#![cfg(feature = "jev")]

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::{json, Value};
use sys_mcp::config::JevConfig;
use sys_mcp::decision::ui_loop::{
    preview_goal_with, run_goal_with, UiDecider, UiStatus, MAX_CANDIDATES, MAX_STEPS_LIMIT,
};
use sys_mcp::decision::{Answer, DecisionResponse, Question, QuestionType};
use sys_mcp::error::GuiError;
use sys_mcp::gui::accessibility::AccessibilityCapability;
use sys_mcp::gui::backend::GuiBackend;
use sys_mcp::gui::display::DisplayCapability;
use sys_mcp::gui::input::InputCapability;
use sys_mcp::gui::types::{
    AccessibilityNode, AccessibilityQuery, MonitorInfo, MouseButton, Point, Region, Resolution,
    Screenshot, ScrollDirection, WindowInfo,
};
use sys_mcp::gui::window::WindowCapability;
use sys_mcp::gui::GuiClient;

// ─── Scripted GUI backend ──────────────────────────────────────────────────

/// A GUI backend with a scripted accessibility tree and recorded inputs.
///
/// Each observation pops the next tree from the queue; when the queue is
/// empty the last tree repeats (a stable UI). Recording every click, typed
/// string and key press lets tests assert that held/refused actions never
/// reached the input layer at all.
struct ScriptedBackend {
    window: WindowInfo,
    trees: Mutex<VecDeque<AccessibilityNode>>,
    observe_error: Mutex<Option<GuiError>>,
    clicks: Mutex<Vec<(u32, u32)>>,
    typed: Mutex<Vec<String>>,
    keys: Mutex<Vec<String>>,
    /// When true, every observation gets a fresh non-actionable status node
    /// so the loop's "did anything change?" fingerprint differs between
    /// steps. Without this, a static scripted UI makes the unchanged-guard
    /// stop multi-step scenarios after two actions.
    distinct_observations: bool,
    epoch: AtomicUsize,
}

impl ScriptedBackend {
    fn new(window_title: &str, trees: Vec<AccessibilityNode>) -> Self {
        Self {
            window: WindowInfo {
                id: 42,
                title: window_title.to_string(),
                x: 0,
                y: 0,
                width: 800,
                height: 600,
                is_minimized: false,
                is_maximized: false,
                is_focused: true,
                process_name: Some("scripted.exe".to_string()),
                process_id: Some(1),
            },
            trees: Mutex::new(trees.into()),
            observe_error: Mutex::new(None),
            clicks: Mutex::new(Vec::new()),
            typed: Mutex::new(Vec::new()),
            keys: Mutex::new(Vec::new()),
            distinct_observations: false,
            epoch: AtomicUsize::new(0),
        }
    }

    /// Variant whose observations differ step-to-step (see field docs).
    fn new_changing(window_title: &str, trees: Vec<AccessibilityNode>) -> Self {
        let mut b = Self::new(window_title, trees);
        b.distinct_observations = true;
        b
    }

    /// A simple window with `n` buttons ("B0".."Bn-1") at distinct positions.
    fn with_buttons(n: usize) -> Self {
        Self::new("Scripted", vec![button_tree(n)])
    }

    fn fail_next_observe_with(&self, err: GuiError) {
        *self.observe_error.lock().unwrap() = Some(err);
    }

    fn typed_text(&self) -> Vec<String> {
        self.typed.lock().unwrap().clone()
    }

    fn clicks(&self) -> Vec<(u32, u32)> {
        self.clicks.lock().unwrap().clone()
    }

    fn next_tree(&self) -> AccessibilityNode {
        let mut q = self.trees.lock().unwrap();
        let mut tree = if q.len() > 1 {
            q.pop_front().expect("len checked")
        } else {
            q.back().cloned().unwrap_or_else(|| button_tree(1))
        };
        drop(q);
        if self.distinct_observations {
            // A status_bar is not an actionable role, so candidates are
            // unaffected, but the fingerprint sees the change.
            let tick = self.epoch.fetch_add(1, Ordering::SeqCst);
            tree.children.push(AccessibilityNode {
                id: format!("uia:tick{tick}"),
                role: "status_bar".to_string(),
                name: Some(format!("tick-{tick}")),
                value: None,
                description: None,
                x: Some(0),
                y: Some(580),
                width: Some(800),
                height: Some(20),
                cx: Some(400),
                cy: Some(590),
                children: vec![],
                actions: vec![],
                states: vec![],
            });
        }
        tree
    }
}

fn button_tree(n: usize) -> AccessibilityNode {
    AccessibilityNode {
        id: "uia:root".to_string(),
        role: "window".to_string(),
        name: Some("Scripted".to_string()),
        value: None,
        description: None,
        x: Some(0),
        y: Some(0),
        width: Some(800),
        height: Some(600),
        cx: Some(400),
        cy: Some(300),
        children: (0..n)
            .map(|i| AccessibilityNode {
                id: format!("uia:btn{i}"),
                role: "button".to_string(),
                name: Some(format!("B{i}")),
                value: None,
                description: None,
                x: Some(10 + i as i32 * 20),
                y: Some(10),
                width: Some(16),
                height: Some(16),
                cx: Some(18 + i as i32 * 20),
                cy: Some(18),
                children: vec![],
                actions: vec!["invoke".to_string()],
                states: vec![],
            })
            .collect(),
        actions: vec![],
        states: vec![],
    }
}

/// A tree with one button and one editable text field.
fn form_tree() -> AccessibilityNode {
    let mut root = button_tree(1);
    root.children.push(AccessibilityNode {
        id: "uia:field0".to_string(),
        role: "text".to_string(),
        name: Some("Search".to_string()),
        value: None,
        description: None,
        x: Some(100),
        y: Some(10),
        width: Some(120),
        height: Some(16),
        cx: Some(160),
        cy: Some(18),
        children: vec![],
        actions: vec![],
        states: vec![],
    });
    root
}

#[async_trait]
impl DisplayCapability for ScriptedBackend {
    async fn screenshot(&self) -> Result<Screenshot, GuiError> {
        Ok(Screenshot {
            data: vec![],
            width: 800,
            height: 600,
        })
    }
    async fn screenshot_region(&self, _region: Region) -> Result<Screenshot, GuiError> {
        Ok(Screenshot {
            data: vec![],
            width: 100,
            height: 100,
        })
    }
    async fn get_screen_size(&self) -> Result<Resolution, GuiError> {
        Ok(Resolution {
            width: 800,
            height: 600,
        })
    }
    async fn list_monitors(&self) -> Result<Vec<MonitorInfo>, GuiError> {
        Ok(vec![])
    }
}

#[async_trait]
impl InputCapability for ScriptedBackend {
    async fn mouse_click(&self, x: u32, y: u32, _button: MouseButton) -> Result<(), GuiError> {
        self.clicks.lock().unwrap().push((x, y));
        Ok(())
    }
    async fn mouse_double_click(
        &self,
        _x: u32,
        _y: u32,
        _button: MouseButton,
    ) -> Result<(), GuiError> {
        Ok(())
    }
    async fn mouse_move(&self, _x: u32, _y: u32) -> Result<(), GuiError> {
        Ok(())
    }
    async fn mouse_position(&self) -> Result<Point, GuiError> {
        Ok(Point { x: 0, y: 0 })
    }
    async fn mouse_drag(
        &self,
        _from: Point,
        _to: Point,
        _button: MouseButton,
    ) -> Result<(), GuiError> {
        Ok(())
    }
    async fn mouse_scroll(
        &self,
        _x: u32,
        _y: u32,
        _direction: ScrollDirection,
        _amount: i32,
    ) -> Result<(), GuiError> {
        Ok(())
    }
    async fn type_text(&self, text: &str) -> Result<(), GuiError> {
        self.typed.lock().unwrap().push(text.to_string());
        Ok(())
    }
    async fn key_press(&self, key: &str) -> Result<(), GuiError> {
        self.keys.lock().unwrap().push(key.to_string());
        Ok(())
    }
    async fn key_down(&self, _key: &str) -> Result<(), GuiError> {
        Ok(())
    }
    async fn key_up(&self, _key: &str) -> Result<(), GuiError> {
        Ok(())
    }
    async fn key_combo(&self, _keys: &[String]) -> Result<(), GuiError> {
        Ok(())
    }
}

#[async_trait]
impl WindowCapability for ScriptedBackend {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        Ok(vec![self.window.clone()])
    }
    async fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        if let Some(err) = self.observe_error.lock().unwrap().take() {
            return Err(err);
        }
        Ok(self.window.clone())
    }
    async fn focus_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }
    async fn move_window(&self, _window_id: u64, _x: i32, _y: i32) -> Result<(), GuiError> {
        Ok(())
    }
    async fn resize_window(
        &self,
        _window_id: u64,
        _width: u32,
        _height: u32,
    ) -> Result<(), GuiError> {
        Ok(())
    }
    async fn minimize_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }
    async fn maximize_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }
    async fn restore_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }
    async fn close_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }
    async fn get_window_title(&self, _window_id: u64) -> Result<String, GuiError> {
        Ok(self.window.title.clone())
    }
    async fn get_window_bounds(&self, _window_id: u64) -> Result<Region, GuiError> {
        Ok(Region {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        })
    }
    async fn find_windows_by_title(&self, _query: &str) -> Result<Vec<WindowInfo>, GuiError> {
        Ok(vec![self.window.clone()])
    }
}

#[async_trait]
impl AccessibilityCapability for ScriptedBackend {
    async fn get_tree(
        &self,
        _window_id: Option<u64>,
        _max_depth: Option<u32>,
    ) -> Result<AccessibilityNode, GuiError> {
        if let Some(err) = self.observe_error.lock().unwrap().take() {
            return Err(err);
        }
        Ok(self.next_tree())
    }
    async fn find_elements(
        &self,
        _query: AccessibilityQuery,
    ) -> Result<Vec<AccessibilityNode>, GuiError> {
        Ok(vec![])
    }
    async fn get_element_properties(
        &self,
        _element_id: &str,
    ) -> Result<AccessibilityNode, GuiError> {
        Err(GuiError::UnsupportedCapability("test".to_string()))
    }
    async fn invoke_element_action(
        &self,
        _element_id: &str,
        _action: &str,
    ) -> Result<(), GuiError> {
        Ok(())
    }
}

impl GuiBackend for ScriptedBackend {
    fn as_window(&self) -> Option<&dyn WindowCapability> {
        Some(self)
    }
    fn as_accessibility(&self) -> Option<&dyn AccessibilityCapability> {
        Some(self)
    }
}

// ─── Scripted decider ──────────────────────────────────────────────────────

/// Canned JEV answers, one `DecisionResponse` per call.
struct ScriptedDecider {
    responses: Mutex<VecDeque<DecisionResponse>>,
    calls: AtomicUsize,
}

impl ScriptedDecider {
    fn new(responses: Vec<DecisionResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl UiDecider for ScriptedDecider {
    async fn decide(
        &self,
        _state: Value,
        _questions: std::collections::BTreeMap<String, Question>,
    ) -> Result<DecisionResponse, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| "scripted decider exhausted".to_string())
    }
}

/// Build a `DecisionResponse` from the three canned answers.
fn response(choice: &str, done: f64, irreversible: f64) -> DecisionResponse {
    let mut answers = HashMap::new();
    answers.insert(
        "next_action".to_string(),
        Answer {
            answer_type: QuestionType::Choice,
            noul: None,
            choice: Some(choice.to_string()),
            score: None,
            probabilities: None,
            confidence: None,
            legend: None,
        },
    );
    answers.insert(
        "goal_achieved".to_string(),
        Answer {
            answer_type: QuestionType::Noul,
            noul: Some(done),
            choice: None,
            score: None,
            probabilities: None,
            confidence: None,
            legend: None,
        },
    );
    answers.insert(
        "irreversible".to_string(),
        Answer {
            answer_type: QuestionType::Noul,
            noul: Some(irreversible),
            choice: None,
            score: None,
            probabilities: None,
            confidence: None,
            legend: None,
        },
    );
    DecisionResponse {
        answers,
        model: Some("scripted".to_string()),
        usage: None,
    }
}

// GuiClient::new takes ownership, but tests need to inspect the backend
// afterwards. A thin proxy that borrows the same backend via a leaked box
// would overcomplicate this; instead the proxy forwards to a shared clone
// handle. Since ScriptedBackend is only used through Arc in practice, wrap
// it: the proxy holds a raw pointer that stays valid because the test owns
// the backend for the whole run and ScriptedBackend is not moved after.
struct ScriptedBackendProxy(&'static ScriptedBackend);

#[async_trait]
impl DisplayCapability for ScriptedBackendProxy {
    async fn screenshot(&self) -> Result<Screenshot, GuiError> {
        self.0.screenshot().await
    }
    async fn screenshot_region(&self, region: Region) -> Result<Screenshot, GuiError> {
        self.0.screenshot_region(region).await
    }
    async fn get_screen_size(&self) -> Result<Resolution, GuiError> {
        self.0.get_screen_size().await
    }
    async fn list_monitors(&self) -> Result<Vec<MonitorInfo>, GuiError> {
        self.0.list_monitors().await
    }
}

#[async_trait]
impl InputCapability for ScriptedBackendProxy {
    async fn mouse_click(&self, x: u32, y: u32, b: MouseButton) -> Result<(), GuiError> {
        self.0.mouse_click(x, y, b).await
    }
    async fn mouse_double_click(&self, x: u32, y: u32, b: MouseButton) -> Result<(), GuiError> {
        self.0.mouse_double_click(x, y, b).await
    }
    async fn mouse_move(&self, x: u32, y: u32) -> Result<(), GuiError> {
        self.0.mouse_move(x, y).await
    }
    async fn mouse_position(&self) -> Result<Point, GuiError> {
        self.0.mouse_position().await
    }
    async fn mouse_drag(&self, f: Point, t: Point, b: MouseButton) -> Result<(), GuiError> {
        self.0.mouse_drag(f, t, b).await
    }
    async fn mouse_scroll(
        &self,
        x: u32,
        y: u32,
        d: ScrollDirection,
        a: i32,
    ) -> Result<(), GuiError> {
        self.0.mouse_scroll(x, y, d, a).await
    }
    async fn type_text(&self, text: &str) -> Result<(), GuiError> {
        self.0.type_text(text).await
    }
    async fn key_press(&self, key: &str) -> Result<(), GuiError> {
        self.0.key_press(key).await
    }
    async fn key_down(&self, key: &str) -> Result<(), GuiError> {
        self.0.key_down(key).await
    }
    async fn key_up(&self, key: &str) -> Result<(), GuiError> {
        self.0.key_up(key).await
    }
    async fn key_combo(&self, keys: &[String]) -> Result<(), GuiError> {
        self.0.key_combo(keys).await
    }
}

#[async_trait]
impl WindowCapability for ScriptedBackendProxy {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        self.0.list_windows().await
    }
    async fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        self.0.get_active_window().await
    }
    async fn focus_window(&self, id: u64) -> Result<(), GuiError> {
        self.0.focus_window(id).await
    }
    async fn move_window(&self, id: u64, x: i32, y: i32) -> Result<(), GuiError> {
        self.0.move_window(id, x, y).await
    }
    async fn resize_window(&self, id: u64, w: u32, h: u32) -> Result<(), GuiError> {
        self.0.resize_window(id, w, h).await
    }
    async fn minimize_window(&self, id: u64) -> Result<(), GuiError> {
        self.0.minimize_window(id).await
    }
    async fn maximize_window(&self, id: u64) -> Result<(), GuiError> {
        self.0.maximize_window(id).await
    }
    async fn restore_window(&self, id: u64) -> Result<(), GuiError> {
        self.0.restore_window(id).await
    }
    async fn close_window(&self, id: u64) -> Result<(), GuiError> {
        self.0.close_window(id).await
    }
    async fn get_window_title(&self, id: u64) -> Result<String, GuiError> {
        self.0.get_window_title(id).await
    }
    async fn get_window_bounds(&self, id: u64) -> Result<Region, GuiError> {
        self.0.get_window_bounds(id).await
    }
    async fn find_windows_by_title(&self, q: &str) -> Result<Vec<WindowInfo>, GuiError> {
        self.0.find_windows_by_title(q).await
    }
}

#[async_trait]
impl AccessibilityCapability for ScriptedBackendProxy {
    async fn get_tree(
        &self,
        window_id: Option<u64>,
        max_depth: Option<u32>,
    ) -> Result<AccessibilityNode, GuiError> {
        self.0.get_tree(window_id, max_depth).await
    }
    async fn find_elements(
        &self,
        query: AccessibilityQuery,
    ) -> Result<Vec<AccessibilityNode>, GuiError> {
        self.0.find_elements(query).await
    }
    async fn get_element_properties(&self, id: &str) -> Result<AccessibilityNode, GuiError> {
        self.0.get_element_properties(id).await
    }
    async fn invoke_element_action(&self, id: &str, action: &str) -> Result<(), GuiError> {
        self.0.invoke_element_action(id, action).await
    }
}

impl GuiBackend for ScriptedBackendProxy {
    fn as_window(&self) -> Option<&dyn WindowCapability> {
        Some(self)
    }
    fn as_accessibility(&self) -> Option<&dyn AccessibilityCapability> {
        Some(self)
    }
}

/// Leak the backend so the loop can own it through `GuiClient` while the
/// test keeps a handle for post-run assertions. `Box::leak` is fine here:
/// these are per-test allocations in a short-lived test binary.
fn lease(backend: ScriptedBackend) -> (&'static ScriptedBackend, GuiClient) {
    let leased: &'static ScriptedBackend = Box::leak(Box::new(backend));
    (leased, GuiClient::new(ScriptedBackendProxy(leased)))
}

// ─── Loop behaviour ────────────────────────────────────────────────────────

#[test]
fn irreversible_action_is_held_without_confirm_and_never_reaches_input() {
    let (backend, proxy) = lease(ScriptedBackend::with_buttons(2));
    let decider = ScriptedDecider::new(vec![response("click:0", 0.1, 0.9)]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Delete the file",
        &json!({}),
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::NeedsConfirmation);
    assert!(out.irreversible_score.unwrap() >= 0.5);
    assert!(
        backend.clicks().is_empty(),
        "held action must not reach the input layer"
    );
    assert!(
        out.message.contains("confirm"),
        "message must say how to proceed: {}",
        out.message
    );
    assert!(
        out.message.contains("Nothing was executed"),
        "got: {}",
        out.message
    );
}

#[test]
fn irreversible_action_executes_with_confirm_true() {
    let (backend, proxy) = lease(ScriptedBackend::new_changing(
        "Scripted",
        vec![button_tree(2)],
    ));
    let decider = ScriptedDecider::new(vec![
        response("click:0", 0.1, 0.9), // held unless confirm
        response("stop", 0.99, 0.0),   // done gate trips before 'stop' is parsed
    ]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Delete the file",
        &json!({}),
        5,
        true, // caller confirmed
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::LikelyDone, "message: {}", out.message);
    assert_eq!(backend.clicks(), vec![(18, 18)], "one confirmed click");
    assert!(out.actions.iter().any(|a| a.note.contains("confirm")));
}

#[test]
fn type_action_without_a_literal_is_refused_and_nothing_is_typed() {
    let (backend, proxy) = lease(ScriptedBackend::new("Scripted", vec![form_tree()]));
    let decider = ScriptedDecider::new(vec![response("type:1", 0.1, 0.0)]); // field is candidate 1

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Search for invoices",
        &json!({}), // no literals supplied
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::Error);
    assert!(
        out.message.contains("literal"),
        "message must explain the literal requirement: {}",
        out.message
    );
    assert!(
        backend.typed_text().is_empty(),
        "the model must never cause generated text to be typed"
    );
    assert!(backend.clicks().is_empty(), "not even a focus click");
}

#[test]
fn type_action_types_exactly_the_caller_literal() {
    let (backend, proxy) = lease(ScriptedBackend::new_changing("Scripted", vec![form_tree()]));
    let decider = ScriptedDecider::new(vec![
        response("type:1", 0.1, 0.0),
        response("stop", 0.99, 0.0), // done gate trips on step 2
    ]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Search for invoices",
        &json!({ "Search": "invoice 42" }), // literal keyed by element name
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::LikelyDone, "message: {}", out.message);
    assert_eq!(
        backend.typed_text(),
        vec!["invoice 42"],
        "only the caller's literal is ever typed, verbatim"
    );
    // The field was focused by a click at its centre (160, 18) before typing.
    assert!(backend.clicks().contains(&(160, 18)));
}

#[test]
fn candidate_cap_reports_ambiguous_and_calls_the_model_zero_times() {
    // MAX_CANDIDATES + 1 actionable buttons — over the bound.
    let (backend, proxy) = lease(ScriptedBackend::with_buttons(MAX_CANDIDATES + 1));
    let decider = ScriptedDecider::new(vec![response("click:0", 0.0, 0.0)]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Do anything",
        &json!({}),
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::Ambiguous);
    assert_eq!(decider.calls(), 0, "no model call when refusing to act");
    assert!(
        out.message.contains(&format!("{}", MAX_CANDIDATES + 1)),
        "message must state the real count: {}",
        out.message
    );
    assert!(backend.clicks().is_empty());
}

#[test]
fn exactly_at_the_cap_is_not_ambiguous() {
    let (backend, proxy) = lease(ScriptedBackend::with_buttons(MAX_CANDIDATES));
    let decider = ScriptedDecider::new(vec![response("goal_achieved", 0.99, 0.0)]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Do anything",
        &json!({}),
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::LikelyDone, "message: {}", out.message);
    assert_eq!(decider.calls(), 1);
    let _ = backend;
}

#[test]
fn completion_is_likely_done_never_done() {
    let (backend, proxy) = lease(ScriptedBackend::with_buttons(2));
    let decider = ScriptedDecider::new(vec![response("click:0", 0.95, 0.0)]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Click B0",
        &json!({}),
        5,
        false,
    ))
    .unwrap();

    assert_eq!(
        out.status,
        UiStatus::LikelyDone,
        "model judgment must be likely_done"
    );
    assert!(backend.clicks().is_empty(), "completion check runs first");
    assert_eq!(out.done_score, Some(0.95), "score reported for auditing");
}

#[test]
fn max_steps_is_clamped_to_twenty() {
    // 100 requested, but every step does something observable (key press)
    // and each observation differs, so the budget — not the unchanged-guard
    // — is what stops the loop, proving the clamp.
    let (backend, proxy) = lease(ScriptedBackend::new_changing(
        "Scripted",
        vec![button_tree(2)],
    ));
    let responses: Vec<DecisionResponse> = (0..(MAX_STEPS_LIMIT + 50))
        .map(|_| response("key:tab", 0.1, 0.0))
        .collect();
    let decider = ScriptedDecider::new(responses);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Wander",
        &json!({}),
        100,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::Stuck, "budget exhaustion is stuck");
    assert_eq!(
        out.steps_taken, MAX_STEPS_LIMIT,
        "hard cap must clamp 100 to {MAX_STEPS_LIMIT}"
    );
    assert_eq!(
        backend.keys.lock().unwrap().len(),
        MAX_STEPS_LIMIT,
        "one key press per step"
    );
}

#[test]
fn permission_failure_observing_maps_to_blocked() {
    let (backend, proxy) = lease(ScriptedBackend::with_buttons(2));
    backend.fail_next_observe_with(GuiError::PlatformError(
        "ElementFromHandle failed: Access is denied (0x80070005)".to_string(),
    ));
    let decider = ScriptedDecider::new(vec![response("click:0", 0.0, 0.0)]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Do something",
        &json!({}),
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::Blocked);
    assert_eq!(decider.calls(), 0, "an inaccessible window is never driven");
    assert!(backend.clicks().is_empty());
}

#[test]
fn stale_target_stops_the_loop_instead_of_acting() {
    // Observation order: initial, step-1 freshness, step-1 verify, step-2
    // freshness. Step 2 must still SEE the buttons (else the choice would be
    // rejected as out-of-range instead of exercising the freshness gate), so
    // the third observation is full and only the fourth — step 2's freshness
    // re-check — shows the target gone.
    let moved = {
        let mut t = button_tree(2);
        t.children.clear(); // B0 and B1 are gone
        t
    };
    let (backend, proxy) = lease(ScriptedBackend::new(
        "Scripted",
        vec![button_tree(2), button_tree(2), button_tree(2), moved],
    ));
    let decider = ScriptedDecider::new(vec![
        response("click:0", 0.1, 0.0),
        response("click:1", 0.1, 0.0),
    ]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Click things",
        &json!({}),
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::Stuck, "message: {}", out.message);
    assert!(
        out.message.contains("stale") || out.message.contains("changed"),
        "got: {}",
        out.message
    );
    assert_eq!(backend.clicks(), vec![(18, 18)], "only the fresh click ran");
}

#[test]
fn unchanged_twice_is_stuck_not_a_retry_loop() {
    // A static UI: every click leaves the observation identical.
    let (backend, proxy) = lease(ScriptedBackend::with_buttons(2));
    let decider = ScriptedDecider::new(vec![
        response("click:0", 0.1, 0.0),
        response("click:0", 0.1, 0.0),
    ]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Click B0 forever",
        &json!({}),
        10,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::Stuck);
    assert_eq!(decider.calls(), 2, "stops after two unchanged rounds");
    assert_eq!(backend.clicks().len(), 2, "no third blind retry");
}

#[test]
fn hallucinated_choice_is_error_and_nothing_executes() {
    let (backend, proxy) = lease(ScriptedBackend::with_buttons(2));
    let decider = ScriptedDecider::new(vec![response("shutdown_the_machine", 0.1, 0.0)]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Do something",
        &json!({}),
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::Error);
    assert!(out.message.contains("not one of the offered"));
    assert!(backend.clicks().is_empty());
    assert!(backend.keys.lock().unwrap().is_empty());
}

#[test]
fn missing_irreversible_answer_holds_the_action() {
    // A response with no `irreversible` answer: unknown must lose to caution.
    let (backend, proxy) = lease(ScriptedBackend::with_buttons(2));
    let mut answers = HashMap::new();
    answers.insert(
        "next_action".to_string(),
        Answer {
            answer_type: QuestionType::Choice,
            noul: None,
            choice: Some("click:0".to_string()),
            score: None,
            probabilities: None,
            confidence: None,
            legend: None,
        },
    );
    answers.insert(
        "goal_achieved".to_string(),
        Answer {
            answer_type: QuestionType::Noul,
            noul: Some(0.1),
            choice: None,
            score: None,
            probabilities: None,
            confidence: None,
            legend: None,
        },
    );
    let decider = ScriptedDecider::new(vec![DecisionResponse {
        answers,
        model: None,
        usage: None,
    }]);

    let out = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "Do something",
        &json!({}),
        5,
        false,
    ))
    .unwrap();

    assert_eq!(out.status, UiStatus::NeedsConfirmation);
    assert!(backend.clicks().is_empty());
}

#[test]
fn bad_values_shape_is_an_err_not_an_outcome() {
    let (_backend, proxy) = lease(ScriptedBackend::with_buttons(2));
    let decider = ScriptedDecider::new(vec![]);
    let err = tokio_test::block_on(run_goal_with(
        &decider,
        &proxy,
        "goal",
        &json!("not-an-object"),
        5,
        false,
    ))
    .unwrap_err();
    assert!(err.contains("values"), "got: {err}");
}

#[test]
fn preview_reports_candidates_and_one_decision_without_executing() {
    let (backend, proxy) = lease(ScriptedBackend::new("Scripted", vec![form_tree()]));
    let decider = ScriptedDecider::new(vec![response("type:1", 0.2, 0.1)]);

    let out = tokio_test::block_on(preview_goal_with(
        &decider,
        &proxy,
        "Search for invoices",
        &json!({ "Search": "invoice 42" }),
    ))
    .unwrap();

    assert_eq!(decider.calls(), 1);
    let keys: Vec<&str> = out.candidates.iter().map(|c| c.key.as_str()).collect();
    assert!(keys.contains(&"click:0"));
    assert!(keys.contains(&"type:1"));
    let type_one = out
        .candidates
        .iter()
        .find(|c| c.key == "type:1")
        .expect("type:1 offered");
    assert!(type_one.has_literal, "literal was supplied for Search");
    let decision = out.decision.as_ref().expect("one decision");
    assert_eq!(decision.next_action.as_deref(), Some("type:1"));
    assert_eq!(decision.irreversible, Some(0.1));
    assert!(
        backend.clicks().is_empty() && backend.typed_text().is_empty(),
        "preview must never execute"
    );
    assert!(out.message.contains("no mouse or keyboard"));
}

// ─── Tool surface: availability & wiring ───────────────────────────────────

use sys_mcp::mcp::handlers::handle_tool_call;
use sys_mcp::StubBackend;

#[tokio::test]
async fn ui_decide_tool_is_advertised_with_required_goal() {
    let tools = sys_mcp::mcp::tools::all_tools();
    let tool = tools
        .iter()
        .find(|t| t["name"] == "ui_decide")
        .expect("ui_decide must be listed");
    let schema = &tool["inputSchema"];
    assert_eq!(schema["required"], json!(["goal"]));
    let props = schema["properties"].as_object().unwrap();
    assert!(props.contains_key("goal"));
    assert!(props.contains_key("values"));
    assert!(props.contains_key("max_steps"));
    assert!(props.contains_key("confirm"));
}

/// With the `jev` feature on but the opt-in flag off (the default, and the
/// state the test environment is in unless the operator opted in), the tool
/// must fail with an actionable "how to enable" message — not drive input.
#[tokio::test]
async fn ui_decide_is_disabled_by_default_even_with_jev_feature() {
    if std::env::var("SYS_MCP_JEV_UI_LOOP").is_ok() {
        return; // ambient environment opted in; default-off cannot be asserted
    }
    let client = GuiClient::new(StubBackend);
    let result = handle_tool_call(&client, "ui_decide", json!({ "goal": "anything" })).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("--jev-ui-loop"), "got: {text}");
    assert!(text.contains("SYS_MCP_JEV_UI_LOOP"), "got: {text}");
    assert!(
        !text.to_lowercase().contains("api key"),
        "must never mention a key"
    );
}

/// The config-off message must come before argument validation, and the
/// feature-off message must mention the rebuild flag. Both are asserted in
/// one test because which one applies depends on the build, and both are
/// "clear unavailability" by contract.
#[tokio::test]
async fn ui_decide_missing_goal_reports_config_off_not_missing_arg() {
    if std::env::var("SYS_MCP_JEV_UI_LOOP").is_ok() {
        return;
    }
    let client = GuiClient::new(StubBackend);
    let result = handle_tool_call(&client, "ui_decide", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    // Feature-off builds mention the cargo feature; feature-on builds mention
    // the opt-in flag. Either way: never "unknown tool", never a hang.
    assert!(
        text.contains("--jev-ui-loop") || text.contains("--features jev"),
        "got: {text}"
    );
}

// ─── JevConfig construction for the dead-port pattern ──────────────────────

#[allow(dead_code)]
fn dead_port_config() -> JevConfig {
    JevConfig {
        endpoint: "http://127.0.0.1:9/v1/systemone".to_string(),
        api_key: "unused".to_string(),
        ..JevConfig::default()
    }
}
