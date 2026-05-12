use core_foundation::array::CFArray;
use core_foundation::base::{CFIndex, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation::url::CFURL;
use core_graphics::display::{CGPoint, CGRect, CGSize};
use core_graphics::window::{CGWindowListCopyWindowInfo, kCGWindowListOptionOnScreenOnly, kCGNullWindowID};
use core_graphics::window::{kCGWindowName, kCGWindowOwnerName, kCGWindowNumber, kCGWindowBounds, kCGWindowOwnerPID, kCGWindowLayer};

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
            let list_ref = CGWindowListCopyWindowInfo(
                kCGWindowListOptionOnScreenOnly,
                kCGNullWindowID,
            );

            if list_ref.is_null() {
                return Err(GuiError::PlatformError(
                    "CGWindowListCopyWindowInfo returned null".into(),
                ));
            }

            let array = CFArray::<CFDictionary>::wrap_under_create_rule(list_ref as *mut _);
            let count = array.len() as CFIndex;

            for i in 0..count {
                if let Some(dict) = array.get(i as CFIndex) {
                    let window_id = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowNumber) })
                        .and_then(|v| v.downcast::<CFNumber>())
                        .and_then(|n| {
                            let mut val: i64 = 0;
                            n.get_value(&mut val).then_some(val as u64)
                        })
                        .unwrap_or(0);

                    let layer = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowLayer) })
                        .and_then(|v| v.downcast::<CFNumber>())
                        .and_then(|n| {
                            let mut val: i32 = 0;
                            n.get_value(&mut val).then_some(val)
                        })
                        .unwrap_or(0);

                    // Skip non-normal windows (desktop, dock, menu bar extras)
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
                        .and_then(|n| {
                            let mut val: i32 = 0;
                            n.get_value(&mut val).then_some(val as u32)
                        });

                    let bounds = dict
                        .find(unsafe { &CFString::wrap_under_get_rule(kCGWindowBounds) })
                        .and_then(|v| v.downcast::<CFDictionary>());

                    let (x, y, w, h) = bounds
                        .map(|b| {
                            let rect = CGRect::from_dict(b);
                            (
                                rect.origin.x as i32,
                                rect.origin.y as i32,
                                rect.size.width as u32,
                                rect.size.height as u32,
                            )
                        })
                        .unwrap_or((0, 0, 0, 0));

                    windows.push(WindowInfo {
                        id: window_id,
                        title,
                        x,
                        y,
                        width: w,
                        height: h,
                        is_minimized: false,
                        is_maximized: false,
                        is_focused: false,
                        process_name,
                        process_id,
                    });
                }
            }
        }

        // Mark the frontmost application's window as focused
        if let Ok(focused_pid) = self.get_frontmost_pid() {
            // On macOS, we mark the first window matching the frontmost PID
            if let Some(pos) = windows.iter().position(|w| w.process_id == Some(focused_pid)) {
                windows[pos].is_focused = true;
            }
        }

        Ok(windows)
    }

    fn get_frontmost_pid(&self) -> Result<u32, GuiError> {
        unsafe {
            let workspace = objc::msg_send![
                objc::class!(NSWorkspace),
                alloc
            ];
            let workspace: *mut objc::runtime::Object = objc::msg_send![workspace, init];
            let app: *mut objc::runtime::Object = objc::msg_send![workspace, frontmostApplication];
            let pid: i32 = objc::msg_send![app, processIdentifier];
            let _: () = objc::msg_send![workspace, release];
            Ok(pid as u32)
        }
    }

    fn ax_window_ref(&self, window_id: u64) -> Result<*mut objc::runtime::Object, GuiError> {
        let wid = window_id as u32;
        unsafe {
            let app_ref = AXUIElementCreateApplication(wid);
            if app_ref.is_null() {
                return Err(GuiError::PlatformError(
                    format!("AXUIElementCreateApplication failed for PID {wid}"),
                ));
            }
            Ok(app_ref)
        }
    }

    fn ax_get_window_list(&self, pid: u32) -> Result<Vec<u64>, GuiError> {
        unsafe {
            let app = AXUIElementCreateApplication(pid);
            if app.is_null() {
                return Ok(vec![]);
            }

            let cf_string = CFString::new("AXWindows");
            let mut windows_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                app,
                cf_string.as_concrete_TypeRef(),
                &mut windows_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );

            CFRelease(app as *mut _);

            if result != 0 || windows_ref.is_null() {
                return Ok(vec![]);
            }

            let mut ids = Vec::new();
            let count: CFIndex = CFArrayGetCount(windows_ref as *mut _);
            for i in 0..count {
                let ax_window = CFArrayGetValueAtIndex(windows_ref as *mut _, i);
                if !ax_window.is_null() {
                    let wid = AXUIElementGetWindow(ax_window as *mut _);
                    if wid > 0 {
                        ids.push(wid as u64);
                    }
                }
            }
            CFRelease(windows_ref as *mut _);
            Ok(ids)
        }
    }

    fn ax_get_attribute_i32(
        &self,
        ax_window: *mut objc::runtime::Object,
        attribute: &CFString,
    ) -> Result<i32, GuiError> {
        unsafe {
            let mut val_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                ax_window,
                attribute.as_concrete_TypeRef(),
                &mut val_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );
            if result != 0 || val_ref.is_null() {
                return Err(GuiError::PlatformError("AX attribute not found".into()));
            }
            let num = CFNumber::wrap_under_get_rule(val_ref as *mut _);
            let mut val: i32 = 0;
            if num.get_value(&mut val) {
                Ok(val)
            } else {
                Err(GuiError::PlatformError("AX attribute not a number".into()))
            }
        }
    }

    fn ax_set_attribute_i32(
        &self,
        ax_window: *mut objc::runtime::Object,
        attribute: &CFString,
        value: i32,
    ) -> Result<(), GuiError> {
        unsafe {
            let num = CFNumber::from(value);
            let result = AXUIElementSetAttributeValue(
                ax_window,
                attribute.as_concrete_TypeRef(),
                num.as_concrete_TypeRef(),
            );
            if result != 0 {
                return Err(GuiError::PlatformError(
                    format!("AX set attribute failed: {result}"),
                ));
            }
            Ok(())
        }
    }

    fn ax_set_point(
        &self,
        ax_window: *mut objc::runtime::Object,
        attribute: &CFString,
        x: f32,
        y: f32,
    ) -> Result<(), GuiError> {
        unsafe {
            let point = CGPoint::new(x as f64, y as f64);
            let dict = point.to_dict();
            let result = AXUIElementSetAttributeValue(
                ax_window,
                attribute.as_concrete_TypeRef(),
                dict.as_concrete_TypeRef(),
            );
            if result != 0 {
                return Err(GuiError::PlatformError(
                    format!("AX set point failed: {result}"),
                ));
            }
            Ok(())
        }
    }

    fn ax_set_size(
        &self,
        ax_window: *mut objc::runtime::Object,
        attribute: &CFString,
        w: f32,
        h: f32,
    ) -> Result<(), GuiError> {
        unsafe {
            let size = CGSize::new(w as f64, h as f64);
            let dict = size.to_dict();
            let result = AXUIElementSetAttributeValue(
                ax_window,
                attribute.as_concrete_TypeRef(),
                dict.as_concrete_TypeRef(),
            );
            if result != 0 {
                return Err(GuiError::PlatformError(
                    format!("AX set size failed: {result}"),
                ));
            }
            Ok(())
        }
    }

    fn ax_perform_action(
        &self,
        ax_window: *mut objc::runtime::Object,
        action: &CFString,
    ) -> Result<(), GuiError> {
        unsafe {
            let result =
                AXUIElementPerformAction(ax_window, action.as_concrete_TypeRef());
            if result != 0 {
                return Err(GuiError::PlatformError(
                    format!("AX perform action failed: {result}"),
                ));
            }
            Ok(())
        }
    }

    fn with_window_ax<F>(&self, window_id: u64, f: F) -> Result<(), GuiError>
    where
        F: Fn(*mut objc::runtime::Object) -> Result<(), GuiError>,
    {
        // We need the PID to create the AXUIElement
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
                return Err(GuiError::PlatformError(
                    format!("AXUIElementCreateApplication failed for PID {pid}"),
                ));
            }

            let ax_windows = CFString::new("AXWindows");
            let mut windows_ref: *mut objc::runtime::Object = std::ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(
                app,
                ax_windows.as_concrete_TypeRef(),
                &mut windows_ref as *mut *mut objc::runtime::Object as *mut *mut _,
            );

            CFRelease(app as *mut _);

            if result != 0 || windows_ref.is_null() {
                return Err(GuiError::PlatformError(
                    "Failed to get AX windows list".into(),
                ));
            }

            let count: CFIndex = CFArrayGetCount(windows_ref as *mut _);
            let mut found = false;

            for i in 0..count {
                let ax_win = CFArrayGetValueAtIndex(windows_ref as *mut _, i);
                if ax_win.is_null() {
                    continue;
                }

                let wid = AXUIElementGetWindow(ax_win as *mut _);
                if wid as u64 == window_id {
                    f(ax_win as *mut _)?;
                    found = true;
                    break;
                }
            }

            CFRelease(windows_ref as *mut _);

            if !found {
                return Err(GuiError::PlatformError(
                    format!("AX window {window_id} not found"),
                ));
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
        use core_foundation::boolean::CFBoolean;

        self.with_window_ax(window_id, |ax_win| {
            unsafe {
                let focused = CFString::new("AXFocused");
                let true_val = CFBoolean::true_value();
                let result = AXUIElementSetAttributeValue(
                    ax_win,
                    focused.as_concrete_TypeRef(),
                    true_val.as_concrete_TypeRef(),
                );
                if result != 0 {
                    return Err(GuiError::PlatformError(format!(
                        "AX set focused failed: {result}"
                    )));
                }
            }
            // Also raise the window
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
        let minimize = CFString::new("AXPressMiniaturize");
        let mini_button = CFString::new("AXMiniaturizeButton");
        self.with_window_ax(window_id, |ax_win| {
            // Try pressing the minimize button
            let result = self.ax_perform_action(ax_win, &mini_button);
            if result.is_err() {
                // Fallback: AXMiniaturize
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
        // On macOS, restore = deminiaturize + unzoom
        let demini = CFString::new("AXDeminiaturize");
        let _ = self.with_window_ax(window_id, |ax_win| {
            self.ax_perform_action(ax_win, &demini)
        });
        // Also unzoom if maximized
        let zoom = CFString::new("AXZoom");
        self.with_window_ax(window_id, |ax_win| self.ax_perform_action(ax_win, &zoom))
    }

    pub fn close_window(&self, window_id: u64) -> Result<(), GuiError> {
        let close = CFString::new("AXPressCloseButton");
        let close_button = CFString::new("AXCloseButton");
        self.with_window_ax(window_id, |ax_win| {
            let result = self.ax_perform_action(ax_win, &close_button);
            if result.is_err() {
                // Fallback: simulate Cmd+W via the close button press
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
                let mut val_ref: *mut objc::runtime::Object = std::ptr::null_mut();
                let r = AXUIElementCopyAttributeValue(
                    ax_win,
                    title_attr.as_concrete_TypeRef(),
                    &mut val_ref as *mut *mut objc::runtime::Object as *mut *mut _,
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
                // Get position
                let mut pos_ref: *mut objc::runtime::Object = std::ptr::null_mut();
                if AXUIElementCopyAttributeValue(
                    ax_win,
                    position_attr.as_concrete_TypeRef(),
                    &mut pos_ref as *mut *mut objc::runtime::Object as *mut *mut _,
                ) == 0
                {
                    let pos_dict = CFDictionary::wrap_under_get_rule(pos_ref as *mut _);
                    let point = CGRect::from_dict(&pos_dict);
                    region.x = point.origin.x as u32;
                    region.y = point.origin.y as u32;
                }

                // Get size
                let mut size_ref: *mut objc::runtime::Object = std::ptr::null_mut();
                if AXUIElementCopyAttributeValue(
                    ax_win,
                    size_attr.as_concrete_TypeRef(),
                    &mut size_ref as *mut *mut objc::runtime::Object as *mut *mut _,
                ) == 0
                {
                    let size_dict = CFDictionary::wrap_under_get_rule(size_ref as *mut _);
                    let cg_size = CGSize::from_dict(&size_dict);
                    region.width = cg_size.width as u32;
                    region.height = cg_size.height as u32;
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

// External C functions
extern "C" {
    fn AXUIElementCreateApplication(pid: u32) -> *mut objc::runtime::Object;
    fn AXUIElementCopyAttributeValue(
        element: *mut objc::runtime::Object,
        attribute: *mut objc::runtime::Object,
        value: *mut *mut objc::runtime::Object,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: *mut objc::runtime::Object,
        attribute: *mut objc::runtime::Object,
        value: *mut objc::runtime::Object,
    ) -> i32;
    fn AXUIElementPerformAction(
        element: *mut objc::runtime::Object,
        action: *mut objc::runtime::Object,
    ) -> i32;
    fn AXUIElementGetWindow(element: *mut objc::runtime::Object) -> u32;
    fn CFArrayGetCount(array: *mut objc::runtime::Object) -> CFIndex;
    fn CFArrayGetValueAtIndex(
        array: *mut objc::runtime::Object,
        index: CFIndex,
    ) -> *mut objc::runtime::Object;
    fn CFRelease(obj: *mut objc::runtime::Object);
}
