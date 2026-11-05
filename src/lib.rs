pub mod config;
pub mod decision;
#[cfg(any(feature = "ocr", feature = "detection"))]
pub mod download;
pub mod error;
pub mod gui;
pub mod mcp;
pub mod platform;
pub mod protocol;
pub mod sysinfo;
pub mod system;
pub mod terminal;
pub mod web;

pub use gui::backend::stub::{
    StubA11yBackend, StubA11yRecorder, StubBackend, StubSystem, StubSystemRecorder,
};
pub use mcp::hub::McpHub;
