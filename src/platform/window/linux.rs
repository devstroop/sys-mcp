use crate::error::GuiError;
use crate::gui::types::*;

pub struct PlatformWindowManager;

impl PlatformWindowManager {
    pub fn new() -> Result<Self, GuiError> {
        Ok(Self)
    }

    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        // TODO: implement via x11rb
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn focus_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn move_window(&self, _window_id: u64, _x: i32, _y: i32) -> Result<(), GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn resize_window(&self, _window_id: u64, _width: u32, _height: u32) -> Result<(), GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn minimize_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn maximize_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn restore_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn close_window(&self, _window_id: u64) -> Result<(), GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn get_window_title(&self, _window_id: u64) -> Result<String, GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn get_window_bounds(&self, _window_id: u64) -> Result<Region, GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }

    pub fn find_windows_by_title(&self, _query: &str) -> Result<Vec<WindowInfo>, GuiError> {
        Err(GuiError::PlatformError("Linux window management not yet implemented".into()))
    }
}
