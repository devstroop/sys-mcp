# Issues & Next-Phase Development Gaps (sys-mcp)

**Last verified: 2026-11-05** — every status below was checked against the code in this worktree.
Issues #35–#46 are the output of a full source audit of branch `feat/ocr-onnx-a11y-tools` @ `343ff06`
(72 declared tools, 3 platform backends, 2 guard layers, both workflows); each carries the file:line
it was read from.

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
| 15 | No resource or prompt support in MCP protocol | ✅ Fixed |
| 16 | Template matching (Phase 7) | ✅ Fixed |
| 17 | Key combo duplication | ✅ Fixed |
| 18 | Model downloads not atomic/verified/retried | ✅ Fixed |
| 19 | `Box::leak` in PTY default shell resolution | ✅ Fixed |
| 20 | No graceful shutdown for child processes in MCP Hub | ✅ Fixed |
| 21 | Stale `gui_*` tool-name references after tool rename | ✅ Fixed |
| 22 | Blocking process calls inside async handlers | ✅ Fixed |
| 23 | `shell_exec` fixed 500ms wait loses output | ✅ Fixed |
| 24 | Tool coverage gap (22 of 64 tools tested) | ✅ Fixed (171 tests) |
| 25 | Tests lack platform cfg gating | ✅ Resolved (see note) |
| 26 | `network_connections` ignores its `limit` parameter | ✅ Fixed |
| 27 | `detect_objects` model URL 404 (dead asset link) | ✅ Fixed |
| 28 | `Screenshot.data` doc claimed raw RGBA (it holds PNG bytes) | ✅ Fixed |
| 29 | `detection` feature made opt-in (heaviest feature, +56 crates) | ✅ Fixed |
| 30 | Advertised tools unreachable when their feature is off (`detect_objects`, `web_preview`) | ✅ Fixed |
| 31 | OCR inference compiled unoptimized made `screen_read` exceed client timeout | ✅ Fixed (dev-profile opt-level 3) |
| 32 | stdio transport serializes every request — one slow tool stalls all tools | 🟡 Open |
| 33 | `screen_read` returned crop-relative coordinates for region reads, so `mouse_click` missed the target | ✅ Fixed |
| 34 | Full-screen OCR has lower small-text recall than a region read of the same pixels | 🟡 Open |
| 35 | `shell_write` reaches the same PTY as `shell_exec` but is neither guarded nor denylisted | ✅ Fixed |
| 36 | Guard fail-mode is ignored on the decision-client construction path (fails open) | ✅ Fixed |
| 37 | Windows window enumeration never resolves `process_name` | ✅ Fixed |
| 38 | Windows accessibility exposes no value/description/state and only Invoke + Window patterns | ✅ Fixed |
| 39 | `accessibility_properties` re-mints an id and mutates the tree cache it only reads | ✅ Fixed |
| 40 | Serialization failures return an empty tool payload instead of an error | ✅ Fixed |
| 41 | `system_logs` defaults to Information-only, reads the System log only, and reports every failure as `[]` | 🟡 Open |
| 42 | `screen_read` selects region mode on the presence of `x` alone; unknown `detail` silently picks the heaviest branch | 🟢 Open |
| 43 | 20 of 72 declared tools have no test reference; the four accessibility tools are untestable at handler level | 🟡 Open |
| 44 | CI never runs for branch pushes and tests no feature combination; the release workflow has never run | 🟡 Open |
| 45 | Documentation that contradicts the shipped behavior | 🟢 Open |
| 46 | No accessibility path writes a value into an element (`set_value` missing from the trait) | 🟡 Open |

---

## 🔴 Critical Bugs

### 23. `shell_exec` fixed 500ms wait loses output

**File:** `src/mcp/handlers/shell.rs` (`handle_shell_exec`)
**Problem (verified):** After sending the command and closing the PTY, the handler sleeps a fixed 500ms and then reads the output buffer. Any command slower than 500ms silently loses or truncates its output. There is no completion detection (no exit status check, no read-until-EOF).
**Status:** ✅ Fixed — the fixed 500ms sleep was replaced with quiet-period polling (50ms poll,
250ms of silence to consider the command finished, 30s hard cap), so slower commands no longer lose
output.
**Severity:** 🔴 Critical — silently incorrect results.

### 33. `screen_read` with a region returned crop-relative coordinates

**File:** `src/mcp/handlers/ocr.rs` (`handle_read_screen`)
**Problem (verified 2026-11-04):** `LocalBackend::screenshot_region` crops with `crop_imm` and returns an image of
*region* size, `ocr::read_screen` reports coordinates in that image's space, and the handler never added the
region origin back. The tool description tells callers to feed `cx`/`cy` straight into `mouse_click`, so every
region-based click landed up and left of the target by exactly (x, y) — measured live while driving BUSY: a menu
item OCR'd inside a region at (320,40,260,220) came back with `cx` ≈ 36 and the click hit a different window.
**Status:** ✅ Fixed — the translation lives in `ocr::translate_to_screen` (`src/gui/ocr.rs`), called by the
handler, and is covered by four unit tests (origin added to line/word `x`/`y` and word `cx`/`cy`; centres stay
consistent with the moved box; extents untouched; zero origin is identity). Verified against UIA ground truth:
reading a (200,60) region over BUSY's "Report Options" form returned `OK (F2)` at cx=257, cy=368 while UIA reports
the button center at 257,369, and clicking the OCR coordinate completed the form. The summary header now labels a
region read as `Region WxH at (X,Y) — coordinates are screen-space` instead of `Screen WxH`, because the OCR result
carries the *crop* dimensions and reporting them as the screen size put `cy` beyond the stated height.
**Severity:** 🔴 Critical — the tool's own advertised contract produced wrong clicks.

### 35. `shell_write` reaches the same PTY as `shell_exec` but is neither guarded nor denylisted

**Files:** `src/decision/guard.rs` (`guarded_tools`, `check_denylist`), `src/mcp/handlers/shell.rs` (`handle_shell_write`)
**Problem (verified 2026-11-05):** the destructive-command denylist matches patterns (`format `, `diskpart`, `rm -rf /`, `mkfs`, …) only against the `command` argument of `shell_exec`, and `guarded_tools()` is `shell_exec, fs_delete, process_kill, service_stop, service_start, process_start` — `shell_write` is in neither set. `handle_shell_write` pushes raw bytes into a live PTY session created by `shell_open`. So `shell_open` → `shell_write "format C: /q"` executes with **zero** denylist coverage, which is the layer documented as "never bypassed by `confirm: true`". The same shape applies to `fs_write` (overwrite any path) and `mcp_exec` (arbitrary tool call on a registered child server).
**Status:** ✅ Fixed — `shell_write` joins the default guarded set, its `input` goes through the same `deny_reason_for_command` patterns as `shell_exec`'s `command`, and its schema now declares the `confirm` property the held-call message tells the caller to use (drift test `guarded_tool_schemas_advertise_confirm` enforces that). Two structural changes came out of the same hole: **the denylist is now keyed on what a tool does, not on the guarded set** — `guard_gate` checks it *before* `is_guarded`, so narrowing `--jev-guard-tools` can no longer delete a deterministic block — and `fs_write` is consequently covered by the path rules (overwriting `C:\Windows\…` is as fatal as deleting it) **without** being model-judged, deliberately: routine file writing is the main agent workflow and a network round trip per write would make the guard useless. `mcp_exec` stays out of both sets — its arguments are a *child server's* tool name and payload, which this process has no vocabulary to classify; when the child is a sys-mcp instance the child applies its own guard, and a non-sys-mcp child is trusted by construction. Known, documented limitation: pattern matching is **per call**, so input split across several `shell_write` calls into one PTY is never assembled here — that case is left to the model layer, and `DESTRUCTIVE_COMMAND_PATTERNS`' doc says so. Covered by `denylist_covers_shell_write_input`, `denylist_covers_fs_write_paths`, `shell_write_input_is_screened_by_the_command_denylist` and `denylist_applies_to_tools_outside_the_guarded_set`.
**Severity:** 🔴 Critical — the guard's hardest guarantee is bypassable by the tool with the same name prefix.

### 36. Guard fail-mode is ignored on the decision-client construction path (fails open)

**File:** `src/mcp/handlers/mod.rs` (`guard_gate`)
**Problem (verified 2026-11-05):** `assess_with` honours `--jev-guard-fail-mode` correctly — a model transport failure under the default `GuardFailMode::Closed` returns `Ok(Confirm)` and the call is held (`src/decision/guard.rs`, live module). But the dispatcher constructs the client first, and `Err` from `DecisionClient::new(&config)` returns `None` — *proceed* — with no reference to the configured fail mode (`mod.rs` "Fail open (denylist already applied)"). A configuration mistake in endpoint/timeout/key is exactly the case where an operator who enabled `--jev-guardrails` expects a hold, and instead gets the destructive call executed. Its sibling arm `Err(e) => … (failing open)` is unreachable, because `assess` maps every outcome to `Ok`.
**Status:** ✅ Fixed — `guard_gate` now reads `config.guard_fail_mode` on **both** paths that cannot consult the model: `DecisionClient::new` failure (held under `Closed`, skipped under `Open`) and the previously-unreachable `assess` `Err` arm, which is now routed through the same policy instead of asserting "do not block on plumbing errors". The shared `held(...)` message keeps the caller's escape hatch (`re-call with confirm: true`), so `Closed` remains a hold and never a hard block. The `guard_gate` docstring claiming "model failure fails open" is corrected to describe the configured mode. Caveat kept honest: `DecisionClient::new` only fails if `reqwest::Client::builder().build()` fails, which is not reachable from a test without injecting the constructor, so that arm is verified by inspection plus the end-to-end `handler_fails_closed_when_model_unreachable` test that exercises the sibling path through the dispatcher.
**Severity:** 🔴 Critical — a safety control that silently becomes a no-op under misconfiguration.

---

## 🟡 High-Priority Gaps

### 4. Accessibility — all platforms are stubs — ✅ Fixed

**Fixed:** `src/platform/accessibility/{windows,macos,linux}.rs` all contain real implementations (Windows: COM `IUIAutomation` with `ControlViewWalker`; macOS: `AXUIElement` C FFI via `objc`/`core-foundation`; Linux: AT-SPI2 over D-Bus via `zbus`). All three implement `get_tree`, `find_elements`, `get_element_properties`, and `invoke_element_action`; all four tools (`accessibility_tree`, `accessibility_find`, `accessibility_properties`, `accessibility_action`) are dispatched in `src/mcp/handlers/mod.rs` and declared in `tools.rs`. Element ids stay resolvable between calls: Windows/macOS retain the elements from the most recent tree walk in a per-tree cache (COM refs / `CFRetain`ed `AXUIElementRef`s, cleared on each `get_tree` and on `Drop`), while Linux ids are stable AT-SPI D-Bus object paths that resolve directly. `invoke_element_action` maps generic verbs (invoke/press/click/focus/increment/decrement/close/minimize/maximize/restore) onto the platform action plus raw platform action names. Residual gap: the Linux implementation uses blocking `zbus` calls and ignores `window_id` (always walks from the AT-SPI root).

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

### 31. OCR inference compiled unoptimized made `screen_read` exceed the client timeout

**Files:** `Cargo.toml` (`[profile.dev.package.*]`), `src/gui/ocr.rs`
**Problem (measured 2026-11-04):** the server the IDE launches is `target\debug\sys-mcp.exe`, so `rten`/`ocrs`
ran at opt-level 0. Phase timings for a 1920x1080 capture: model load 25ms + 16ms and `OcrEngine::new` 80µs
(init is *not* the cost), `prepare_input` 152ms, `detect_words` **18.4s**, `find_text_lines` 200ms, and
`recognize_text` over 115 lines did not finish within 4 minutes (~4 cores saturated). Any MCP client timeout
is far shorter, so full-screen `screen_read` returned error 40504 while small regions succeeded.
**Status:** ✅ Fixed — `opt-level = 3` applied to the OCR hot-path crates (`ocrs`, `image`, `rten` and its
`rten-*` sub-crates) in the dev profile; the `sys-mcp` crate itself stays unoptimized. Full-screen read now
completes in **27.4s** (95 lines) and a cold 400x120 region read in **1.68s**. Prefer passing a `region` and
`detail: "text"`; full-screen reads remain the slowest call the server makes.
**Re-measured 2026-11-04 (optimized build):** full-screen `screen_read` on a BUSY screen: 8.6s (32 lines), 10.4s
(54 lines), 6.0s (24 lines); region reads 1.9-5.5s; `screen_find_text` 5.9s. Wall time scales with the number of
lines recognised, not with frame size, so a busy screen costs more than an empty one.
**Severity:** 🟡 High — was making a headline tool unusable.

### 32. stdio transport serializes every request, so one slow tool stalls all tools

**File:** `src/mcp/server.rs` (`run_stdio`)
**Problem (verified):** the read loop takes `self.handler.lock().await` and awaits `handle_request` inline
before reading the next line. While a request is in flight — e.g. the full-screen OCR above, which kept running
after the client gave up — *every* other tool call queues behind it and also times out. Observed live:
`window_list` and `screen_size` returned 40504 while an abandoned `screen_read` was still computing, which
looks exactly like a hung server and sent debugging down the wrong path.
**Status:** 🟡 Open — fixing it needs `handle_request` to stop requiring `&mut self` (interior mutability or a
per-request clone) before handling can be spawned concurrently; response writes are already serialized behind
the stdout lock, and JSON-RPC ids let responses land out of order.
**Severity:** 🟡 High — a single slow call degrades the whole server's availability.

### 34. Full-screen OCR has lower small-text recall than a region read of the same pixels

**Files:** `src/gui/ocr.rs` (`read_screen`), `src/mcp/handlers/ocr.rs` (`handle_find_text`)
**Problem (observed 2026-11-04):** BUSY's 11-14px menu-pane text was read reliably by a 420x140 region (8/11 lines
recognised) while a full-screen `screen_read` of the same pixels alternated between reading it and returning 0 lines
for it. `ocrs` rescales the input to a working size derived from the image dimensions, so a 1920x1080 frame puts
small glyphs through a different detection scale than a tight crop. Consequence: `screen_find_text` (full-screen only)
can report "No text matching 'Trial Balance' found on screen" for text that `screen_read` on the region finds at the
same instant. Recognition accuracy also drifts with scale ("Day Book" read as "Dav Book", "OK (F2)" as "OKE)l").
**Status:** 🟡 Open — the fix is a search strategy, not a bug fix: for a region-scoped lookup, run detection on
overlapping tiles at native scale (or upscale small crops) instead of one whole-screen pass, and document that
`screen_find_text` is a full-screen best-effort locator while `screen_read` + region is the precise one.
**Severity:** 🟡 High — an agent can conclude UI is absent when it is only missed.

### 37. Windows window enumeration never resolves `process_name`

**File:** `src/platform/window/windows.rs` (`get_window_info`)
**Problem (verified 2026-11-05):** `get_window_info` hardcodes `process_name: None`, and it is the single constructor used by `list_windows`, `get_active_window` and `find_windows_by_title`, so every `WindowInfo` produced on Windows has a null process. Linux resolves `WM_CLASS` (`src/platform/window/linux.rs`, `get_process_name`) and macOS reads `kCGWindowOwnerName` (`src/platform/window/macos.rs`) — Windows, the platform the product is actually used on, is the only one missing it. Two consequences: `ui_decide` sends `"window": { "process": null }` into the JEV prompt (`src/decision/ui_loop.rs`), so the model decides with no idea which application owns the target; and a caller cannot prove a window belongs to the app it believes it does, or disambiguate same-titled windows, which is the failure class behind wrong-window clicks in desktop automation.
**Status:** ✅ Fixed — `get_window_info` resolves the image name from the `GetWindowThreadProcessId` pid through `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` and reports the **file stem**, so the value matches the bare names Linux/macOS produce and the `ProcessName` that `process_list` already returns; it stays `None` only when the handle genuinely cannot be opened (elevated / protected processes) or the query fails.
**Severity:** 🟡 High — silently degrades a documented field and a model input on the primary platform.

### 38. Windows accessibility exposes no value/description/state and only Invoke + Window patterns

**File:** `src/platform/accessibility/windows.rs`
**Problem (verified 2026-11-05):** `build_node` sets `value: None, description: None` unconditionally; `get_element_states` reports only `disabled` and `focused`; and `get_element_actions` advertises only `invoke` plus the four window verbs. `invoke_element_action` matches `invoke | focus | close | minimize | maximize | restore` and every other string errors, so the generic verbs `press`, `click`, `increment`, `decrement` — which the macOS backend aliases (`macos.rs`, `invoke_element_action`) and which `docs/ISSUES.md` #4 claims are mapped "on all platforms" — are rejected on Windows. No `TogglePattern`, `ValuePattern`, `ExpandCollapsePattern`, `SelectionItemPattern` or `RangeValuePattern` is imported at all, so none of their read state or actions are reachable. Net effect on the platform that matters: an agent cannot read what is in a text field, tick a checkbox, open a combo box, select a list item, or tell whether a node is expanded/offscreen through UIA, and has to fall back to OCR for all of it.
**Status:** ✅ Fixed — `build_node` now fills `value` (ValuePattern → RangeValuePattern → LegacyIAccessible, capped at 4096 chars because a document view reports its whole text as its value) and `description` (`CurrentHelpText` → LegacyIAccessible description; UIA has no `CurrentFullDescription` on the client element). States add `offscreen` / `password` / `required` / `readonly` / `checked` / `indeterminate` / `expanded` / `partially_expanded` / `collapsed` / `selected` / `focus`, each read from the live pattern rather than assumed. Actions are derived from the patterns the element actually implements — Invoke, Toggle, ExpandCollapse, SelectionItem, RangeValue, ScrollItem, Window — and `invoke_element_action` was rewritten to canonicalize verbs case-insensitively with `-` read as `_`, accept `press` / `click` / `tap` / `do_default_action` as "activate" (Invoke → Toggle → legacy default action, so a caller stating intent works against a checkbox and against a VCL button alike), and implement `toggle` / `expand` / `collapse` / `select` / `increment` / `decrement` (RangeValue stepped by `CurrentSmallChange` and clamped to `CurrentMinimum`…`CurrentMaximum` — UIA publishes no Increment method) / `scroll_into_view` / `focus`. Unsupported verbs fail with the element's live action list echoed in the error, so the caller can self-correct from the response alone. **Not** fixed: writing a value into a field, which the trait cannot express — see #46.
**Severity:** 🟡 High — this is the gap that forces OCR onto every form field.

### 40. Serialization failures return an empty tool payload instead of an error

**Files:** `src/mcp/handlers/*.rs` (`window.rs`, `accessibility.rs`, `ocr.rs`, `system.rs`, `ui.rs`, `filesystem.rs`, `mcp_hub.rs`, `shell.rs`, `decision.rs`, `ui_loop.rs`)
**Problem (verified 2026-11-05):** nine handler sites serialize with `serde_json::to_string[_pretty](…).unwrap_or_default()`. `unwrap_or_default()` on `String` is `""`, and because the `Ok` arm is taken the `ToolResult` carries **no `is_error` flag** and no text. An agent that receives an empty result from `window_list` reads it as "no windows are open", from `accessibility_find` as "no matching element", from `screen_find_text` as "the text is not on screen" — the exact false-absence conclusion issue #34 warns about, manufactured inside the handler rather than by the model.
**Status:** ✅ Fixed — `json_pretty` / `json_compact` in `src/mcp/handlers/mod.rs` return `Result<String, String>` and every serialization site now propagates with `?`, so a serialize bug is reported as an error instead of posing as an empty result. The sweep found the pattern in more places than the original nine and they are fixed too:
- `unwrap_or("[]".to_string())` / `unwrap_or("{}".to_string())` in `system.rs` (10 sites), `filesystem.rs`, `mcp_hub.rs`, `shell.rs` — the issue text called these "already correct", but a sentinel is **indistinguishable from a genuinely empty list**, which is the same false absence in a nicer costume. They propagate now.
- `unwrap_or_else(|_| payload.to_string())` in `decision.rs` (3 sites) — fallback was dead code (`json!` values always serialize), and `ui_loop.rs`'s `to_value(&outcome).unwrap_or_else(… { "status": "error" })` fabricated an error-shaped *success* payload.
- `String::from_utf8(buffer).unwrap_or_default()` in `shell.rs` (2 sites): one non-UTF-8 byte — routine for a cp850 box-drawing character in a Windows PTY — erased the **entire** `shell_exec` / `shell_read` output. Now `from_utf8_lossy`, which keeps every line and substitutes U+FFFD for the byte that is not valid.
- `opt_str_array` in `mod.rs` replaces the `args.get("args").and_then(as_array).map(…).unwrap_or_default()` idiom in `process_start` and `mcp_register`: a present-but-not-an-array value (a string, an object) used to be absorbed into *no arguments*, so the process or child MCP server started without the caller's arguments and the tool reported success. Non-string elements are rejected too instead of being dropped by `filter_map`. Covered by three tests in `tests/handler_tests.rs`.
- `spawn_blocking_get_buffer` in `shell.rs` returns `Result` now; the old `unwrap_or_default()` reported a poisoned output mutex as "no output".
**Not fixed here, recorded as remaining:** `src/system.rs` and `src/sysinfo.rs` read command output through nested `if let Ok(text) = String::from_utf8(output.stdout)`, so a parse failure skips the branch and the function returns an empty vec — same family, but inside the platform command layer rather than the tool handlers, and fixing it means threading errors through `SystemMonitor` / `ProcessManager` return types. The `--jev-guard-tools` denylist work in #35 is unaffected.
**Severity:** 🟡 High — silent-absence answers are worse than errors for an autonomous caller.

### 41. `system_logs` defaults to Information-only, reads the System log only, and reports every failure as `[]`

**Files:** `src/system.rs` (`LogViewer::get_system_logs`), `src/mcp/handlers/system.rs`
**Problem (verified 2026-11-05):** `level.unwrap_or_else(|| "Information")` maps to `Get-WinEvent … Level=4`, so calling `system_logs` with no arguments returns only informational entries and **silently omits every Error and Warning** — the entries the tool is called for. The query is pinned to `LogName='System'`, so the Application log (where application crashes land) is unreachable although the tool is advertised as "system/event logs". Finally every failure surface collapses to the same empty answer: `Get-WinEvent` writes to stderr and exits non-zero with empty stdout when a filter matches nothing, and `String::from_utf8` / `serde_json::from_str` failures are swallowed by `if let Ok(...)`, so "log empty", "no events matched", "powershell not on PATH" and "parse failed" are all indistinguishable `[]`. Untested.
**Status:** 🟡 Open — make "no level" mean all levels (omit the key from the filter hashtable), accept a `log` parameter (`System` / `Application`), and surface a real error when the command fails rather than returning `[]`.
**Severity:** 🟡 High — a diagnostic tool that hides the failures being diagnosed.

### 43. 20 of 72 declared tools have no test reference; the four accessibility tools are untestable at handler level

**Files:** `tests/`, `src/gui/backend/stub.rs`
**Problem (verified 2026-11-05):** the declared/dispatched sets match exactly (0 orphan tools — `tests/tool_schema_tests.rs` is a real drift guard and works), but 20 advertised tools appear in no test file: `accessibility_tree`, `accessibility_find`, `accessibility_properties`, `accessibility_action`, `screen_scroll_to_text`, `decision_rank`, `decision_stats`, `detect_click_object`, `disk_usage`, `system_stats`, `system_logs`, and all eight `mcp_*` hub tools. The four accessibility tools are structurally untestable today: `StubBackend` implements no `AccessibilityCapability`, so `GuiClient::accessibility()` returns `UnsupportedCapability` and no handler path beyond the error can be exercised. `tests/handler_tests.rs` covers display/mouse/keyboard/clipboard/window only.
**Status:** 🟡 Open — give the stub backends an accessibility implementation (fixed tree + id cache semantics, recording invoked actions), then add handler tests for all four tools plus the id-invalidation contract; add tests for `system_logs` / `system_stats` / `disk_usage` argument handling and the `mcp_*` lifecycle against a fake child server.
**Severity:** 🟡 High — the newest, most intricate surface (element-id caching) shipped with no coverage at all.

### 44. CI never runs for branch pushes and tests no feature combination; the release workflow has never run

**Files:** `.github/workflows/ci.yml`, `.github/workflows/release.yml`
**Problem (verified 2026-11-05):**
- `on: push: branches: [main, master]` — a push to a feature branch triggers **no** workflow, so nothing on `feat/ocr-onnx-a11y-tools` has remote CI signal; it only starts at PR time. Every claim that the branch is "CI-clean" is a local claim.
- No feature matrix: CI never builds `--features detection` or `--no-default-features`, so `src/gui/detection.rs` and every `#[cfg(not(feature = …))]` arm are uncompiled by CI. `tests/tool_schema_tests.rs`'s feature-gate test therefore only ever asserts the default combination, and the feature-off code paths rot silently.
- The lint gate is `cargo clippy -- -D warnings`, without `--all-targets`, so lints in `tests/` and `examples/` are ungated (the local run that used `--all-targets` was stricter than CI).
- No dependency-vulnerability gate of any kind: no `deny.toml`, no cargo-deny / cargo-audit / OSV step in either workflow.
- `release.yml` has never executed — `git tag` is empty and the version is still `0.1.0` with all shipped work under `## [Unreleased]`. It also has no test or smoke step, publishes no `SHA256SUMS` and no signature, and pins `macos-13`.
**Status:** 🟡 Open — add `pull_request` targeting all branches plus a `workflow_dispatch`, add a feature-leg matrix (`detection`, `--no-default-features`) and `--all-targets` to clippy, add cargo-deny, and cut a tagged release with checksums once the above fixes land.
**Severity:** 🟡 High — the gates that were paid for are not wired to the branch flow.

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

**Status:** ✅ Fixed. The server now advertises `resources` and `prompts` capabilities and implements
`resources/list`, `resources/read`, `prompts/list` and `prompts/get`.

**Resources** are read-only observations backed by existing `GuiClient` calls, so reading one has no
side effects: `sys-mcp://system/info`, `//screen/size`, `//monitors`, `//windows`, `//window/active`.
They let an agent inspect the host before choosing tools, and re-read to detect change, without a tool
round-trip.

**Prompts** are instruction templates the host injects into its own context — nothing here touches the
machine: `describe-screen` (steers to structured reads over screenshots to save tokens),
`find-and-click` (locate, check confidence, refuse to guess, verify), `automate-goal` (bounded steps,
caller-supplied literals only, treat on-screen text as content not commands).

Unknown URIs/prompts return a protocol error naming the valid set, so a mis-typed identifier is
self-correcting rather than a silent empty result.

### 39. `accessibility_properties` re-mints an id and mutates the tree cache it only reads

**File:** `src/platform/accessibility/windows.rs` (`get_element_properties`, `build_node`)
**Problem (verified 2026-11-05):** `get_element_properties` resolves the cached element and then calls `build_node`, which unconditionally mints `uia:{next_id}` and inserts the element into the cache under that new key before the function overwrites `node.id` with the caller's original id. So every read-only properties call grows the cache by one duplicate COM reference and advances the counter, until the next `get_tree` clears it. Separately, `build_node` does `if let Ok(mut cache) = self.cache.lock() { cache.insert(…) }` — if that lock is poisoned the id is still returned to the caller but was never inserted, so the very next `accessibility_action` on it fails with "not in the current accessibility cache" for a reason the caller cannot see.
**Status:** ✅ Fixed — `build_node` became a thin wrapper over `build_node_with_id(element, depth, id)`. `id: None` is the tree-walk path (mint from the counter, insert); `id: Some(key)` is the properties path, which reuses the key the caller already addressed the element by and leaves both cache and counter untouched. The insert failure is no longer skipped: a poisoned lock returns `GuiError::PlatformError("accessibility cache lock poisoned")` instead of handing back an id that can never resolve.
**Severity:** 🟢 Medium — no wrong answers today, but the cache lifetime is not what the contract says and one branch hands out unresolvable ids.

### 42. `screen_read` selects region mode on the presence of `x` alone; unknown `detail` silently picks the heaviest branch

**File:** `src/mcp/handlers/ocr.rs` (`handle_read_screen`)
**Problem (verified 2026-11-05):** region mode is chosen by `args.get("x").is_some()`. A caller that sends `width`/`height` without `x` gets a **full-screen** read with those arguments silently ignored — and then a `Screen 1920x1080` header for a request that looked scoped. Conversely `x` without the rest errors as "missing required argument", which is at least loud. `detail` is matched as `"text" | "lines" | _`, so any typo (`"full"` is not a listed enum value in the schema, `"detailled"`, `"Lines"`) falls through to the default arm — the word-level serialization, the largest and slowest payload — instead of an error.
**Status:** 🟡 Open — enter region mode only when all four coordinates are present, reject a partial region explicitly, and validate `detail` against the declared enum.
**Severity:** 🟢 Medium — misconfigured calls answer a different question than the one asked.

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

### 18. Model downloads not atomic, verified or retried

**Status:** ✅ Fixed. The stated gap was the missing retry; the worse half was that both fetchers —
OCR weights (`src/gui/ocr.rs`) and YOLOv8n (`src/gui/detection.rs`) — wrote **straight to the final
path**. An interrupted download therefore left a truncated file at the destination, and the
`path.exists()` check then treated it as a valid model **forever**. No amount of retrying repairs
that: the cache stays corrupt until manually cleared.

Both now go through `src/download.rs`, which downloads to a sibling temp file, verifies the byte
count against `Content-Length`, syncs to disk and only then `rename`s into place — atomic within one
directory on both Windows and Unix, so the destination is either absent or complete. Three attempts
with 2s/4s backoff, temp file cleared between tries, pid-scoped name so two processes cannot collide.

`tests/download_tests.rs` drives it against a local HTTP stub (no network): the key case asserts a
short body is rejected **and not published at the destination** — the exact regression that used to
poison the model cache. Connection pooling was considered and rejected: these are one-shot downloads
per machine, so pooling would add machinery for no benefit.

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

**Status:** ✅ Resolved — and deliberately *not* by adding blanket `#[cfg]` gates.

The concern was that test validity outside the author's platform was unverified. That is now covered
more strongly than cfg-gating would: CI runs the suite on `ubuntu-latest`, `macos-latest` **and**
`windows-latest` with `fail-fast: false`, so every test is executed — not skipped — on all three.

How the suite actually handles platform differences:

- **Platform-independent by construction.** The large majority of tests drive `StubBackend`, so they
  never touch the OS and need no gating.
- **Four tests exercise real OS interfaces** — `process_list`, `service_list`, `network_info`,
  `network_connections` — which shell out to `powershell`/`systemctl`/`launchctl`/`netstat`. They
  assert portable invariants (no error, well-formed JSON array) rather than OS-specific contents, and
  the CI matrix verifies them on each platform. Gating these would *reduce* coverage: a `#[cfg]` skips
  the test instead of proving it works.
- **One test genuinely needs platform data.** `tests/monitor_tests.rs`'s Windows enumeration test is
  `#[cfg(windows)]` and additionally degrades to a skip when no desktop session is available (e.g.
  session-0 CI), so it cannot fail on infrastructure it does not control. This is the suite's first
  cfg-gated test and the pattern to copy if another platform-specific case appears.

So the rule adopted: gate only where the *behaviour itself* is platform-specific; otherwise write
portable assertions and let the CI matrix prove them.

### 45. Documentation that contradicts the shipped behavior

**Files:** `CHANGELOG.md`, `src/mcp/handlers/mod.rs`, `docs/ISSUES.md`, `src/gui/ocr.rs`, `src/mcp/handlers/accessibility.rs`, `README.md`
**Problem (verified 2026-11-05):** five separate statements that the code does not support.
1. `CHANGELOG.md` — "model failure fails open, the denylist never does", and the same sentence in the `guard_gate` docstring in `src/mcp/handlers/mod.rs`. The default is `GuardFailMode::Closed`, which *holds* the call (see #36 for the one path that genuinely fails open).
2. `docs/ISSUES.md` #4 claims `invoke_element_action` maps `invoke/press/click/focus/increment/decrement/…` on all platforms. macOS aliases them; Windows rejects all but invoke/focus/window verbs (see #38).
3. `src/gui/ocr.rs` — four doc comments on `OcrWord.cx/cy` and `OcrMatch.cx/cy` still tell callers to "use this directly for `gui_click`", the pre-rename name. #21 was closed as "all 24 stale references corrected" but only covered `tools.rs` and the handlers.
4. `src/mcp/handlers/accessibility.rs` ends with a "Template matching handlers" banner with nothing under it, and both that file and `src/mcp/handlers/ocr.rs` open with `#![allow(dead_code)]`, which now suppresses real dead-code findings in two live modules from the `-D warnings` gate.
5. `README.md` self-admits its per-feature crate-count table predates `jev` joining the default set.
**Status:** 🟡 Open — correct each to match the code, drop the stale banner and the module-level `allow(dead_code)`.
**Severity:** 🔵 Low — but an integrator reading the security posture sentence gets the opposite of what ships.

### 46. No accessibility path writes a value into an element (`set_value` missing from the trait)

**Files:** `src/gui/accessibility.rs` (`AccessibilityCapability`), all three `src/platform/accessibility/*`
**Problem (verified 2026-11-05):** `#38` is closed without a way to put text into an edit field, deliberately. `AccessibilityCapability::invoke_element_action(element_id, action)` takes an action *name* and no value argument, so `ValuePattern::SetValue` / `RangeValuePattern::SetValue` are unreachable through the public API on every platform — advertising a `set_value` action would advertise a call that cannot carry the value it needs. Form driving therefore still ends in keyboard injection via `shell`/`gui` input tools rather than the accessibility tree, which is slower and loses the field's identity. Fixing it properly means widening the trait (an `action` + optional `value` pair, or a separate `set_element_value`), implementing it on Windows (Value → RangeValue), macOS (`AXValue`) and Linux (AT-SPI `SetContents`), then extending the tool schema, the handler, `get_element_actions` and the `StubBackend` recording from #43.
**Status:** 🟡 Open — design decision recorded while fixing #38; needs the trait change, not a pattern call.
**Severity:** 🟡 High — the remaining reason a form-automation session falls back to keystrokes and OCR.

### 26. `network_connections` ignores its `limit` parameter

**File:** `src/mcp/handlers/system.rs` (`handle_network_connections`)
**Problem (verified):** The tool schema in `tools.rs` declares a `limit` parameter (default 50), and the handler reads it into `let _limit = …` but never applies it — `NetworkManager::list_connections()` results are returned untruncated. (`process_list` by contrast does apply `limit` via `processes.truncate(limit)`.)

---

## Summary of Next-Phase Development Gaps

Current state of the planned feature phases:

### Phase 6 — Accessibility ✅ Done
- All three platforms implemented: Windows UI Automation (COM), macOS AXUIElement, Linux AT-SPI2 (zbus). All four tools are live in the dispatch and in `tools/list` (see issue #4).
- `get_element_properties` / `invoke_element_action` now implemented on all platforms (element caches on Windows/macOS, stable D-Bus paths on Linux).
- Remaining: Linux ignores `window_id` and uses blocking D-Bus calls.
- Audit 2026-11-05: Windows now reads value/description and the expanded / checked / selected / offscreen states and drives the Toggle, Value, ExpandCollapse, SelectionItem, RangeValue and ScrollItem patterns (#38, #39). Writing a value into an element is still unreachable on any platform because the capability trait has no value parameter — tracked as #46.

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
- Audit 2026-11-05: the suite is now 248 tests and the schema/dispatch drift guard is in place, but 20 advertised tools still have no test reference and `StubBackend` cannot exercise the accessibility handlers at all — tracked as issue #43.

### Phase 12 — CI/CD ✅ Done
- Three-OS test matrix in `ci.yml`; release workflow builds cross-platform binaries on tag push (`release.yml`).
- Audit 2026-11-05: the matrix works but is not wired to the branch flow — feature-branch pushes trigger nothing, no feature combination is ever built, and `release.yml` has never run because no tag exists. Tracked as issue #44.
