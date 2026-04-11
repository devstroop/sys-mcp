use async_trait::async_trait;

use crate::error::GuiError;

#[async_trait]
pub trait ClipboardCapability: Send + Sync {
    async fn get_text(&self) -> Result<String, GuiError>;
    async fn set_text(&self, text: &str) -> Result<(), GuiError>;
    async fn clear(&self) -> Result<(), GuiError>;
}
