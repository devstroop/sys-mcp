use async_trait::async_trait;

use crate::error::GuiError;
use crate::gui::types::*;

#[async_trait]
pub trait InputCapability: Send + Sync {
    async fn mouse_click(&self, x: u32, y: u32, button: MouseButton) -> Result<(), GuiError>;
    async fn mouse_double_click(&self, x: u32, y: u32, button: MouseButton)
        -> Result<(), GuiError>;
    async fn mouse_move(&self, x: u32, y: u32) -> Result<(), GuiError>;
    async fn mouse_position(&self) -> Result<Point, GuiError>;
    async fn mouse_drag(&self, from: Point, to: Point, button: MouseButton)
        -> Result<(), GuiError>;
    async fn mouse_scroll(
        &self,
        x: u32,
        y: u32,
        direction: ScrollDirection,
        amount: i32,
    ) -> Result<(), GuiError>;
    async fn type_text(&self, text: &str) -> Result<(), GuiError>;
    async fn key_press(&self, key: &str) -> Result<(), GuiError>;
    async fn key_down(&self, key: &str) -> Result<(), GuiError>;
    async fn key_up(&self, key: &str) -> Result<(), GuiError>;
    async fn key_combo(&self, keys: &[String]) -> Result<(), GuiError>;
}
