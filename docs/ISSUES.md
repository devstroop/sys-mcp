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
| 6 | Test coverage is critically low | ✅ Fixed (341 passing / 4 ignored today) |
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
| 24 | Tool coverage gap (22 of 64 tools tested) | ✅ Fixed (341 passing / 4 ignored today; 7 of 72 uncovered — #43) |
| 25 | Tests lack platform cfg gating | ✅ Resolved (see note) |
| 26 | `network_connections` ignores its `limit` parameter | ✅ Fixed |
| 27 | `detect_objects` model URL 404 (dead asset link) | ✅ Fixed |
| 28 | `Screenshot.data` doc claimed raw RGBA (it holds PNG bytes) | ✅ Fixed |
| 29 | `detection` feature made opt-in (second-heaviest alone, +56; `jev` costs +59) | ✅ Fixed |
| 30 | Advertised tools unreachable when their feature is off (`detect_objects`, `web_preview`) | ✅ Fixed |
| 31 | OCR inference compiled unoptimized made `screen_read` exceed client timeout | ✅ Fixed (dev-profile opt-level 3) |
| 32 | stdio transport serializes every request — one slow tool stalls all tools | ✅ Fixed (`handle_request` takes `&self`; requests run in a `JoinSet` capped at 32 in flight; EOF drains; the HTTP transport shared the same lock and is fixed too) |
| 33 | `screen_read` returned crop-relative coordinates for region reads, so `mouse_click` missed the target | ✅ Fixed |
| 34 | Full-screen OCR has lower small-text recall than a region read of the same pixels | ✅ Fixed (native-scale window sweep at the detector's 600x800 working size; `strategy` on the three OCR tools; every read reports its own coverage) |
| 35 | `shell_write` reaches the same PTY as `shell_exec` but is neither guarded nor denylisted | ✅ Fixed |
| 36 | Guard fail-mode is ignored on the decision-client construction path (fails open) | ✅ Fixed |
| 37 | Windows window enumeration never resolves `process_name` | ✅ Fixed |
| 38 | Windows accessibility exposes no value/description/state and only Invoke + Window patterns | ✅ Fixed |
| 39 | `accessibility_properties` re-mints an id and mutates the tree cache it only reads | ✅ Fixed |
| 40 | Serialization failures return an empty tool payload instead of an error | ✅ Fixed |
| 41 | `system_logs` defaults to Information-only, reads the System log only, and reports every failure as `[]` | ✅ Fixed |
| 42 | `screen_read` selects region mode on the presence of `x` alone; unknown `detail` silently picks the heaviest branch | ✅ Fixed |
| 43 | 20 of 72 declared tools have no test reference; the four accessibility tools are untestable at handler level | 🟡 Partially fixed (7 of 72 remain — the `mcp_*` hub tools need a fake child server) |
| 44 | CI never runs for branch pushes and tests no feature combination; the release workflow has never run | 🟡 Partially fixed (gates wired + proven remotely; supply-chain triaged green; release side still unexercised) |
| 45 | Documentation that contradicts the shipped behavior | ✅ Fixed |
| 46 | No accessibility path writes a value into an element (`set_value` missing from the trait) | ✅ Fixed (`invoke_element_action` carries an optional value; `set_value` writes on all three platforms) |
| 47 | The `StubBackend` suite is not hermetic: system handlers shell out for real, and the detector loads its model | ✅ Fixed (SystemCapability + Ocr/Detection seams; the default suite no longer shells out or loads a model) |
| 48 | Reading the JEV config before `init` permanently disabled the guardrails (denylist silently off) | ✅ Fixed |

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

### 48. Reading the JEV config before `init` permanently disabled the guardrails

**Files:** `src/decision/mod.rs` (`config`, `init`), `src/mcp/handlers/mod.rs` (`guard_gate`), `src/decision/guard.rs` (`assess`), `tests/guard_eval.rs`
**Problem (root-caused 2026-11-05 from the ubuntu-latest failure in CI run 41943845280 — issue #44):** `config()` was `JEV_CONFIG.get_or_init(JevConfig::default).clone()` while `init()` was `let _ = JEV_CONFIG.set(config)`. `OnceLock` is first-*writer*-wins, and `get_or_init` writes: merely *reading* the process config installed `JevConfig::default()` — whose `guardrails` is `false` (`config.rs:132` resolves clap defaults) — and the real install was then discarded silently by `set()` returning `Err`. `guard_gate` opens with `if !config.guardrails { return None }` (= proceed), so the denylist, the guarded-set check, `confirm` handling and the model judgment all stopped running for the rest of the process lifetime — destructive tools dispatched unguarded, with no log line and no error.
Any caller could arm it: `guard::assess` reads `config().guard_threshold` (guard.rs:388) and `config().guard_fail_mode` (guard.rs:468, 470), and `handlers/decision.rs` (lines 20, 89) plus `handlers/ui_loop.rs` (line 35) read it too. `main` calls `decision::init` first today, so the production window is narrow — but it is a *read* that opens it, not a misconfiguration, and the failure direction is fail-open on the guard's hardest layer.
**Why Windows hid it (nothing platform-specific):** in `tests/guard_eval.rs` the `handler` module's tests each call an `Once`-guarded `init_guarded_config()` first, but the separate `mod assess` (line 689) calls `assess()` directly — `fail_closed_when_model_unreachable`, `denylist_still_applies_when_model_unreachable`, `threshold_from_config_is_honored_by_assess_with` install nothing. Different module, same process, so its `config()` read poisons the lock the `handler` tests depend on. Under libtest's default multithreaded run, which test executes first decides whether the denylist is alive for the whole binary: windows-latest drew an installer first, ubuntu-latest drew a reader. That is also why the regression test is written to be run with `--exact`, and why #47's isolation problem is a correctness problem here, not a speed one.
**Symptom on ubuntu-latest — `guard_eval` 10 passed / 4 failed / 2 ignored, every other suite green (same messages reproduced verbatim by reverting the fix, below):**

| test | assertion output |
| --- | --- |
| `denylist_refusal_in_handler_and_confirm_cannot_bypass` | `expected denylist refusal, got: File not found: C:\Windows\System32\kernel32.dll` |
| `shell_write_input_is_screened_by_the_command_denylist` | `expected denylist refusal, got: Session not found: does-not-exist` |
| `denylist_applies_to_tools_outside_the_guarded_set` | `expected denylist refusal, got: Invalid base64: Invalid input length: 1` |
| `handler_fails_closed_when_model_unreachable` | `assertion \`left == right\` failed: expected the call to be held, got: Some("Deleted /tmp/.tmpXLBHRJ/fail-closed.txt")` |

Each message is a *handler* answer where a *guard* answer was expected, i.e. `guard_gate` returned `None` and the destructive call ran for real — the third one is `fs_write` reaching its base64 decode, the fourth actually deleted a file it was supposed to hold.
**Fixed 2026-11-05:** `config()` now reads without installing — `JEV_CONFIG.get().cloned().unwrap_or_default()` — so `init` is authoritative in either order; doc comments on both `init` and `config` state the invariant so the next reader does not "simplify" it back. Regression test `handler::a_config_read_before_init_does_not_disable_the_guard` reads the config *first*, installs, asserts `guardrails` survived, and then asserts a denylisted `fs_delete` is still refused with "denylist" in the message — the interception is proven through the dispatcher, not just the flag.
**Proven to fail, on the platform that reported it:** the fix was reverted with `sed` **inside the container copy only** (checkout untouched, verified by printing `config()` from `/work`) and `cargo test --test guard_eval -- --test-threads=1` on `rust:1-slim-bookworm` gave **5 failed / 10 passed / 2 ignored** — the four messages above, byte-identical, plus the new test's own assert at `guard_eval.rs:512`. Restoring the fix: 15 passed / 0 failed / 2 ignored. Running just the new test with the fix reverted on Windows (`--exact`, so nothing else installs) failed identically, which is how the defect was shown to be order-specific rather than Linux-specific.
**Related:** this is a second, sharper reason for #47. A process-global config that tests reach through real code paths makes *safety* assertions order-dependent, and ordering can just as easily produce a passing run as a silently disarmed guard.
**Status:** ✅ Fixed — verified on Windows (**287 passed / 0 failed / 3 ignored**) and on Linux in Docker (**282 / 0 / 3**; the gap is the `cfg(windows)` tests), plus `cargo fmt --check` 0 and `cargo clippy --all-targets -- -D warnings` 0.
**Severity:** 🔴 Critical — the guard's master switch was flippable by a read, in the direction that executes destructive calls.

---

## 🟡 High-Priority Gaps

### 4. Accessibility — all platforms are stubs — ✅ Fixed

**Fixed:** `src/platform/accessibility/{windows,macos,linux}.rs` all contain real implementations (Windows: COM `IUIAutomation` with `ControlViewWalker`; macOS: `AXUIElement` C FFI via `objc`/`core-foundation`; Linux: AT-SPI2 over D-Bus via `zbus`). All three implement `get_tree`, `find_elements`, `get_element_properties`, and `invoke_element_action`; all four tools (`accessibility_tree`, `accessibility_find`, `accessibility_properties`, `accessibility_action`) are dispatched in `src/mcp/handlers/mod.rs` and declared in `tools.rs`. Element ids stay resolvable between calls: Windows/macOS retain the elements from the most recent tree walk in a per-tree cache (COM refs / `CFRetain`ed `AXUIElementRef`s, cleared on each `get_tree` and on `Drop`), while Linux ids are stable AT-SPI D-Bus object paths that resolve directly.
**Verb coverage is not uniform across the three platforms** (corrected 2026-11-05 while fixing #45 — the
earlier sentence here claimed the full generic set everywhere):

| platform | generic verbs mapped | unmapped name becomes |
| --- | --- | --- |
| Windows | `invoke`/`press`/`click`/`tap`/`do_default_action`, `toggle`, `expand`/`collapse`, `select`, `increment`/`decrement`, `focus`, `close`/`minimize`/`maximize`/`restore` (all via UIA patterns, #38) | error listing the element's available actions |
| macOS | only `invoke`/`press`/`click` → `AXPress`, `increment`/`increase`, `decrement`/`decrease` | passed through verbatim as a raw AX action name, so `toggle`/`focus`/`close` fail with an `AXError` and the available-action list |
| Linux | none — no alias table | matched case-insensitively against the element's AT-SPI action names; fails with the available-action list if absent |

So only Windows answers a generic verb the caller did not read off the tree. macOS and Linux fail loudly
rather than silently, which is the right shape, but a caller that treats the four tools as portable will
still lose the non-Windows cases. Value *writing* (`set_value`) was added by #46 and is spelled the same
way on all three platforms, but it goes through each platform's value API rather than its action list, so
the alias table above still only applies to the non-writing verbs.
Residual gap: the Linux implementation uses blocking `zbus` calls and ignores `window_id` (always walks from
the AT-SPI root).

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
**Root cause:** the mutex was never protecting request state. `handle_request` takes `&mut self`, but every
method it reaches — `handle_initialize`, `handle_tools_list`, `handle_resource_read`, `handle_prompt_get`,
`handle_tools_call`, `handle_web_preview` — only needs `&self`; the sole real writer is `start_web_preview`,
which runs once at startup before any request arrives. So a signature artifact put a process-wide lock across
every `await`, and the loop could not read the next line until the current tool finished. `handle_tools_call`
is `&mut` for no reason at all — it only calls `&self` methods.
**Status:** ✅ Fixed — `handle_request` and `handle_tools_call` take `&self`; `web_server` moved behind a
`std::sync::Mutex<Option<WebServer>>` (written once, read per request; a poisoned lock keeps the value rather
than reporting "web preview is not running" for a server that is still listening), so `GuiMcpServer.handler` is
now a plain `Arc<McpRequestHandler>` with nothing locked around it.
The read loop became `serve_lines(handler, reader, sink)`, generic over both ends so the transport is testable
without owning the process's stdin and stdout: each parsed request goes into a `JoinSet`, and the loop keeps
reading. Responses are still written one whole line at a time behind a sink mutex, and the line is *encoded
before* that mutex is taken, so a slow or failing encode cannot hold up other writes.
Four decisions inside that loop, all measured rather than assumed:
- **Admission control, not an unbounded spawn.** `MAX_IN_FLIGHT = 32`; past that the read loop waits for a slot.
  A flood of requests each holding a screenshot would otherwise trade one stall for an OOM. 32 is above the
  concurrency any single MCP session uses today, so normal traffic never touches the cap.
- **EOF drains.** After stdin closes, the loop awaits every in-flight task before returning. A piped client
  writes its requests and hangs up; without the drain, the slowest response of the batch would vanish when the
  process exits — #40's false absence one layer down.
- **A panic costs one request.** It used to poison the shared handler mutex, so every later request on the
  connection failed for the rest of the session. Now the task set reaps it, logs that the id will never be
  answered, and the connection stays up.
- **A response that will not encode comes back as `-32603`** with the request's own id instead of silence.
**Also fixed here:** the HTTP transport held the *same* `Arc<Mutex<McpRequestHandler>>` across `handle_request`,
so one slow tool serialized every HTTP session, not just the stdio connection it arrived on. `web-preview`
built sessions inherited the stdio bug; it is gone with the lock.
**Consequences callers should know:** responses may now land out of order, correlated by JSON-RPC id (this is
ordinary JSON-RPC; the stdio framing is still one message per line). Concurrency does not interleave desktop
events — `LocalBackend` already serializes `RustAutoGui` behind its own mutex, so input actions queue at the
backend while slow read-only tools (OCR, system probes, waits) run alongside each other, which is the case that
was hurting.
**Verified:** 6 unit tests in `src/mcp::server::tests` — `a_slow_tool_does_not_hold_up_the_next_request` (ping
answered within 300ms while a 600ms `system_wait` is still in flight, and first on the wire),
`eof_drains_the_responses_still_in_flight`, `every_response_carries_the_id_of_the_request_that_produced_it`,
`a_malformed_line_is_answered_and_the_loop_keeps_reading`, `more_requests_than_the_admission_cap_all_complete`
(40 requests, all answered, no deadlock), `one_shared_handler_overlaps_two_requests` (two 300ms waits on one
`Arc` finish in well under 600ms). Mutation-checked: replacing `in_flight.spawn(...)` with an inline `await`
makes the headline test fail ("ping was still unanswered after 300ms"), so the assertion is load-bearing and not
a timing artifact. `system_wait` is the slow request because it is the only handler that awaits on purpose and
needs no backend at all — no shell-out, no model (`docs/ISSUES.md` #47).
**Severity:** 🟡 High — a single slow call degrades the whole server's availability.

### 34. Full-screen OCR has lower small-text recall than a region read of the same pixels

**Files:** `src/gui/ocr.rs` (`read_screen`), `src/mcp/handlers/ocr.rs` (`handle_find_text`)
**Problem (observed 2026-11-04):** BUSY's 11-14px menu-pane text was read reliably by a 420x140 region (8/11 lines
recognised) while a full-screen `screen_read` of the same pixels alternated between reading it and returning 0 lines
for it. Consequence: `screen_find_text` (full-screen only) can report "No text matching 'Trial Balance' found on screen"
for text that `screen_read` on the region finds at the same instant. Recognition accuracy also drifts with scale
("Day Book" read as "Dav Book", "OK (F2)" as "OKE)l").
**Root cause (measured 2026-11-05, against `ocrs` 0.13.1 and the shipped model, not assumed):** the working size is
fixed **by the model**, not derived from the image — `TextDetector::detect_text_pixels` reads its input dimensions from
the tensor, and the shipped `text-detection.onnx` declares `[1, 1, 800, 600]`, i.e. 600 wide × 800 tall. An image
*smaller* than that is padded with `BLACK_VALUE`, so glyph pixel size is preserved; an image *larger* is passed through
`resize_image([in_height, in_width])`, which **ignores the aspect ratio**. A 1920×1080 frame is therefore squeezed to
**0.31× horizontally** (0.74× vertically), turning an 11px glyph into a ~3px-wide feature the segmentation model never
fires on, while a 420×140 crop of the same pixels is padded and read at native scale. `OcrEngineParams` exposes no scale
or max-side knob (only the two models, `debug`, `decode_method`, `alphabet`, `allowed_chars`) and
`TextDetectorParams::default()`'s `min_area: 100.` / `text_threshold: 0.2` are applied in *original* pixels, so the only
lever available is the image handed to the detector. The original hypothesis in this entry — that `ocrs` derives the
working size from the input — was wrong; the loss is not a quality drift with scale, it is a cliff at the tensor size.
**Measured on one live 1920×1080 desktop capture**, over 12 lines that a tight crop of the same pixels reads correctly:
a whole-screen pass reads **5**, a 1200-wide window **7**, and windows no larger than 600×800 **12/12**. Run through the
shipped seam on the same capture, `read_screen` returned **71** lines and `read_screen_thorough` **179**. Overlap is
load-bearing: counting distinct line texts across the sweep, the same 600×800 grid with zero overlap returned **108**
lines where 96px of overlap returned **132**, because a line straddling a window edge is read as a fragment or lost.
**Status:** ✅ Fixed — the search strategy the entry asked for, with the tile geometry taken from the measurement rather
than guessed. In `src/gui/ocr.rs`: `DETECT_WORKING_SIZE` (600×800, the tensor) and `SEARCH_WINDOW_OVERLAP` (96px) as
documented constants, `axis_positions` + `search_windows` for the grid (a frame that already fits the working size stays
**one** window — it is padded, not rescaled, so a region read costs nothing extra), and `merge_window_results` to bring
each window's boxes back into screen space. The duplicate test is *same text **and** centres within the overlap*, because
collapsing on text alone would delete a ledger column of "1"s and invent, in the merge, the very absence this issue is
about removing. `OcrCapability` gained **provided** methods — `read_screen_thorough` and `search_text`, which returns
`TextSearch { matches, scope }` — so the strategy is geometry over the backend's own primitives and a stub exercises the
same sweep the desktop does (`FindStrategy` / `SearchScope` / `match_lines` are all pure, all unit-tested, no model).
Wired into the tools: `screen_read` takes `strategy: "single" | "thorough"` (**default `single`**, the previous
behaviour, kept because the sweep costs one inference pass per window — 8 on 1920×1080 — and `screen_read` is the tool an
agent calls most); `screen_find_text` and `screen_scroll_to_text` take `strategy: "auto" | "single"` with **`auto` as the
default**, escalating only on a miss, because a reported absence is the answer that must not be a lie and the caller
cannot see the difference in an empty list. Every response now names its own coverage: a default whole-screen read says
"one downscaled pass … strategy 'thorough' re-reads it as 8 native-scale 600x800 windows", a miss says "Search covered
8 native-scale windows", and `strategy: "single"` has to admit it looked once. Unknown values are refused by name rather
than run as the cheaper leg. Two vocabularies on one argument name are deliberate — `screen_read` chooses how much of the
frame to read, the search tools choose when to escalate — and each tool rejects the other's words. **Found in the same
code and fixed here:** `screen_scroll_to_text` checked *then* scrolled, so the frame it settled on was never read before
"not found after N scrolls" was returned; the final screen is now captured and searched (at native scale unless the
caller opted out) before absence is claimed. Coverage stays honest in both directions — a hit on the first pass reports
no sweep, and a frame that is already native scale does not pay for a second identical pass.
**Severity:** 🟡 High — an agent can conclude UI is absent when it is only missed.

### 37. Windows window enumeration never resolves `process_name`

**File:** `src/platform/window/windows.rs` (`get_window_info`)
**Problem (verified 2026-11-05):** `get_window_info` hardcodes `process_name: None`, and it is the single constructor used by `list_windows`, `get_active_window` and `find_windows_by_title`, so every `WindowInfo` produced on Windows has a null process. Linux resolves `WM_CLASS` (`src/platform/window/linux.rs`, `get_process_name`) and macOS reads `kCGWindowOwnerName` (`src/platform/window/macos.rs`) — Windows, the platform the product is actually used on, is the only one missing it. Two consequences: `ui_decide` sends `"window": { "process": null }` into the JEV prompt (`src/decision/ui_loop.rs`), so the model decides with no idea which application owns the target; and a caller cannot prove a window belongs to the app it believes it does, or disambiguate same-titled windows, which is the failure class behind wrong-window clicks in desktop automation.
**Status:** ✅ Fixed — `get_window_info` resolves the image name from the `GetWindowThreadProcessId` pid through `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` and reports the **file stem**, so the value matches the bare names Linux/macOS produce and the `ProcessName` that `process_list` already returns; it stays `None` only when the handle genuinely cannot be opened (elevated / protected processes) or the query fails.
**Severity:** 🟡 High — silently degrades a documented field and a model input on the primary platform.

### 38. Windows accessibility exposes no value/description/state and only Invoke + Window patterns

**File:** `src/platform/accessibility/windows.rs`
**Problem (verified 2026-11-05):** `build_node` sets `value: None, description: None` unconditionally; `get_element_states` reports only `disabled` and `focused`; and `get_element_actions` advertises only `invoke` plus the four window verbs. `invoke_element_action` matches `invoke | focus | close | minimize | maximize | restore` and every other string errors, so the generic verbs `press`, `click`, `increment`, `decrement` — which the macOS backend aliases (`macos.rs`, `invoke_element_action`) and which `docs/ISSUES.md` #4 claims are mapped "on all platforms" — are rejected on Windows. No `TogglePattern`, `ValuePattern`, `ExpandCollapsePattern`, `SelectionItemPattern` or `RangeValuePattern` is imported at all, so none of their read state or actions are reachable. Net effect on the platform that matters: an agent cannot read what is in a text field, tick a checkbox, open a combo box, select a list item, or tell whether a node is expanded/offscreen through UIA, and has to fall back to OCR for all of it.
**Status:** ✅ Fixed — `build_node` now fills `value` (ValuePattern → RangeValuePattern → LegacyIAccessible, capped at 4096 chars because a document view reports its whole text as its value) and `description` (`CurrentHelpText` → LegacyIAccessible description; UIA has no `CurrentFullDescription` on the client element). States add `offscreen` / `password` / `required` / `readonly` / `checked` / `indeterminate` / `expanded` / `partially_expanded` / `collapsed` / `selected` / `focus`, each read from the live pattern rather than assumed. Actions are derived from the patterns the element actually implements — Invoke, Toggle, ExpandCollapse, SelectionItem, RangeValue, ScrollItem, Window — and `invoke_element_action` was rewritten to canonicalize verbs case-insensitively with `-` read as `_`, accept `press` / `click` / `tap` / `do_default_action` as "activate" (Invoke → Toggle → legacy default action, so a caller stating intent works against a checkbox and against a VCL button alike), and implement `toggle` / `expand` / `collapse` / `select` / `increment` / `decrement` (RangeValue stepped by `CurrentSmallChange` and clamped to `CurrentMinimum`…`CurrentMaximum` — UIA publishes no Increment method) / `scroll_into_view` / `focus`. Unsupported verbs fail with the element's live action list echoed in the error, so the caller can self-correct
from the response alone. **Not** fixed at that time: writing a value into a field, which the trait could not
express — #46 added the value parameter and the `set_value` verb that needed it.
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
**Status:** ✅ Fixed — `LogViewer::get_system_logs` now returns `Result<Vec<LogEntry>, String>` and takes a `LogLevel` plus an optional `log` name.
- **All levels by default.** A `LogLevel` enum (`Critical`/`Error`/`Warning`/`Information`/`Verbose`, parsed case-insensitively and accepting the platforms' own abbreviations `err`/`warn`/`info`/`debug`) replaces the string that used to fall through to `Level=4`. With no `level`, the `Level` key is omitted from the `-FilterHashtable` entirely, so a bare call sweeps every severity. An unrecognized name is now an argument error instead of a silent downgrade to Information — a test pins both halves.
- **`log` parameter.** Windows callers can read `Application`, `Security`, `Setup`, `Microsoft-Windows-…/…`, anything. The value is interpolated into a PowerShell hashtable literal, so it passes an explicit character allowlist first (`'`, backtick, backslash, `;`, `(`, `)` and length > 128 are refused); `tests/handler_tests.rs` asserts an injection-shaped name is rejected. On Linux/macOS, supplying `log` returns an error naming the platform's own mechanism rather than being ignored, because an ignored selector reads as if the caller's choice had been honored.
- **Failure is failure.** Spawn failure, non-zero exit and unparseable output now return `Err`. `Get-WinEvent`'s "nothing matched" case (error record + non-zero exit + empty stdout) is recognized as a genuine empty answer, and an empty answer says so in words ("No entries for level Error in the 'Application' log") so it is not a bare `[]`. `count` is validated to 1–1000 — above the cap the call is refused rather than silently returning fewer entries than asked for.
- **Also fixed while in here:** `TimeCreated` was serialized by `ConvertTo-Json` as PowerShell's `\/Date(…)\/` form, which is not a usable timestamp; it now goes through a calculated property emitting ISO 8601 with offset. journald read via `-o json` instead of the text format, because the text line carries no severity and the old code labelled **every** Linux entry `"Info"` — the requested filter's name, not the record's level; `PRIORITY` is now mapped back to a level name. macOS `--style compact` genuinely has no per-entry level, so that field is empty rather than invented, and journald/`log show` levels are documented as thresholds (that severity and above) while Windows matches exactly. Entries are returned newest-first on all three platforms; messages over 2000 characters are cut with a visible `…[message truncated at 2000 chars]` marker.
- **Verified on the real transport:** `tests/handler_tests.rs` runs the actual `system_logs` query through the dispatcher on Windows and asserts the returned entries carry an ISO 8601 timestamp, the requested level, and a populated provider — proof that `$_` survives `Command`-line marshalling into the calculated property, which a pure string test cannot show.
**Remaining (recorded, not fixed):** the Linux and macOS branches compile-checked only for the Windows target on this machine (no other `rustup` target is installed), so their runtime behavior waits on the CI platform matrix in #44. The macOS `log show` level vocabulary (debug/info/error/fault, no `warning`) is from documented behavior, not verified on a Mac. The timestamp format still differs per platform (ISO 8601 on Windows, decimal epoch-seconds on Linux, whatever `log show` printed on macOS) — normalizing it is a format decision, not a bug fix. macOS is pinned to `--last 1h`; a `since` parameter is not implemented. macOS per-entry level needs `--style json`.
**Severity:** 🟡 High — a diagnostic tool that hides the failures being diagnosed.

### 43. 20 of 72 declared tools have no test reference; the four accessibility tools are untestable at handler level

**Files:** `tests/`, `src/gui/backend/stub.rs`
**Problem (verified 2026-11-05):** the declared/dispatched sets match exactly (0 orphan tools — `tests/tool_schema_tests.rs` is a real drift guard and works), but 20 advertised tools appear in no test file: `accessibility_tree`, `accessibility_find`, `accessibility_properties`, `accessibility_action`, `screen_scroll_to_text`, `decision_rank`, `decision_stats`, `detect_click_object`, `disk_usage`, `system_stats`, `system_logs`, and all eight `mcp_*` hub tools. The four accessibility tools are structurally untestable today: `StubBackend` implements no `AccessibilityCapability`, so `GuiClient::accessibility()` returns `UnsupportedCapability` and no handler path beyond the error can be exercised. `tests/handler_tests.rs` covers display/mouse/keyboard/clipboard/window only.
**Status:** 🟡 Partially fixed (2026-11-05) — the accessibility half and the cheap argument/result half are closed; 7 tools remain, all of them `mcp_*` hub tools.
- New `StubA11yBackend` in `src/gui/backend/stub.rs`: the `StubBackend` display/input behavior plus a fixed nine-element tree, and the id-cache contract the platform backends implement — every walk re-mints `stub:<n>` ids depth-first and invalidates the previous walk's ids, so `accessibility_properties` / `accessibility_action` behave the same way here as on Windows (issue #39). `max_depth` is honored (trimmed children are not issued ids), name and role are filtered as separate substring matches, and an unresolvable id is refused with the same wording.
- `StubA11yRecorder` is handed back by `StubA11yBackend::new()` because `GuiClient` takes ownership of the backend. It exists because a tool response can echo an argument it never used: the tests assert the `(element_id, action, value)` triples the backend actually received, and that a refused action records nothing. (#46 later widened that recorder to carry the value; see its status below.)
- `StubBackend::as_accessibility()` stays `None` on purpose, so the `UnsupportedCapability` path keeps a test. `handler_tests.rs` gained 9 accessibility tests; `tool_schema_tests.rs` gained 2 (schema `required` vs what the handler enforces, and the id-contract wording in the descriptions).
- **Proven to fail, not just to pass:** with the cache replacement in `walk()` mutated to keep the first walk's tree and ids, `test_accessibility_ids_are_invalidated_by_the_next_walk` is the only test that fails — the invalidation assertion is load-bearing rather than decorative. The mutation was reverted and the suite re-run.
- Measured coverage after this change (script over `tools.rs` names vs the test-bearing corpus): **13 of 72** declared tools still appear in no test — `decision_rank`, `decision_stats`, `detect_click_object`, `disk_usage`, `mcp_discover`, `mcp_exec`, `mcp_start`, `mcp_stop`, `mcp_tool_groups`, `mcp_tools`, `mcp_unregister`, `screen_scroll_to_text`, `system_stats`. (`system_logs` and `mcp_register`/`mcp_list` left the list via #41 and #40.)
- Found and fixed while verifying: `cargo check --no-default-features --all-targets` could not complete at all — `tests/download_tests.rs` referenced `sys_mcp::download`, a module gated on `ocr`/`detection`, and `test_screen_find_text_missing_query_errors` asserted an OCR-feature message in a slim build. Both are gated now, and `cargo test --no-default-features` runs green (recorded under #44, which is what makes that leg permanent).
**Remaining:** the 13 tools above, of which the eight `mcp_*` hub tools need a fake child server to test the lifecycle and `system_stats`/`disk_usage`/`decision_*`/`detect_click_object`/`screen_scroll_to_text` need only argument and result-shape tests. Verb-alias normalization (`press`/`click`/`tap` → activate) still lives inside the Windows backend and has no test — the stub passes the verb through unchanged, which is what its tests assert.
- Second batch 2026-11-05 (the cheap tier of that list): 10 tests added to `tests/handler_tests.rs` covering `system_stats`, `disk_usage`, `decision_stats` (read + `reset`), `decision_rank` (missing `text`/`question`, `top_k: 0`, and the small-input return-whole branch — which needs no gateway call because `FLOOR_CHARS` is 8 KiB), `detect_click_object` (missing label, stub-box click at (125, 215), no-match, out-of-range index), and `screen_scroll_to_text` (missing query, immediate stub match). Coverage is now **7 of 72** uncovered: the `mcp_*` tools that need a fake child server (`mcp_discover`, `mcp_exec`, `mcp_start`, `mcp_stop`, `mcp_tool_groups`, `mcp_tools`, `mcp_unregister`). `screen_scroll_to_text` left the list, but only its *found* path and argument refusal are covered — the not-found branch is unreachable against `StubOcr`, which answers every `find_text` query with one match, so exercising it needs a stub fixture that can return no matches.
  Two things that writing those tests proved, both now encoded as `#[cfg]` gates rather than assumptions:
  - **A feature-off fallback handler never validates its arguments.** `handle_click_object` and `handle_scroll_to_text` are replaced wholesale under `#[cfg(not(feature = …))]`, so `{}` returns "rebuild with `--features detection`" / "build with 'ocr' feature", not `missing 'label' argument`. An argument-refusal test is therefore only true under its own feature, and the `screen_scroll_to_text` missing-query test failed on the slim leg for exactly this reason before it was gated.
  - **`decision_rank`'s block count is the splitter's business, not the handler's contract.** `split_blocks` hard-splits an over-long line at `max_chars`, so a 1-char cap yields one block per *character* (16 for a 16-byte input). The assertions bind to the promised invariants instead: `kept == total_blocks`, `dropped == 0`, `batches == 0`, and `content` equal to the input verbatim.
- Re-measured after this batch: default `cargo test --all-targets` on Windows = **298 passed / 0 failed / 4 ignored** (302 total); `--no-default-features` = 230 passed / 1 ignored; `--features detection` = 301 passed / 4 ignored (handler_tests 135 in 0.06 s, still no model load). `fmt --check` and `clippy --all-targets -- -D warnings` clean.
**Severity:** 🟡 High — the newest, most intricate surface (element-id caching) shipped with no coverage at all.

### 44. CI never runs for branch pushes and tests no feature combination; the release workflow has never run

**Files:** `.github/workflows/ci.yml`, `.github/workflows/release.yml`
**Problem (verified 2026-11-05):**
- `on: push: branches: [main, master]` — a push to a feature branch triggers **no** workflow, so nothing on `feat/ocr-onnx-a11y-tools` has remote CI signal; it only starts at PR time. Every claim that the branch is "CI-clean" is a local claim.
- No feature matrix: CI never builds `--features detection` or `--no-default-features`, so `src/gui/detection.rs` and every `#[cfg(not(feature = …))]` arm are uncompiled by CI. `tests/tool_schema_tests.rs`'s feature-gate test therefore only ever asserts the default combination, and the feature-off code paths rot silently.
- The lint gate is `cargo clippy -- -D warnings`, without `--all-targets`, so lints in `tests/` and `examples/` are ungated (the local run that used `--all-targets` was stricter than CI).
- No dependency-vulnerability gate of any kind: no `deny.toml`, no cargo-deny / cargo-audit / OSV step in either workflow.
- `release.yml` has never executed — `git tag` is empty and the version is still `0.1.0` with all shipped work under `## [Unreleased]`. It also has no test or smoke step, publishes no `SHA256SUMS` and no signature, and pins `macos-13`.
- **Measured 2026-11-05 while fixing #43:** the missing feature leg was not merely untested, it was uncompilable — `cargo check --no-default-features --all-targets` failed outright (`tests/download_tests.rs` names `sys_mcp::download`, which only exists under `ocr`/`detection`), and one handler test asserted an OCR-only message. Both are gated as of #43, so `cargo test --no-default-features` now runs green locally and a slim CI leg is actually addable.

**Fixed 2026-11-05 (`ci.yml`, `release.yml`, new `deny.toml`):**
- Triggers: `push: branches: ['**']` (was `[main, master]`) + `pull_request` + `workflow_dispatch`, with `concurrency: ci-…-${{ github.ref }}` and `cancel-in-progress: true` so a superseding push stops the older run.
- The lint gate is now `cargo clippy --all-targets -- -D warnings`, i.e. as strict as the local run.
- New `features` job (ubuntu, two legs). `--no-default-features` compiles **and** runs the suite — the combination nobody had ever compiled, which turned out to be uncompilable (bullet above). `--features detection` compiles and links with `--no-run` instead of executing, because running the suite provisions the real YOLO model to assert nothing — see #47.
- New report-only `supply-chain` job (`EmbarkStudios/cargo-deny-action@v2`, `continue-on-error: true`) with a `deny.toml` seed.
- `release.yml`: `workflow_dispatch` added so the pipeline can be exercised without a tag, with the `release` job gated on `if: github.event_name == 'push'` so a manual run never publishes; a `--version` smoke step per leg; a flatten step that fails if asset names collide across legs; `SHA256SUMS` generated over the flat asset set. The x86_64 macOS leg originally stayed on `macos-13` under the assumption that it was load-bearing (last Intel macOS image); GitHub has since retired that runner label, so the leg now cross-compiles `x86_64-apple-darwin` from `macos-latest` (universal SDK links both arch slices; the smoke test runs under pre-installed Rosetta 2). A job targeting the retired label would queue indefinitely for a runner that never picks it up.

**Verified locally:** both workflow files pass `action-validator` against the GitHub Actions schema (exit 0 each). `deny.toml` parses as TOML and every key was checked against cargo-deny's current configuration docs — the keys newer cargo-deny rejects (`advisories.vulnerability/notice/severity-threshold`, `licenses.unlicensed/copyleft/deny/default/unknown`) are absent, and the two sections whose defaults could not be verified are omitted rather than guessed. `cargo check --all-targets --features detection` and `cargo test --all-targets --features detection` both exit 0 on Windows.

**Full local dry-run of the gate set (2026-11-05, after #45).** This is what the `ci.yml` gates will do, run by hand on Windows — the platform CI does *not* cover, so it is complementary evidence rather than a substitute for the remote run:

> Count note: the `cargo test --all-targets` figure in that table (286) is the dry-run's own measurement.
> After #48's regression test the suite is **287 passed / 0 failed / 3 ignored** on Windows and
> **282 / 0 / 3** on Linux (`rust:1-slim-bookworm` with `systemd`/`net-tools` installed); the 5-test gap is
> the `#[cfg(windows)]` tests.

| gate | result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` (default) | exit 0, **0** diagnostics, genuine rebuild |
| `cargo clippy --no-default-features --all-targets -- -D warnings` | exit 0, **0** diagnostics |
| `cargo clippy --features detection --all-targets -- -D warnings` | exit 0, **0** diagnostics |
| `cargo test --all-targets` | **286 passed, 0 failed, 3 ignored** across 14 suites |
| `cargo test --no-default-features --all-targets` (the leg CI executes) | **224 passed, 0 failed** |
| `cargo build --release` | exit 0 in 7m31s, 0 warnings, 22.6 MB binary |
| release-binary smoke, the step `release.yml` adds | `sys-mcp --version` → `sys-mcp 0.1.0`, exit 0 |
| `cargo check --no-default-features --features opencl --all-targets` (added for the README table) | exit 0, `ocl v0.19.7` compiles |

One real failure surfaced on the way: `cargo build --release` **aborted** with
`failed to remove file target\release\sys-mcp.exe — Access is denied (os error 5)`, because an
IDE-attached `sys-mcp.exe` (PID 18672) was running from exactly that path and Windows will not replace a
loaded image. Worked around with `--target-dir` rather than killing the live server; the gotcha and the
workaround are now documented in `CONTRIBUTING.md`.

**Owed / remaining (as written before the push; see the remote-run record below for what changed):**
`cargo-deny` itself has never run here (not installed on this machine), so the `deny.toml` schema is
doc-verified only — and it predicted correctly that the report would be red until triaged. The feature legs
run on Linux while all of the above ran on Windows, which is exactly the gap that produced the two failures
below. The `ci.yml` test matrix uses `macos-latest`, so that leg ran on a current image and reached
`cargo check`. The `release.yml` `macos-13` x86_64 leg has now been switched to cross-compile from
`macos-latest` (GitHub retired the Intel macOS image), so no leg targets an unavailable runner. `release.yml` has
still never published (`git tag` is empty, version `0.1.0`, all work under `## [Unreleased]`). Binary signing
is deliberately **not** added: `SHA256SUMS` lets a downloader detect corruption, not establish provenance, and
cosign/minisign or Windows code signing each need a key decision from the owner.

#### First remote run — 2026-11-05, and it earned its keep

The branch was pushed (`343ff06..be0eea4`), the new `push: branches: ['**']` trigger fired, and run
**41943845280** (`CI`, event `push`, sha `be0eea4`) concluded **failure** in 3 of 6 jobs:

| job | conclusion | failing step |
| --- | --- | --- |
| `Test (windows-latest)` | ✅ success | — |
| `Features (no-default-features)` | ✅ success | — |
| `Features (detection)` | ✅ success | — |
| `Test (macos-latest)` | ❌ failure | `Check` |
| `Test (ubuntu-latest)` | ❌ failure | `Test` |
| `Dependency audit (report only)` | ❌ failure | `cargo-deny` |

Both test-leg failures were **real defects on this branch that Windows-only verification could not see** —
which is precisely the return the trigger change was bought for. The two feature legs added here passed on
their first remote execution, including the `--no-default-features` suite that nobody had ever compiled
before #43 found it uncompilable.

- **macOS `Check`:** `error[E0716]: temporary value dropped while borrowed --> src/system.rs:1087`, in the
  `log show` parser added by `a858350` (#41) — the `Cow` from `String::from_utf8_lossy` was borrowed inline
  into a `Vec<&str>` that outlived the statement. Reproduced without a Mac via
  `rustup target add aarch64-apple-darwin` + `cargo check --no-default-features --all-targets --target
  aarch64-apple-darwin` (exit 101 → exit 0 after the fix, with a visible `Checking sys-mcp` line so the
  result was a rebuild, not a cache hit). Fixed in `147ad79`. The slim cross-check covers *all* macOS-only
  code: there are zero `cfg(all(<platform>, feature = …))` sites and zero platform cfgs inside the ocr /
  detection / decision modules, so no feature-gated path is macOS-specific. `--features ocr` could not be
  cross-checked because `ring v0.17.14`'s build script needs a C compiler for the Apple target — a toolchain
  limit on this machine, not a code finding.
- **ubuntu `Test`:** `cargo test --all-targets` → `guard_eval` 10 passed / **4 failed** / 2 ignored, every
  other suite green. Root cause is **issue #48**: a process-wide config read was silently turning the whole
  guard layer off, and only test *ordering* decided it, not the OS. Worth recording how long the honest
  version of this took: the first three Linux containers **passed**, because `rust:1-slim-bookworm` has no
  `journalctl`/`systemctl`/`netstat`. Installing `systemd` + `net-tools` made the real-OS handlers take the
  same paths ubuntu-latest takes — the runner's *binaries* changing the suite outcome is #47's argument
  demonstrated by the platform, not by a stopwatch.
- **`Dependency audit (report only)`:** the job carries job-level `continue-on-error: true` (`ci.yml:119`)
  yet the API reports its conclusion as `failure`. This run cannot attribute that: it failed for the two
  reasons above regardless. The next push carries neither, so the question answers itself empirically
  instead of by reading Actions documentation twice.

**Method note (why the local reproduction was trustworthy):** CI job *logs* need auth (`/actions/jobs/{id}/logs`
→ 403 without a token; `gh` was not logged in), while `/actions/runs`, `…/jobs` and `…/annotations` are public.
Annotations give exit codes only (`101`), so every detail above came from reproducing the gate locally on the
same platform image, not from reading CI output.
**Status:** 🟡 Partially fixed — gates wired and now proven remotely; two defects found by the first run are fixed.
**Severity:** 🟡 High — what remains is release-side (a tag, a `workflow_dispatch` run, the signing decision) plus the `deny.toml` triage, not gate-side.

#### Second remote run — plumbing bug in the supply-chain job

The `Dependency audit (report only)` job kept rendering red after `41943845280`, but for a reason unrelated to
the tree: its `actions/checkout@v4` step omitted `submodules: true`, so `cargo-deny` could not resolve the
`rustautogui` path dep and `cargo metadata` exited before any check ran. Every other job (test matrix,
features matrix, real-os probes, release) already fetches submodules; only this step missed the flag.
Fixed in `0228f8f`. Once the checkout works, cargo-deny runs and reports the real supply-chain findings for
the first time — the triage below.

**Fixed 2026-11-05 (#44, second batch: supply-chain triage):** all nine RUSTSEC advisories from run
`42008071452` are closed by patch-level bumps in `Cargo.lock` — `anyhow 1.0.102 → 1.0.103`,
`crossbeam-epoch 0.9.18 → 0.9.20`, `rustls 0.23.45 → 0.23.46`, `rustls-webpki 0.103.15 → 0.103.16`,
`webpki-roots 1.0.7 → 1.0.10`, `tokio 1.52.3 → 1.53.3` (this one crossed the advisory's
`>=1.51.6, <1.52.0 OR >=1.53.3` window, not a same-line patch). No source changes — the affected range
was on the transitive side of every entry, and each crate's own Cargo.toml constraint (`1.0`, `0.23`,
`1`) admitted the fixed version already.

The four rejected licenses are allowed in `deny.toml` with the reason recorded next to each entry:
BSL-1.0 (`clipboard-win` / `error-code` via `arboard`), NCSA (`libfuzzer-sys` via `rav1e → ravif → image`),
and CDLA-Permissive-2.0 (`webpki-roots` data license for its Mozilla-derived root bundle). All three are
OSI-approved; the third is a data license and there is no source-license alternative for that crate.

The single wildcard is the workspace's own path dep: `rustautogui = { path = "rustautogui", ... }`
with no `version`. Rather than a `[bans] skip` entry that would paper over future regressions, the path
dep now carries `version = "2.5.0"` — the same string as the submodule's `Cargo.toml`, so a submodule
bump fails to build instead of silently re-opening the wildcard.

**Verified locally (Windows; cargo-deny not installed here, CI is the actual judge):** `cargo fmt --check`
exit 0; `cargo clippy --all-targets -- -D warnings` and `cargo clippy --all-features --all-targets --
-D warnings` exit 0; `cargo test --all-targets` **341 / 0 / 4**, `--no-default-features` **245 / 0 / 1**,
`--features detection` **344 / 0 / 4**. The counts match `a68d041` exactly, so no test disappeared under
the bumps. `continue-on-error: true` on the `supply-chain` job stays until CI shows one green leg; that
flip is the last remaining gate-side item on this issue.

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
**Problem (verified 2026-11-05):** region mode is chosen by `args.get("x").is_some()`. A caller that sends `width`/`height` without `x` gets a **full-screen** read with those arguments silently ignored — and then a `Screen 1920x1080` header for a request that looked scoped. Conversely `x` without the rest errors as "missing required argument", which is at least loud. `detail` is matched as `"text" | "lines" | _`, so any typo (`"detailled"`, `"Lines"`, `"txt"`) falls through to the default arm — the word-level serialization, the largest and slowest payload — instead of an error.
**Status:** ✅ Fixed (2026-11-05)
- `parse_region()` selects region mode on **any** of `x`/`y`/`width`/`height` being present, and requires all four. A partial region is refused naming exactly the missing keys and naming the way to get the whole-screen read back ("Omit all four to OCR the whole screen"). Absent-or-null for all four is whole-screen mode, so `{"x": null}` is not treated as a region request that then fails on `x`.
- Coordinates must be non-negative integers (`"10"`, `-1`, `1.5`, `true` are all refused per key), and `width`/`height` must be greater than zero. A zero-sized crop is a rectangle that contains no pixels, so it cannot answer a question about the screen; it is refused as an argument error instead of going through the capture. (Not measured on the old build — the old code reached `screenshot_region` with whatever numbers arrived.)
- `parse_detail()` returns a `Detail` enum with arms for `text`/`lines`/`full` only, trimmed and case-insensitive. An unknown string or a non-string `detail` is refused listing the three values instead of paying for word-level coordinates the caller had asked not to receive. `Detail::Full` still matches the declared `"default": "full"`.
- Both parsers run **before** `client.screenshot()`, so a malformed call costs nothing instead of an 8-second OCR pass. Verified, not asserted: the two handler-level tests finish in 0.00s while a real full-screen `screen_read` on this machine measures 6-10s (issue #31).
- Schema descriptions in `src/mcp/tools.rs` now state the all-four-or-none rule, the positive size requirement, and that an unlisted `detail` is rejected. (`detail` was already declared as `enum ["full", "lines", "text"]` with `default: "full"`; the earlier draft of this issue wrongly implied `"full"` was missing from the enum.)
- Tests: 6 unit tests in `src/mcp/handlers/ocr.rs` (`#[cfg(all(test, feature = "ocr"))]`) covering default, case-insensitivity, prefix/typo rejection, wrong type, all-four parse, present-but-null, each partial-region combination, each bad coordinate, zero width and zero height. 2 handler-level tests in `tests/handler_tests.rs` (`#[cfg(feature = "ocr")]`) going through `handle_tool_call` to prove the messages reach the client as `is_error`. Full suite green (46 lib, 111 handler), `cargo clippy --all-targets` and `cargo check --no-default-features` clean.
**Remaining (recorded, not fixed):** a whole-screen read still succeeds when the caller meant a region but sent none — there is no way to distinguish "forgot the coordinates" from "wants everything". `screen_find_text` and `scroll_to_text` still take no region at all (issue #34).
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

**Files:** `CHANGELOG.md`, `README.md`, `Cargo.toml`, `SECURITY.md`, `CONTRIBUTING.md`, `docs/ISSUES.md`, `src/gui/ocr.rs`, `src/gui/types.rs`, `src/mcp/handlers/*.rs`
**Problem (verified 2026-11-05):** five statements the code does not support.
1. `CHANGELOG.md` — "model failure fails open, the denylist never does", and the same sentence in the `guard_gate` docstring in `src/mcp/handlers/mod.rs`. The default is `GuardFailMode::Closed`, which *holds* the call (see #36 for the one path that genuinely fails open).
2. `docs/ISSUES.md` #4 claims `invoke_element_action` maps `invoke/press/click/focus/increment/decrement/…` on all platforms. macOS aliases them; Windows rejects all but invoke/focus/window verbs (see #38).
3. `src/gui/ocr.rs` — four doc comments on `OcrWord.cx/cy` and `OcrMatch.cx/cy` still tell callers to "use this directly for `gui_click`", the pre-rename name. #21 was closed as "all 24 stale references corrected" but only covered `tools.rs` and the handlers.
4. `src/mcp/handlers/accessibility.rs` ends with a "Template matching handlers" banner with nothing under it, and both that file and `src/mcp/handlers/ocr.rs` open with `#![allow(dead_code)]`, which now suppresses real dead-code findings in two live modules from the `-D warnings` gate.
5. `README.md` self-admits its per-feature crate-count table predates `jev` joining the default set.

**Corrections to this issue's own text, found while fixing it.** Items 1-4 each under-described the defect,
and one of the counts was wrong:
- Item 1's second half was already fixed: the `guard_gate` docstring was rewritten under #36, so only
  `CHANGELOG.md` still carried the fail-open sentence. `--jev-guard-fail-mode` was also missing from the
  CHANGELOG's own "New config" list, one line below the sentence being corrected.
- Item 2 was written before #38 landed and is now wrong in the opposite direction. The real split: Windows
  maps the full generic set, macOS aliases only `invoke`/`press`/`click`, `increment`, `decrement`, and
  Linux has **no** alias table at all. A per-platform table replaced the sentence in #4.
- Item 3 is six references across two files, not four: `ocr.rs` (`OcrWord.cx/cy`, `OcrMatch.cx/cy`) and
  `types.rs` (`AccessibilityNode.cx/cy`), which the issue did not list.
- Item 4: the blanket `#![allow(dead_code)]` sits at line 1 of **all ten** `src/mcp/handlers/*.rs` modules,
  not two.

**Fixed:**
- `CHANGELOG.md` guardrails bullet now states the denylist is deterministic and never bypassed, that an
  unreachable model is decided by `--jev-guard-fail-mode`, and that the default `closed` means a guard
  failure is never a silent allow; the flag and its env var joined the config list.
- `docs/ISSUES.md` #4 carries the per-platform verb table and pointed at #46 for the value-writing gap, which
  #46 has since closed.
- All six `gui_click` doc comments renamed to `mouse_click`; the two on `AccessibilityNode.cx/cy` now point
  at `accessibility_action` with the node's `id` when `actions` is non-empty, which is the more reliable path.
- 14 further pre-rename names in the still-current `[Unreleased]` block of `CHANGELOG.md`
  (`gui_read_file`…`gui_create_dir` → `fs_*`, `gui_shell_*` → `shell_*`, `gui_detect_objects` →
  `detect_objects`, `gui_click_object` → `detect_click_object`) and 3 more on 2 lines of `SECURITY.md`
  (`gui_web_preview` → `web_preview`, `gui_shell_exec`/`gui_shell_open` → `shell_exec`/`shell_open`), each
  checked against the dispatch arms in `src/mcp/handlers/mod.rs`. The `[0.1.0]` section keeps its `gui_*`
  names on purpose — that is what that release shipped.
- The empty "Template matching handlers" banner is gone, replaced by a plain statement of what lives where.
- `#![allow(dead_code)]` removed from all ten handler modules; README "Build profiles" and the `Cargo.toml`
  `detection` comment rewritten with re-measured numbers. The table claimed to cover "every optional
  capability" but omitted two of the seven features: `opencl` (+11, it forwards to the vendored
  `rustautogui`, where `opencl = ["ocl", "full"]`) and `ml` — which is `ml = []` with **zero**
  `cfg(feature = "ml")` sites anywhere in `src/`, and measures +0 crates. Both are now in the table, with
  `ml` labelled inert rather than quietly dropped from the list.

- Two test-count claims in `docs/ISSUES.md` itself disagreed with each other ("171 tests" in the status
  table rows for #6 and #24, "248 tests" in the Phase 11 audit note). Measured: **286 passed, 0 failed,
  3 ignored** across 14 suites. Both figures were stale; the rows now carry the measured one (287 after #48's
  regression test).
- Found late, in a file this issue did not list: `CONTRIBUTING.md` "Feature Flags" said
  `cargo build --release # all defaults (ocr, clipboard, web-preview, detection)` and listed
  "`detection` — YOLO object detection (default)". `detection` was made opt-in by #29 and `jev` was missing
  entirely — the list described a default set that had not existed for two issues. Rewritten against
  `Cargo.toml`, `ml` labelled inert, and the slim description widened (it is display + input + windows +
  fs + shell + system, not "display + input only").

**Verified — the dead-code removal, and one arithmetic error in the audit:**
- Three legs, each producing a genuine `Compiling sys-mcp` rebuild: `clippy --all-targets -- -D warnings`,
  `clippy --no-default-features --all-targets -- -D warnings`, `clippy --features detection --all-targets --
  -D warnings` → **0** diagnostics in all three, `cargo fmt --check` clean. So the ten allows suppressed
  nothing that exists today.
- To prove the removal buys the safety net rather than just tidying, a probe went into `handlers/ui.rs`: an
  unused `pub(crate) fn`, an unused private `fn`, an unused struct → three `dead_code` warnings plus one
  `unused_variables`, under `cargo check` as well as `cargo clippy`. The first probe attempt used `__`
  -prefixed names and reported nothing, which briefly looked like "`dead_code` is disabled crate-wide"; the
  underscore prefix was the cause, not the toolchain. Instrument lesson: piping file *paths* into
  `Select-String` searches the path strings, not the files — the first "0 stale names" sweep was a false
  clean until a positive control (`mouse_click` → 26 hits) exposed it.
- `detection` is **not** "the heaviest feature by a wide margin", and 223-vs-168 is not "nearly doubling".
  Measured twice on Windows x86_64 with `cargo tree -e normal` (unique `name@version`, counting `sys-mcp`):
  slim 167, clipboard 170 (+3), web-preview 194 (+27), ocr 199 (+32), **jev 226 (+59)**, detection 223 (+56),
  default 256 (+89), default+detection 295 (+39). `jev` out-costs `detection` alone, and detection's marginal
  halves once the default set is present — marginals do not sum, which the table now says explicitly.

**Status:** ✅ Fixed — every statement above now matches the code, with the counts re-measured rather than inherited.
**Severity:** 🔵 Low — but an integrator reading the security posture sentence gets the opposite of what ships.

### 46. No accessibility path writes a value into an element (`set_value` missing from the trait)

**Files:** `src/gui/accessibility.rs` (`AccessibilityCapability`), all three `src/platform/accessibility/*`
**Problem (verified 2026-11-05):** `#38` is closed without a way to put text into an edit field, deliberately. `AccessibilityCapability::invoke_element_action(element_id, action)` takes an action *name* and no value argument, so `ValuePattern::SetValue` / `RangeValuePattern::SetValue` are unreachable through the public API on every platform — advertising a `set_value` action would advertise a call that cannot carry the value it needs. Form driving therefore still ends in keyboard injection via `shell`/`gui` input tools rather than the accessibility tree, which is slower and loses the field's identity. Fixing it properly means widening the trait (an `action` + optional `value` pair, or a separate `set_element_value`), implementing it on Windows (Value → RangeValue), macOS (`AXValue`) and Linux (AT-SPI `SetContents`), then extending the tool schema, the handler, `get_element_actions` and the `StubBackend` recording from #43.
**Status:** ✅ Fixed — the trait now carries the operand, and all three platforms write through it.

- **The decision #38 deferred went to widening `invoke_element_action` with `value: Option<&str>` rather than
  a separate `set_element_value` tool.** `set_value` is a verb in the same dispatch the other verbs already
  use, so a second tool would need its own schema, its own `required`-drift test in `tool_schema_tests`, and a
  second path through three platform managers, for something that belongs in the element's own action list.
  `Option<&str>` rather than `&str` defaulting to empty, because "no value" and "an empty value" are different
  operations: the first is a refusal for `set_value`, the second clears the field.
- **Windows** (`src/platform/accessibility/windows.rs`): a `set_value` verb — ValuePattern (refused with its own
  message when `CurrentIsReadOnly`) → RangeValuePattern (the operand must parse as a number; NaN and infinity are
  refused because they *do* parse, and a value outside `CurrentMinimum`…`CurrentMaximum` is refused rather than
  silently clamped, which would answer a different question than the caller asked) → LegacyIAccessible `SetValue`,
  the MSAA bridge a VCL control that never implemented ValuePattern still answers to. `get_element_actions`
  advertises `set_value` only when a **writable** value pattern exists, so the list keeps its promise that nothing
  in it can fail. `setvalue` / `set_text` normalize to it through the same case-and-`-` folding that already maps
  `press`/`click`/`tap`.
- **macOS**: `AXUIElementSetAttributeValue(element, "AXValue", CFString)`. This is an *attribute* write, not a
  `PerformAction` — which is exactly why no AX action name could ever have carried it. Always written as a string:
  an edit field that holds digits holds text, and guessing the attribute type from the operand's shape would
  change what is written. Numeric controls use `AXIncrement`/`AXDecrement`, already reachable.
- **Linux**: `org.a11y.atspi.EditableText.SetTextContents` (the original text of this issue said `SetContents`;
  the real D-Bus method is `SetTextContents`). The Action interface never lists it, so
  `get_element_properties` appends `set_value` to the advertised actions when `Accessible.GetInterfaces` reports
  EditableText on the element.
- **Tool layer** (`tools.rs`, `handlers/accessibility.rs`): `accessibility_action` gained the optional `value`,
  declared `["string", "number"]` because `{"value": 42}` is the natural way to address a spinner. Two refusals
  land before anything is invoked: a value paired with a verb that cannot use it (dropping one of the caller's
  arguments and reporting success is the #40 failure class), and `set_value` with no value at all. The success
  text says "with the given value" but never echoes the value itself — a field write can be a password.
- **Stub and recorder** (#43's seam): `StubA11yRecorder` records `(element_id, action, value)` triples —
  `StubA11yCall` — because "the verb arrived" and "the operand arrived" are different claims, and the `edit`
  element in the fixture now advertises `set_value` the way the real backend does.
- **Verified:** 8 tests added (7 in `tests/handler_tests.rs`, 1 schema guard in `tests/tool_schema_tests.rs`)
  covering the value reaching the backend, a JSON number and an empty string both being accepted, the two
  refusals leaving the recorder empty, a non-scalar `value` being rejected, the verb aliases, and the backend
  holding the contract on its own for a direct API caller. **Mutation-checked:** passing `None` instead of the
  parsed value in the handler makes the two threading tests fail, so the recorder assertion is load-bearing.
  Measured after the batch: **306 passed / 0 failed / 4 ignored** default, 238 / 0 / 1 with
  `--no-default-features`, 309 / 0 / 4 with `--features detection`; `fmt --check` and
  `clippy --all-targets -- -D warnings` clean.
- **Not verified on hardware:** the macOS and Linux writes are compile-gated only — CI's platform legs (#44)
  build them, and this worktree has no Mac, no X server, and no second `rustup` target installed, so neither
  was even `cargo check`ed locally. The Windows path is the one that will meet a real UIA call first, and it is
  the platform the product is used on.
- **Deliberately not added:** a value on `increment`/`decrement`. RangeValue already steps by the control's own
  `CurrentSmallChange`, and an explicit target is expressible as `set_value`; accepting a number there would give
  one argument two meanings.
**Severity:** 🟡 High — the remaining reason a form-automation session falls back to keystrokes and OCR.

### 47. The `StubBackend` suite is not hermetic: system handlers shell out for real, and the detector loads its model

**Files:** `tests/tool_schema_tests.rs` (`every_declared_tool_is_dispatchable`), `src/mcp/handlers/system.rs`, `src/system.rs`, `src/gui/mod.rs` (`GuiClient::detect_objects`), `src/gui/detection.rs`, `src/gui/ocr.rs`, `src/gui/backend/stub.rs`
**Problem (measured 2026-11-05 while adding the `detection` CI leg for #44):** `StubBackend` only stands in for the paths that go through a backend trait. Everything else in `handle_tool_call` ignores the client — the `system_*`/`network_*`/`process_*`/`service_*` handlers all take `_client: &GuiClient` and call `tokio::task::spawn_blocking(SystemMonitor::… | NetworkManager::… | ServiceManager::…)` directly, and the detection path calls the module-level `detection::detect_objects(&screenshot)`. So a "stub" test does real OS work. Established by instrumenting the drift-guard loop with a per-tool `Instant` and running it in both the slim and the `detection` builds (instrumentation reverted; `git diff` on that file is empty):

| dispatched with `{}` | `--no-default-features` | `--features detection` |
| --- | --- | --- |
| whole `every_declared_tool_is_dispatchable` | **11.62 s** | **15.77 s** |
| `network_info` | 4.54 s | 5.15 s |
| `system_stats` | 2.18 s | 2.36 s |
| `disk_usage` | 1.92 s | 1.90 s |
| `process_list` | 1.04 s | 0.90 s |
| `service_list` | 0.87 s | 0.84 s |
| `system_logs` | 0.63 s | 0.74 s |
| `detect_objects` | (feature off) | 2.85 s |
| `screen_read` | (feature off) | **0.02 s** |

Three things fall out of that table:
- **~11.6 s of the drift guard is real OS probing.** `src/system.rs` shells out — `Command::new` appears with `powershell` (5 sites), `sc`, `netstat`, `ipconfig`, `df`, `systemctl`, `launchctl`, `top`, `journalctl`, `ss`, … — so the test that exists to catch schema/dispatch drift spawns child processes and reads the machine's state on every run, on every CI OS, with ±0.6 s of run-to-run variance visible on `network_info` alone. Its duration and its green-ness depend on the runner, not on this crate.
- **`detect_objects` provisions the real detector before it can fail.** `detection::detect_objects` runs `ensure_model()` first, so 2.85 s goes into loading the 12.85 MB YOLOv8n ONNX into tract — and `cargo test --all-targets --features detection` was observed *creating* `%LOCALAPPDATA%\sys-mcp\models\yolov8n.onnx` (12,851,049 bytes, written 02:23:36 → 02:23:56), i.e. the test fetched it from `ultralytics/assets`, the URL that already 404-ed once (#27). `models_dir()` falls back to `PathBuf::from(".")` when `dirs_next::cache_dir()` is `None`, so without a cache dir the model lands in the working directory — the checkout, in CI.
- **No test in the suite has ever run OCR.** `StubBackend::screenshot()` returns `data: vec![]` (with width/height 1920×1080), so `ocr::read_screen` fails at image decode in 0.02 s, before `get_engine()` loads a model. Nothing else in `tests/` or in the `#[cfg(test)]` modules calls `read_screen` or `ocr::` at all (grepped), so the OCR inference path is untested in CI — every `screen_read` behind #31's timings was a manual call — and the two OCR models have never been downloaded by a test. Detection is the mirror case: it pays the expensive step (model provisioning) before the cheap one (image decode), so it spends 2.85 s and still asserts nothing beyond `!text.contains("unknown tool")`.

**Consequences (pre-fix):** the `detection` CI leg downloaded a 12.85 MB third-party asset per run to prove nothing, which is why #44 had to run that leg with `--no-run`; the ~11.6 s OS-probing floor measured in the slim build above was paid by every leg, on every OS, and could not be cached; and "it passed with `StubBackend`" was not evidence that a handler avoided real side effects — the assumption behind #42's validation-order test (provable there only by timing: 0.00 s). The seams below remove all three.

**Status:** ✅ Fixed (2026-11-05) — the two seams this issue called for are in, and the default suite no longer touches the machine.
- **Stage 1 — `SystemCapability` (`src/system.rs`)**: a 13-method async trait answered by `LocalSystem` (production, off the runtime via `spawn_blocking`) and `StubSystem` (fixed fixtures + a call recorder) in tests. The `process_*`/`service_*`/`network_*`/`system_stats`/`disk_usage`/`system_logs` handlers now reach the machine only through `GuiClient::system()`. `handler_tests` dropped from ~11.6 s of OS probing to **0.05 s**, and four `system_logs` tests that used to be `#[cfg(windows)]` + best-effort are now deterministic on every platform.
- **Stage 2 — `OcrCapability` / `DetectionCapability` (`src/gui/ocr.rs`, `src/gui/detection.rs`)**: mirrors the existing `VisionCapability` seam. Handlers reach inference through `GuiClient::ocr()`/`detection()`; `LocalOcr`/`LocalDetection` run the real engines, `StubOcr`/`StubDetection` hand back a fixed result with **no ONNX download and no model load**. This is what lets the `detection` CI leg drop its `--no-run` and actually execute: `cargo test --all-targets --features detection` runs 123 handler tests green in 0.05 s with no `.onnx` touched.
- **Real-OS policy** (the user's chosen "ignore + gated job"): the one remaining live-desktop probe, `monitor_tests::windows_enumeration_reports_valid_monitors`, is `#[ignore]`d with a `real_os` marker, joining the ignored platform-log probe. A manual `workflow_dispatch`-only `real-os` job in `ci.yml` runs `cargo test --all-targets -- --ignored --nocapture` on all three OSes, so the probes are exercised without making the PR gate depend on runner-installed binaries.

Measured after both stages: default `cargo test --all-targets` on Windows = **288 passed / 0 failed / 4 ignored** (292 total); `--no-default-features` and `--features detection` legs both run green. The table above (the 11.6 s floor, the 2.85 s model load, the untested OCR path) is what these seams removed. (That count was taken before #43's second batch added 10 tests; the current figure is in the Status index above.)

**Original shape of the fix** (kept for context): this codebase already used the pattern twice (`DisplayCapability`, `AccessibilityCapability`); handing `StubBackend` a decodable PNG was never the fix — it would only have moved the OCR download into the default job. Both moves needed the trait, not the buffer.
- Confirmed by the platform, not just by timing (2026-11-05, while root-causing #48): a `rust:1-slim-bookworm`
  container ran this suite **green**, and the same binaries failed on ubuntu-latest, because slim has no
  `journalctl`/`systemctl`/`netstat`. Installing `systemd` + `net-tools` reproduced CI exactly. So the suite's
  green-ness is a property of the runner's installed binaries — the strongest form of #47's claim, and the
  reason its fix needs the trait rather than a `cfg(target_os)` band-aid.
**Severity:** 🟡 High — the drift guard's cost and its outcome depend on child processes and a third-party URL, and the OCR path it appears to cover is not covered at all.

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
- Audit 2026-11-05: Windows now reads value/description and the expanded / checked / selected / offscreen states and drives the Toggle, Value, ExpandCollapse, SelectionItem, RangeValue and ScrollItem patterns (#38, #39). Writing a value into an element was unreachable on any platform because the capability trait had no value parameter — closed the same day by #46: `invoke_element_action` takes `Option<&str>` and each platform writes through its own value API (UIA Value/RangeValue/Legacy, `AXValue`, AT-SPI EditableText).

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
- Audit 2026-11-05: the schema/dispatch drift guard is in place, but 20 advertised tools still had no test
  reference and `StubBackend` could not exercise the accessibility handlers at all — tracked as issue #43.
  The "248 tests" figure written in that audit was already stale. Re-measured on the same day, after #43's
  first batch: `cargo test` → **286 passed, 0 failed, 3 ignored** across 14 suites (identical with and
  without `--all-targets`; this crate has no doc tests). Rows 6 and 24 above now carry that number instead
  of the two figures that disagreed with each other.
- Fix 2026-11-05 (#47): the suite is now hermetic. System handlers go through a `SystemCapability`
  trait (`StubSystem`) and inference through `OcrCapability`/`DetectionCapability` (`StubOcr`/`StubDetection`),
  so nothing in `cargo test --all-targets` shells out, reads the event log, builds a live backend or loads an
  ONNX model. Two OCR handler tests run under the default feature set and the `detection` leg now executes
  instead of compile-only; the one live-desktop probe is `#[ignore]`d into a manual `real-os` CI job. Re-measured:
  **288 passing / 0 failed / 4 ignored** (292 total) on Windows.
- Fix 2026-11-05 (#43, second batch): the cheap half of the remaining coverage gap is closed — 10 argument and
  result-shape tests for `system_stats`, `disk_usage`, `decision_stats`, `decision_rank`, `detect_click_object` and
  `screen_scroll_to_text`, all served by the #47 stubs (no shell-out, no model, no gateway). 7 of 72 tools are now
  uncovered — the `mcp_*` hub tools, which need a fake child server to exercise the lifecycle. Re-measured:
  **298 passing / 0 failed / 4 ignored** (302 total) on Windows.
- Fix 2026-11-05 (#46): `AccessibilityCapability::invoke_element_action` takes `value: Option<&str>`, and the new
  `set_value` verb writes through UIA Value/RangeValue/Legacy, macOS `AXValue` and AT-SPI EditableText — the last
  reason a form-driving session had to fall back to keystrokes and OCR. `StubA11yRecorder` now records the value
  it received, and the tool layer refuses a value on a verb that cannot use it instead of dropping the argument.
  8 tests added, mutation-checked by dropping the value at the handler. Re-measured: **306 passing / 0 failed /
  4 ignored** (310 total) on Windows; 238 / 0 / 1 slim; 309 / 0 / 4 with `detection`.
- Fix 2026-11-05 (#34): full-screen OCR now reads small UI text. The detector's working size is fixed by the model
  tensor (`text-detection.onnx` declares `[1,1,800,600]`), so a 1920×1080 frame was being squeezed to **0.31×**
  horizontally instead of being tiled — an 11px glyph became a ~3px feature. `src/gui/ocr.rs` gained the measured
  grid (`DETECT_WORKING_SIZE` 600×800, `SEARCH_WINDOW_OVERLAP` 96px, `axis_positions`/`search_windows`,
  `merge_window_results`) and `OcrCapability` gained **provided** `read_screen_thorough`/`search_text`, so the stub
  sweeps the same geometry the desktop does. `screen_read` takes `strategy: single|thorough` (default `single`),
  `screen_find_text`/`screen_scroll_to_text` take `strategy: auto|single` (default `auto`: escalate only on a miss),
  every response names its own coverage, and unknown values are refused by name instead of running the cheaper leg.
  Also fixed in here: `screen_scroll_to_text` declared absence without ever reading the screen it settled on.
  Recall on one live capture went 5 → 12 of 12 verified lines by window size, and 71 → 179 lines through the shipped
  seam. 29 tests added (pure geometry/merge/matching, `ScriptedOcr` pass counts, handler vocabulary validation, nine
  end-to-end `StubBackend` coverages, one schema guard). Re-measured: **335 passing / 0 failed / 4 ignored**
  (339 total) on Windows; 239 / 0 / 1 with `--no-default-features`; 338 / 0 / 4 with `detection`; `fmt --check`,
  `clippy --all-targets -- -D warnings` and `clippy --all-targets --all-features -- -D warnings` clean.
- Fix 2026-11-05 (#32): requests on one connection are handled at the same time. The stdio loop held a
  `Mutex<McpRequestHandler>` across every `await` only because `handle_request` took `&mut self` — nothing in the
  request path writes to the handler except the one-time web-preview start, which now goes through a
  `std::sync::Mutex<Option<WebServer>>`. `serve_lines` puts each parsed request in a `JoinSet` and keeps reading,
  capped at 32 in flight as admission control; writes stay one whole line each and are encoded before the sink
  lock; stdin EOF drains in-flight answers instead of dropping them; a panic costs one response instead of
  poisoning the handler for the session; an unencodable response comes back as `-32603` rather than silence. The
  HTTP transport held the same lock, so every session on the server was serialized too — gone with it. 6 unit
  tests added, mutation-checked: replacing the spawn with an inline `await` fails
  `a_slow_tool_does_not_hold_up_the_next_request`. Re-measured: **341 passing / 0 failed / 4 ignored** (345 total)
  on Windows; 245 / 0 / 1 with `--no-default-features`; 344 / 0 / 4 with `detection`; `fmt --check`,
  `clippy --all-targets -- -D warnings` and `clippy --all-targets --all-features -- -D warnings` clean.
- Fix 2026-11-05 (#48): the guard tests' process-global config made a *safety* assertion order-dependent —
  three `assess`-level tests read the config without installing it, and the old `get_or_init` reader turned
  that first read into `guardrails: false` for the whole process. `init` is now authoritative in either order,
  and `a_config_read_before_init_does_not_disable_the_guard` pins it (run with `--exact`, so nothing else can
  install first). Suite after the fix: **287 passed / 0 failed / 3 ignored** on Windows, **282 / 0 / 3** on Linux.
- Fix 2026-11-05 (#45): the documentation that contradicted the code is corrected — the guard fail-open sentence, the per-platform accessibility verb claims, 23 pre-rename `gui_*` tool name references across `src/`, `CHANGELOG.md` and `SECURITY.md`, the empty handler banner, and the README/`Cargo.toml` feature-cost numbers (re-measured: `jev` +59 outweighs `detection` +56). The blanket `#![allow(dead_code)]` is gone from all ten handler modules, with `dead_code` proven live by an injected probe.

### Phase 12 — CI/CD 🔓 Partially done
- Three-OS test matrix in `ci.yml`; release workflow builds cross-platform binaries on tag push (`release.yml`).
- Audit 2026-11-05: the matrix works but is not wired to the branch flow — feature-branch pushes trigger nothing, no feature combination is ever built, and `release.yml` has never run because no tag exists. Tracked as issue #44.
- Fix 2026-11-05 (#44): every branch now triggers on push, `workflow_dispatch` exists on both workflows, clippy runs `--all-targets`, two feature legs (`--no-default-features`, `--features detection`) run in a new `features` job, a report-only cargo-deny job with a `deny.toml` seed runs, and `release.yml` smoke-tests each binary, flattens assets with a collision check and publishes `SHA256SUMS`. Executed remotely for the first time on 2026-11-05 (run 41943845280): both feature legs green on Linux, and the matrix caught two defects local Windows verification could not see — a macOS `E0716` (fixed in `147ad79`) and the guard-config ordering bug (fixed as #48). After #47 the `detection` leg dropped its `--no-run` and a manual `real-os` job was added for the ignored machine probes. Still never published a release, and signing is still missing (needs a key decision).
- Fix 2026-11-05 (#44, second batch: supply-chain triage): the `supply-chain` job's checkout gained `submodules: true` (`0228f8f`) so `cargo-deny` can read `rustautogui`'s manifest and actually run. Its first clean report has been triaged in the same batch: nine RUSTSEC advisories closed by patch-level bumps to `anyhow`/`crossbeam-epoch`/`rustls`/`rustls-webpki`/`webpki-roots`/`tokio`; three OSI-approved licenses (BSL-1.0, NCSA, CDLA-Permissive-2.0) added to `deny.toml` with per-entry reasons; the wildcard on the `rustautogui` path dep removed by pinning `version = "2.5.0"` in `Cargo.toml` rather than a `[bans] skip`. Suite unchanged: 341 / 245 / 344 across the three legs, 0 failed each. `continue-on-error` on the job stays until CI proves one green leg.
