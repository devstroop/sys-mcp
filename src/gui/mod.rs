pub mod accessibility;
pub mod backend;
pub mod clipboard;
pub mod display;
pub mod input;
#[cfg(feature = "ocr")]
pub mod ocr;
pub mod types;
pub mod window;
#[cfg(feature = "detection")]
pub mod detection;

fn get_os_version() -> String {
    #[cfg(windows)]
    {
        use std::mem;
        use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
        unsafe {
            let mut info: OSVERSIONINFOW = mem::zeroed();
            info.dwOSVersionInfoSize = mem::size_of::<OSVERSIONINFOW>() as u32;
            if windows::Win32::System::SystemInformation::GetVersionExW(
                &mut info as *mut _ as *mut OSVERSIONINFOW,
            )
            .as_bool()
            {
                return format!("{}.{}.{}", info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber);
            }
        }
        "unknown".to_string()
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sw_vers").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut version = String::new();
            for line in text.lines() {
                if line.starts_with("ProductVersion:") {
                    version = line
                        .split(':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    break;
                }
            }
            if !version.is_empty() {
                return version;
            }
        }
        "unknown".to_string()
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let val = line
                        .split('=')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .trim_matches('"')
                        .to_string();
                    if !val.is_empty() {
                        return val;
                    }
                }
            }
        }
        "unknown".to_string()
    }
}

use crate::error::GuiError;
use crate::gui::backend::GuiBackend;
use crate::gui::types::*;

/// High-level facade over a [`GuiBackend`].
/// MCP tool handlers call methods here.
pub struct GuiClient {
    backend: Box<dyn GuiBackend>,
}

impl GuiClient {
    pub fn new(backend: impl GuiBackend + 'static) -> Self {
        Self {
            backend: Box::new(backend),
        }
    }

    // ── Display (always available) ─────────────────────────────────────

    pub async fn screenshot(&self) -> Result<Screenshot, GuiError> {
        self.backend.screenshot().await
    }

    pub async fn screenshot_region(&self, region: Region) -> Result<Screenshot, GuiError> {
        self.backend.screenshot_region(region).await
    }

    pub async fn get_screen_size(&self) -> Result<Resolution, GuiError> {
        self.backend.get_screen_size().await
    }

    pub async fn list_monitors(&self) -> Result<Vec<MonitorInfo>, GuiError> {
        self.backend.list_monitors().await
    }

    // ── Input (always available) ───────────────────────────────────────

    pub async fn click(&self, x: u32, y: u32, button: MouseButton) -> Result<(), GuiError> {
        self.backend.mouse_click(x, y, button).await
    }

    pub async fn double_click(&self, x: u32, y: u32, button: MouseButton) -> Result<(), GuiError> {
        self.backend.mouse_double_click(x, y, button).await
    }

    pub async fn mouse_move(&self, x: u32, y: u32) -> Result<(), GuiError> {
        self.backend.mouse_move(x, y).await
    }

    pub async fn mouse_position(&self) -> Result<Point, GuiError> {
        self.backend.mouse_position().await
    }

    pub async fn drag(&self, from: Point, to: Point, button: MouseButton) -> Result<(), GuiError> {
        self.backend.mouse_drag(from, to, button).await
    }

    pub async fn scroll(&self, x: u32, y: u32, direction: ScrollDirection, amount: i32) -> Result<(), GuiError> {
        self.backend.mouse_scroll(x, y, direction, amount).await
    }

    pub async fn type_text(&self, text: &str) -> Result<(), GuiError> {
        self.backend.type_text(text).await
    }

    pub async fn press_key(&self, key: &str) -> Result<(), GuiError> {
        self.backend.key_press(key).await
    }

    pub async fn key_combo(&self, keys: &[String]) -> Result<(), GuiError> {
        self.backend.key_combo(keys).await
    }

    // ── Window management ──────────────────────────────────────────────

    fn window(&self) -> Result<&dyn crate::gui::window::WindowCapability, GuiError> {
        self.backend
            .as_window()
            .ok_or_else(|| GuiError::UnsupportedCapability("window management".into()))
    }

    pub async fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        self.window()?.list_windows().await
    }

    pub async fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        self.window()?.get_active_window().await
    }

    pub async fn focus_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window()?.focus_window(window_id).await
    }

    pub async fn move_window(&self, window_id: u64, x: i32, y: i32) -> Result<(), GuiError> {
        self.window()?.move_window(window_id, x, y).await
    }

    pub async fn resize_window(&self, window_id: u64, width: u32, height: u32) -> Result<(), GuiError> {
        self.window()?.resize_window(window_id, width, height).await
    }

    pub async fn minimize_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window()?.minimize_window(window_id).await
    }

    pub async fn maximize_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window()?.maximize_window(window_id).await
    }

    pub async fn restore_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window()?.restore_window(window_id).await
    }

    pub async fn close_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window()?.close_window(window_id).await
    }

    pub async fn find_windows_by_title(&self, query: &str) -> Result<Vec<WindowInfo>, GuiError> {
        self.window()?.find_windows_by_title(query).await
    }

    // ── Clipboard ──────────────────────────────────────────────────────

    fn clipboard(&self) -> Result<&dyn crate::gui::clipboard::ClipboardCapability, GuiError> {
        self.backend
            .as_clipboard()
            .ok_or_else(|| GuiError::UnsupportedCapability("clipboard".into()))
    }

    pub async fn get_clipboard_text(&self) -> Result<String, GuiError> {
        self.clipboard()?.get_text().await
    }

    pub async fn set_clipboard_text(&self, text: &str) -> Result<(), GuiError> {
        self.clipboard()?.set_text(text).await
    }

    // ── Accessibility ──────────────────────────────────────────────────

    fn accessibility(&self) -> Result<&dyn crate::gui::accessibility::AccessibilityCapability, GuiError> {
        self.backend
            .as_accessibility()
            .ok_or_else(|| GuiError::UnsupportedCapability("accessibility".into()))
    }

    pub async fn get_accessibility_tree(&self, window_id: Option<u64>, max_depth: Option<u32>) -> Result<AccessibilityNode, GuiError> {
        self.accessibility()?.get_tree(window_id, max_depth).await
    }

    pub async fn find_ui_elements(&self, query: AccessibilityQuery) -> Result<Vec<AccessibilityNode>, GuiError> {
        self.accessibility()?.find_elements(query).await
    }

    // ── OCR ─────────────────────────────────────────────────────────

    #[cfg(feature = "ocr")]
    pub async fn read_screen(&self, region: Option<Region>) -> Result<crate::gui::ocr::OcrResult, GuiError> {
        let screenshot = match region {
            Some(r) => self.backend.screenshot_region(r).await?,
            None => self.backend.screenshot().await?,
        };
        tokio::task::spawn_blocking(move || crate::gui::ocr::read_screen(&screenshot))
            .await
            .map_err(|e| GuiError::OcrError(e.to_string()))?
    }

    // ── Object Detection ──────────────────────────────────────────────

    #[cfg(feature = "detection")]
    pub async fn detect_objects(&self) -> Result<crate::gui::detection::DetectionResult, GuiError> {
        let screenshot = self.backend.screenshot().await?;
        let result = tokio::task::spawn_blocking(move || crate::gui::detection::detect_objects(&screenshot))
            .await
            .map_err(|e| GuiError::DetectionError(e.to_string()))?;
        result.map_err(|e| GuiError::DetectionError(e))
    }

    // ── System info ────────────────────────────────────────────────────

    pub async fn system_info(&self) -> Result<SystemInfo, GuiError> {
        let res = self.get_screen_size().await?;
        let mut capabilities = vec![
            "screenshot".to_string(),
            "mouse".to_string(),
            "keyboard".to_string(),
        ];

        if self.backend.as_window().is_some() {
            capabilities.push("window_management".to_string());
        }
        if self.backend.as_clipboard().is_some() {
            capabilities.push("clipboard".to_string());
        }
        if self.backend.as_accessibility().is_some() {
            capabilities.push("accessibility".to_string());
        }
        #[cfg(feature = "ocr")]
        capabilities.push("ocr".to_string());

        capabilities.push("template_matching".to_string());

        let os_version = get_os_version();

        Ok(SystemInfo {
            os: std::env::consts::OS.to_string(),
            os_version,
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default(),
            screen_width: res.width,
            screen_height: res.height,
            capabilities,
        })
    }
}
