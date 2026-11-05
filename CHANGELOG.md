# Changelog

All notable changes to sys-mcp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **JEV decision layer** (cargo feature `jev`, on by default):
  - `decision` tool — evaluate a state against typed JEV (`/v1/systemone`)
    questions (`noul` / `choice` / `score`) with secret redaction before send
  - `ui_decide` tool (Phase 4, opt-in via `--jev-ui-loop` /
    `SYS_MCP_JEV_UI_LOOP=1`, default **off**) — bounded action loop that
    drives the foreground window toward a goal: observe → JEV picks one
    action from a bounded candidate set → Rust safety gates → execute →
    re-observe. Completion is reported `likely_done` (never `done`);
    irreversible actions are held for a per-call `confirm: true`; typed text
    comes only from caller-supplied literals; more than 40 actionable
    elements is `ambiguous` (never a silent truncation); two unchanged
    rounds or an exhausted step budget (default 12, cap 20) end as `stuck`;
    an inaccessible/elevated window is `blocked` and never driven.
  - Opt-in guardrails for destructive tools (`--jev-guardrails`, default off):
    a hardcoded Rust denylist (offline, model-independent) plus a JEV `noul`
    risk check that holds calls for confirmation (`confirm: true` to proceed).
    The denylist is deterministic and never bypassed; when the model cannot be
    reached, `--jev-guard-fail-mode` decides and defaults to `closed`, so a
    guard failure is never a silent allow
  - Guarded by default when enabled: `shell_exec`, `fs_delete`, `process_kill`,
    `service_stop`, `service_start`, `process_start`
  - New config: `--jev-guardrails` / `SYS_MCP_JEV_GUARDRAILS`,
    `--jev-guard-threshold` / `SYS_MCP_JEV_GUARD_THRESHOLD` (default 0.5),
    `--jev-guard-tools` / `SYS_MCP_JEV_GUARD_TOOLS`,
    `--jev-guard-fail-mode` / `SYS_MCP_JEV_GUARD_FAIL_MODE` (default `closed`)

- **Accessibility element-level tools** (all platforms — Windows UI Automation,
  macOS `AXUIElement`, Linux AT-SPI2):
  - `accessibility_properties` — resolve a single element by the id handed out
    by `accessibility_tree` / `accessibility_find` and return its current
    role, name, value, position, actions, and states
  - `accessibility_action` — invoke a named action on an element by id
    (generic verbs `invoke`/`press`/`click`/`focus`/`increment`/`decrement`/
    `close`/`minimize`/`maximize`/`restore`, or a raw platform action name).
    `set_value` plus the optional `value` argument writes text or a number
    straight into the control — UIA Value/RangeValue/Legacy `SetValue`, macOS
    `AXValue`, AT-SPI `EditableText.SetTextContents` — instead of typing it,
    so the field keeps its identity and the session does not depend on
    keyboard focus
  - Element ids stay resolvable between calls: Windows/macOS retain the most
    recent tree walk's elements in a per-tree cache (COM refs / `CFRetain`ed
    `AXUIElementRef`s, cleared on each `get_tree` and on `Drop`); Linux ids are
    stable AT-SPI D-Bus object paths

- **File System** tools:
  - `fs_read` — Read file as base64
  - `fs_write` — Write base64-encoded content to file
  - `fs_list` — List directory contents
  - `fs_exists` — Check if file/directory exists
  - `fs_delete` — Delete a file
  - `fs_create_dir` — Create directory (with parents)

- **Shell/Terminal** tools:
  - `shell_exec` — Execute single shell command and capture output
  - `shell_open` — Open interactive shell session (returns session_id)
  - `shell_write` — Write input to shell session
  - `shell_read` — Read output from shell session
  - `shell_close` — Close shell session
  - `shell_list` — List all open shell sessions

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
  - `detect_objects` — Detect UI elements using YOLOv8
  - `detect_click_object` — Click a detected object by label

- **Terminal/PTY** module:
  - `src/terminal/` — New module using `portable-pty` for cross-platform PTY support
  - Supports Windows (PowerShell), Linux (bash), and macOS (zsh/bash)

- **MCP Hub** module:
  - `src/mcp/hub.rs` — New module for MCP server discovery and passthrough
  - Enables sys-mcp to tunnel to local MCP servers when running remotely

### Changed

- Backend now uses `portable-pty` instead of platform-specific PTY APIs
- MCP Hub integrates with discovery from npm packages containing "mcp"
- **OCR coverage is now an argument, and every OCR response states what it
  covered** (`docs/ISSUES.md` #34). `screen_read` takes
  `strategy: "single" | "thorough"` (default `single`, the previous behaviour);
  `screen_find_text` and `screen_scroll_to_text` take
  `strategy: "auto" | "single"` (default `auto`). `thorough`/`auto` read the
  frame at the detector's native scale by sweeping 600x800 windows with 96px
  overlap instead of one downscaled pass, and an unknown value is refused rather
  than run as the cheaper leg — the argument exists precisely so the caller
  knows how hard the tool looked. Whole-screen reads now label themselves
  ("one downscaled pass … strategy 'thorough' re-reads it as 8 native-scale
  600x800 windows") and a text miss names its own coverage
  ("Search covered 8 native-scale windows")
- **Requests on one connection are now handled concurrently, so responses can
  arrive out of order** (`docs/ISSUES.md` #32). Correlate them by JSON-RPC `id`,
  as the protocol expects; the framing is still exactly one message per line.
  A client that needs ordering (click, then read what the click did) must await
  each response as before — what changed is that an unanswered slow request no
  longer holds up everything else on the connection.

### Fixed

- **One slow tool stalled the whole server** (`docs/ISSUES.md` #32). The stdio
  read loop held a `Mutex<McpRequestHandler>` across each `await`, so `window_list`
  and `screen_size` timed out behind an abandoned full-screen `screen_read` and the
  server looked hung. The mutex existed only because `handle_request` took
  `&mut self`, which nothing in the request path actually needed — the sole writer
  is the one-time web-preview start. `handle_request` now takes `&self`, requests
  run in a `JoinSet` while the loop keeps reading, and the same lock removed from
  stdio also frees the HTTP transport, which had been serializing every session on
  the server. Writes stay one whole line each (encoded before the sink lock is
  taken), in-flight requests are capped at 32 as admission control rather than
  spawned without limit, stdin EOF drains in-flight answers instead of dropping
  them, and a panicking request now costs its own response instead of poisoning the
  handler for the rest of the session.
- Fixed session manager test in `src/mcp/session.rs`
- **Full-screen OCR missed small UI text** (`docs/ISSUES.md` #34). `ocrs` hands
  every image to the detection model's fixed input tensor (600x800 for the
  shipped model): a larger frame is resized with the aspect ratio ignored, so a
  1920-wide screen is squeezed to ~0.31x and 11-14px glyphs become ~3px features
  the detector never fires on. Measured on one live 1920x1080 capture, a whole-
  screen pass read 5 of 12 lines that a tight crop reads correctly and returned
  71 lines where the native-scale sweep returned 179. `screen_find_text` now
  escalates to that sweep automatically on a miss, so it can no longer report
  text absent from a screen it only looked at once
- **`screen_scroll_to_text` declared absence without reading the final screen.**
  The loop checked, then scrolled, so the frame it settled on was never
  inspected; it is now captured and searched (at native scale by default) before
  "not found after N scrolls" is returned

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