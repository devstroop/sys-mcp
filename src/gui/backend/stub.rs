use async_trait::async_trait;
use crate::error::GuiError;
use crate::gui::accessibility::AccessibilityCapability;
use crate::gui::backend::GuiBackend;
use crate::gui::clipboard::ClipboardCapability;
use crate::gui::display::DisplayCapability;
use crate::gui::input::InputCapability;
use crate::gui::types::{MonitorInfo, MouseButton, Point, Region, Resolution, ScrollDirection, Screenshot, WindowInfo};
use crate::gui::window::WindowCapability;

pub struct StubBackend;

impl StubBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DisplayCapability for StubBackend {
    async fn screenshot(&self) -> Result<Screenshot, GuiError> {
        Ok(Screenshot {
            data: vec![],
            width: 1920,
            height: 1080,
        })
    }

    async fn screenshot_region(&self, _region: Region) -> Result<Screenshot, GuiError> {
        Ok(Screenshot {
            data: vec![],
            width: 100,
            height: 100,
        })
    }

    async fn get_screen_size(&self) -> Result<Resolution, GuiError> {
        Ok(Resolution {
            width: 1920,
            height: 1080,
        })
    }

    async fn list_monitors(&self) -> Result<Vec<MonitorInfo>, GuiError> {
        Ok(vec![MonitorInfo {
            id: 0,
            name: "Primary".to_string(),
            primary: true,
            resolution: Resolution {
                width: 1920,
                height: 1080,
            },
            position: (0, 0),
        }])
    }
}

#[async_trait]
impl InputCapability for StubBackend {
    async fn mouse_click(&self, _x: u32, _y: u32, _button: MouseButton) -> Result<(), GuiError> {
        Ok(())
    }

    async fn mouse_double_click(&self, _x: u32, _y: u32, _button: MouseButton) -> Result<(), GuiError> {
        Ok(())
    }

    async fn mouse_move(&self, _x: u32, _y: u32) -> Result<(), GuiError> {
        Ok(())
    }

    async fn mouse_position(&self) -> Result<Point, GuiError> {
        Ok(Point { x: 0, y: 0 })
    }

    async fn mouse_drag(&self, _from: Point, _to: Point, _button: MouseButton) -> Result<(), GuiError> {
        Ok(())
    }

    async fn mouse_scroll(&self, _x: u32, _y: u32, _direction: ScrollDirection, _amount: i32) -> Result<(), GuiError> {
        Ok(())
    }

    async fn type_text(&self, _text: &str) -> Result<(), GuiError> {
        Ok(())
    }

    async fn key_press(&self, _key: &str) -> Result<(), GuiError> {
        Ok(())
    }

    async fn key_down(&self, _key: &str) -> Result<(), GuiError> {
        Ok(())
    }

    async fn key_up(&self, _key: &str) -> Result<(), GuiError> {
        Ok(())
    }

    async fn key_combo(&self, _keys: &[String]) -> Result<(), GuiError> {
        Ok(())
    }
}

#[async_trait]
impl WindowCapability for StubBackend {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        Ok(vec![])
    }

    async fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        Err(GuiError::WindowError("No active window".to_string()))
    }

    async fn focus_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }

    async fn move_window(&self, _window_id: u64, _x: i32, _y: i32) -> Result<(), GuiError> {
        Ok(())
    }

    async fn resize_window(&self, _window_id: u64, _width: u32, _height: u32) -> Result<(), GuiError> {
        Ok(())
    }

    async fn minimize_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }

    async fn maximize_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }

    async fn restore_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }

    async fn close_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Ok(())
    }

    async fn get_window_title(&self, _window_id: u64) -> Result<String, GuiError> {
        Ok("".to_string())
    }

    async fn get_window_bounds(&self, _window_id: u64) -> Result<Region, GuiError> {
        Ok(Region {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        })
    }

    async fn find_windows_by_title(&self, _query: &str) -> Result<Vec<WindowInfo>, GuiError> {
        Ok(vec![])
    }
}

#[async_trait]
impl ClipboardCapability for StubBackend {
    async fn get_text(&self) -> Result<String, GuiError> {
        Ok("".to_string())
    }

    async fn set_text(&self, _text: &str) -> Result<(), GuiError> {
        Ok(())
    }

    async fn clear(&self) -> Result<(), GuiError> {
        Ok(())
    }
}

impl GuiBackend for StubBackend {
    fn as_window(&self) -> Option<&dyn WindowCapability> {
        Some(self)
    }

    fn as_clipboard(&self) -> Option<&dyn ClipboardCapability> {
        Some(self)
    }

    fn as_accessibility(&self) -> Option<&dyn AccessibilityCapability> {
        None
    }
}