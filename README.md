# sys-mcp

MCP server for system automation — screen capture, OCR, input control, window management, file system, process monitoring, and MCP server passthrough.

Built in Rust for performance, powered by [rustautogui](https://gitlab.devstroop.com/devstroop/rustautogui) for cross-platform GUI automation and [ocrs](https://github.com/robertknight/ocrs) for on-device OCR.

[![CI](https://github.com/devstroop/sys-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/devstroop/sys-mcp/actions)
[![Release](https://github.com/devstroop/sys-mcp/actions/workflows/release.yml/badge.svg)](https://github.com/devstroop/sys-mcp/releases)

## Install

Download a prebuilt binary for your platform:

| Platform | Architecture | Download |
|---|---|---|
| Linux | x86_64 | [sys-mcp-x86_64-linux](https://github.com/devstroop/sys-mcp/releases/latest) |
| macOS | Intel | [sys-mcp-x86_64-macos](https://github.com/devstroop/sys-mcp/releases/latest) |
| macOS | Apple Silicon | [sys-mcp-aarch64-macos](https://github.com/devstroop/sys-mcp/releases/latest) |
| Windows | x86_64 | [sys-mcp-x86_64-windows.exe](https://github.com/devstroop/sys-mcp/releases/latest) |

Or build from source:

```bash
git clone --recurse-submodules https://github.com/devstroop/sys-mcp.git
cd sys-mcp
cargo build --release
# Binary: target/release/sys-mcp (or sys-mcp.exe on Windows)
```

## Tools

### Display
`screen_capture` · `screen_capture_region` · `screen_size` · `screen_list_monitors`

### OCR
`screen_read` · `screen_find_text` · `screen_scroll_to_text`

### Input Control
`mouse_click` · `mouse_double_click` · `mouse_move` · `mouse_position` · `mouse_drag` · `mouse_scroll`
`keyboard_type` · `keyboard_press`

### Clipboard
`clipboard_get` · `clipboard_set`

### Window Management
`window_list` · `window_active` · `window_focus` · `window_find` · `window_move_resize` · `window_action`

### Accessibility
`accessibility_tree` · `accessibility_find`

### Object Detection
`detect_objects` · `detect_click_object`

### File System
`fs_read` · `fs_write` · `fs_list` · `fs_exists` · `fs_delete` · `fs_create_dir`

### Shell / Terminal
`shell_exec` · `shell_open` · `shell_write` · `shell_read` · `shell_close` · `shell_list`

### Process Manager
`process_list` · `process_kill` · `process_info` · `process_start`

### Service Manager
`service_list` · `service_start` · `service_stop` · `service_status`

### Network
`network_info` · `network_connections`

### System Monitoring
`system_stats` · `disk_usage` · `system_logs`

### MCP Hub
`mcp_discover` · `mcp_list` · `mcp_register` · `mcp_unregister` · `mcp_start` · `mcp_stop` · `mcp_tools` · `mcp_tool_groups` · `mcp_exec`

### Utilities
`web_preview` · `system_info` · `system_wait`

## MCP Hub

Discover, register, and proxy to other MCP servers on the same machine. Supports `~/.mcp/`, `.mcp/`, and npm global packages.

```json
// Discover available MCP servers
{ "tool": "mcp_discover" }

// Register a server
{ "tool": "mcp_register", "args": { "name": "chrome", "command": "npx", "args": ["-y", "chrome-devtools-mcp"] } }

// Start and use
{ "tool": "mcp_start", "args": { "name": "chrome" } }
{ "tool": "mcp_exec", "args": { "server": "chrome", "tool": "navigate_page", "args": { "url": "..." } } }
```

## Configuration

| Option | Default | Description |
|---|---|---|
| `PORT` | `3000` | HTTP server port |
| `OCR` | enabled | On-device text recognition (~15MB model on first use) |
| `clipboard` | enabled | System clipboard |
| `web-preview` | enabled | Live screen viewer in browser |

```bash
PORT=3000 ./sys-mcp
```

Disable features at build time:

```toml
[dependencies]
sys-mcp = { path = ".", default-features = false, features = ["clipboard"] }
```

## Connecting to MCP Clients

### OpenCode / VS Code

```json
{
  "mcp": {
    "sys-mcp": {
      "type": "remote",
      "url": "http://localhost:3000/mcp",
      "enabled": true
    }
  }
}
```

### Claude Desktop

```json
{
  "mcpServers": {
    "sys-mcp": {
      "command": "path/to/sys-mcp"
    }
  }
}
```

### Remote / RDP

When running on a remote machine, sys-mcp can tunnel to MCP servers on your local machine. Configure as `local` type in your MCP client.

## Platforms

| Platform | Window Mgmt | Accessibility | Input | OCR |
|---|---|---|---|---|
| **Windows** | Win32 API | Windows UI Automation | yes | yes |
| **Linux** | X11 | — | yes | yes |
| **macOS** | AppKit/CoreGraphics | macOS Accessibility API | yes | yes |

## License

MIT