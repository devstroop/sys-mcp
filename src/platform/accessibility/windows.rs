use windows::core::Interface;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, ExpandCollapseState_Collapsed, ExpandCollapseState_Expanded,
    ExpandCollapseState_PartiallyExpanded, IUIAutomation, IUIAutomationElement,
    IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern,
    IUIAutomationLegacyIAccessiblePattern, IUIAutomationRangeValuePattern,
    IUIAutomationScrollItemPattern, IUIAutomationSelectionItemPattern, IUIAutomationTogglePattern,
    IUIAutomationValuePattern, IUIAutomationWindowPattern, ToggleState_Indeterminate,
    ToggleState_Off, ToggleState_On, TreeScope_Children, UIA_ButtonControlTypeId,
    UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId, UIA_EditControlTypeId,
    UIA_ExpandCollapsePatternId, UIA_HeaderControlTypeId, UIA_HyperlinkControlTypeId,
    UIA_ImageControlTypeId, UIA_InvokePatternId, UIA_LegacyIAccessiblePatternId,
    UIA_ListControlTypeId, UIA_ListItemControlTypeId, UIA_MenuControlTypeId,
    UIA_MenuItemControlTypeId, UIA_RadioButtonControlTypeId, UIA_RangeValuePatternId,
    UIA_ScrollBarControlTypeId, UIA_ScrollItemPatternId, UIA_SelectionItemPatternId,
    UIA_SliderControlTypeId, UIA_StatusBarControlTypeId, UIA_TabControlTypeId,
    UIA_TableControlTypeId, UIA_TogglePatternId, UIA_ToolBarControlTypeId, UIA_TreeControlTypeId,
    UIA_TreeItemControlTypeId, UIA_ValuePatternId, UIA_WindowControlTypeId, UIA_WindowPatternId,
    WindowVisualState_Maximized, WindowVisualState_Minimized, WindowVisualState_Normal,
    UIA_CONTROLTYPE_ID, UIA_PATTERN_ID,
};

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

    /// Cast a live pattern off an element, or `None` when the element does not
    /// implement it. Every capability probe goes through here, so a missing
    /// pattern is an ordinary `None` rather than an error each caller has to
    /// remember to tolerate.
    fn pattern<T: Interface>(element: &IUIAutomationElement, id: UIA_PATTERN_ID) -> Option<T> {
        unsafe {
            element
                .GetCurrentPattern(id)
                .ok()
                .and_then(|unknown| unknown.cast::<T>().ok())
        }
    }

    // The windows crate names UIA enum members `ToggleState_On` / `ExpandCollapseState_Expanded`,
    // which do not match the non_camel_case / non_upper_case_globals lint expectations.
    #[allow(non_upper_case_globals)]
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

            // The state an automation caller most needs before clicking. UIA
            // keeps reporting a bounding rectangle for an element scrolled out
            // of its container, so without this a `cx`/`cy` click lands on
            // whatever is now painted at those coordinates.
            if element
                .CurrentIsOffscreen()
                .map(|v| v.as_bool())
                .unwrap_or(false)
            {
                states.push("offscreen".to_string());
            }

            if element
                .CurrentIsPassword()
                .map(|v| v.as_bool())
                .unwrap_or(false)
            {
                states.push("password".to_string());
            }

            if element
                .CurrentIsRequiredForForm()
                .map(|v| v.as_bool())
                .unwrap_or(false)
            {
                states.push("required".to_string());
            }

            if let Some(value) =
                Self::pattern::<IUIAutomationValuePattern>(element, UIA_ValuePatternId)
            {
                if value
                    .CurrentIsReadOnly()
                    .map(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    states.push("readonly".to_string());
                }
            }

            if let Some(toggle) =
                Self::pattern::<IUIAutomationTogglePattern>(element, UIA_TogglePatternId)
            {
                match toggle.CurrentToggleState().unwrap_or(ToggleState_Off) {
                    ToggleState_On => states.push("checked".to_string()),
                    ToggleState_Indeterminate => states.push("indeterminate".to_string()),
                    _ => {}
                }
            }

            // Only report expand/collapse when the pattern exists — otherwise
            // every element would look "collapsed".
            if let Some(expand) = Self::pattern::<IUIAutomationExpandCollapsePattern>(
                element,
                UIA_ExpandCollapsePatternId,
            ) {
                match expand
                    .CurrentExpandCollapseState()
                    .unwrap_or(ExpandCollapseState_Collapsed)
                {
                    ExpandCollapseState_Expanded => states.push("expanded".to_string()),
                    ExpandCollapseState_PartiallyExpanded => {
                        states.push("partially_expanded".to_string())
                    }
                    _ => states.push("collapsed".to_string()),
                }
            }

            if let Some(select) = Self::pattern::<IUIAutomationSelectionItemPattern>(
                element,
                UIA_SelectionItemPatternId,
            ) {
                if select
                    .CurrentIsSelected()
                    .map(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    states.push("selected".to_string());
                }
            }
        }

        states
    }

    /// The action names this element accepts from `accessibility_action`.
    /// Derived from live patterns, so the advertised list is exactly the set
    /// that cannot fail — the agent can self-correct from an unsupported verb
    /// by reading this list back off the element.
    fn get_element_actions(element: &IUIAutomationElement) -> Vec<String> {
        let mut actions = Vec::new();

        if Self::pattern::<IUIAutomationInvokePattern>(element, UIA_InvokePatternId).is_some() {
            actions.push("invoke".to_string());
        }
        if Self::pattern::<IUIAutomationTogglePattern>(element, UIA_TogglePatternId).is_some() {
            actions.push("toggle".to_string());
        }
        if Self::pattern::<IUIAutomationExpandCollapsePattern>(element, UIA_ExpandCollapsePatternId)
            .is_some()
        {
            actions.push("expand".to_string());
            actions.push("collapse".to_string());
        }
        if Self::pattern::<IUIAutomationSelectionItemPattern>(element, UIA_SelectionItemPatternId)
            .is_some()
        {
            actions.push("select".to_string());
        }
        if Self::pattern::<IUIAutomationRangeValuePattern>(element, UIA_RangeValuePatternId)
            .is_some()
        {
            actions.push("increment".to_string());
            actions.push("decrement".to_string());
        }
        if Self::pattern::<IUIAutomationScrollItemPattern>(element, UIA_ScrollItemPatternId)
            .is_some()
        {
            actions.push("scroll_into_view".to_string());
        }
        if Self::pattern::<IUIAutomationWindowPattern>(element, UIA_WindowPatternId).is_some() {
            actions.push("close".to_string());
            actions.push("minimize".to_string());
            actions.push("maximize".to_string());
            actions.push("restore".to_string());
        }
        // Focusability is a UIA property rather than a pattern, so `focus` is
        // advertised only for elements that can actually take keyboard focus.
        // Without it in this list the list would also be an incomplete answer
        // for the caller that read it to decide what to send next.
        if unsafe { element.CurrentIsKeyboardFocusable() }
            .map(|v| v.as_bool())
            .unwrap_or(false)
        {
            actions.push("focus".to_string());
        }

        actions
    }

    /// Error for a verb the element does not expose. Echoes the element's live
    /// action list, so an agent can correct itself from the response alone
    /// instead of spending another round trip on `accessibility_properties`.
    fn unsupported_action(
        element_id: &str,
        action: &str,
        element: &IUIAutomationElement,
    ) -> GuiError {
        let supported = Self::get_element_actions(element);
        GuiError::PlatformError(format!(
            "element '{element_id}' does not support the '{action}' action; supported actions: {}",
            if supported.is_empty() {
                "none — this element exposes no actionable UIA pattern".to_string()
            } else {
                supported.join(", ")
            }
        ))
    }

    /// The element's live value — the text in an edit box, the number behind a
    /// slider, the string MSAA exposes. Without this an agent has to OCR a
    /// field to learn what is in it, which is the single most common reason a
    /// form-driving session falls back to pixels.
    ///
    /// Capped because a document view reports its entire text as its value; a
    /// bounded prefix is enough to decide whether the field holds what was
    /// expected.
    fn get_element_value(element: &IUIAutomationElement) -> Option<String> {
        const MAX_VALUE_CHARS: usize = 4096;

        let from_value_pattern =
            Self::pattern::<IUIAutomationValuePattern>(element, UIA_ValuePatternId)
                .and_then(|value| unsafe { value.CurrentValue() }.ok())
                .map(|bstr| bstr.to_string())
                .filter(|s| !s.is_empty());

        let from_range_value =
            Self::pattern::<IUIAutomationRangeValuePattern>(element, UIA_RangeValuePatternId)
                .and_then(|range| unsafe { range.CurrentValue() }.ok())
                .map(|v| v.to_string());

        // Legacy MSAA bridge. Plenty of Win32/VCL/Delphi-era controls expose
        // their content only here, and those are exactly the apps this backend
        // tends to get pointed at.
        let from_legacy = Self::pattern::<IUIAutomationLegacyIAccessiblePattern>(
            element,
            UIA_LegacyIAccessiblePatternId,
        )
        .and_then(|legacy| unsafe { legacy.CurrentValue() }.ok())
        .map(|bstr| bstr.to_string())
        .filter(|s| !s.is_empty());

        from_value_pattern
            .or(from_range_value)
            .or(from_legacy)
            .map(|s| {
                if s.chars().count() > MAX_VALUE_CHARS {
                    let head: String = s.chars().take(MAX_VALUE_CHARS).collect();
                    format!("{head}…")
                } else {
                    s
                }
            })
    }

    /// Help text, falling back to the MSAA description. Kept separate from
    /// `name` so a labelled form field stays distinguishable from its caption.
    fn get_element_description(element: &IUIAutomationElement) -> Option<String> {
        let help = unsafe { element.CurrentHelpText() }
            .ok()
            .map(|bstr| bstr.to_string())
            .filter(|s| !s.is_empty());

        help.or_else(|| {
            Self::pattern::<IUIAutomationLegacyIAccessiblePattern>(
                element,
                UIA_LegacyIAccessiblePatternId,
            )
            .and_then(|legacy| unsafe { legacy.CurrentDescription() }.ok())
            .map(|bstr| bstr.to_string())
            .filter(|s| !s.is_empty())
        })
    }

    /// Build a node for `element`.
    ///
    /// `id == None` mints the next id from the walk counter and caches the
    /// element under it — that is the tree-walk path. `id == Some(existing)`
    /// reuses the id the caller already addressed the element by and leaves the
    /// cache and counter untouched, so a read-only `get_element_properties`
    /// call cannot grow the cache it is only inspecting.
    fn build_node_with_id(
        &self,
        element: &IUIAutomationElement,
        depth: u32,
        id: Option<String>,
    ) -> Result<AccessibilityNode, GuiError> {
        let id = match id {
            Some(existing) => existing,
            None => {
                let fresh = format!("uia:{}", self.next_id.fetch_add(1, Ordering::Relaxed));
                // Keep this element alive under the id we hand out so a later
                // get_element_properties / invoke_element_action call can
                // resolve it. Overwriting an existing entry is fine (same
                // element, one extra ref).
                //
                // A lock failure is reported instead of skipped: silently not
                // inserting still hands the caller an id that can never
                // resolve, and the follow-up error would name the caller's
                // argument rather than the poisoned mutex.
                let mut cache = self.cache.lock().map_err(|_| {
                    GuiError::PlatformError("accessibility cache lock poisoned".into())
                })?;
                cache.insert(fresh.clone(), element.clone());
                fresh
            }
        };

        let name = Self::get_element_name(element);
        let role = control_type_to_role(Self::get_element_control_type(element));
        let actions = Self::get_element_actions(element);
        let states = Self::get_element_states(element);
        let value = Self::get_element_value(element);
        let description = Self::get_element_description(element);

        let (x, y, w, h) = Self::get_element_bounds(element);
        let cx = if w > 0 { Some(x + w as i32 / 2) } else { None };
        let cy = if h > 0 { Some(y + h as i32 / 2) } else { None };

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
                            if let Ok(node) = self.build_node_with_id(&child, depth - 1, None) {
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
            value,
            description,
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

    fn build_node(
        &self,
        element: &IUIAutomationElement,
        depth: u32,
    ) -> Result<AccessibilityNode, GuiError> {
        self.build_node_with_id(element, depth, None)
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
        // The id is passed through unchanged — the caller addressed the element
        // by the key the last get_tree stored, and reusing it keeps the
        // properties -> action round-trip resolvable against the same cache
        // entry without minting a duplicate (issue #39).
        self.build_node_with_id(&element, 0, Some(element_id.to_string()))
    }

    /// Perform one of the actions the element advertises.
    ///
    /// Verbs are matched case-insensitively with `-` read as `_`. `press`,
    /// `click`, `tap` and `do_default_action` are accepted as "activate this
    /// control": they use Invoke when the element has it, and otherwise fall
    /// through to Toggle and then the MSAA default action. That matters because
    /// a checkbox usually exposes only Toggle while a Win32/VCL button exposes
    /// only Invoke or the legacy default action, and a caller describing its
    /// intent should not have to know which of the three a given toolkit
    /// implemented.
    pub fn invoke_element_action(&self, element_id: &str, action: &str) -> Result<(), GuiError> {
        let element = self.lookup(element_id)?;
        let normalized = action.trim().to_ascii_lowercase().replace('-', "_");
        let verb = match normalized.as_str() {
            "press" | "click" | "tap" | "do_default_action" => "activate",
            other => other,
        };

        unsafe {
            match verb {
                "activate" => {
                    if let Some(invoke) =
                        Self::pattern::<IUIAutomationInvokePattern>(&element, UIA_InvokePatternId)
                    {
                        invoke
                            .Invoke()
                            .map_err(|e| GuiError::PlatformError(format!("Invoke: {e}")))?;
                    } else if let Some(toggle) =
                        Self::pattern::<IUIAutomationTogglePattern>(&element, UIA_TogglePatternId)
                    {
                        toggle
                            .Toggle()
                            .map_err(|e| GuiError::PlatformError(format!("Toggle: {e}")))?;
                    } else if let Some(legacy) = Self::pattern::<
                        IUIAutomationLegacyIAccessiblePattern,
                    >(
                        &element, UIA_LegacyIAccessiblePatternId
                    ) {
                        legacy.DoDefaultAction().map_err(|e| {
                            GuiError::PlatformError(format!("DoDefaultAction: {e}"))
                        })?;
                    } else {
                        return Err(Self::unsupported_action(element_id, action, &element));
                    }
                }
                "invoke" => {
                    let invoke =
                        Self::pattern::<IUIAutomationInvokePattern>(&element, UIA_InvokePatternId)
                            .ok_or_else(|| {
                                Self::unsupported_action(element_id, action, &element)
                            })?;
                    invoke
                        .Invoke()
                        .map_err(|e| GuiError::PlatformError(format!("Invoke: {e}")))?;
                }
                "toggle" => {
                    let toggle =
                        Self::pattern::<IUIAutomationTogglePattern>(&element, UIA_TogglePatternId)
                            .ok_or_else(|| {
                                Self::unsupported_action(element_id, action, &element)
                            })?;
                    toggle
                        .Toggle()
                        .map_err(|e| GuiError::PlatformError(format!("Toggle: {e}")))?;
                }
                "expand" | "collapse" => {
                    let expand = Self::pattern::<IUIAutomationExpandCollapsePattern>(
                        &element,
                        UIA_ExpandCollapsePatternId,
                    )
                    .ok_or_else(|| Self::unsupported_action(element_id, action, &element))?;
                    if verb == "expand" {
                        expand.Expand()
                    } else {
                        expand.Collapse()
                    }
                    .map_err(|e| GuiError::PlatformError(format!("ExpandCollapse: {e}")))?;
                }
                "select" => {
                    let selection = Self::pattern::<IUIAutomationSelectionItemPattern>(
                        &element,
                        UIA_SelectionItemPatternId,
                    )
                    .ok_or_else(|| Self::unsupported_action(element_id, action, &element))?;
                    selection
                        .Select()
                        .map_err(|e| GuiError::PlatformError(format!("Select: {e}")))?;
                }
                "increment" | "decrement" => {
                    // UIA's RangeValue pattern has no Increment/Decrement
                    // method — it publishes a numeric value plus the control's
                    // own small change, so the step comes from the control and
                    // the result is clamped to the range it declares. Spinning
                    // a number that way is what the arrow buttons on a VCL edit
                    // box do.
                    let range = Self::pattern::<IUIAutomationRangeValuePattern>(
                        &element,
                        UIA_RangeValuePatternId,
                    )
                    .ok_or_else(|| Self::unsupported_action(element_id, action, &element))?;
                    let current = range
                        .CurrentValue()
                        .map_err(|e| GuiError::PlatformError(format!("CurrentValue: {e}")))?;
                    let small_change = range.CurrentSmallChange().unwrap_or(1.0);
                    let step = if small_change.is_finite() && small_change != 0.0 {
                        small_change
                    } else {
                        1.0
                    };
                    let wanted = if verb == "increment" {
                        current + step
                    } else {
                        current - step
                    };
                    let minimum = range.CurrentMinimum().unwrap_or(wanted);
                    let maximum = range.CurrentMaximum().unwrap_or(wanted);
                    let target = if minimum <= maximum {
                        wanted.clamp(minimum, maximum)
                    } else {
                        wanted
                    };
                    range
                        .SetValue(target)
                        .map_err(|e| GuiError::PlatformError(format!("SetValue(range): {e}")))?;
                }
                "scroll_into_view" => {
                    let scroll = Self::pattern::<IUIAutomationScrollItemPattern>(
                        &element,
                        UIA_ScrollItemPatternId,
                    )
                    .ok_or_else(|| Self::unsupported_action(element_id, action, &element))?;
                    scroll
                        .ScrollIntoView()
                        .map_err(|e| GuiError::PlatformError(format!("ScrollIntoView: {e}")))?;
                }
                "focus" => element
                    .SetFocus()
                    .map_err(|e| GuiError::PlatformError(format!("SetFocus: {e}")))?,
                "close" | "minimize" | "maximize" | "restore" => {
                    let window =
                        Self::pattern::<IUIAutomationWindowPattern>(&element, UIA_WindowPatternId)
                            .ok_or_else(|| {
                                Self::unsupported_action(element_id, action, &element)
                            })?;
                    let result = match verb {
                        "close" => window.Close(),
                        "minimize" => window.SetWindowVisualState(WindowVisualState_Minimized),
                        "maximize" => window.SetWindowVisualState(WindowVisualState_Maximized),
                        _ => window.SetWindowVisualState(WindowVisualState_Normal),
                    };
                    result.map_err(|e| {
                        GuiError::PlatformError(format!("WindowPattern {verb}: {e}"))
                    })?;
                }
                other => {
                    let supported = Self::get_element_actions(&element);
                    return Err(GuiError::PlatformError(format!(
                        "unrecognized action '{other}'; this element supports: {}",
                        if supported.is_empty() {
                            "none — this element exposes no actionable UIA pattern".to_string()
                        } else {
                            supported.join(", ")
                        }
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
