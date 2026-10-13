use async_trait::async_trait;

use crate::error::GuiError;
use crate::gui::types::*;

/// A single template-match hit. `(x, y)` is the CENTER of the matched
/// template on screen (rustautogui returns center coordinates), which is
/// directly usable with `mouse_click`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageMatch {
    pub x: u32,
    pub y: u32,
    pub confidence: f32,
}

/// Template matching (normalized cross-correlation) via the vendored
/// `rustautogui` crate — no OpenCL required, the CPU FFT path ships with
/// rustautogui's `full` feature which this workspace always enables.
///
/// `template_encoded` must be ENCODED image bytes (PNG/JPEG/...), e.g. the
/// output of `fs_read` or `Screenshot.data`. The backend decodes them with
/// `image::load_from_memory`, which sniffs the format from the leading bytes
/// — raw pixel data is rejected by the handlers before we get here.
///
/// Platform limitation (documented by rustautogui): **on Windows only the
/// MAIN MONITOR is searched; on Linux all monitors are searched.**
#[async_trait]
pub trait VisionCapability: Send + Sync {
    /// Search the screen (optionally restricted to `region`) for the template.
    /// Returns all matches with correlation >= `precision`, highest first.
    async fn find_image(
        &self,
        template_encoded: Vec<u8>,
        precision: f32,
        region: Option<Region>,
    ) -> Result<Vec<ImageMatch>, GuiError>;

    /// Poll [`find_image`](VisionCapability::find_image) (every 200 ms) until
    /// a match is found or `timeout_ms` elapses. Returns the matches (empty
    /// vector on timeout) and the elapsed milliseconds.
    async fn wait_for_image(
        &self,
        template_encoded: Vec<u8>,
        precision: f32,
        region: Option<Region>,
        timeout_ms: u64,
    ) -> Result<(Vec<ImageMatch>, u64), GuiError>;
}
