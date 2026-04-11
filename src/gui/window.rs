use async_trait::async_trait;

use crate::error::GuiError;
use crate::gui::types::*;

#[async_trait]
pub trait WindowCapability: Send + Sync {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError>;
    async fn get_active_window(&self) -> Result<WindowInfo, GuiError>;
    async fn focus_window(&self, window_id: u64) -> Result<(), GuiError>;
    async fn move_window(&self, window_id: u64, x: i32, y: i32) -> Result<(), GuiError>;
    async fn resize_window(&self, window_id: u64, width: u32, height: u32) -> Result<(), GuiError>;
    async fn minimize_window(&self, window_id: u64) -> Result<(), GuiError>;
    async fn maximize_window(&self, window_id: u64) -> Result<(), GuiError>;
    async fn restore_window(&self, window_id: u64) -> Result<(), GuiError>;
    async fn close_window(&self, window_id: u64) -> Result<(), GuiError>;
    async fn get_window_title(&self, window_id: u64) -> Result<String, GuiError>;
    async fn get_window_bounds(&self, window_id: u64) -> Result<Region, GuiError>;
    async fn find_windows_by_title(&self, query: &str) -> Result<Vec<WindowInfo>, GuiError>;
}
