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

    /// Perform `action` on the element, with the operand the action needs.
    ///
    /// `value` is the write operand for the actions that take one — today only
    /// `set_value`, which pushes text or a number into a control through its
    /// value pattern. That is the reason it exists at all: a UIA/AX `SetValue`
    /// cannot be expressed as an action *name*, so before it the only way to
    /// put content into a field was keystroke injection, which loses the
    /// field's identity (`docs/ISSUES.md` #46).
    ///
    /// Contract every backend implements:
    /// - an action that requires a value and receives `None` must fail with an
    ///   explicit error rather than silently doing nothing;
    /// - `Some("")` is a real value — clearing a field is a legitimate edit, so
    ///   an empty string must never be read as absent;
    /// - actions that take no value ignore the parameter, and the tool layer
    ///   refuses a call that pairs a value with such a verb instead of dropping
    ///   the caller's argument (#40).
    async fn invoke_element_action(
        &self,
        element_id: &str,
        action: &str,
        value: Option<&str>,
    ) -> Result<(), GuiError>;
}
