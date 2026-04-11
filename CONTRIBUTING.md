# Contributing to gui-mcp

## Getting Started

```bash
# Clone with submodules
git clone --recurse-submodules <repo-url>
cd gui-mcp

# Build
cargo build

# Run with debug logging
cargo run -- --debug
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
│   └── backend/
│       ├── mod.rs       # GuiBackend super-trait
│       └── local.rs     # LocalBackend — wraps rustautogui + platform modules
├── mcp/
│   ├── server.rs        # MCP JSON-RPC server (stdin/stdout loop)
│   ├── tools.rs         # Tool schema definitions (all_tools())
│   └── handlers.rs      # Tool call dispatch and handler functions
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

- Run `cargo fmt` before committing.
- Run `cargo clippy` and address warnings.
- Use `thiserror` and the `GuiError` enum for error handling — don't panic.
- Feature-gate optional functionality with `#[cfg(feature = "...")]`.
- Keep `unsafe` blocks minimal and documented — currently only needed for `AutoGuiWrapper` Send/Sync and Win32 FFI.

## Testing

```bash
# Build and run
cargo build --release
cargo run

# Test with a JSON-RPC request via stdin
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | cargo run
```

Manual testing workflow:
1. Start the server: `cargo run`
2. Send `initialize` → `tools/list` → `tools/call` messages via stdin
3. Or configure in VS Code / Claude Desktop and test interactively

## Feature Flags

Build with specific features:

```bash
cargo build --release                          # all defaults (ocr, clipboard, web-preview)
cargo build --release --no-default-features    # minimal: display + input only
cargo build --release --features opencl        # GPU-accelerated template matching
```

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
