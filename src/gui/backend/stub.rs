use crate::error::GuiError;
use crate::gui::accessibility::AccessibilityCapability;
use crate::gui::backend::GuiBackend;
use crate::gui::clipboard::ClipboardCapability;
use crate::gui::display::DisplayCapability;
use crate::gui::input::InputCapability;
use crate::gui::types::{
    AccessibilityNode, AccessibilityQuery, MonitorInfo, MouseButton, Point, Region, Resolution,
    Screenshot, ScrollDirection, WindowInfo,
};
use crate::gui::window::WindowCapability;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct StubBackend;

impl Default for StubBackend {
    fn default() -> Self {
        Self
    }
}

impl StubBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DisplayCapability for StubBackend {
    async fn screenshot(&self) -> Result<Screenshot, GuiError> {
        Ok(Screenshot {
            data: vec![],
            width: 1920,
            height: 1080,
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
            width: 1920,
            height: 1080,
        })
    }

    async fn list_monitors(&self) -> Result<Vec<MonitorInfo>, GuiError> {
        Ok(vec![MonitorInfo {
            id: 0,
            name: "Primary".to_string(),
            primary: true,
            resolution: Resolution {
                width: 1920,
                height: 1080,
            },
            position: (0, 0),
        }])
    }
}

#[async_trait]
impl InputCapability for StubBackend {
    async fn mouse_click(&self, _x: u32, _y: u32, _button: MouseButton) -> Result<(), GuiError> {
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

    async fn type_text(&self, _text: &str) -> Result<(), GuiError> {
        Ok(())
    }

    async fn key_press(&self, _key: &str) -> Result<(), GuiError> {
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
impl WindowCapability for StubBackend {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        Ok(vec![])
    }

    async fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        Err(GuiError::WindowError("No active window".to_string()))
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
        Ok("".to_string())
    }

    async fn get_window_bounds(&self, _window_id: u64) -> Result<Region, GuiError> {
        Ok(Region {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        })
    }

    async fn find_windows_by_title(&self, _query: &str) -> Result<Vec<WindowInfo>, GuiError> {
        Ok(vec![])
    }
}

#[async_trait]
impl ClipboardCapability for StubBackend {
    async fn get_text(&self) -> Result<String, GuiError> {
        Ok("".to_string())
    }

    async fn set_text(&self, _text: &str) -> Result<(), GuiError> {
        Ok(())
    }

    async fn clear(&self) -> Result<(), GuiError> {
        Ok(())
    }
}

impl GuiBackend for StubBackend {
    fn as_window(&self) -> Option<&dyn WindowCapability> {
        Some(self)
    }

    fn as_clipboard(&self) -> Option<&dyn ClipboardCapability> {
        Some(self)
    }

    /// Deliberately `None`: the plain stub has no accessibility tree, so a test
    /// that wants the `accessibility_*` handlers must use [`StubA11yBackend`].
    /// Keeping this `None` is what lets the capability-unavailable error path
    /// stay reachable from tests.
    fn as_accessibility(&self) -> Option<&dyn AccessibilityCapability> {
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Accessibility stub — a fixed tree with the platform id-cache contract
// ═══════════════════════════════════════════════════════════════════════════

/// Test handle for [`StubA11yBackend`]: what the backend was actually asked to
/// do.
///
/// A tool response can echo an argument it never used, so asserting on the
/// response text alone cannot prove `accessibility_action` reached the backend.
/// The recorder is the other side of that call.
#[derive(Clone, Default)]
pub struct StubA11yRecorder {
    invoked: Arc<Mutex<Vec<(String, String)>>>,
}

impl StubA11yRecorder {
    /// `(element_id, action)` pairs in the order the backend received them.
    pub fn invocations(&self) -> Vec<(String, String)> {
        self.invoked
            .lock()
            .expect("stub accessibility recorder lock poisoned")
            .clone()
    }

    fn record(&self, element_id: &str, action: &str) {
        self.invoked
            .lock()
            .expect("stub accessibility recorder lock poisoned")
            .push((element_id.to_string(), action.to_string()));
    }
}

/// The tree handed out by the most recent walk, plus the ids that walk issued.
struct StubA11yWalk {
    root: AccessibilityNode,
    live: HashSet<String>,
}

/// [`StubBackend`] plus a fixed accessibility tree, so the four
/// `accessibility_*` tools can be driven through the real handler path instead
/// of stopping at `UnsupportedCapability`.
///
/// The id contract mirrors the platform backends (issue #39): every walk
/// re-issues `stub:<n>` ids in depth-first order and invalidates the ids of the
/// walk before it, so a stale id fails here exactly the way it fails on Windows.
/// The tree itself is fixed — this tests the plumbing, not UI discovery.
pub struct StubA11yBackend {
    base: StubBackend,
    recorder: StubA11yRecorder,
    walk: Mutex<Option<StubA11yWalk>>,
}

impl StubA11yBackend {
    /// The backend and the recorder a test keeps in order to read back what the
    /// backend received after the backend has been moved into a `GuiClient`.
    pub fn new() -> (Self, StubA11yRecorder) {
        let recorder = StubA11yRecorder::default();
        (
            Self {
                base: StubBackend,
                recorder: recorder.clone(),
                walk: Mutex::new(None),
            },
            recorder,
        )
    }

    /// Run a walk: mint ids depth-first up to `max_depth`, remember the tree and
    /// its ids, and return the tree. Every call replaces the previous walk, so
    /// ids from an earlier call stop resolving — the contract under test.
    fn walk(&self, max_depth: Option<u32>) -> AccessibilityNode {
        let mut root = stub_tree();
        let limit = max_depth.unwrap_or(10);
        let mut counter = 0u64;
        let mut live = HashSet::new();
        mint_ids(&mut root, &mut counter, &mut live, limit);

        let mut walk = self
            .walk
            .lock()
            .expect("stub accessibility walk lock poisoned");
        *walk = Some(StubA11yWalk {
            root: root.clone(),
            live,
        });
        root
    }

    /// Resolve an id against the current walk, with the same wording the Windows
    /// backend uses so a test asserting on it means the same thing on both.
    fn resolve(&self, element_id: &str) -> Result<AccessibilityNode, GuiError> {
        let walk = self
            .walk
            .lock()
            .expect("stub accessibility walk lock poisoned");
        let walk = walk
            .as_ref()
            .ok_or_else(|| GuiError::PlatformError(stale_id_message(element_id)))?;
        if !walk.live.contains(element_id) {
            return Err(GuiError::PlatformError(stale_id_message(element_id)));
        }
        find_by_id(&walk.root, element_id)
            .cloned()
            .map(|mut node| {
                // Properties of the element itself, not a re-walk of its subtree
                // — matches `build_node_with_id(element, 0, id)`.
                node.children.clear();
                node
            })
            .ok_or_else(|| GuiError::PlatformError(stale_id_message(element_id)))
    }
}

fn stale_id_message(element_id: &str) -> String {
    format!(
        "element id '{element_id}' is not in the current accessibility cache; call accessibility_tree or accessibility_find again and use a fresh id"
    )
}

fn node(
    role: &str,
    name: Option<&str>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> AccessibilityNode {
    AccessibilityNode {
        id: String::new(),
        role: role.to_string(),
        name: name.map(String::from),
        value: None,
        description: None,
        x: Some(x),
        y: Some(y),
        width: Some(width),
        height: Some(height),
        cx: Some(x + width as i32 / 2),
        cy: Some(y + height as i32 / 2),
        children: Vec::new(),
        actions: Vec::new(),
        states: Vec::new(),
    }
}

/// The fixture: a window, a menu bar with one item, a checkbox, an edit field,
/// a button and a nameless separator. Chosen to cover one of each shape the
/// tools advertise — nested children, a `value`, a `description`, an absent
/// `name`, and elements with and without actions — rather than to look like any
/// particular application.
fn stub_tree() -> AccessibilityNode {
    let mut window = node("window", Some("Stub Window"), 0, 0, 800, 600);
    window.states = vec!["enabled".to_string()];

    let mut menu_bar = node("menu bar", Some("Menu"), 0, 0, 800, 24);
    let mut file_menu = node("menu item", Some("File"), 4, 4, 40, 16);
    file_menu.actions = vec!["expand".to_string(), "collapse".to_string()];
    file_menu.states = vec!["enabled".to_string()];
    menu_bar.children = vec![file_menu];

    let mut checkbox = node("checkbox", Some("Trial Balance"), 20, 60, 16, 16);
    checkbox.actions = vec!["toggle".to_string()];
    checkbox.states = vec!["checked".to_string(), "focusable".to_string()];
    checkbox.value = Some("1".to_string());

    let mut edit = node("edit", Some("Voucher No"), 120, 60, 120, 22);
    edit.value = Some("12060623".to_string());
    edit.description = Some("Voucher number for the current transaction".to_string());
    edit.states = vec!["enabled".to_string(), "focusable".to_string()];

    let mut button = node("button", Some("OK (F2)"), 640, 560, 80, 24);
    button.actions = vec!["invoke".to_string(), "press".to_string()];
    button.states = vec!["enabled".to_string()];

    // A node with no name: `accessibility_find` must not match an absent name,
    // and the JSON must carry `"name": null` rather than dropping the key.
    let separator = node("separator", None, 0, 550, 800, 1);

    let mut grid = node("table", Some("Day Book"), 0, 90, 800, 460);
    grid.children = vec![node("text", Some("Row 1"), 0, 90, 800, 20)];

    window.children = vec![menu_bar, checkbox, edit, separator, button, grid];
    window
}

/// Assign ids in depth-first pre-order and drop the children below `remaining`,
/// so `live` ends up holding exactly the ids the caller was given.
fn mint_ids(
    node: &mut AccessibilityNode,
    counter: &mut u64,
    live: &mut HashSet<String>,
    remaining: u32,
) {
    node.id = format!("stub:{counter}");
    *counter += 1;
    live.insert(node.id.clone());
    if remaining == 0 {
        node.children.clear();
        return;
    }
    for child in &mut node.children {
        mint_ids(child, counter, live, remaining - 1);
    }
}

fn find_by_id<'a>(node: &'a AccessibilityNode, id: &str) -> Option<&'a AccessibilityNode> {
    if node.id == id {
        return Some(node);
    }
    node.children.iter().find_map(|c| find_by_id(c, id))
}

/// Case-insensitive substring match on name and role, matching
/// `PlatformAccessibility::search_tree` on Windows. An absent name never
/// matches a name query.
fn stub_search(
    node: &AccessibilityNode,
    query: &AccessibilityQuery,
    out: &mut Vec<AccessibilityNode>,
) {
    let name_matches = match (&query.name, &node.name) {
        (Some(q), Some(n)) => n.to_lowercase().contains(&q.to_lowercase()),
        (Some(_), None) => false,
        (None, _) => true,
    };
    let role_matches = query
        .role
        .as_ref()
        .map(|q| node.role.to_lowercase().contains(&q.to_lowercase()))
        .unwrap_or(true);
    if name_matches && role_matches {
        out.push(node.clone());
    }
    for child in &node.children {
        stub_search(child, query, out);
    }
}

#[async_trait]
impl DisplayCapability for StubA11yBackend {
    async fn screenshot(&self) -> Result<Screenshot, GuiError> {
        self.base.screenshot().await
    }
    async fn screenshot_region(&self, region: Region) -> Result<Screenshot, GuiError> {
        self.base.screenshot_region(region).await
    }
    async fn get_screen_size(&self) -> Result<Resolution, GuiError> {
        self.base.get_screen_size().await
    }
    async fn list_monitors(&self) -> Result<Vec<MonitorInfo>, GuiError> {
        self.base.list_monitors().await
    }
}

#[async_trait]
impl InputCapability for StubA11yBackend {
    async fn mouse_click(&self, x: u32, y: u32, button: MouseButton) -> Result<(), GuiError> {
        self.base.mouse_click(x, y, button).await
    }
    async fn mouse_double_click(
        &self,
        x: u32,
        y: u32,
        button: MouseButton,
    ) -> Result<(), GuiError> {
        self.base.mouse_double_click(x, y, button).await
    }
    async fn mouse_move(&self, x: u32, y: u32) -> Result<(), GuiError> {
        self.base.mouse_move(x, y).await
    }
    async fn mouse_position(&self) -> Result<Point, GuiError> {
        self.base.mouse_position().await
    }
    async fn mouse_drag(
        &self,
        from: Point,
        to: Point,
        button: MouseButton,
    ) -> Result<(), GuiError> {
        self.base.mouse_drag(from, to, button).await
    }
    async fn mouse_scroll(
        &self,
        x: u32,
        y: u32,
        direction: ScrollDirection,
        amount: i32,
    ) -> Result<(), GuiError> {
        self.base.mouse_scroll(x, y, direction, amount).await
    }
    async fn type_text(&self, text: &str) -> Result<(), GuiError> {
        self.base.type_text(text).await
    }
    async fn key_press(&self, key: &str) -> Result<(), GuiError> {
        self.base.key_press(key).await
    }
    async fn key_down(&self, key: &str) -> Result<(), GuiError> {
        self.base.key_down(key).await
    }
    async fn key_up(&self, key: &str) -> Result<(), GuiError> {
        self.base.key_up(key).await
    }
    async fn key_combo(&self, keys: &[String]) -> Result<(), GuiError> {
        self.base.key_combo(keys).await
    }
}

#[async_trait]
impl AccessibilityCapability for StubA11yBackend {
    async fn get_tree(
        &self,
        _window_id: Option<u64>,
        max_depth: Option<u32>,
    ) -> Result<AccessibilityNode, GuiError> {
        Ok(self.walk(max_depth))
    }

    async fn find_elements(
        &self,
        query: AccessibilityQuery,
    ) -> Result<Vec<AccessibilityNode>, GuiError> {
        // A fresh walk, like the platform backends: the ids in the results are
        // the ids that resolve until the next walk replaces them.
        let tree = self.walk(query.max_depth);
        let mut results = Vec::new();
        stub_search(&tree, &query, &mut results);
        Ok(results)
    }

    async fn get_element_properties(
        &self,
        element_id: &str,
    ) -> Result<AccessibilityNode, GuiError> {
        self.resolve(element_id)
    }

    async fn invoke_element_action(&self, element_id: &str, action: &str) -> Result<(), GuiError> {
        let element = self.resolve(element_id)?;
        if action.trim().is_empty() {
            return Err(GuiError::PlatformError(format!(
                "unrecognized action ''; this element supports: {}",
                if element.actions.is_empty() {
                    "none — this element exposes no actionable patterns".to_string()
                } else {
                    element.actions.join(", ")
                }
            )));
        }
        self.recorder.record(element_id, action);
        Ok(())
    }
}

impl GuiBackend for StubA11yBackend {
    fn as_accessibility(&self) -> Option<&dyn AccessibilityCapability> {
        Some(self)
    }
}
