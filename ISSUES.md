# Issues & Next-Phase Development Gaps (sys-mcp)

## Priority Legend

| Label | Meaning |
|-------|---------|
| 🔴 Critical | Bug or blocker — causes incorrect behavior |
| 🟡 High | Missing feature or gap in a core capability |
| 🟢 Medium | Polish, DX, or non-blocking improvement |
| 🔵 Low | Nice-to-have or future enhancement |

---

## Status

| # | Issue | Status |
|---|-------|--------|
| 1 | `send_jsonrpc()` truncates multi-line responses | ✅ Fixed |
| 2 | macOS system monitoring uses Linux-only commands | ✅ Fixed |
| 3 | Linux & macOS window management are stubs | ✅ Fixed |
| 4 | Accessibility — all platforms are stubs | ✅ Fixed |
| 5 | Object detection is a stub | ✅ Fixed |
| 6 | Test coverage is critically low | ✅ Fixed (66 tests) |
| 7 | `handlers.rs` is too large (1546 lines) | ✅ Fixed |
| 8 | CLI argument parsing is fragile | ✅ Fixed (clap) |
| 9 | MCP Hub JSON-RPC I/O framing | 🟢 Open |
| 10 | ANSI escape stripping is heuristic | 🟢 Open |
| 11 | Multi-monitor support is incomplete | 🟢 Open |
| 12 | No rate limiting on web preview | 🟢 Open |
| 13 | CORS is overly permissive | 🟢 Open |
| 14 | CI only runs on Linux | 🟢 Open |
| 15 | No resource or prompt support in MCP protocol | 🟢 Open |
| 16 | Template matching (Phase 7) | 🔵 Open |
| 17 | Key combo duplication | 🔵 Open |
| 18 | No connection pooling for model downloads | 🔵 Open |
| 19 | `Box::leak` in PTY default shell resolution | 🔵 Open |
| 20 | No graceful shutdown for child processes in MCP Hub | 🔵 Open |

---

## 🔴 Critical Bugs (None remaining)

All critical bugs have been resolved.

---

## 🟡 High-Priority Gaps

### 4. Accessibility — all platforms are stubs

**Files:** `src/platform/accessibility/{windows,linux,macos}.rs`
**Problem:** All three are placeholder comments only. Tools `gui_accessibility_tree` and `gui_find_ui_element` exist in the handler dispatch but are commented out of the tools list.
**Planned APIs:** Windows → UI Automation (COM), Linux → AT-SPI2 (D-Bus), macOS → AXUIElement.
**Severity:** 🟡 High

### 5. Object detection is a stub

**File:** `src/gui/detection.rs`
**Problem:** Returns hardcoded mock detections (laptop, mouse, keyboard, cup). No YOLOv8 ONNX inference implemented. Model download stub logs a message but never downloads.
**Severity:** 🟡 High

### 6. Test coverage is critically low

**Count:** 8 tests total (6 hub integration + 2 session unit) for ~6000 lines of Rust and 54+ MCP tools.
**Missing:** No tests for handlers, protocol serialization, backends (StubBackend exists but is untested), web server, platform code, OCR, terminal/PTY, CLI config, error paths.
**Foundation:** `StubBackend` provides a good test double — tests should be written against it.
**Severity:** 🟡 High

---

## 🟢 Medium-Priority Improvements

### 7. `handlers.rs` is too large (1546 lines)

**File:** `src/mcp/handlers.rs`
**Problem:** Single file contains the dispatch match statement and all 50+ tool handler functions. Hard to navigate, review, and maintain.
**Suggestion:** Split into a directory: `src/mcp/handlers/` with files per category (e.g., `display.rs`, `input.rs`, `window.rs`, `ocr.rs`, `filesystem.rs`, `shell.rs`, `mcp_hub.rs`, `system.rs`).

### 8. CLI argument parsing is fragile

**File:** `src/config.rs`
**Problem:** Manual `std::env::args()` parsing instead of a library like `clap`. No subcommand support, no environment variable overrides, inconsistent error messages.
**Suggestion:** Migrate to `clap` with derive macros.

### 9. `mcphub` single-line read for JSON-RPC I/O

**File:** `src/mcp/hub.rs:302-322`
**Related to:** Bug #1 above, but the deeper issue is the I/O strategy. Instead of a line-based protocol reader, use a proper framing mechanism (e.g., newline-delimited JSON with buffered reading until a complete JSON value is formed).

### 10. ANSI escape stripping is heuristic

**File:** `src/mcp/handlers.rs:1257-1294`
**Problem:** `strip_ansi_codes()` uses a state machine that handles common CSI sequences but may miss or mishandle edge cases (SGR with multiple parameters, OSC sequences, DCS sequences, etc.).
**Suggestion:** Use a well-tested crate like `strip-ansi-escapes` or `console`.

### 11. Multi-monitor support is incomplete

**File:** `src/gui/backend/local.rs`
**Problem:** `list_monitors()` always returns a single primary monitor at (0,0) with the screen resolution. The `rustautogui` library may return monitor information, but it's not plumbed through.

### 12. No rate limiting on web preview

**File:** `src/web/server.rs`
**Problem:** The web preview server has no rate limiting on its API routes. A misbehaving client could hammer the screenshot/click/type endpoints.
**Suggestion:** Add a simple token-bucket or request-per-second limiter.

### 13. CORS is overly permissive

**File:** `src/mcp/http_transport.rs:43-44`
**Problem:** `CorsLayer::new().allow_origin(Any)` allows any origin to make requests.
**Suggestion:** Make the CORS origin configurable, defaulting to a safe value or documenting that `Any` is not production-safe.

### 14. CI only runs on Linux

**File:** `.github/workflows/ci.yml`
**Problem:** No Windows or macOS runners. Platform-specific code (Win32 window management, macOS frameworks) is never compiled or tested in CI.
**Suggestion:** Add a matrix build with `os: [ubuntu-latest, windows-latest, macos-latest]`.

### 15. No resource or prompt support in MCP protocol

**File:** `src/mcp/server.rs`
**Problem:** The server only advertises `"tools": {}` capabilities. The MCP spec also supports `resources` and `prompts`.
**Suggestion:** Add resource templates for file system access and prompt templates for common GUI automation tasks.

---

## 🔵 Low-Priority / Future Enhancements

### 16. Template matching (Phase 7)

**Files:** `src/mcp/handlers.rs:621-628`
**Status:** Handlers for `gui_find_image` and `gui_wait_for_image` return permanent errors. The `rustautogui` submodule likely has template matching capabilities that need to be wired up.

### 17. Key combo duplication

**Problem:** `gui_press_key` splits key names on `+` in the handler (`handlers.rs:472-477`) to call `client.key_combo()`. But `GuiClient.key_combo()` also exists as a separate method. This dual path could lead to confusion about where key combo logic lives.

### 18. No connection pooling for model downloads

**File:** `src/gui/ocr.rs:92-101`
**Problem:** `ureq` is used synchronously inside `tokio::task::spawn_blocking` for downloading OCR models. For a one-time download this is acceptable, but it blocks a worker thread.

### 19. `Box::leak` in PTY default shell resolution

**File:** `src/terminal/pty.rs`
**Problem:** The `$SHELL` env var is leaked via `Box::leak` to produce a `&'static str`. Minor memory leak (process-lifetime only) but worth noting.

### 20. No graceful shutdown for child processes in MCP Hub

**File:** `src/mcp/hub.rs:208-224`
**Problem:** `stop_server()` calls `child.kill().await` but does not attempt a graceful SIGTERM before SIGKILL on Unix, nor does it wait for the process to exit.

---

## Summary of Next-Phase Development Gaps

The following represents the planned feature phases and remaining work:

### Phase 6 — Accessibility (Unstarted)
- Implement UI Automation COM on Windows
- Implement AT-SPI2 via D-Bus on Linux
- Implement AXUIElement on macOS
- Wire up `gui_accessibility_tree` and `gui_find_ui_element` in `tools.rs`

### Phase 7 — Template Matching (Unstarted)
- Wire up `rustautogui` template matching in `gui_find_image` and `gui_wait_for_image`
- Uncomment tool schemas in `tools.rs`

### Phase 8 — Window Management: Linux + macOS (Unstarted)
- Implement `PlatformWindowManager` for Linux using `x11rb`
- Implement `PlatformWindowManager` for macOS using `CoreGraphics` + `Cocoa`
- Test on both platforms

### Phase 9 — Object Detection (Stub → Real)
- Implement actual YOLOv8 ONNX inference (choose `ort` or `tract-onnx` crate)
- Implement model download
- Remove mock data

### Phase 10 — Cross-Platform System Monitoring
- Add macOS-specific branches using `sysctl`, `vm_stat`, `launchctl`, `log show`, etc.
- Test all system utilities on macOS

### Phase 11 — Testing & Quality
- Add unit tests for protocol serialization
- Add integration tests for all tool handlers using `StubBackend`
- Add web server integration tests
- Add CLI config parsing tests
- Add property-based testing for JSON-RPC roundtrips

### Phase 12 — CI/CD
- Add macOS and Windows CI runners
- Add release workflow with binary publishing
- Add cargo-deny for dependency auditing
