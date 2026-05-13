# Sys MCP

MCP server for system automation — screen capture, OCR, input control, window management, file system, process monitoring, and MCP server passthrough.

Built in Rust for performance, powered by [rustautogui](https://gitlab.devstroop.com/devstroop/rustautogui) for cross-platform GUI automation and [ocrs](https://github.com/robertknight/ocrs) for on-device OCR.

[![CI](https://github.com/devstroop/gui-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/devstroop/gui-mcp/actions)
[![Release](https://github.com/devstroop/gui-mcp/actions/workflows/release.yml/badge.svg)](https://github.com/devstroop/gui-mcp/releases)

## Install

Download a prebuilt binary for your platform:

| Platform | Architecture | Download |
|---|---|---|
| Linux | x86_64 | [sys-mcp-x86_64-linux](https://github.com/devstroop/gui-mcp/releases/latest) |
| macOS | Intel | [sys-mcp-x86_64-macos](https://github.com/devstroop/gui-mcp/releases/latest) |
| macOS | Apple Silicon | [sys-mcp-aarch64-macos](https://github.com/devstroop/gui-mcp/releases/latest) |
| Windows | x86_64 | [sys-mcp-x86_64-windows.exe](https://github.com/devstroop/gui-mcp/releases/latest) |

Or build from source:

```bash
git clone --recurse-submodules https://github.com/devstroop/gui-mcp.git
cd sys-mcp
cargo build --release
# Binary: target/release/sys-mcp (or sys-mcp.exe on Windows)
```

## Features

### Display & Input
| Tool | Description |
|---|---|
| `gui_screenshot` | Full screen capture |
| `gui_screenshot_region` | Capture a specific region |
| `gui_get_screen_size` | Get screen dimensions |
| `gui_list_monitors` | List all monitors |

### OCR & Text
| Tool | Description |
|---|---|
| `gui_read_screen` | OCR on a region (on-device, no cloud) |
| `gui_find_text` | Find text in a region |

### Input Control
| Tool | Description |
|---|---|
| `gui_click`, `gui_double_click` | Mouse clicks |
| `gui_mouse_move`, `gui_mouse_position` | Mouse movement |
| `gui_drag` | Drag operations |
| `gui_scroll` | Scroll wheel |
| `gui_type_text`, `gui_press_key` | Keyboard input |

### Clipboard
| Tool | Description |
|---|---|
| `gui_get_clipboard` | Read clipboard |
| `gui_set_clipboard` | Write clipboard |

### Window Management
| Tool | Description |
|---|---|
| `gui_list_windows` | List open windows |
| `gui_get_active_window` | Get focused window |
| `gui_focus_window` | Focus a window |
| `gui_find_windows` | Find windows by name |
| `gui_move_resize_window` | Move/resize windows |
| `gui_window_action` | Min/max/close/restore |

### Accessibility
| Tool | Description |
|---|---|
| `gui_accessibility_tree` | Full accessibility tree for a window |
| `gui_find_ui_element` | Search UI elements by name/role |

### Template Matching
| Tool | Description |
|---|---|
| `gui_find_image` | Find an image on screen |
| `gui_wait_for_image` | Wait for an image to appear |

### File System
| Tool | Description |
|---|---|
| `gui_read_file`, `gui_write_file` | Read/write files |
| `gui_list_dir`, `gui_file_exists` | List directories, check existence |
| `gui_delete_file`, `gui_create_dir` | Delete files, create directories |

### Shell / Terminal
| Tool | Description |
|---|---|
| `gui_shell_exec` | Execute a shell command |
| `gui_shell_open`, `gui_shell_close`, `gui_shell_list` | Manage persistent shells |
| `gui_shell_write`, `gui_shell_read` | Interact with active shells |

### System & Monitoring
| Tool | Description |
|---|---|
| `process_list`, `process_kill`, `process_info` | Process management |
| `service_list`, `service_start`, `service_stop` | Windows service manager |
| `network_info`, `network_connections` | Network information |
| `system_stats`, `disk_usage` | System monitoring |
| `system_logs` | View system logs |

### MCP Hub
Discover, register, and proxy to other MCP servers on the same machine. Supports `~/.mcp/`, `.mcp/`, and npm global packages.

| Tool | Description |
|---|---|
| `mcp_discover` | Auto-discover MCP servers |
| `mcp_register` | Register a new MCP server |
| `mcp_start` / `mcp_stop` | Start/stop a server |
| `mcp_list` | List registered servers |
| `mcp_tools` | List tools from a server |
| `mcp_exec` | Execute a tool from a server |

## MCP Hub Usage

```json
// Discover available MCP servers
{ "tool": "mcp_discover" }

// Register a server
{
  "tool": "mcp_register",
  "args": { "name": "chrome", "command": "npx", "args": ["-y", "chrome-devtools-mcp"] }
}

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
| **Windows** | Win32 API | Windows UI Automation | ✅ | ✅ |
| **Linux** | X11 | — | ✅ | ✅ |
| **macOS** | AppKit/CoreGraphics | macOS Accessibility API | ✅ | ✅ |

## License

MIT