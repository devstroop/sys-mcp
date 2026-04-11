#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
pub use windows::PlatformWindowManager;
#[cfg(target_os = "linux")]
pub use linux::PlatformWindowManager;
#[cfg(target_os = "macos")]
pub use macos::PlatformWindowManager;
