use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::error::GuiError;
use crate::gui::types::*;

pub struct PlatformWindowManager;

impl PlatformWindowManager {
    pub fn new() -> Result<Self, GuiError> {
        Ok(Self)
    }

    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        let mut windows: Vec<WindowInfo> = Vec::new();
        let foreground = unsafe { GetForegroundWindow() };

        unsafe {
            let _ = EnumWindows(
                Some(enum_window_proc),
                LPARAM(&mut windows as *mut Vec<WindowInfo> as isize),
            );
        }

        // Mark the foreground window
        if !foreground.0.is_null() {
            for w in &mut windows {
                if w.id == foreground.0 as u64 {
                    w.is_focused = true;
                }
            }
        }

        Ok(windows)
    }

    pub fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return Err(GuiError::WindowError("no active window".into()));
        }
        get_window_info(hwnd)
    }

    pub fn focus_window(&self, window_id: u64) -> Result<(), GuiError> {
        let hwnd = HWND(window_id as *mut _);
        unsafe {
            // Restore if minimized
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
        Ok(())
    }

    pub fn move_window(&self, window_id: u64, x: i32, y: i32) -> Result<(), GuiError> {
        let hwnd = HWND(window_id as *mut _);
        let mut rect = RECT::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect)
                .map_err(|e| GuiError::WindowError(format!("GetWindowRect: {e}")))?;
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            MoveWindow(hwnd, x, y, w, h, TRUE)
                .map_err(|e| GuiError::WindowError(format!("MoveWindow: {e}")))?;
        }
        Ok(())
    }

    pub fn resize_window(&self, window_id: u64, width: u32, height: u32) -> Result<(), GuiError> {
        let hwnd = HWND(window_id as *mut _);
        let mut rect = RECT::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect)
                .map_err(|e| GuiError::WindowError(format!("GetWindowRect: {e}")))?;
            MoveWindow(hwnd, rect.left, rect.top, width as i32, height as i32, TRUE)
                .map_err(|e| GuiError::WindowError(format!("MoveWindow: {e}")))?;
        }
        Ok(())
    }

    pub fn minimize_window(&self, window_id: u64) -> Result<(), GuiError> {
        let hwnd = HWND(window_id as *mut _);
        unsafe { let _ = ShowWindow(hwnd, SW_MINIMIZE); }
        Ok(())
    }

    pub fn maximize_window(&self, window_id: u64) -> Result<(), GuiError> {
        let hwnd = HWND(window_id as *mut _);
        unsafe { let _ = ShowWindow(hwnd, SW_MAXIMIZE); }
        Ok(())
    }

    pub fn restore_window(&self, window_id: u64) -> Result<(), GuiError> {
        let hwnd = HWND(window_id as *mut _);
        unsafe { let _ = ShowWindow(hwnd, SW_RESTORE); }
        Ok(())
    }

    pub fn close_window(&self, window_id: u64) -> Result<(), GuiError> {
        let hwnd = HWND(window_id as *mut _);
        unsafe {
            let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        Ok(())
    }

    pub fn get_window_title(&self, window_id: u64) -> Result<String, GuiError> {
        let hwnd = HWND(window_id as *mut _);
        Ok(get_title(hwnd))
    }

    pub fn get_window_bounds(&self, window_id: u64) -> Result<Region, GuiError> {
        let hwnd = HWND(window_id as *mut _);
        let mut rect = RECT::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect)
                .map_err(|e| GuiError::WindowError(format!("GetWindowRect: {e}")))?;
        }
        Ok(Region {
            x: rect.left.max(0) as u32,
            y: rect.top.max(0) as u32,
            width: (rect.right - rect.left).max(0) as u32,
            height: (rect.bottom - rect.top).max(0) as u32,
        })
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

fn get_title(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len == 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buf) };
    OsString::from_wide(&buf[..copied as usize])
        .to_string_lossy()
        .to_string()
}

fn get_window_info(hwnd: HWND) -> Result<WindowInfo, GuiError> {
    let title = get_title(hwnd);
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut rect);
    }

    let is_minimized = unsafe { IsIconic(hwnd).as_bool() };
    let is_maximized = unsafe { IsZoomed(hwnd).as_bool() };

    // Get process ID
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }

    Ok(WindowInfo {
        id: hwnd.0 as u64,
        title,
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left).max(0) as u32,
        height: (rect.bottom - rect.top).max(0) as u32,
        is_minimized,
        is_maximized,
        is_focused: false,
        process_name: None,
        process_id: Some(pid),
    })
}

unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // Skip invisible windows
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    let title = get_title(hwnd);
    // Skip windows with empty titles
    if title.is_empty() {
        return TRUE;
    }

    // Skip tool windows and cloaked (UWP) windows
    let ex_style = WINDOW_EX_STYLE(GetWindowLongW(hwnd, GWL_EXSTYLE) as u32);
    if ex_style.contains(WS_EX_TOOLWINDOW) {
        return TRUE;
    }

    if let Ok(info) = get_window_info(hwnd) {
        let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);
        windows.push(info);
    }

    TRUE
}
