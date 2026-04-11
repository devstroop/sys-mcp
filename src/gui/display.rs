use async_trait::async_trait;

use crate::error::GuiError;
use crate::gui::types::*;

#[async_trait]
pub trait DisplayCapability: Send + Sync {
    async fn screenshot(&self) -> Result<Screenshot, GuiError>;
    async fn screenshot_region(&self, region: Region) -> Result<Screenshot, GuiError>;
    async fn get_screen_size(&self) -> Result<Resolution, GuiError>;
    async fn list_monitors(&self) -> Result<Vec<MonitorInfo>, GuiError>;
}
