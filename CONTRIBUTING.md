# Contributing to sys-mcp

## Getting Started

```bash
# Clone with submodules
git clone --recurse-submodules <repo-url>
cd sys-mcp

# Build
cargo build

# Run with debug logging
RUST_LOG=debug cargo run
```

If you already cloned without `--recurse-submodules`:

```bash
git submodule update --init --recursive
```

## Project Structure

```
src/
├── main.rs              # Entrypoint — parses config, creates backend, starts server
├── lib.rs               # Module declarations
├── config.rs            # CLI argument parsing (ServerConfig)
├── error.rs             # GuiError enum (thiserror)
├── gui/
│   ├── mod.rs           # GuiClient facade — high-level API used by handlers
│   ├── types.rs         # Shared data types (Screenshot, WindowInfo, etc.)
│   ├── display.rs       # DisplayCapability trait
│   ├── input.rs         # InputCapability trait
│   ├── window.rs        # WindowCapability trait
│   ├── clipboard.rs     # ClipboardCapability trait
│   ├── accessibility.rs # AccessibilityCapability trait
│   ├── ocr.rs           # OCR engine (ocrs) with model management
│   ├── detection.rs     # Object detection (YOLO via tract-onnx)
│   └── backend/
│       ├── mod.rs       # GuiBackend super-trait
│       ├── local.rs     # LocalBackend — wraps rustautogui + platform modules
│       └── stub.rs     # StubBackend — for testing
├── mcp/
│   ├── server.rs        # MCP JSON-RPC server (stdin/stdout loop)
│   ├── tools.rs         # Tool schema definitions (all_tools())
│   ├── handlers.rs      # Tool call dispatch and handler functions
│   ├── hub.rs           # MCP Hub for managing other MCP servers
│   └── session.rs       # Session management for MCP protocol
├── terminal/
│   ├── mod.rs           # Terminal/PTY module
│   ├── pty.rs           # PTY manager using portable-pty
│   └── error.rs         # Terminal-specific errors
├── protocol/
│   └── mcp.rs           # McpRequest, McpResponse, ToolResult, ContentItem
├── platform/
│   ├── window/          # PlatformWindowManager per OS (windows.rs, linux.rs, macos.rs)
│   └── accessibility/   # Platform accessibility stubs
└── web/
    └── server.rs        # Axum-based web preview server
```

## Adding a New Tool

1. **Define the schema** in `src/mcp/tools.rs` — add a `json!({...})` entry to the `all_tools()` vector with `name`, `description`, and `inputSchema`.

2. **Add the handler** in `src/mcp/handlers.rs` — add a match arm in `handle_tool_call()` that extracts arguments and calls through to `GuiClient`.

3. **Add the client method** in `src/gui/mod.rs` — delegate to the appropriate backend capability.

4. **Implement in the backend** — add the method to the relevant capability trait and implement it in `LocalBackend` (`src/gui/backend/local.rs`).

## Adding a New MCP Hub Tool

MCP Hub tools allow sys-mcp to manage other MCP servers:

1. **Define schema** in `src/mcp/tools.rs` — add MCP Hub tool definitions.

2. **Implement handler** in `src/mcp/handlers.rs` — use the `MCP_HUB` static to interact with registered servers.

3. **Logic in hub.rs** — the `McpHub` struct manages:
   - Discovery (scanning `.mcp.json`, npm packages)
   - Registration (adding MCP servers)
   - Lifecycle (start/stop processes)
   - Passthrough (forwarding tool calls)

## Adding a New Terminal/Shell Tool

Terminal tools use the `portable-pty` crate for PTY management:

1. **Define schema** in `src/mcp/tools.rs`.

2. **Implement handler** in `src/mcp/handlers.rs` — uses `PtyManager` from `src/terminal/`.

3. **PTY Manager** (`src/terminal/pty.rs`) handles:
   - Spawning new terminal sessions
   - Reading/writing to PTY
   - Resizing terminals
   - Closing sessions

## Adding a New Capability

1. Create the trait in `src/gui/` (e.g., `src/gui/my_capability.rs`).
2. Add an `as_my_capability()` method to `GuiBackend` (default returns `None`).
3. Implement for `LocalBackend`.
4. Add accessor methods to `GuiClient`.
5. Wire up tools and handlers.

## Adding Platform Support

Platform-specific code lives in `src/platform/`. Each platform module implements the same struct (`PlatformWindowManager`) with the same methods, conditionally compiled via `cfg`:

- `src/platform/window/windows.rs` — Win32 API
- `src/platform/window/linux.rs` — X11 (x11rb)
- `src/platform/window/macos.rs` — AppKit/CoreGraphics

## Code Style

- Run `cargo fmt` before committing (`cargo fmt --check` is the CI gate).
- Run `cargo clippy --all-targets -- -D warnings` and address warnings. `--all-targets` is what CI enforces; without it lints in `tests/` are skipped locally and fail on the push.
- Use `thiserror` and the `GuiError` enum for error handling — don't panic.
- Feature-gate optional functionality with `#[cfg(feature = "...")]`.
- Keep `unsafe` blocks minimal and documented — currently only needed for `AutoGuiWrapper` Send/Sync and Win32 FFI.

## Testing

```bash
# Run all tests
cargo test

# The two feature combinations CI also builds. Both must compile; only the slim
# one is executed (see .github/workflows/ci.yml, and issue #47 for why).
cargo check --all-targets --no-default-features
cargo test --all-targets --no-default-features
cargo check --all-targets --features detection

# Build and run
cargo build --release
cargo run

# Test with a JSON-RPC request via stdin
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | cargo run
```

Manual testing workflow:
1. Start the server: `cargo run`
2. Send `initialize` → `tools/list` → `tools/call` messages via stdin
3. Or configure in VS Code / OpenCode / Claude Desktop and test interactively

## Feature Flags

Build with specific features:

```bash
cargo build --release                          # defaults: ocr, clipboard, web-preview, jev
cargo build --release --no-default-features    # core only: display, input, windows, fs, shell, system
cargo build --release --features detection     # defaults + YOLO object detection
cargo build --no-default-features --features opencl   # slim + GPU template matching in rustautogui
```

Available features (see README "Build profiles" for the measured cost of each):
- `ocr` — on-device OCR via `ocrs`/`rten` (default)
- `clipboard` — clipboard read/write via `arboard` (default)
- `web-preview` — HTTP transport + live viewer via `axum` (default)
- `jev` — JEV decision tools, guardrails and the `ui_decide` loop (default; the
  guardrails and the loop still stay off at runtime behind their flags)
- `detection` — YOLOv8n object detection via `tract-onnx` (**opt-in** — it is not
  in the default set, and it downloads a ~12 MB model on first use)
- `opencl` — forwards `ocl` into the vendored `rustautogui` for GPU image
  matching (off by default)
- `ml` — **inert**. Declared as `ml = []` with no dependencies and no
  `cfg(feature = "ml")` site anywhere; enabling it changes nothing.

### Release builds while a server is running

On Windows `cargo build --release` fails with `failed to remove file
target\release\sys-mcp.exe — Access is denied (os error 5)` if a `sys-mcp.exe`
is still running from that path (an IDE-attached MCP server will be). The
linker cannot replace a running image. Stop that client's connection, or build
to a throwaway directory instead — it exercises the identical graph:

```powershell
cargo build --release --target-dir "$env:TEMP\sys-mcp-release-verify"
```

## License

By contributing, you agree that your contributions will be licensed under the MIT License.