pub mod local;
pub mod stub;

use crate::gui::accessibility::AccessibilityCapability;
use crate::gui::clipboard::ClipboardCapability;
#[cfg(feature = "detection")]
use crate::gui::detection::DetectionCapability;
use crate::gui::display::DisplayCapability;
use crate::gui::input::InputCapability;
#[cfg(feature = "ocr")]
use crate::gui::ocr::OcrCapability;
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

    /// On-device OCR. `None` means the backend cannot run inference, which the
    /// handlers surface as a capability error rather than loading the ONNX model
    /// in a test (`docs/ISSUES.md` #47).
    #[cfg(feature = "ocr")]
    fn as_ocr(&self) -> Option<&dyn OcrCapability> {
        None
    }

    /// Object detection. Same contract as [`GuiBackend::as_ocr`].
    #[cfg(feature = "detection")]
    fn as_detection(&self) -> Option<&dyn DetectionCapability> {
        None
    }
}
