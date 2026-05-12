use std::ops::{Deref, DerefMut};
use std::sync::Mutex;

use async_trait::async_trait;
use rustautogui::RustAutoGui;

use crate::error::GuiError;
use crate::gui::accessibility::AccessibilityCapability;
use crate::gui::backend::GuiBackend;
use crate::gui::clipboard::ClipboardCapability;
use crate::gui::display::DisplayCapability;
use crate::gui::input::InputCapability;
use crate::gui::types::*;
use crate::gui::window::WindowCapability;
use crate::platform::window::PlatformWindowManager;

/// Wrapper around `RustAutoGui` to implement `Send + Sync`.
///
/// `RustAutoGui` contains raw pointers (e.g. HDC / HBITMAP on Windows) which
/// make it `!Send + !Sync`.  Access is serialised by the `Mutex` in `LocalBackend`,
/// and this type is **never** exposed outside that Mutex, so sending the wrapper
/// across threads is sound.
struct AutoGuiWrapper(RustAutoGui);

// SAFETY: AutoGuiWrapper is always accessed through the Mutex in LocalBackend.
// The type is never constructed or accessed outside of the Mutex lock, so
// there is no way to violate aliasing rules.
unsafe impl Send for AutoGuiWrapper {}
unsafe impl Sync for AutoGuiWrapper {}

impl Deref for AutoGuiWrapper {
    type Target = RustAutoGui;
    fn deref(&self) -> &RustAutoGui {
        &self.0
    }
}

impl DerefMut for AutoGuiWrapper {
    fn deref_mut(&mut self) -> &mut RustAutoGui {
        &mut self.0
    }
}

/// Local backend — wraps RustAutoGui + platform modules for window/clipboard/accessibility.
pub struct LocalBackend {
    autogui: Mutex<AutoGuiWrapper>,
    window_manager: PlatformWindowManager,
    #[cfg(feature = "clipboard")]
    clipboard: Mutex<arboard::Clipboard>,
}

impl LocalBackend {
    pub fn new(debug: bool) -> Result<Self, GuiError> {
        let autogui = RustAutoGui::new(debug)
            .map_err(|e| GuiError::BackendError(format!("failed to initialize RustAutoGui: {e}")))?;

        #[cfg(feature = "clipboard")]
        let clipboard = arboard::Clipboard::new()
            .map_err(|e| GuiError::ClipboardError(format!("failed to initialize clipboard: {e}")))?;

        Ok(Self {
            autogui: Mutex::new(AutoGuiWrapper(autogui)),
            window_manager: PlatformWindowManager::new()?,
            #[cfg(feature = "clipboard")]
            clipboard: Mutex::new(clipboard),
        })
    }
}

// ─── DisplayCapability ─────────────────────────────────────────────────────

#[async_trait]
impl DisplayCapability for LocalBackend {
async fn screenshot(&self) -> Result<Screenshot, GuiError> {
        // Capture to temp file via RustAutoGui, then read back
        let tmp = std::env::temp_dir().join(format!("gui-mcp-{}.png", uuid::Uuid::new_v4()));
        let tmp_path = tmp.to_string_lossy().to_string();

        let mut autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui
            .save_screenshot(&tmp_path)
            .map_err(|e| GuiError::ScreenshotFailed(e.to_string()))?;
        drop(autogui);

        let data =
            std::fs::read(&tmp).map_err(|e| GuiError::ScreenshotFailed(e.to_string()))?;
        std::fs::remove_file(&tmp).ok();

        let img = image::load_from_memory(&data)
            .map_err(|e| GuiError::ScreenshotFailed(e.to_string()))?;
        let (w, h) = (img.width(), img.height());

        Ok(Screenshot {
            data,
            width: w,
            height: h,
        })
    }

    async fn screenshot_region(&self, region: Region) -> Result<Screenshot, GuiError> {
        let full = self.screenshot().await?;
        let img = image::load_from_memory(&full.data)
            .map_err(|e| GuiError::ScreenshotFailed(e.to_string()))?;

        let (x, y, w, h) = (
            region.x as u32,
            region.y as u32,
            region.width,
            region.height,
        );
        let cropped = image::imageops::crop_imm(&img, x, y, w, h).to_image();

        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        image::ImageEncoder::write_image(
            encoder,
            cropped.as_raw(),
            region.width,
            region.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| GuiError::ScreenshotFailed(e.to_string()))?;

        Ok(Screenshot {
            data: buf,
            width: region.width,
            height: region.height,
        })
    }

    async fn get_screen_size(&self) -> Result<Resolution, GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        let (w, h) = autogui.get_screen_size();
        Ok(Resolution {
            width: w as u32,
            height: h as u32,
        })
    }

    async fn list_monitors(&self) -> Result<Vec<MonitorInfo>, GuiError> {
        let (w, h) = {
            let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
            autogui.get_screen_size()
        };
        Ok(vec![MonitorInfo {
            id: 0,
            name: "Primary".to_string(),
            primary: true,
            resolution: Resolution {
                width: w as u32,
                height: h as u32,
            },
            position: (0, 0),
        }])
    }
}

// ─── InputCapability ───────────────────────────────────────────────────────

#[async_trait]
impl InputCapability for LocalBackend {
    async fn mouse_click(&self, x: u32, y: u32, button: MouseButton) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.move_mouse_to_pos(x, y, 0.0)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        match button {
            MouseButton::Left => autogui.left_click(),
            MouseButton::Right => autogui.right_click(),
            MouseButton::Middle => autogui.middle_click(),
        }
        .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn mouse_double_click(&self, x: u32, y: u32, button: MouseButton) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.move_mouse_to_pos(x, y, 0.0)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        match button {
            MouseButton::Left => autogui.double_click(),
            // rustautogui only supports left double-click natively;
            // for right/middle we simulate with two rapid clicks.
            MouseButton::Right => {
                autogui.right_click().map_err(|e| GuiError::InputError(e.to_string()))?;
                autogui.right_click()
            }
            MouseButton::Middle => {
                autogui.middle_click().map_err(|e| GuiError::InputError(e.to_string()))?;
                autogui.middle_click()
            }
        }
        .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn mouse_move(&self, x: u32, y: u32) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.move_mouse_to_pos(x, y, 0.0)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn mouse_position(&self) -> Result<Point, GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        let (x, y) = autogui
            .get_mouse_position()
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(Point {
            x: x as u32,
            y: y as u32,
        })
    }

    async fn mouse_drag(&self, from: Point, to: Point, _button: MouseButton) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.move_mouse_to_pos(from.x, from.y, 0.0)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        autogui.drag_mouse_to_pos(to.x, to.y, 0.15)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn mouse_scroll(&self, x: u32, y: u32, direction: ScrollDirection, amount: i32) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.move_mouse_to_pos(x, y, 0.0)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        let amt = amount.unsigned_abs();
        match direction {
            ScrollDirection::Up => autogui.scroll_up(amt),
            ScrollDirection::Down => autogui.scroll_down(amt),
            ScrollDirection::Left => autogui.scroll_left(amt),
            ScrollDirection::Right => autogui.scroll_right(amt),
        }
        .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn type_text(&self, text: &str) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.keyboard_input(text)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn key_press(&self, key: &str) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.keyboard_command(key)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn key_down(&self, key: &str) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.key_down(key)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn key_up(&self, key: &str) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        autogui.key_up(key)
            .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }

    async fn key_combo(&self, keys: &[String]) -> Result<(), GuiError> {
        let autogui = self.autogui.lock().map_err(|e| GuiError::BackendError(e.to_string()))?;
        match keys {
            [a, b] => autogui.keyboard_multi_key(a, b, None),
            [a, b, c] => autogui.keyboard_multi_key(a, b, Some(c)),
            _ => {
                return Err(GuiError::InputError(
                    "key_combo requires 2 or 3 keys".to_string(),
                ))
            }
        }
        .map_err(|e| GuiError::InputError(e.to_string()))?;
        Ok(())
    }
}

// ─── WindowCapability ──────────────────────────────────────────────────────

#[async_trait]
impl WindowCapability for LocalBackend {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        self.window_manager.list_windows()
    }
    async fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        self.window_manager.get_active_window()
    }
    async fn focus_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window_manager.focus_window(window_id)
    }
    async fn move_window(&self, window_id: u64, x: i32, y: i32) -> Result<(), GuiError> {
        self.window_manager.move_window(window_id, x, y)
    }
    async fn resize_window(&self, window_id: u64, width: u32, height: u32) -> Result<(), GuiError> {
        self.window_manager.resize_window(window_id, width, height)
    }
    async fn minimize_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window_manager.minimize_window(window_id)
    }
    async fn maximize_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window_manager.maximize_window(window_id)
    }
    async fn restore_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window_manager.restore_window(window_id)
    }
    async fn close_window(&self, window_id: u64) -> Result<(), GuiError> {
        self.window_manager.close_window(window_id)
    }
    async fn get_window_title(&self, window_id: u64) -> Result<String, GuiError> {
        self.window_manager.get_window_title(window_id)
    }
    async fn get_window_bounds(&self, window_id: u64) -> Result<Region, GuiError> {
        self.window_manager.get_window_bounds(window_id)
    }
    async fn find_windows_by_title(&self, query: &str) -> Result<Vec<WindowInfo>, GuiError> {
        self.window_manager.find_windows_by_title(query)
    }
}

// ─── ClipboardCapability ───────────────────────────────────────────────────

#[cfg(feature = "clipboard")]
#[async_trait]
impl ClipboardCapability for LocalBackend {
    async fn get_text(&self) -> Result<String, GuiError> {
        let mut cb = self.clipboard.lock().map_err(|e| GuiError::ClipboardError(e.to_string()))?;
        cb.get_text()
            .map_err(|e| GuiError::ClipboardError(e.to_string()))
    }

    async fn set_text(&self, text: &str) -> Result<(), GuiError> {
        let mut cb = self.clipboard.lock().map_err(|e| GuiError::ClipboardError(e.to_string()))?;
        cb.set_text(text.to_string())
            .map_err(|e| GuiError::ClipboardError(e.to_string()))
    }

    async fn clear(&self) -> Result<(), GuiError> {
        let mut cb = self.clipboard.lock().map_err(|e| GuiError::ClipboardError(e.to_string()))?;
        cb.clear()
            .map_err(|e| GuiError::ClipboardError(e.to_string()))
    }
}

// ─── GuiBackend trait ──────────────────────────────────────────────────────

impl GuiBackend for LocalBackend {
    fn as_window(&self) -> Option<&dyn WindowCapability> {
        Some(self)
    }

    #[cfg(feature = "clipboard")]
    fn as_clipboard(&self) -> Option<&dyn ClipboardCapability> {
        Some(self)
    }

    #[cfg(not(feature = "clipboard"))]
    fn as_clipboard(&self) -> Option<&dyn ClipboardCapability> {
        None
    }

    fn as_accessibility(&self) -> Option<&dyn AccessibilityCapability> {
        None // Phase 6
    }
}
