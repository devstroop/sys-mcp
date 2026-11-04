use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, TreeScope_Children,
    UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId,
    UIA_EditControlTypeId, UIA_HeaderControlTypeId, UIA_HyperlinkControlTypeId,
    UIA_ImageControlTypeId, UIA_InvokePatternId, UIA_ListControlTypeId, UIA_ListItemControlTypeId,
    UIA_MenuControlTypeId, UIA_MenuItemControlTypeId, UIA_RadioButtonControlTypeId,
    UIA_ScrollBarControlTypeId, UIA_SliderControlTypeId, UIA_StatusBarControlTypeId,
    UIA_TabControlTypeId, UIA_TableControlTypeId, UIA_ToolBarControlTypeId, UIA_TreeControlTypeId,
    UIA_TreeItemControlTypeId, UIA_WindowControlTypeId, UIA_WindowPatternId, UIA_CONTROLTYPE_ID,
    IUIAutomationInvokePattern, IUIAutomationWindowPattern, WindowVisualState_Maximized,
    WindowVisualState_Minimized, WindowVisualState_Normal,
};
use windows::core::Interface;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::error::GuiError;
use crate::gui::types::*;

unsafe impl Send for PlatformAccessibility {}
unsafe impl Sync for PlatformAccessibility {}

#[allow(non_upper_case_globals)]
fn control_type_to_role(control_type: UIA_CONTROLTYPE_ID) -> String {
    match control_type {
        UIA_ButtonControlTypeId => "button".to_string(),
        UIA_CheckBoxControlTypeId => "checkbox".to_string(),
        UIA_ComboBoxControlTypeId => "combobox".to_string(),
        UIA_EditControlTypeId => "text".to_string(),
        UIA_HyperlinkControlTypeId => "link".to_string(),
        UIA_ImageControlTypeId => "image".to_string(),
        UIA_ListItemControlTypeId => "list_item".to_string(),
        UIA_ListControlTypeId => "list".to_string(),
        UIA_MenuControlTypeId => "menu".to_string(),
        UIA_MenuItemControlTypeId => "menu_item".to_string(),
        UIA_RadioButtonControlTypeId => "radio_button".to_string(),
        UIA_ScrollBarControlTypeId => "scroll_bar".to_string(),
        UIA_SliderControlTypeId => "slider".to_string(),
        UIA_StatusBarControlTypeId => "status_bar".to_string(),
        UIA_TabControlTypeId => "tab".to_string(),
        UIA_TableControlTypeId => "table".to_string(),
        UIA_ToolBarControlTypeId => "toolbar".to_string(),
        UIA_TreeControlTypeId => "tree".to_string(),
        UIA_TreeItemControlTypeId => "tree_item".to_string(),
        UIA_HeaderControlTypeId => "header".to_string(),
        UIA_WindowControlTypeId => "window".to_string(),
        _ => format!("control_type_{}", control_type.0),
    }
}

pub struct PlatformAccessibility {
    // ManuallyDrop so Drop can release the COM pointer at a controlled moment.
    // Left as a plain field it would be dropped AFTER Drop::drop ran, i.e.
    // after CoUninitialize — see the Drop impl below.
    automation: std::mem::ManuallyDrop<IUIAutomation>,
    // Elements from the most recent tree walk, keyed by the id we handed out.
    // Keeping the COM refs alive is what makes an id resolvable by a later
    // get_element_properties / invoke_element_action call — the old
    // pointer-only id dangled once the walk dropped its wrappers. Cleared at
    // the start of every get_tree (bounding it to one tree) and in Drop before
    // COM is torn down.
    cache: Mutex<HashMap<String, IUIAutomationElement>>,
    // Monotonic id source for the current tree walk. Ids only need to be unique
    // within one cache lifetime (the contract is "an id handed out by the latest
    // get_tree resolves until the next get_tree"), and a counter guarantees
    // that. Deriving ids from `element as *const _` did NOT: the walk passed
    // `&child` where `child` was a fresh local reusing the same stack slot every
    // iteration, so distinct elements collapsed onto one address and collided in
    // the cache. Reset at the start of every get_tree.
    next_id: AtomicU64,
}

impl PlatformAccessibility {
    pub fn new() -> Result<Self, GuiError> {
        unsafe {
            // COM must be initialized MULTITHREADED (MTA), not apartment-
            // threaded. This type is `unsafe impl Send + Sync` and is held by
            // `LocalBackend`, which tokio moves between worker threads — so the
            // `IUIAutomation` pointer gets released on a *different* thread
            // from the one that created it. An STA pointer is thread-affine,
            // and releasing it off-apartment faults with STATUS_ACCESS_VIOLATION
            // (0xc0000005) on drop, crashing the process at shutdown. MTA
            // objects are valid to release from any thread. UIA supports MTA.
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| GuiError::PlatformError(format!("COM init failed: {e}")))?;
        }

        let automation: IUIAutomation = unsafe {
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| GuiError::PlatformError(format!("UIAutomation create failed: {e}")))?
        };

        Ok(Self {
            automation: std::mem::ManuallyDrop::new(automation),
            cache: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
        })
    }

    fn get_element_name(element: &IUIAutomationElement) -> Option<String> {
        unsafe {
            let name = element.CurrentName().ok()?;
            let s = name.to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
    }

    fn get_element_control_type(element: &IUIAutomationElement) -> UIA_CONTROLTYPE_ID {
        unsafe {
            element
                .CurrentControlType()
                .unwrap_or(UIA_CONTROLTYPE_ID(0))
        }
    }

    fn get_element_bounds(element: &IUIAutomationElement) -> (i32, i32, u32, u32) {
        unsafe {
            let rect = element.CurrentBoundingRectangle().ok();
            match rect {
                Some(r) => (
                    r.left,
                    r.top,
                    (r.right - r.left) as u32,
                    (r.bottom - r.top) as u32,
                ),
                None => (0, 0, 0, 0),
            }
        }
    }

    fn get_element_states(element: &IUIAutomationElement) -> Vec<String> {
        let mut states = Vec::new();

        unsafe {
            if let Ok(is_enabled) = element.CurrentIsEnabled() {
                if !is_enabled.as_bool() {
                    states.push("disabled".to_string());
                }
            }

            if let Ok(has_focus) = element.CurrentHasKeyboardFocus() {
                if has_focus.as_bool() {
                    states.push("focused".to_string());
                }
            }
        }

        states
    }

    fn get_element_actions(element: &IUIAutomationElement) -> Vec<String> {
        let mut actions = Vec::new();

        unsafe {
            if element.GetCurrentPattern(UIA_InvokePatternId).is_ok() {
                actions.push("invoke".to_string());
            }
            if element.GetCurrentPattern(UIA_WindowPatternId).is_ok() {
                actions.push("close".to_string());
                actions.push("minimize".to_string());
                actions.push("maximize".to_string());
                actions.push("restore".to_string());
            }
        }

        actions
    }

    fn build_node(
        &self,
        element: &IUIAutomationElement,
        depth: u32,
    ) -> Result<AccessibilityNode, GuiError> {
        let name = Self::get_element_name(element);
        let role = control_type_to_role(Self::get_element_control_type(element));
        let actions = Self::get_element_actions(element);
        let states = Self::get_element_states(element);

        let (x, y, w, h) = Self::get_element_bounds(element);
        let cx = if w > 0 { Some(x + w as i32 / 2) } else { None };
        let cy = if h > 0 { Some(y + h as i32 / 2) } else { None };

        let id = format!("uia:{}", self.next_id.fetch_add(1, Ordering::Relaxed));

        // Keep this element alive under the id we hand out so a later
        // get_element_properties / invoke_element_action call can resolve it.
        // Overwriting an existing entry is fine (same element, one extra ref).
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(id.clone(), element.clone());
        }

        let mut children = Vec::new();
        if depth > 0 {
            unsafe {
                if let Ok(child_array) = element.FindAll(
                    TreeScope_Children,
                    &self.automation.CreateTrueCondition().map_err(|e| {
                        GuiError::PlatformError(format!("CreateTrueCondition: {e}"))
                    })?,
                ) {
                    let count = child_array
                        .Length()
                        .map_err(|e| GuiError::PlatformError(format!("Length: {e}")))?
                        as usize;
                    for i in 0..count {
                        if let Ok(child) = child_array.GetElement(i as i32) {
                            if let Ok(node) = self.build_node(&child, depth - 1) {
                                children.push(node);
                            }
                        }
                    }
                }
            }
        }

        Ok(AccessibilityNode {
            id,
            role,
            name,
            value: None,
            description: None,
            x: Some(x),
            y: Some(y),
            width: Some(w),
            height: Some(h),
            cx,
            cy,
            children,
            actions,
            states,
        })
    }

    pub fn get_tree(
        &self,
        window_id: Option<u64>,
        max_depth: Option<u32>,
    ) -> Result<AccessibilityNode, GuiError> {
        let depth = max_depth.unwrap_or(10);

        // A fresh walk invalidates every id handed out before it, so drop the
        // previous tree's cached elements first (this also bounds the cache to
        // one tree). COM is initialized here, so releasing them now is safe.
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
        self.next_id.store(0, Ordering::Relaxed);

        unsafe {
            let root = if let Some(wid) = window_id {
                let hwnd = windows::Win32::Foundation::HWND(wid as *mut _);
                self.automation.ElementFromHandle(hwnd).map_err(|e| {
                    GuiError::PlatformError(format!("ElementFromHandle failed: {e}"))
                })?
            } else {
                let focused = self.automation.GetFocusedElement().map_err(|e| {
                    GuiError::PlatformError(format!("GetFocusedElement failed: {e}"))
                })?;

                let tree_walker = self
                    .automation
                    .ControlViewWalker()
                    .map_err(|e| GuiError::PlatformError(format!("ControlViewWalker: {e}")))?;
                let mut current = focused;
                while let Ok(parent) = tree_walker.GetParentElement(&current) {
                    let ct = Self::get_element_control_type(&parent);
                    if ct == UIA_WindowControlTypeId
                        || parent
                            .CurrentName()
                            .ok()
                            .map(|n| n.is_empty())
                            .unwrap_or(true)
                    {
                        break;
                    }
                    current = parent;
                }
                current
            };

            self.build_node(&root, depth)
        }
    }

    pub fn find_elements(
        &self,
        query: AccessibilityQuery,
    ) -> Result<Vec<AccessibilityNode>, GuiError> {
        let tree = self.get_tree(query.window_id, query.max_depth)?;
        let mut results = Vec::new();
        Self::search_tree(&tree, &query, &mut results);
        Ok(results)
    }

    fn search_tree(
        node: &AccessibilityNode,
        query: &AccessibilityQuery,
        results: &mut Vec<AccessibilityNode>,
    ) {
        let name_matches = query
            .name
            .as_ref()
            .map(|q| {
                node.name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&q.to_lowercase()))
                    .unwrap_or(false)
            })
            .unwrap_or(true);

        let role_matches = query
            .role
            .as_ref()
            .map(|q| node.role.to_lowercase().contains(&q.to_lowercase()))
            .unwrap_or(true);

        if name_matches && role_matches {
            results.push(node.clone());
        }

        for child in &node.children {
            Self::search_tree(child, query, results);
        }
    }

    /// Resolve an element id handed out by the most recent `get_tree`.
    fn lookup(&self, element_id: &str) -> Result<IUIAutomationElement, GuiError> {
        let cache = self
            .cache
            .lock()
            .map_err(|_| GuiError::PlatformError("accessibility cache lock poisoned".into()))?;
        cache.get(element_id).cloned().ok_or_else(|| {
            GuiError::PlatformError(format!(
                "element id '{element_id}' is not in the current accessibility cache; call accessibility_tree or accessibility_find again and use a fresh id"
            ))
        })
    }

    pub fn get_element_properties(&self, element_id: &str) -> Result<AccessibilityNode, GuiError> {
        let element = self.lookup(element_id)?;
        // depth 0: this element's own properties, not a re-walk of its subtree.
        let mut node = self.build_node(&element, 0)?;
        // build_node minted a fresh counter id for this single-element call;
        // the caller addressed the element by the id the last get_tree handed
        // out, and that is the key still present in the cache. Report it back
        // verbatim so accessibility_properties -> accessibility_action round-
        // trips against the same id.
        node.id = element_id.to_string();
        Ok(node)
    }

    pub fn invoke_element_action(&self, element_id: &str, action: &str) -> Result<(), GuiError> {
        let element = self.lookup(element_id)?;
        unsafe {
            match action {
                "invoke" => {
                    let pattern: IUIAutomationInvokePattern = element
                        .GetCurrentPattern(UIA_InvokePatternId)
                        .map_err(|e| {
                            GuiError::PlatformError(format!("GetCurrentPattern(invoke): {e}"))
                        })?
                        .cast()
                        .map_err(|_| {
                            GuiError::PlatformError(format!(
                                "element '{element_id}' does not support the 'invoke' action"
                            ))
                        })?;
                    pattern
                        .Invoke()
                        .map_err(|e| GuiError::PlatformError(format!("Invoke: {e}")))?;
                }
                "focus" => element
                    .SetFocus()
                    .map_err(|e| GuiError::PlatformError(format!("SetFocus: {e}")))?,
                "close" | "minimize" | "maximize" | "restore" => {
                    let pattern: IUIAutomationWindowPattern = element
                        .GetCurrentPattern(UIA_WindowPatternId)
                        .map_err(|e| {
                            GuiError::PlatformError(format!("GetCurrentPattern(window): {e}"))
                        })?
                        .cast()
                        .map_err(|_| {
                            GuiError::PlatformError(format!(
                                "element '{element_id}' does not support window actions"
                            ))
                        })?;
                    match action {
                        "close" => pattern
                            .Close()
                            .map_err(|e| GuiError::PlatformError(format!("Close: {e}")))?,
                        "minimize" => pattern
                            .SetWindowVisualState(WindowVisualState_Minimized)
                            .map_err(|e| {
                                GuiError::PlatformError(format!("SetWindowVisualState: {e}"))
                            })?,
                        "maximize" => pattern
                            .SetWindowVisualState(WindowVisualState_Maximized)
                            .map_err(|e| {
                                GuiError::PlatformError(format!("SetWindowVisualState: {e}"))
                            })?,
                        _ => pattern
                            .SetWindowVisualState(WindowVisualState_Normal)
                            .map_err(|e| {
                                GuiError::PlatformError(format!("SetWindowVisualState: {e}"))
                            })?,
                    }
                }
                other => {
                    return Err(GuiError::PlatformError(format!(
                        "unsupported action '{other}' on Windows; supported actions: invoke, focus, close, minimize, maximize, restore (see the element's `actions` list)"
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Drop for PlatformAccessibility {
    fn drop(&mut self) {
        unsafe {
            // Order matters and was previously inverted, which segfaulted the
            // process with STATUS_ACCESS_VIOLATION (0xc0000005) on every
            // LocalBackend shutdown. Rust drops fields AFTER this body, so the
            // old code ran CoUninitialize() and only then released the
            // IUIAutomation pointer — releasing a COM object after its
            // apartment is gone faults. Release the pointer first, then tear
            // down COM.
            // Release cached UIA elements FIRST: they are COM refs, and
            // dropping them after CoUninitialize faults exactly like the
            // automation pointer used to (see below).
            if let Ok(cache) = self.cache.get_mut() {
                cache.clear();
            }
            std::mem::ManuallyDrop::drop(&mut self.automation);
            CoUninitialize();
        }
    }
}
