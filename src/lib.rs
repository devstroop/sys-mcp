pub mod config;
pub mod error;
pub mod gui;
pub mod mcp;
pub mod platform;
pub mod protocol;
pub mod terminal;
pub mod web;

pub use gui::backend::stub::StubBackend;
pub use mcp::hub::McpHub;
