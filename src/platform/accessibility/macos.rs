use std::collections::HashMap;

use core_foundation::array::CFArray;
use core_foundation::base::{CFIndex, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::display::CGRect;

use crate::error::GuiError;
use crate::gui::types::*;

pub struct PlatformAccessibility;

extern "C" {
    fn AXUIElementCreateApplication(pid: u32) -> *mut objc::runtime::Object;
    fn AXUIElementCopyAttributeValue(
        element: *mut objc::runtime::Object,
        attribute: *mut objc::runtime::Object,
        value: *mut *mut objc::runtime::Object,
    ) -> i32;
    fn AXUIElementCopyAttributeNames(
        element: *mut objc::runtime::Object,
        names: *mut *mut objc::runtime::Object,
    ) -> i32;
    fn AXUIElementCopyActionNames(
        element: *mut objc::runtime::Object,
        names: *mut *mut objc::runtime::Object,
    ) -> i32;
    fn AXUIElementIsAttributeSettable(
        element: *mut objc::runtime::Object,
        attribute: *mut objc::runtime::Object,
        settable: *mut u8,
    ) -> i32;
    fn CFArrayGetCount(array: *mut objc::runtime::Object) -> CFIndex;
    fn CFArrayGetValueAtIndex(
        array: *mut objc::runtime::Object,
        index: CFIndex,
    ) -> *mut objc::runtime::Object;
    fn CFRelease(obj: *mut objc::runtime::Object);
}

impl PlatformAccessibility {
    pub fn new() -> Result<Self, GuiError> {
        Ok(Self)
    }

    fn get_ax_focused_app() -> Result<u32, GuiError> {
        unsafe {
            let workspace = objc::msg_send![objc::class!(NSWorkspace), alloc];
            let workspace: *mut objc::runtime::Object = objc::msg_send![workspace, init];
            let app: *mut objc::runtime::Object = objc::msg_send![workspace, frontmostApplication];
            let pid: i32 = objc::msg_send![app, processIdentifier];
            let _: () = objc::msg_send![workspace, release];
            Ok(pid as u32)
        }
    }

    fn get_ax_app_ref(pid: u32) -> Result<*mut objc::runtime::Object, GuiError> {
        unsafe {
            let app = AXUIElementCreateApplication(pid);
            if app.is_null() {
                return Err(GuiError::PlatformError(format!(
                    "AXUIElementCreateApplication failed for PID {pid}"
                )));
            }
            Ok(app)
        }
    }

    fn get_ax_windows(
        app: *mut objc::runtime::Object,
    ) -> Result<Vec<*mut objc::runtime::Object>, GuiError> {
        unsafe {
            let attr = CFString::new("AXWindows");
            let mut windows_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                app,
                attr.as_concrete_TypeRef(),
                &mut windows_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result != 0 || windows_ref.is_null() {
                return Ok(vec![]);
            }

            let count = CFArrayGetCount(windows_ref as *mut _);
            let mut windows = Vec::new();
            for i in 0..count {
                let win = CFArrayGetValueAtIndex(windows_ref as *mut _, i);
                if !win.is_null() {
                    windows.push(win as *mut _);
                }
            }
            CFRelease(windows_ref as *mut _);
            Ok(windows)
        }
    }

    fn get_ax_children(
        element: *mut objc::runtime::Object,
    ) -> Result<Vec<*mut objc::runtime::Object>, GuiError> {
        unsafe {
            let attr = CFString::new("AXChildren");
            let mut children_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef(),
                &mut children_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result != 0 || children_ref.is_null() {
                return Ok(vec![]);
            }

            let count = CFArrayGetCount(children_ref as *mut _);
            let mut children = Vec::new();
            for i in 0..count {
                let child = CFArrayGetValueAtIndex(children_ref as *mut _, i);
                if !child.is_null() {
                    children.push(child as *mut _);
                }
            }
            CFRelease(children_ref as *mut _);
            Ok(children)
        }
    }

    fn get_ax_string_attribute(
        element: *mut objc::runtime::Object,
        attr: &CFString,
    ) -> Option<String> {
        unsafe {
            let mut val_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef(),
                &mut val_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result == 0 && !val_ref.is_null() {
                let cf_str = CFString::wrap_under_get_rule(val_ref as *mut _);
                let s = cf_str.to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
            None
        }
    }

    fn get_ax_number_attribute(
        element: *mut objc::runtime::Object,
        attr: &CFString,
    ) -> Option<f64> {
        unsafe {
            let mut val_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef(),
                &mut val_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result == 0 && !val_ref.is_null() {
                let num = CFNumber::wrap_under_get_rule(val_ref as *mut _);
                let mut val: f64 = 0.0;
                if num.get_value(&mut val) {
                    return Some(val);
                }
            }
            None
        }
    }

    fn get_ax_bool_attribute(element: *mut objc::runtime::Object, attr: &CFString) -> Option<bool> {
        unsafe {
            let mut val_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef(),
                &mut val_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result == 0 && !val_ref.is_null() {
                let boolean = CFBoolean::wrap_under_get_rule(val_ref as *mut _);
                return Some(boolean == CFBoolean::true_value());
            }
            None
        }
    }

    fn get_ax_position(element: *mut objc::runtime::Object) -> Option<(f64, f64)> {
        unsafe {
            let attr = CFString::new("AXPosition");
            let mut val_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef(),
                &mut val_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result == 0 && !val_ref.is_null() {
                let dict = CFDictionary::wrap_under_get_rule(val_ref as *mut _);
                let point = CGRect::from_dict(&dict);
                return Some((point.origin.x, point.origin.y));
            }
            None
        }
    }

    fn get_ax_size(element: *mut objc::runtime::Object) -> Option<(f64, f64)> {
        unsafe {
            let attr = CFString::new("AXSize");
            let mut val_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef(),
                &mut val_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result == 0 && !val_ref.is_null() {
                let dict = CFDictionary::wrap_under_get_rule(val_ref as *mut _);
                let size = CGRect::from_dict(&dict).origin; // abuse CGRect to get CGSize values
                return Some((size.x, size.y));
            }
            None
        }
    }

    fn get_ax_actions(element: *mut objc::runtime::Object) -> Vec<String> {
        unsafe {
            let mut names_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyActionNames(
                element,
                &mut names_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result != 0 || names_ref.is_null() {
                return vec![];
            }

            let count = CFArrayGetCount(names_ref as *mut _);
            let mut actions = Vec::new();
            for i in 0..count {
                let name = CFArrayGetValueAtIndex(names_ref as *mut _, i);
                if !name.is_null() {
                    let cf_str = CFString::wrap_under_get_rule(name as *mut _);
                    actions.push(cf_str.to_string());
                }
            }
            CFRelease(names_ref as *mut _);
            actions
        }
    }

    fn get_ax_states(element: *mut objc::runtime::Object) -> Vec<String> {
        let mut states = Vec::new();

        if let Some(focused) = Self::get_ax_bool_attribute(element, &CFString::new("AXFocused")) {
            if focused {
                states.push("focused".to_string());
            }
        }
        if Self::get_ax_bool_attribute(element, &CFString::new("AXEnabled"))
            .map(|v| !v)
            .unwrap_or(false)
        {
            states.push("disabled".to_string());
        }
        if Self::get_ax_bool_attribute(element, &CFString::new("AXExpanded")).unwrap_or(false) {
            states.push("expanded".to_string());
        }

        // Check if there's a value that indicates checked state
        let value_attr = CFString::new("AXValue");
        if let Some(val) = Self::get_ax_string_attribute(element, &value_attr) {
            if val == "1" || val.to_lowercase() == "true" {
                states.push("checked".to_string());
            }
        }

        states
    }

    fn get_ax_element_id(element: *mut objc::runtime::Object) -> String {
        let pid = unsafe { AXUIElementGetWindow(element) };
        format!("ax:{}", pid)
    }

    fn build_ax_node(
        element: *mut objc::runtime::Object,
        depth: u32,
    ) -> Result<AccessibilityNode, GuiError> {
        let id = Self::get_ax_element_id(element);
        let role =
            Self::get_ax_string_attribute(element, &CFString::new("AXRole")).unwrap_or_default();
        let name = Self::get_ax_string_attribute(element, &CFString::new("AXTitle"))
            .or_else(|| Self::get_ax_string_attribute(element, &CFString::new("AXDescription")));
        let value = Self::get_ax_string_attribute(element, &CFString::new("AXValue"));
        let description = Self::get_ax_string_attribute(element, &CFString::new("AXHelp"));
        let actions = Self::get_ax_actions(element);
        let states = Self::get_ax_states(element);

        let (x, y, w, h) = Self::get_ax_position(element)
            .map(|(px, py)| {
                let (sw, sh) = Self::get_ax_size(element).unwrap_or((0.0, 0.0));
                (px as i32, py as i32, sw as u32, sh as u32)
            })
            .unwrap_or((0, 0, 0, 0));

        let cx = if w > 0 { x + w as i32 / 2 } else { None };
        let cy = if h > 0 { y + h as i32 / 2 } else { None };

        let mut children = Vec::new();
        if depth > 0 {
            if let Ok(child_refs) = Self::get_ax_children(element) {
                for child_ref in child_refs {
                    if let Ok(node) = Self::build_ax_node(child_ref, depth - 1) {
                        children.push(node);
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

    pub fn get_tree(
        &self,
        window_id: Option<u64>,
        max_depth: Option<u32>,
    ) -> Result<AccessibilityNode, GuiError> {
        let pid = if let Some(wid) = window_id {
            // Look up the PID for this window from the window manager
            use crate::platform::window::PlatformWindowManager;
            let wm = PlatformWindowManager::new()?;
            let windows = wm.list_windows()?;
            let win = windows
                .iter()
                .find(|w| w.id == wid)
                .ok_or(GuiError::PlatformError(format!("Window {wid} not found")))?;
            win.process_id
                .ok_or(GuiError::PlatformError("Window has no PID".into()))? as u32
        } else {
            Self::get_ax_focused_app()?
        };

        let depth = max_depth.unwrap_or(10);
        let app = Self::get_ax_app_ref(pid)?;
        let windows = Self::get_ax_windows(app)?;

        if let Some(ax_win) = windows.first() {
            let node = Self::build_ax_node(*ax_win, depth)?;
            unsafe {
                CFRelease(app as *mut _);
            }
            return Ok(node);
        }

        unsafe {
            CFRelease(app as *mut _);
        }
        Err(GuiError::PlatformError(
            "No accessible windows found".into(),
        ))
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
            "get_element_properties by ID not yet implemented on macOS".into(),
        ))
    }

    pub fn invoke_element_action(&self, _element_id: &str, _action: &str) -> Result<(), GuiError> {
        Err(GuiError::UnsupportedCapability(
            "invoke_element_action not yet implemented on macOS".into(),
        ))
    }
}
