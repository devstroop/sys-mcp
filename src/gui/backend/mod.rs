pub mod local;
pub mod stub;

use crate::gui::accessibility::AccessibilityCapability;
use crate::gui::clipboard::ClipboardCapability;
use crate::gui::display::DisplayCapability;
use crate::gui::input::InputCapability;
use crate::gui::vision::VisionCapability;
use crate::gui::window::WindowCapability;
use crate::system::SystemCapability;

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
    fn as_vision(&self) -> Option<&dyn VisionCapability> {
        None
    }

    /// System probes (processes, services, network, stats, disk, event logs).
    /// `None` means this backend cannot answer them, which the handlers surface
    /// as a capability error rather than falling back to shelling out — the
    /// whole point of the seam is that the real path is opt-in, not the default
    /// (`docs/ISSUES.md` #47).
    fn as_system(&self) -> Option<&dyn SystemCapability> {
        None
    }
}
