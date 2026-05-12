use async_trait::async_trait;

use crate::error::GuiError;
use crate::gui::types::*;

#[async_trait]
pub trait AccessibilityCapability: Send + Sync {
    async fn get_tree(
        &self,
        window_id: Option<u64>,
        max_depth: Option<u32>,
    ) -> Result<AccessibilityNode, GuiError>;
    async fn find_elements(
        &self,
        query: AccessibilityQuery,
    ) -> Result<Vec<AccessibilityNode>, GuiError>;
    async fn get_element_properties(&self, element_id: &str)
        -> Result<AccessibilityNode, GuiError>;
    async fn invoke_element_action(&self, element_id: &str, action: &str) -> Result<(), GuiError>;
}
