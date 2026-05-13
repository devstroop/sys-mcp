use std::ffi::c_void;

use core_foundation::base::{CFIndex, CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use objc::{msg_send, sel, sel_impl, class};

use crate::error::GuiError;
use crate::gui::types::*;

pub struct PlatformAccessibility;

extern "C" {
    fn AXUIElementCreateApplication(pid: u32) -> *mut c_void;
    fn AXUIElementCopyAttributeValue(
        element: *const c_void,
        attribute: *const c_void,
        value: *mut *mut c_void,
    ) -> i32;
    fn AXUIElementCopyActionNames(
        element: *const c_void,
        names: *mut *mut c_void,
    ) -> i32;
    fn CFArrayGetCount(array: *const c_void) -> CFIndex;
    fn CFArrayGetValueAtIndex(
        array: *const c_void,
        index: CFIndex,
    ) -> *mut c_void;
    fn CFRelease(obj: *const c_void);
}

impl PlatformAccessibility {
    pub fn new() -> Result<Self, GuiError> {
        Ok(Self)
    }

    fn get_ax_focused_app() -> Result<u32, GuiError> {
        #[allow(unexpected_cfgs)]
        unsafe {
            let workspace: *mut objc::runtime::Object = msg_send![class!(NSWorkspace), alloc];
            let workspace: *mut objc::runtime::Object = msg_send![workspace, init];
            let app: *mut objc::runtime::Object = msg_send![workspace, frontmostApplication];
            let pid: i32 = msg_send![app, processIdentifier];
            let _: () = msg_send![workspace, release];
            Ok(pid as u32)
        }
    }

    fn get_ax_app_ref(pid: u32) -> Result<*mut c_void, GuiError> {
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
        app: *const c_void,
    ) -> Result<Vec<*mut c_void>, GuiError> {
        unsafe {
            let attr = CFString::new("AXWindows");
            let mut windows_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                app,
                attr.as_concrete_TypeRef() as *const c_void,
                &mut windows_ref,
            );
            if result != 0 || windows_ref.is_null() {
                return Ok(vec![]);
            }

            let count = CFArrayGetCount(windows_ref);
            let mut windows = Vec::new();
            for i in 0..count {
                let win = CFArrayGetValueAtIndex(windows_ref, i);
                if !win.is_null() {
                    windows.push(win);
                }
            }
            CFRelease(windows_ref);
            Ok(windows)
        }
    }

    fn get_ax_children(
        element: *const c_void,
    ) -> Result<Vec<*mut c_void>, GuiError> {
        unsafe {
            let attr = CFString::new("AXChildren");
            let mut children_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef() as *const c_void,
                &mut children_ref,
            );
            if result != 0 || children_ref.is_null() {
                return Ok(vec![]);
            }

            let count = CFArrayGetCount(children_ref);
            let mut children = Vec::new();
            for i in 0..count {
                let child = CFArrayGetValueAtIndex(children_ref, i);
                if !child.is_null() {
                    children.push(child);
                }
            }
            CFRelease(children_ref);
            Ok(children)
        }
    }

    fn get_ax_string_attribute(
        element: *const c_void,
        attr: &CFString,
    ) -> Option<String> {
        unsafe {
            let mut val_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef() as *const c_void,
                &mut val_ref,
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

    #[allow(dead_code)]
    fn get_ax_number_attribute(
        element: *const c_void,
        attr: &CFString,
    ) -> Option<f64> {
        unsafe {
            let mut val_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef() as *const c_void,
                &mut val_ref,
            );
            if result == 0 && !val_ref.is_null() {
                let num = CFNumber::wrap_under_get_rule(val_ref as *mut _);
                num.to_f64()
            } else {
                None
            }
        }
    }

    fn get_ax_bool_attribute(element: *const c_void, attr: &CFString) -> Option<bool> {
        unsafe {
            let mut val_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef() as *const c_void,
                &mut val_ref,
            );
            if result == 0 && !val_ref.is_null() {
                let boolean = CFBoolean::wrap_under_get_rule(val_ref as *mut _);
                return Some(boolean == CFBoolean::true_value());
            }
            None
        }
    }

    fn get_ax_position(element: *const c_void) -> Option<(f64, f64)> {
        unsafe {
            let attr = CFString::new("AXPosition");
            let mut val_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef() as *const c_void,
                &mut val_ref,
            );
            if result == 0 && !val_ref.is_null() {
                let dict = CFDictionary::<CFString, CFType>::wrap_under_get_rule(val_ref as *mut _);
                return dict_to_point(&dict);
            }
            None
        }
    }

    fn get_ax_size(element: *const c_void) -> Option<(f64, f64)> {
        unsafe {
            let attr = CFString::new("AXSize");
            let mut val_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                element,
                attr.as_concrete_TypeRef() as *const c_void,
                &mut val_ref,
            );
            if result == 0 && !val_ref.is_null() {
                let dict = CFDictionary::<CFString, CFType>::wrap_under_get_rule(val_ref as *mut _);
                return dict_to_size(&dict);
            }
            None
        }
    }

    fn get_ax_actions(element: *const c_void) -> Vec<String> {
        unsafe {
            let mut names_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyActionNames(
                element,
                &mut names_ref,
            );
            if result != 0 || names_ref.is_null() {
                return vec![];
            }

            let count = CFArrayGetCount(names_ref);
            let mut actions = Vec::new();
            for i in 0..count {
                let name = CFArrayGetValueAtIndex(names_ref, i);
                if !name.is_null() {
                    let cf_str = CFString::wrap_under_get_rule(name as *mut _);
                    actions.push(cf_str.to_string());
                }
            }
            CFRelease(names_ref);
            actions
        }
    }

    fn get_ax_states(element: *const c_void) -> Vec<String> {
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

        let value_attr = CFString::new("AXValue");
        if let Some(val) = Self::get_ax_string_attribute(element, &value_attr) {
            if val == "1" || val.to_lowercase() == "true" {
                states.push("checked".to_string());
            }
        }

        states
    }

    fn get_ax_element_id(element: *const c_void) -> String {
        format!("ax:{:p}", element)
    }

    fn build_ax_node(
        element: *const c_void,
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

        let cx = if w > 0 { Some(x + w as i32 / 2) } else { None };
        let cy = if h > 0 { Some(y + h as i32 / 2) } else { None };

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
                CFRelease(app as *const c_void);
            }
            return Ok(node);
        }

        unsafe {
            CFRelease(app as *const c_void);
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

    pub fn get_element_properties(&self, _element_id: &str) -> Result<AccessibilityNode, GuiError> {
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

fn dict_to_point(dict: &CFDictionary<CFString, CFType>) -> Option<(f64, f64)> {
    let x = dict.find(CFString::new("X"))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())?;
    let y = dict.find(CFString::new("Y"))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())?;
    Some((x, y))
}

fn dict_to_size(dict: &CFDictionary<CFString, CFType>) -> Option<(f64, f64)> {
    let w = dict.find(CFString::new("Width"))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())?;
    let h = dict.find(CFString::new("Height"))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())?;
    Some((w, h))
}
