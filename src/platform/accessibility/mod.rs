// Accessibility platform implementations — Phase 6
// Each platform file will be added when accessibility feature is implemented.

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
