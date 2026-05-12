# Architecture

## Overview

sys-mcp is an MCP (Model Context Protocol) server that bridges AI agents to the local desktop. It reads JSON-RPC 2.0 requests from stdin/HTTP, performs GUI operations, and writes responses to stdout.

```
AI Agent (Claude, OpenCode, etc.)
    │
    │  JSON-RPC 2.0 over stdio or HTTP
    │
┌───▼──────────────────────────────┐
│  GuiMcpServer                    │  src/mcp/server.rs
│  ├── handle_request()            │
│  │   ├── initialize / ping       │
│  │   ├── tools/list → all_tools()│  src/mcp/tools.rs
│  │   └── tools/call              │
│  │       └── handle_tool_call()  │  src/mcp/handlers.rs
│  └── WebServer (optional)        │  src/web/server.rs
└──┬──────────────────────────────┘
    │
┌───▼──────────────────────────────┐
│  GuiClient (facade)              │  src/gui/mod.rs
│  ├── display methods             │
│  ├── input methods               │
│  ├── window methods              │
│  ├── clipboard methods           │
│  ├── accessibility methods       │
│  └── system_info()               │
└──┬──────────────────────────────┘
    │
┌───▼──────────────────────────────┐
│  GuiBackend (trait)              │  src/gui/backend/mod.rs
│  ├── DisplayCapability (required)│  src/gui/display.rs
│  ├── InputCapability  (required) │  src/gui/input.rs
│  ├── WindowCapability (optional) │  src/gui/window.rs
│  ├── ClipboardCapability (opt.)  │  src/gui/clipboard.rs
│  └── AccessibilityCapability(opt)│  src/gui/accessibility.rs
└──┬──────────────────────────────┘
    │
┌───▼──────────────────────────────┐
│  LocalBackend                    │  src/gui/backend/local.rs
│  ├── Mutex<RustAutoGui>          │  rustautogui/ (submodule)
│  ├── PlatformWindowManager       │  src/platform/window/{os}.rs
│  ├── Mutex<arboard::Clipboard>   │  (feature: clipboard)
│  └── OcrEngine (OnceLock)        │  src/gui/ocr.rs (feature: ocr)
└──────────────────────────────────┘
    │
┌───▼──────────────────────────────┐
│  McpHub                          │  src/mcp/hub.rs
│  ├── discover()                  │  Scan .mcp.json, npm packages
│  ├── register() / unregister()    │  Add/remove MCP servers
│  ├── start_server() / stop_server()│  Start/stop MCP processes
│  └── execute_tool()              │  Forward tool calls to MCP servers
└──────────────────────────────────┘
```

## Layer Responsibilities

### MCP Layer (`src/mcp/`)

- **server.rs** — Reads JSON-RPC from stdin, dispatches by method name, writes responses to stdout. Manages the web preview server lifecycle.
- **tools.rs** — Declares all 50+ tool schemas (name, description, inputSchema) returned by `tools/list`.
- **handlers.rs** — Extracts typed arguments from JSON, calls `GuiClient` methods, formats `ToolResult` responses. Handles JPEG compression for screenshots.
- **hub.rs** — MCP Hub for discovering, registering, and tunneling to other MCP servers. Enables passthrough of tool calls to local MCP servers.

### GUI Layer (`src/gui/`)

- **mod.rs (GuiClient)** — Facade over `Box<dyn GuiBackend>`. Provides a clean async API. Checks capability availability via `as_*()` methods, returning `UnsupportedCapability` errors when backends don't support a feature.
- **types.rs** — All shared data types (Screenshot, WindowInfo, Point, Region, etc.). Serializable for JSON responses.
- **Capability traits** — Each trait defines a clean async interface for one capability area. Backends implement what they support.
- **ocr.rs** — Standalone OCR module using the `ocrs` crate. Manages model downloads, runs text detection/recognition, provides text search with click coordinates.
- **detection.rs** — Object detection using YOLOv8 via tract-onnx. Detects common UI elements.

### Backend Layer (`src/gui/backend/`)

- **mod.rs (GuiBackend trait)** — Super-trait combining required capabilities (Display + Input) with optional ones accessed via `as_*()` downcasting.
- **local.rs (LocalBackend)** — The main backend. Wraps `RustAutoGui` in a Mutex for thread safety, delegates window management to platform-specific code, uses `arboard` for clipboard.
- **stub.rs (StubBackend)** — Test backend that returns mock responses for testing.

### Terminal/PTY Layer (`src/terminal/`)

- **pty.rs** — PTY manager using `portable-pty` for spawning interactive shell sessions.
- **session.rs** — Session management for terminal sessions.
- **error.rs** — Terminal-specific error types.

### Platform Layer (`src/platform/`)

- **window/** — OS-specific window management implementations behind a common `PlatformWindowManager` interface. Full Win32 implementation on Windows; stubs on Linux/macOS.
- **accessibility/** — Placeholder stubs for future platform-specific accessibility tree access.

### Protocol Layer (`src/protocol/`)

- **mcp.rs** — JSON-RPC 2.0 types: `McpRequest`, `McpResponse`, `ToolResult`, `ContentItem`. Handles serialization details like untagged enums for success/error responses.

### Web Layer (`src/web/`)

- **server.rs** — Optional Axum HTTP server for the web preview feature. Serves a live interactive desktop viewer with screenshot refresh, click-through, keyboard passthrough, and scroll. Token-authenticated for security.

### MCP Hub Layer (`src/mcp/hub.rs`)

- Provides MCP server discovery from `~/.mcp/`, `.mcp/` directories, and npm global packages
- Manages MCP server registration and lifecycle (start/stop)
- Forwards tool calls to registered MCP servers (passthrough/tunnel)
- Groups tools by category for easier discovery

## Key Design Decisions

### Capability-Based Composition

Instead of a monolithic backend interface, capabilities are split into independent traits. The `GuiBackend` super-trait combines required traits (`Display + Input`) and provides optional access to others via `as_*()` methods. This allows backends to selectively implement features and makes it easy to add new capability areas.

### Thread Safety via Mutex

`RustAutoGui` holds raw platform handles (HDC/HBITMAP on Windows) making it `!Send + !Sync`. Rather than restructuring the underlying library, `LocalBackend` wraps it in `Mutex<AutoGuiWrapper>` with `unsafe impl Send + Sync`. All access is serialized through the mutex, which is safe because the MCP server processes one request at a time.

### OCR Model Management

OCR models (~15MB total) are downloaded on first use from S3 and cached in the user's cache directory (`%LOCALAPPDATA%/sys-mcp/models/` on Windows). The `OcrEngine` is initialized once via `OnceLock` for the process lifetime.

### Screenshot Pipeline

Screenshots flow through: `RustAutoGui.save_screenshot()` → temp PNG file → read bytes → decode with `image` crate → (optional crop/resize/JPEG compress) → base64 encode → MCP response. JPEG compression at quality 60 with 0.5x scaling keeps full-screen screenshots under ~200KB.

### MCP Hub for Server Passthrough

The MCP Hub enables sys-mcp to act as a central hub for multiple MCP servers on the same machine. When running locally, it can discover and forward tool calls to other MCP servers. When running remotely via RDP, it can tunnel back to local MCP servers on the machine where the user is sitting.

### Feature Flags

Optional capabilities are compile-time gated:
- `ocr` — adds ~15MB runtime model download, significant binary size increase
- `clipboard` — adds `arboard` dependency
- `web-preview` — adds `axum` web server dependency
- `detection` — adds `tract-onnx` for YOLO object detection
- `opencl` — enables GPU-accelerated template matching in rustautogui

## MCP Protocol

sys-mcp implements MCP protocol version `2024-11-05`:

- **Transport**: stdio (newline-delimited JSON) or HTTP
- **Methods**: `initialize`, `initialized`, `tools/list`, `tools/call`, `ping`
- **Capabilities**: `{ "tools": {} }`

Tool results use MCP's `ContentItem` format with `type: "text"` for text results and `type: "image"` with base64-encoded data for screenshots.

## MCP Hub Protocol

When using MCP Hub tools, the flow is:

1. `mcp_discover` — Scan for available MCP servers
2. `mcp_register` — Register a new MCP server
3. `mcp_start` — Start the MCP server process
4. `mcp_exec` — Forward tool call to the MCP server
5. `mcp_stop` — Stop the MCP server when done

This enables sys-mcp to proxy any MCP server without the client needing to know the server details.