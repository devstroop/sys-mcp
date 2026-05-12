# Changelog

All notable changes to sys-mcp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **File System** tools:
  - `gui_read_file` — Read file as base64
  - `gui_write_file` — Write base64-encoded content to file
  - `gui_list_dir` — List directory contents
  - `gui_file_exists` — Check if file/directory exists
  - `gui_delete_file` — Delete a file
  - `gui_create_dir` — Create directory (with parents)

- **Shell/Terminal** tools:
  - `gui_shell_exec` — Execute single shell command and capture output
  - `gui_shell_open` — Open interactive shell session (returns session_id)
  - `gui_shell_write` — Write input to shell session
  - `gui_shell_read` — Read output from shell session
  - `gui_shell_close` — Close shell session
  - `gui_shell_list` — List all open shell sessions

- **MCP Hub** (MCP Server Passthrough/Tunnel):
  - `mcp_discover` — Scan for MCP servers in `~/.mcp/`, `.mcp/`, and npm packages
  - `mcp_list` — List registered MCP servers
  - `mcp_register` — Register a new MCP server
  - `mcp_unregister` — Unregister an MCP server
  - `mcp_start` — Start an MCP server
  - `mcp_stop` — Stop an MCP server
  - `mcp_tools` — List all tools from running MCP servers
  - `mcp_tool_groups` — Get tools grouped by category (YouTrack pattern)
  - `mcp_exec` — Execute tool from any running MCP server

- **Object Detection** tools:
  - `gui_detect_objects` — Detect UI elements using YOLOv8
  - `gui_click_object` — Click a detected object by label

- **Terminal/PTY** module:
  - `src/terminal/` — New module using `portable-pty` for cross-platform PTY support
  - Supports Windows (PowerShell), Linux (bash), and macOS (zsh/bash)

- **MCP Hub** module:
  - `src/mcp/hub.rs` — New module for MCP server discovery and passthrough
  - Enables sys-mcp to tunnel to local MCP servers when running remotely

### Changed

- Backend now uses `portable-pty` instead of platform-specific PTY APIs
- MCP Hub integrates with discovery from npm packages containing "mcp"

### Fixed

- Fixed session manager test in `src/mcp/session.rs`

## [0.1.0] - 2025-04-12

### Added

- MCP server with JSON-RPC 2.0 over stdio
- **Display**: `gui_screenshot`, `gui_screenshot_region`, `gui_get_screen_size`, `gui_list_monitors`
- **OCR**: `gui_read_screen` (full screen or region), `gui_find_text` (text search with click coordinates)
  - On-device OCR via `ocrs` — models auto-downloaded on first use (~15MB)
  - Returns structured word-level bounding boxes with center coordinates
- **Mouse**: `gui_click`, `gui_double_click`, `gui_mouse_move`, `gui_mouse_position`, `gui_drag`, `gui_scroll`
- **Keyboard**: `gui_type_text`, `gui_press_key` (supports key combos with `+`)
- **Clipboard**: `gui_get_clipboard`, `gui_set_clipboard`
- **Window management**: `gui_list_windows`, `gui_get_active_window`, `gui_focus_window`, `gui_find_windows`, `gui_move_resize_window`, `gui_window_action`
  - Full Win32 API implementation on Windows
  - X11 stubs on Linux, AppKit stubs on macOS
- **Accessibility**: `gui_accessibility_tree`, `gui_find_ui_element` (trait and tool schemas defined)
- **Template matching**: `gui_find_image`, `gui_wait_for_image` (tool schemas defined)
- **Web preview**: `gui_web_preview` — launches a token-authenticated Axum web server with live desktop viewer, click-through, keyboard passthrough, and scroll support
- **System**: `gui_system_info` — reports OS, screen size, and available capabilities
- Cross-platform architecture with capability-based trait system
- `rustautogui` submodule for core GUI automation
- Optional feature flags: `ocr`, `clipboard`, `web-preview` (on by default), `opencl` (off)

### Not Yet Implemented

- `gui_find_image` / `gui_wait_for_image` — tool schemas present, handlers return error
- Accessibility backend — trait defined, `as_accessibility()` returns `None`
- Multi-monitor support in `list_monitors` — returns primary monitor only