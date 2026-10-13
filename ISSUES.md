# Issues & Next-Phase Development Gaps (sys-mcp)

**Last verified: 2026-10-13** — every status below was checked against the code in this worktree.

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
| 6 | Test coverage is critically low | ✅ Fixed (171 tests) |
| 7 | `handlers.rs` is too large (1546 lines) | ✅ Fixed |
| 8 | CLI argument parsing is fragile | ✅ Fixed (clap) |
| 9 | MCP Hub JSON-RPC I/O framing | ✅ Fixed |
| 10 | ANSI escape stripping is heuristic | ✅ Fixed |
| 11 | Multi-monitor support is incomplete | ✅ Fixed (Windows) |
| 12 | No rate limiting on web preview | ✅ Fixed |
| 13 | CORS is overly permissive | ✅ Fixed |
| 14 | CI only runs on Linux | ✅ Fixed |
| 15 | No resource or prompt support in MCP protocol | 🟢 Open |
| 16 | Template matching (Phase 7) | ✅ Fixed |
| 17 | Key combo duplication | ✅ Fixed |
| 18 | No connection pooling for model downloads | 🔵 Open |
| 19 | `Box::leak` in PTY default shell resolution | ✅ Fixed |
| 20 | No graceful shutdown for child processes in MCP Hub | ✅ Fixed |
| 21 | Stale `gui_*` tool-name references after tool rename | ✅ Fixed |
| 22 | Blocking process calls inside async handlers | ✅ Fixed |
| 23 | `shell_exec` fixed 500ms wait loses output | ✅ Fixed |
| 24 | Tool coverage gap (22 of 64 tools tested) | ✅ Fixed (171 tests) |
| 25 | Tests lack platform cfg gating | � In progress |
| 26 | `network_connections` ignores its `limit` parameter | ✅ Fixed |
| 27 | `detect_objects` model URL 404 (dead asset link) | ✅ Fixed |
| 28 | `Screenshot.data` doc claimed raw RGBA (it holds PNG bytes) | ✅ Fixed |
| 29 | `detection` feature made opt-in (heaviest feature, +56 crates) | ✅ Fixed |
| 30 | Advertised tools unreachable when their feature is off (`detect_objects`, `web_preview`) | ✅ Fixed |

---

## 🔴 Critical Bugs

### 23. `shell_exec` fixed 500ms wait loses output

**File:** `src/mcp/handlers/shell.rs` (`handle_shell_exec`)
**Problem (verified):** After sending the command and closing the PTY, the handler sleeps a fixed 500ms and then reads the output buffer. Any command slower than 500ms silently loses or truncates its output. There is no completion detection (no exit status check, no read-until-EOF).
**Status:** ✅ Fixed — the fixed 500ms sleep was replaced with quiet-period polling (50ms poll,
250ms of silence to consider the command finished, 30s hard cap), so slower commands no longer lose
output.
**Severity:** 🔴 Critical — silently incorrect results.

---

## 🟡 High-Priority Gaps

### 4. Accessibility — all platforms are stubs — ✅ Fixed

**Fixed:** `src/platform/accessibility/{windows,macos,linux}.rs` all contain real implementations (Windows: COM `IUIAutomation` with `ControlViewWalker`; macOS: `AXUIElement` C FFI via `objc`/`core-foundation`; Linux: AT-SPI2 over D-Bus via `zbus`). All three implement `get_tree` and `find_elements`; both tools (`accessibility_tree`, `accessibility_find`) are dispatched in `src/mcp/handlers/mod.rs` and declared in `tools.rs`. Residual gaps: `get_element_properties` and `invoke_element_action` return `UnsupportedCapability` on all three platforms; the Linux implementation uses blocking `zbus` calls and ignores `window_id` (always walks from the AT-SPI root).

### 5. Object detection is a stub — ✅ Fixed

**Fixed:** `src/gui/detection.rs` performs real YOLOv8n ONNX inference via `tract-onnx` (letterbox preprocessing, confidence threshold, NMS), downloads the model from the ultralytics assets release URL on first use, and ships the full 80-class COCO label list.

### 6. Test coverage is critically low — ✅ Fixed

**Fixed:** 66 tests total (64 across `tests/{handler,protocol,hub,config}_tests.rs` + 2 unit tests in `src/mcp/session.rs`), up from 8. Remaining coverage gaps are tracked as issue #24.

### 21. Stale `gui_*` tool-name references after tool rename

**Files:** `src/mcp/tools.rs`, `src/mcp/server.rs`
**Problem:** Tools were renamed to a category scheme (`gui_screenshot` → `screen_capture`, etc.), but many tool descriptions still reference the old names (`gui_read_screen`, `gui_screenshot`, `gui_click`, `gui_list_windows`, `gui_shell_open`/`gui_shell_write`, `gui_delete_dir`, `gui_detect_objects`, …). Worse, this is functional in one place: `src/mcp/server.rs` special-cases the tool name `gui_web_preview`, but `tools.rs` advertises the tool as `web_preview` — so the advertised `web_preview` tool falls through to `handle_tool_call`, hits no dispatch arm, and returns "unknown tool".
**Status:** ✅ Fixed — all 24 stale references corrected across `tools.rs` and the handlers, and
`server.rs` now dispatches `web_preview` under its advertised name.
**Severity:** 🟡 High — misleading docs for agent callers plus one uncallable advertised tool.

### 22. Blocking process calls inside async handlers

**Files:** `src/mcp/handlers/system.rs`, `src/system.rs`, `src/sysinfo.rs`
**Problem (verified):** Handlers such as `handle_process_list`, `handle_service_list`, `handle_network_info`, `handle_system_stats`, and `handle_system_logs` call synchronous functions directly inside `async fn` without `spawn_blocking`. Those functions shell out with blocking `std::process::Command::output()` (`sc`, `systemctl`, `launchctl`, `powershell`, `netstat`, `ss`, `top`, `df`, `journalctl`, `log`, …). Long-running commands block a tokio worker thread for their whole duration.
**Status:** ✅ Fixed — all 12 blocking calls in the system handlers are wrapped in
`tokio::task::spawn_blocking`, matching the pattern already used in `ocr.rs` and `shell.rs`.
**Severity:** 🟡 High — runtime stall under load, slow tools.

### 24. Tool coverage gap (22 of 64 tools tested)

**Files:** `tests/handler_tests.rs`, `src/mcp/tools.rs`
**Problem (verified):** `tools.rs` declares 64 tools, but `handler_tests.rs` exercises only 22 distinct tools (display, mouse, keyboard, clipboard, window, `system_info`, `system_wait`). Entire categories have zero handler coverage: `fs_*`, `shell_*`, `mcp_*`, `process_*`, `service_*`, `network_*`, `system_stats`/`disk_usage`/`system_logs`, `screen_read`/`screen_find_text`, accessibility, detection, `web_preview`.
**Status:** Being addressed on branch `test/destructive-tools`.
**Severity:** 🟡 High.

---

## 🟢 Medium-Priority Improvements

### 7. `handlers.rs` is too large (1546 lines) — ✅ Fixed

**Fixed:** Handlers were split into `src/mcp/handlers/` with `mod.rs` (dispatch) + 10 category modules (`accessibility.rs`, `detection.rs`, `filesystem.rs`, `mcp_hub.rs`, `ocr.rs`, `shell.rs`, `system.rs`, `ui.rs`, `vision.rs`, `window.rs`); the largest is ~240 lines.

### 8. CLI argument parsing is fragile — ✅ Fixed

**Fixed:** `src/config.rs` uses `clap` derive (`#[derive(Parser)]`) with long flags, env-var overrides (`SYS_MCP_*`), and `ValueEnum` for `TransportMode`. Covered by `tests/config_tests.rs`.

### 9. `mcphub` single-line read for JSON-RPC I/O — ✅ Fixed

**Fixed:** `send_jsonrpc()` in `src/mcp/hub.rs` reads the child server's stdout through a `BufReader` and parses complete JSON-RPC values; multi-line responses are no longer truncated.

### 10. ANSI escape stripping is heuristic — ✅ Fixed

**Fixed:** The `strip_ansi_codes()` helper in `src/mcp/handlers/shell.rs` is a thin wrapper over the `strip-ansi-escapes` crate (a declared dependency in `Cargo.toml`); the hand-rolled state machine is gone.

### 11. Multi-monitor support is incomplete

**Status:** ✅ Fixed on Windows. `list_monitors` previously returned one synthetic
`"Primary"` entry sized from `get_screen_size()`, so a secondary display was
invisible and any coordinate relative to it was unusable. Windows now enumerates
for real via `EnumDisplayMonitors` + `GetMonitorInfoW`, implemented against the
Win32 GDI API already enabled in `Cargo.toml` rather than through `rustautogui`
(which only reports the primary display's size) — so no vendored-submodule change
was required. **Non-Windows still reports primary only**, now stated explicitly
rather than implied complete.

Gotcha recorded: `GetMonitorInfoW` only populates `szDevice` when `cbSize` is
`sizeof(MONITORINFOEXW)`. Passing `sizeof(MONITORINFO)` — which the struct's
`MONITORINFO` head invites — silently yields an empty device name.

Verified live on a single-display host (`\\.\DISPLAY1`, 3440×1440 at (0,0),
clean shutdown). The multi-monitor branch is therefore exercised by construction,
not observation; `tests/monitor_tests.rs` asserts the invariants so a genuine
multi-display machine will catch a regression.

### 12. No rate limiting on web preview — ✅ Fixed

**Fixed:** `src/web/server.rs` has a `RateLimiter` (sliding-window, keyed per client IP, configured at 60 requests/second) checked by every API route, plus a per-run UUID token guard (`check_token`).

### 13. CORS is overly permissive — ✅ Fixed

**Fixed:** `src/mcp/http_transport.rs` builds the `CorsLayer` conditionally: `allow_origin(Any)` only when binding to `0.0.0.0`/`::`; for any other bind host it allows only the origin `http(s)://{host}:{port}` derived from the configured host/port.

### 14. CI only runs on Linux — ✅ Fixed

**Fixed:** `.github/workflows/ci.yml` runs a matrix over `ubuntu-latest`, `macos-latest`, `windows-latest` (`fail-fast: false`), and `.github/workflows/release.yml` builds cross-platform release binaries on tag push.

### 15. No resource or prompt support in MCP protocol

**File:** `src/mcp/server.rs` (`handle_initialize`)
**Problem (verified):** The `initialize` response still advertises only `"capabilities": { "tools": {} }`, and the dispatch handles only `initialize`, `initialized`, `tools/list`, `tools/call`, and `ping`. No `resources` or `prompts` support.
**Suggestion:** Add resource templates for file system access and prompt templates for common GUI automation tasks.

---

## 🔵 Low-Priority / Future Enhancements

### 16. Template matching (Phase 7)

**Status:** ✅ Fixed — both handlers now run over rustautogui's CPU FFT
normalized-cross-correlation path (no OpenCL feature needed), and both
tools are declared in `tools/list` with full schemas. New `VisionCapability`
backend trait (`src/gui/vision.rs`); the template is accepted as
base64-encoded PNG/JPEG (matching `fs_read` output) or a file path, with
encoded-input validation that rejects raw pixel data with a clear error.
Platform limitation (documented by rustautogui, now in the tool
descriptions and responses): on Windows only the main monitor is searched;
on Linux all monitors.

**Files:** `src/mcp/handlers/vision.rs`, `src/mcp/tools.rs`
**Problem (verified):** `handle_find_image` and `handle_wait_for_image` in `vision.rs` return a permanent "Template matching not yet implemented" error, and their schemas are hidden from `tools/list` (comment block in `tools.rs`). The `rustautogui` submodule contains template-matching implementations (`src/core/template_match/`: slow NCC, segmented NCC, FFT NCC, OpenCL variants) that still need wiring into the handlers and re-exposing in `tools.rs`.

### 17. Key combo duplication

**Status:** ✅ Fixed — the two call sites normalized *differently*, which was a
real defect, not just duplication. `ui.rs` lowercased and trimmed, while
`web/server.rs` did neither, so `Ctrl + S` worked over MCP but sent
`["ctrl ", " s"]` (and failed) over the web API, and single keys were not
lowercased at all on the web path. Extracted `gui::types::split_key_combo()`
and both entry points now share it.

### 18. No connection pooling for model downloads

**File:** `src/gui/ocr.rs:92`
**Problem (verified):** OCR model download uses synchronous `ureq::get()` (`ensure_model`), triggered lazily from `get_engine()` on first use — call sites wrap `read_screen`/`find_text` in `tokio::task::spawn_blocking`, so the runtime is not blocked. Remaining gap: no retry/resume for interrupted downloads.

### 19. `Box::leak` in PTY default shell resolution

**Status:** ✅ Fixed — `get_default_shell()` now returns an owned `String`.
The leak was per-spawn (one allocation per PTY session), so it grew over the
server's lifetime rather than being a one-off.

### 20. No graceful shutdown for child processes in MCP Hub

**Status:** ✅ Fixed — `stop_server()` now drops the child's stdin (sending EOF,
which is the normal shutdown signal for a stdio MCP server), waits up to 3s
for a clean exit, and only then falls back to `kill()`. A wedged child still
cannot block the caller indefinitely.

### 25. Tests lack platform cfg gating

**Files:** `tests/*.rs`
**Problem:** No test is gated with `#[cfg(windows)]`/`#[cfg(unix)]`/`#[cfg(target_os = ...)]`. Whether every test is valid on all three CI platforms is unverified beyond "CI is green" — most handler tests pass through `StubBackend` and so never exercise platform code.

### 26. `network_connections` ignores its `limit` parameter

**File:** `src/mcp/handlers/system.rs` (`handle_network_connections`)
**Problem (verified):** The tool schema in `tools.rs` declares a `limit` parameter (default 50), and the handler reads it into `let _limit = …` but never applies it — `NetworkManager::list_connections()` results are returned untruncated. (`process_list` by contrast does apply `limit` via `processes.truncate(limit)`.)

---

## Summary of Next-Phase Development Gaps

Current state of the planned feature phases:

### Phase 6 — Accessibility ✅ Done (element-level gaps remain)
- All three platforms implemented: Windows UI Automation (COM), macOS AXUIElement, Linux AT-SPI2 (zbus). Both tools are live in the dispatch and in `tools/list` (see issue #4).
- Remaining: `get_element_properties` / `invoke_element_action` unsupported on all platforms; Linux ignores `window_id` and uses blocking D-Bus calls.

### Phase 7 — Template Matching ✅ Done
- `vision_find_image` / `vision_wait_for_image` wired to rustautogui's CPU FFT NCC path via the new `VisionCapability` backend trait; both tool schemas exposed in `tools/list` (issue #16).

### Phase 8 — Window Management: Linux + macOS ✅ Done
- Linux: `x11rb`-based implementation with EWMH atoms (`_NET_CLIENT_LIST`, `_NET_WM_STATE`, …). X11 only — no Wayland support.
- macOS: `CGWindowListCopyWindowInfo` for enumeration plus Cocoa/objc messaging for focus/move/resize/minimize/maximize/restore/close.

### Phase 9 — Object Detection ✅ Done
- Real YOLOv8n ONNX inference via `tract-onnx`, model auto-download, COCO labels (issue #5).

### Phase 10 — Cross-Platform System Monitoring ✅ Done
- `src/system.rs` has per-OS branches (`sc`/`powershell` on Windows, `systemctl`/`ss`/`ip`/`journalctl` on Linux, `launchctl`/`vm_stat`/`sysctl`/`top -l`/`log show` on macOS). CI compiles and tests on all three OSes.

### Phase 11 — Testing & Quality 🔓 Partially done
- Done: 66 tests (handler dispatch via `StubBackend`, protocol serialization, hub, config).
- Remaining: destructive-tool coverage (branch `test/destructive-tools`, issue #24), platform cfg gating (issue #25), web server tests, property-based JSON-RPC roundtrips.

### Phase 12 — CI/CD ✅ Done
- Three-OS test matrix in `ci.yml`; release workflow builds cross-platform binaries on tag push (`release.yml`).
