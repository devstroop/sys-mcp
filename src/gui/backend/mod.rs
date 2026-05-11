pub mod local;
pub mod stub;

use crate::gui::accessibility::AccessibilityCapability;
use crate::gui::clipboard::ClipboardCapability;
use crate::gui::display::DisplayCapability;
use crate::gui::input::InputCapability;
use crate::gui::window::WindowCapability;

/// Backend super-trait. Display + Input are always available for local GUI.
/// Optional capabilities are accessed via `as_*()` methods.
pub trait GuiBackend: DisplayCapability + InputCapability + Send + Sync {
    fn as_window(&self) -> Option<&dyn WindowCapability> {
        None
    }
    fn as_clipboard(&self) -> Option<&dyn ClipboardCapability> {
        None
    }
    fn as_accessibility(&self) -> Option<&dyn AccessibilityCapability> {
        None
    }
}
