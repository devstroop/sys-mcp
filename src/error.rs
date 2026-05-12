//! Application error types.
//!
//! Defines the [`GuiError`] enum covering screenshot, input, window, clipboard,
//! accessibility, OCR, detection, template matching, platform, and backend errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuiError {
    #[error("screenshot failed: {0}")]
    ScreenshotFailed(String),

    #[error("input error: {0}")]
    InputError(String),

    #[error("window management error: {0}")]
    WindowError(String),

    #[error("clipboard error: {0}")]
    ClipboardError(String),

    #[error("accessibility error: {0}")]
    AccessibilityError(String),

    #[error("OCR error: {0}")]
    OcrError(String),

    #[error("detection error: {0}")]
    DetectionError(String),

    #[error("template matching error: {0}")]
    TemplateMatchError(String),

    #[error("capability not supported: {0}")]
    UnsupportedCapability(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("platform error: {0}")]
    PlatformError(String),

    #[error("backend error: {0}")]
    BackendError(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
