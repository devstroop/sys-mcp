# sys-mcp

MCP server for local GUI automation — screen capture, OCR, mouse/keyboard control, window management, file system, shell/terminal, and MCP server passthrough/tunnel.

Built in Rust for performance, powered by [rustautogui](https://gitlab.devstroop.com/devstroop/rustautogui) for cross-platform GUI automation and [ocrs](https://github.com/robertknight/ocrs) for on-device OCR.

## Features

| Category | Tools |
|---|---|
| **Display** | `gui_screenshot`, `gui_screenshot_region`, `gui_get_screen_size`, `gui_list_monitors` |
| **OCR** | `gui_read_screen`, `gui_find_text` |
| **Mouse** | `gui_click`, `gui_double_click`, `gui_mouse_move`, `gui_mouse_position`, `gui_drag`, `gui_scroll` |
| **Keyboard** | `gui_type_text`, `gui_press_key` |
| **Clipboard** | `gui_get_clipboard`, `gui_set_clipboard` |
| **Window Mgmt** | `gui_list_windows`, `gui_get_active_window`, `gui_focus_window`, `gui_find_windows`, `gui_move_resize_window`, `gui_window_action` |
| **Accessibility** | `gui_accessibility_tree`, `gui_find_ui_element` |
| **Template Match** | `gui_find_image`, `gui_wait_for_image` |
| **File System** | `gui_read_file`, `gui_write_file`, `gui_list_dir`, `gui_file_exists`, `gui_delete_file`, `gui_create_dir` |
| **Shell/Terminal** | `gui_shell_exec`, `gui_shell_open`, `gui_shell_write`, `gui_shell_read`, `gui_shell_close`, `gui_shell_list` |
| **Process Manager** | `process_list`, `process_kill`, `process_info`, `process_start` |
| **Service Manager** | `service_list`, `service_start`, `service_stop`, `service_status` |
| **Network** | `network_info`, `network_connections` |
| **System Monitoring** | `system_stats`, `disk_usage` |
| **Log Viewer** | `system_logs` |
| **MCP Hub** | `mcp_discover`, `mcp_list`, `mcp_register`, `mcp_unregister`, `mcp_start`, `mcp_stop`, `mcp_tools`, `mcp_tool_groups`, `mcp_exec` |
| **Web Preview** | `gui_web_preview` |
| **System** | `gui_system_info` |

## MCP Hub (MCP Server Passthrough/Tunnel)

sys-mcp can discover, register, and tunnel to other MCP servers on the same machine. This enables:

- **Auto-discovery** of MCP servers from `~/.mcp/`, `.mcp/` directories, and npm global packages
- **Registration** of any MCP server (npx, python, node, etc.)
- **Passthrough** — forward tool calls to registered MCP servers
- **Tunnel** — when running remotely via RDP, tunnel back to local MCP servers

### MCP Hub Usage

```json
// Discover available MCP servers
{ "tool": "mcp_discover" }

// Register a new MCP server
{ "tool": "mcp_register", "args": { "name": "chrome-devtools", "command": "npx", "args": ["-y", "chrome-devtools-mcp@latest"] } }

// Start the MCP server
{ "tool": "mcp_start", "args": { "name": "chrome-devtools" } }

// List tools from all running MCP servers
{ "tool": "mcp_tool_groups" }

// Execute a tool from a running MCP server
{ "tool": "mcp_exec", "args": { "server": "chrome-devtools", "tool": "navigate_page", "args": { "url": "https://example.com" } } }
```

## Build

```bash
# Clone with submodules
git clone --recurse-submodules <repo-url>
cd sys-mcp

# Build release
cargo build --release
```

The binary is output to `target/release/sys-mcp` (or `sys-mcp.exe` on Windows).

## Usage

sys-mcp communicates over **HTTP** using the MCP JSON-RPC protocol.

### OpenCode / VS Code

Add to `opencode.json` or `.vscode/mcp.json`:

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

### Remote Usage (via RDP)

When running sys-mcp on a remote machine accessed via RDP, you can use the local machine's MCP servers by running sys-mcp locally and tunneling:

```json
{
  "mcp": {
    "sys-mcp": {
      "type": "local",
      "command": ["path/to/sys-mcp"],
      "enabled": true
    }
  }
}
```

This allows sys-mcp to access MCP servers on your local machine while you're controlling a remote desktop.

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "sys-mcp": {
      "command": "path/to/sys-mcp"
    }
  }
}
```

## Configuration

| Option | Default | Description |
|---|---|---|
| Port | `3000` | HTTP server port |
| OCR | enabled | On-device text recognition (downloads ~15MB model on first use) |
| Clipboard | enabled | System clipboard read/write |
| Web Preview | enabled | Live screen viewer via browser |

Configure via environment variables:
```bash
PORT=3000 ./target/release/sys-mcp
```

## Platforms

- **Windows** — full support (Win32 API for window management + accessibility)
- **Linux** — display, input, OCR (X11 for window management)
- **macOS** — display, input, OCR (AppKit/CoreGraphics for window management)

## License

MIT