use std::ffi::c_void;

use core_foundation::array::CFArray;
use core_foundation::base::{CFIndex, CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::window::{
    kCGNullWindowID, kCGWindowListOptionOnScreenOnly, CGWindowListCopyWindowInfo,
};
use core_graphics::window::{
    kCGWindowBounds, kCGWindowLayer, kCGWindowName, kCGWindowNumber, kCGWindowOwnerName,
    kCGWindowOwnerPID,
};
use objc::{msg_send, sel, class};

use crate::error::GuiError;
use crate::gui::types::*;

pub struct PlatformWindowManager;

impl PlatformWindowManager {
    pub fn new() -> Result<Self, GuiError> {
        Ok(Self)
    }

    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        let mut windows = Vec::new();

        unsafe {
            let list_ref =
                CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, kCGNullWindowID);

            if list_ref.is_null() {
                return Err(GuiError::PlatformError(
                    "CGWindowListCopyWindowInfo returned null".into(),
                ));
            }

            let array = CFArray::<CFDictionary<CFString, CFType>>::wrap_under_create_rule(list_ref);
            let count = array.len();

            for i in 0..count {
                if let Some(dict_ref) = array.get(i) {
                    let dict: &CFDictionary<CFString, CFType> = &*dict_ref;

                    let window_id = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowNumber) })
                        .and_then(|v| v.downcast::<CFNumber>())
                        .and_then(|n| n.to_i64())
                        .unwrap_or(0) as u64;

                    let layer = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowLayer) })
                        .and_then(|v| v.downcast::<CFNumber>())
                        .and_then(|n| n.to_i32())
                        .unwrap_or(0);

                    if layer != 0 {
                        continue;
                    }

                    let title = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowName) })
                        .and_then(|v| v.downcast::<CFString>())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    if title.is_empty() {
                        continue;
                    }

                    let process_name = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowOwnerName) })
                        .and_then(|v| v.downcast::<CFString>())
                        .map(|s| s.to_string());

                    let process_id = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowOwnerPID) })
                        .and_then(|v| v.downcast::<CFNumber>())
                        .and_then(|n| n.to_i32())
                        .map(|pid| pid as u32);

                    let (x, y, w, h) = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowBounds) })
                        .and_then(|v| extract_rect_from_dict_value(v))
                        .unwrap_or((0.0, 0.0, 0.0, 0.0));

                    windows.push(WindowInfo {
                        id: window_id,
                        title,
                        x: x as i32,
                        y: y as i32,
                        width: w as u32,
                        height: h as u32,
                        is_minimized: false,
                        is_maximized: false,
                        is_focused: false,
                        process_name,
                        process_id,
                    });
                }
            }
        }

        if let Ok(focused_pid) = self.get_frontmost_pid() {
            if let Some(pos) = windows
                .iter()
                .position(|w| w.process_id == Some(focused_pid))
            {
                windows[pos].is_focused = true;
            }
        }

        Ok(windows)
    }

    fn get_frontmost_pid(&self) -> Result<u32, GuiError> {
        unsafe {
            let workspace: *mut objc::runtime::Object = msg_send![class!(NSWorkspace), alloc];
            let workspace: *mut objc::runtime::Object = msg_send![workspace, init];
            let app: *mut objc::runtime::Object = msg_send![workspace, frontmostApplication];
            let pid: i32 = msg_send![app, processIdentifier];
            let _: () = msg_send![workspace, release];
            Ok(pid as u32)
        }
    }

    fn ax_get_window_list(&self, pid: u32) -> Result<Vec<u64>, GuiError> {
        unsafe {
            let app = AXUIElementCreateApplication(pid);
            if app.is_null() {
                return Ok(vec![]);
            }

            let cf_string = CFString::new("AXWindows");
            let mut windows_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                app as *const c_void,
                cf_string.as_concrete_TypeRef() as *const c_void,
                &mut windows_ref,
            );

            CFRelease(app as *const c_void);

            if result != 0 || windows_ref.is_null() {
                return Ok(vec![]);
            }

            let mut ids = Vec::new();
            let count: CFIndex = CFArrayGetCount(windows_ref);
            for i in 0..count {
                let ax_window = CFArrayGetValueAtIndex(windows_ref, i);
                if !ax_window.is_null() {
                    let wid = ax_get_window_id(ax_window);
                    if wid > 0 {
                        ids.push(wid as u64);
                    }
                }
            }
            CFRelease(windows_ref);
            Ok(ids)
        }
    }

    fn ax_get_attribute_i32(
        &self,
        ax_window: *mut c_void,
        attribute: &CFString,
    ) -> Result<i32, GuiError> {
        unsafe {
            let mut val_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                ax_window as *const c_void,
                attribute.as_concrete_TypeRef() as *const c_void,
                &mut val_ref,
            );
            if result != 0 || val_ref.is_null() {
                return Err(GuiError::PlatformError("AX attribute not found".into()));
            }
            let num = CFNumber::wrap_under_get_rule(val_ref as *mut _);
            num.to_i32().ok_or(GuiError::PlatformError("AX attribute not a number".into()))
        }
    }

    fn ax_set_attribute_i32(
        &self,
        ax_window: *mut c_void,
        attribute: &CFString,
        value: i32,
    ) -> Result<(), GuiError> {
        unsafe {
            let num = CFNumber::from(value);
            let result = AXUIElementSetAttributeValue(
                ax_window as *const c_void,
                attribute.as_concrete_TypeRef() as *const c_void,
                num.as_concrete_TypeRef() as *const c_void,
            );
            if result != 0 {
                return Err(GuiError::PlatformError(format!(
                    "AX set attribute failed: {result}"
                )));
            }
            Ok(())
        }
    }

    fn ax_set_point(
        &self,
        ax_window: *mut c_void,
        attribute: &CFString,
        x: f32,
        y: f32,
    ) -> Result<(), GuiError> {
        unsafe {
            let dict = create_point_dict(x as f64, y as f64);
            let result = AXUIElementSetAttributeValue(
                ax_window as *const c_void,
                attribute.as_concrete_TypeRef() as *const c_void,
                dict.as_concrete_TypeRef() as *const c_void,
            );
            if result != 0 {
                return Err(GuiError::PlatformError(format!(
                    "AX set point failed: {result}"
                )));
            }
            Ok(())
        }
    }

    fn ax_set_size(
        &self,
        ax_window: *mut c_void,
        attribute: &CFString,
        w: f32,
        h: f32,
    ) -> Result<(), GuiError> {
        unsafe {
            let dict = create_size_dict(w as f64, h as f64);
            let result = AXUIElementSetAttributeValue(
                ax_window as *const c_void,
                attribute.as_concrete_TypeRef() as *const c_void,
                dict.as_concrete_TypeRef() as *const c_void,
            );
            if result != 0 {
                return Err(GuiError::PlatformError(format!(
                    "AX set size failed: {result}"
                )));
            }
            Ok(())
        }
    }

    fn ax_perform_action(
        &self,
        ax_window: *mut c_void,
        action: &CFString,
    ) -> Result<(), GuiError> {
        unsafe {
            let result = AXUIElementPerformAction(
                ax_window as *const c_void,
                action.as_concrete_TypeRef() as *const c_void,
            );
            if result != 0 {
                return Err(GuiError::PlatformError(format!(
                    "AX perform action failed: {result}"
                )));
            }
            Ok(())
        }
    }

    fn with_window_ax<F>(&self, window_id: u64, mut f: F) -> Result<(), GuiError>
    where
        F: FnMut(*mut c_void) -> Result<(), GuiError>,
    {
        let windows = self.list_windows()?;
        let win = windows
            .iter()
            .find(|w| w.id == window_id)
            .ok_or(GuiError::PlatformError(format!(
                "Window {} not found",
                window_id
            )))?;

        let pid = win.process_id.ok_or(GuiError::PlatformError(
            "Window has no associated PID".into(),
        ))?;

        unsafe {
            let app = AXUIElementCreateApplication(pid);
            if app.is_null() {
                return Err(GuiError::PlatformError(format!(
                    "AXUIElementCreateApplication failed for PID {pid}"
                )));
            }

            let ax_windows = CFString::new("AXWindows");
            let mut windows_ref: *mut c_void = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                app as *const c_void,
                ax_windows.as_concrete_TypeRef() as *const c_void,
                &mut windows_ref,
            );

            CFRelease(app as *const c_void);

            if result != 0 || windows_ref.is_null() {
                return Err(GuiError::PlatformError(
                    "Failed to get AX windows list".into(),
                ));
            }

            let count: CFIndex = CFArrayGetCount(windows_ref);
            let mut found = false;

            for i in 0..count {
                let ax_win = CFArrayGetValueAtIndex(windows_ref, i);
                if ax_win.is_null() {
                    continue;
                }

                let wid = ax_get_window_id(ax_win);
                if wid as u64 == window_id {
                    f(ax_win)?;
                    found = true;
                    break;
                }
            }

            CFRelease(windows_ref);

            if !found {
                return Err(GuiError::PlatformError(format!(
                    "AX window {window_id} not found"
                )));
            }

            Ok(())
        }
    }

    pub fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        let windows = self.list_windows()?;
        let focused_pid = self.get_frontmost_pid()?;
        windows
            .into_iter()
            .find(|w| w.process_id == Some(focused_pid))
            .ok_or(GuiError::PlatformError("No active window found".into()))
    }

    pub fn focus_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.with_window_ax(window_id, |ax_win| {
            unsafe {
                let focused = CFString::new("AXFocused");
                let true_val = CFBoolean::true_value();
                let result = AXUIElementSetAttributeValue(
                    ax_win as *const c_void,
                    focused.as_concrete_TypeRef() as *const c_void,
                    true_val.as_concrete_TypeRef() as *const c_void,
                );
                if result != 0 {
                    return Err(GuiError::PlatformError(format!(
                        "AX set focused failed: {result}"
                    )));
                }
            }
            let raise = CFString::new("AXRaise");
            let _ = self.ax_perform_action(ax_win, &raise);
            Ok(())
        })
    }

    pub fn move_window(&self, window_id: u64, x: i32, y: i32) -> Result<(), GuiError> {
        let position = CFString::new("AXPosition");
        self.with_window_ax(window_id, |ax_win| {
            self.ax_set_point(ax_win, &position, x as f32, y as f32)
        })
    }

    pub fn resize_window(&self, window_id: u64, width: u32, height: u32) -> Result<(), GuiError> {
        let size = CFString::new("AXSize");
        self.with_window_ax(window_id, |ax_win| {
            self.ax_set_size(ax_win, &size, width as f32, height as f32)
        })
    }

    pub fn minimize_window(&self, window_id: u64) -> Result<(), GuiError> {
        let _minimize = CFString::new("AXPressMiniaturize");
        let mini_button = CFString::new("AXMiniaturizeButton");
        self.with_window_ax(window_id, |ax_win| {
            let result = self.ax_perform_action(ax_win, &mini_button);
            if result.is_err() {
                let mini = CFString::new("AXMiniaturize");
                self.ax_perform_action(ax_win, &mini)
            } else {
                result
            }
        })
    }

    pub fn maximize_window(&self, window_id: u64) -> Result<(), GuiError> {
        let zoom = CFString::new("AXZoom");
        self.with_window_ax(window_id, |ax_win| self.ax_perform_action(ax_win, &zoom))
    }

    pub fn restore_window(&self, window_id: u64) -> Result<(), GuiError> {
        let demini = CFString::new("AXDeminiaturize");
        let _ = self.with_window_ax(window_id, |ax_win| self.ax_perform_action(ax_win, &demini));
        let zoom = CFString::new("AXZoom");
        self.with_window_ax(window_id, |ax_win| self.ax_perform_action(ax_win, &zoom))
    }

    pub fn close_window(&self, window_id: u64) -> Result<(), GuiError> {
        let close = CFString::new("AXPressCloseButton");
        let close_button = CFString::new("AXCloseButton");
        self.with_window_ax(window_id, |ax_win| {
            let result = self.ax_perform_action(ax_win, &close_button);
            if result.is_err() {
                self.ax_perform_action(ax_win, &close)
            } else {
                result
            }
        })
    }

    pub fn get_window_title(&self, window_id: u64) -> Result<String, GuiError> {
        let title_attr = CFString::new("AXTitle");
        let mut result = Err(GuiError::PlatformError("Window not found".into()));

        let _ = self.with_window_ax(window_id, |ax_win| {
            unsafe {
                let mut val_ref: *mut c_void = std::ptr::null_mut();
                let r = AXUIElementCopyAttributeValue(
                    ax_win as *const c_void,
                    title_attr.as_concrete_TypeRef() as *const c_void,
                    &mut val_ref,
                );
                if r == 0 && !val_ref.is_null() {
                    let cf_str = CFString::wrap_under_get_rule(val_ref as *mut _);
                    result = Ok(cf_str.to_string());
                }
            }
            Ok(())
        });

        result
    }

    pub fn get_window_bounds(&self, window_id: u64) -> Result<Region, GuiError> {
        let position_attr = CFString::new("AXPosition");
        let size_attr = CFString::new("AXSize");
        let mut region = Region {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        self.with_window_ax(window_id, |ax_win| {
            unsafe {
                let mut pos_ref: *mut c_void = std::ptr::null_mut();
                if AXUIElementCopyAttributeValue(
                    ax_win as *const c_void,
                    position_attr.as_concrete_TypeRef() as *const c_void,
                    &mut pos_ref,
                ) == 0
                {
                    let pos_dict = CFDictionary::<CFString, CFType>::wrap_under_get_rule(pos_ref as *mut _);
                    if let Some((px, py)) = dict_to_point(&pos_dict) {
                        region.x = px as u32;
                        region.y = py as u32;
                    }
                }

                let mut size_ref: *mut c_void = std::ptr::null_mut();
                if AXUIElementCopyAttributeValue(
                    ax_win as *const c_void,
                    size_attr.as_concrete_TypeRef() as *const c_void,
                    &mut size_ref,
                ) == 0
                {
                    let size_dict = CFDictionary::<CFString, CFType>::wrap_under_get_rule(size_ref as *mut _);
                    if let Some((sw, sh)) = dict_to_size(&size_dict) {
                        region.width = sw as u32;
                        region.height = sh as u32;
                    }
                }
            }
            Ok(())
        })?;

        Ok(region)
    }

    pub fn find_windows_by_title(&self, query: &str) -> Result<Vec<WindowInfo>, GuiError> {
        let all = self.list_windows()?;
        let query_lower = query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|w| w.title.to_lowercase().contains(&query_lower))
            .collect())
    }
}

fn extract_rect_from_dict_value(val: &CFType) -> Option<(f64, f64, f64, f64)> {
    let dict = val.downcast::<CFDictionary>()?;
    unsafe {
        let x = dict
            .find(CFString::new("X").as_CFTypeRef())
            .and_then(|v| CFNumber::wrap_under_get_rule(*v as *const _).to_f64())?;
        let y = dict
            .find(CFString::new("Y").as_CFTypeRef())
            .and_then(|v| CFNumber::wrap_under_get_rule(*v as *const _).to_f64())?;
        let w = dict
            .find(CFString::new("Width").as_CFTypeRef())
            .and_then(|v| CFNumber::wrap_under_get_rule(*v as *const _).to_f64())?;
        let h = dict
            .find(CFString::new("Height").as_CFTypeRef())
            .and_then(|v| CFNumber::wrap_under_get_rule(*v as *const _).to_f64())?;
        Some((x, y, w, h))
    }
}

fn dict_to_point(dict: &CFDictionary<CFString, CFType>) -> Option<(f64, f64)> {
    let x = dict.find(&CFString::new("X"))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())?;
    let y = dict.find(&CFString::new("Y"))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())?;
    Some((x, y))
}

fn dict_to_size(dict: &CFDictionary<CFString, CFType>) -> Option<(f64, f64)> {
    let w = dict.find(&CFString::new("Width"))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())?;
    let h = dict.find(&CFString::new("Height"))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())?;
    Some((w, h))
}

fn create_point_dict(x: f64, y: f64) -> CFDictionary<CFString, CFType> {
    CFDictionary::from_CFType_pairs(&[
        (CFString::new("X"), CFNumber::from(x).as_CFType()),
        (CFString::new("Y"), CFNumber::from(y).as_CFType()),
    ])
}

fn create_size_dict(w: f64, h: f64) -> CFDictionary<CFString, CFType> {
    CFDictionary::from_CFType_pairs(&[
        (CFString::new("Width"), CFNumber::from(w).as_CFType()),
        (CFString::new("Height"), CFNumber::from(h).as_CFType()),
    ])
}

fn ax_get_window_id(ax_win: *mut c_void) -> u32 {
    unsafe {
        let attr = CFString::new("AXWindowID");
        let mut wid_ref: *mut c_void = std::ptr::null_mut();
        let result = AXUIElementCopyAttributeValue(
            ax_win as *const c_void,
            attr.as_concrete_TypeRef() as *const c_void,
            &mut wid_ref,
        );
        if result == 0 && !wid_ref.is_null() {
            let num = CFNumber::wrap_under_get_rule(wid_ref as *mut _);
            num.to_i32().unwrap_or(0) as u32
        } else {
            0
        }
    }
}

extern "C" {
    fn AXUIElementCreateApplication(pid: u32) -> *mut c_void;
    fn AXUIElementCopyAttributeValue(
        element: *const c_void,
        attribute: *const c_void,
        value: *mut *mut c_void,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: *const c_void,
        attribute: *const c_void,
        value: *const c_void,
    ) -> i32;
    fn AXUIElementPerformAction(
        element: *const c_void,
        action: *const c_void,
    ) -> i32;
    fn CFArrayGetCount(array: *const c_void) -> CFIndex;
    fn CFArrayGetValueAtIndex(
        array: *const c_void,
        index: CFIndex,
    ) -> *mut c_void;
    fn CFRelease(obj: *const c_void);
}
