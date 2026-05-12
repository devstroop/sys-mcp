use std::collections::HashMap;

use windows::core::{HSTRING, VARIANT};
use windows::Win32::Foundation::{BOOL, VARIANT_BOOL, VARIANT_TRUE};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationElementArray,
    TreeScope_Children, TreeScope_Descendants, TreeScope_Subtree, UIA_ButtonControlTypeId,
    UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId, UIA_ControlTypePropertyId,
    UIA_EditControlTypeId, UIA_ExpandCollapsePatternId, UIA_ExpandCollapseState,
    UIA_ExpandCollapseState_Collapsed, UIA_ExpandCollapseState_Expanded, UIA_HeaderControlTypeId,
    UIA_HyperlinkControlTypeId, UIA_ImageControlTypeId, UIA_InvokePatternId,
    UIA_LabelControlTypeId, UIA_ListControlTypeId, UIA_ListItemControlTypeId,
    UIA_MenuControlTypeId, UIA_MenuItemControlTypeId, UIA_NamePropertyId,
    UIA_RadioButtonControlTypeId, UIA_ScrollBarControlTypeId, UIA_SliderControlTypeId,
    UIA_StatusBarControlTypeId, UIA_TabControlTypeId, UIA_TableControlTypeId, UIA_TogglePatternId,
    UIA_ToggleState, UIA_ToolBarControlTypeId, UIA_TreeControlTypeId, UIA_TreeItemControlTypeId,
    UIA_ValuePatternId, UIA_WindowControlTypeId, UIA_WindowPatternId,
};

use crate::error::GuiError;
use crate::gui::types::*;

fn control_type_to_role(control_type: i32) -> String {
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
        UIA_LabelControlTypeId => "label".to_string(),
        UIA_WindowControlTypeId => "window".to_string(),
        _ => format!("control_type_{}", control_type),
    }
}

pub struct PlatformAccessibility {
    automation: IUIAutomation,
}

impl PlatformAccessibility {
    pub fn new() -> Result<Self, GuiError> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .map_err(|e| GuiError::PlatformError(format!("COM init failed: {e}")))?;
        }

        let automation: IUIAutomation = unsafe {
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| GuiError::PlatformError(format!("UIAutomation create failed: {e}")))?
        };

        Ok(Self { automation })
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

    fn get_element_control_type(element: &IUIAutomationElement) -> i32 {
        unsafe { element.CurrentControlType().unwrap_or(0) }
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
            // Check if enabled
            if let Ok(is_enabled) = element.CurrentIsEnabled() {
                if is_enabled == VARIANT_FALSE {
                    states.push("disabled".to_string());
                }
            }

            // Check toggle state
            if let Ok(toggle_pattern) = element.GetCurrentPattern(UIA_TogglePatternId) {
                if let Ok(toggle) = toggle_pattern.cast::<IUIAutomationTogglePattern>() {
                    if let Ok(state) = toggle.CurrentToggleState() {
                        match state {
                            UIA_ToggleState_On => states.push("checked".to_string()),
                            UIA_ToggleState_Indeterminate => {
                                states.push("indeterminate".to_string())
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Check expand/collapse state
            if let Ok(expand_pattern) = element.GetCurrentPattern(UIA_ExpandCollapsePatternId) {
                if let Ok(expand) = expand_pattern.cast::<IUIAutomationExpandCollapsePattern>() {
                    if let Ok(state) = expand.CurrentExpandCollapseState() {
                        if state == ExpandCollapseState::UIA_ExpandCollapseState_Expanded {
                            states.push("expanded".to_string());
                        }
                    }
                }
            }

            // Check if focused
            if let Ok(has_focus) = element.CurrentHasKeyboardFocus() {
                if has_focus == VARIANT_TRUE {
                    states.push("focused".to_string());
                }
            }
        }

        states
    }

    fn get_element_value(element: &IUIAutomationElement) -> Option<String> {
        unsafe {
            if let Ok(value_pattern) = element.GetCurrentPattern(UIA_ValuePatternId) {
                if let Ok(value) = value_pattern.cast::<IUIAutomationValuePattern>() {
                    let val = value.CurrentValue().ok()?;
                    let s = val.to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
            None
        }
    }

    fn get_element_actions(element: &IUIAutomationElement) -> Vec<String> {
        let mut actions = Vec::new();

        unsafe {
            if element.GetCurrentPattern(UIA_InvokePatternId).is_ok() {
                actions.push("invoke".to_string());
            }
            if element.GetCurrentPattern(UIA_TogglePatternId).is_ok() {
                actions.push("toggle".to_string());
            }
            if element
                .GetCurrentPattern(UIA_ExpandCollapsePatternId)
                .is_ok()
            {
                actions.push("expand".to_string());
                actions.push("collapse".to_string());
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
        element: &IUIAutomationElement,
        depth: u32,
    ) -> Result<AccessibilityNode, GuiError> {
        let name = Self::get_element_name(element);
        let role = control_type_to_role(Self::get_element_control_type(element));
        let value = Self::get_element_value(element);
        let actions = Self::get_element_actions(element);
        let states = Self::get_element_states(element);

        let (x, y, w, h) = Self::get_element_bounds(element);
        let cx = if w > 0 { Some(x + w as i32 / 2) } else { None };
        let cy = if h > 0 { Some(y + h as i32 / 2) } else { None };

        let id = format!("uia:{:p}", element as *const _ as usize);

        let mut children = Vec::new();
        if depth > 0 {
            unsafe {
                if let Ok(child_array) =
                    element.FindAll(TreeScope_Children, &self.automation.CreateTrueCondition()?)
                {
                    let count = child_array.Length()? as usize;
                    for i in 0..count {
                        if let Ok(child) = child_array.GetElement(i as u32) {
                            if let Ok(node) = Self::build_node(&child, depth - 1) {
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

        unsafe {
            let root = if let Some(wid) = window_id {
                // Get element from window handle
                let hwnd = windows::Win32::Foundation::HWND(wid as *mut _);
                self.automation.ElementFromHandle(hwnd).map_err(|e| {
                    GuiError::PlatformError(format!("ElementFromHandle failed: {e}"))
                })?
            } else {
                // Get the desktop-rooted element (focused app)
                let focused = self.automation.GetFocusedElement().map_err(|e| {
                    GuiError::PlatformError(format!("GetFocusedElement failed: {e}"))
                })?;

                // Walk up to the top-level window
                let mut current = focused;
                loop {
                    if let Ok(parent) = current.GetParentElement() {
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
                    } else {
                        break;
                    }
                }
                current
            };

            Self::build_node(&root, depth)
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

    pub fn get_element_properties(&self, element_id: &str) -> Result<AccessibilityNode, GuiError> {
        Err(GuiError::UnsupportedCapability(
            "get_element_properties by ID not yet implemented on Windows".into(),
        ))
    }

    pub fn invoke_element_action(&self, _element_id: &str, _action: &str) -> Result<(), GuiError> {
        Err(GuiError::UnsupportedCapability(
            "invoke_element_action not yet implemented on Windows".into(),
        ))
    }
}

impl Drop for PlatformAccessibility {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
