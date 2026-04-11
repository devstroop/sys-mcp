# Changelog

All notable changes to gui-mcp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
