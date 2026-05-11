# gui-mcp

MCP server for local GUI automation — screen capture, OCR, mouse/keyboard control, window management, accessibility, and template matching.

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
| **Web Preview** | `gui_web_preview` |
| **System** | `gui_system_info` |

## Build

```bash
# Clone with submodules
git clone --recurse-submodules <repo-url>
cd gui-mcp

# Build release
cargo build --release
```

The binary is output to `target/release/gui-mcp` (or `gui-mcp.exe` on Windows).

## Usage

gui-mcp communicates over **HTTP** using the MCP JSON-RPC protocol.

### OpenCode / VS Code

Add to `opencode.json` or `.vscode/mcp.json`:

```json
{
  "mcp": {
    "gui-mcp": {
      "type": "remote",
      "url": "http://localhost:3000/mcp",
      "enabled": true
    }
  }
}
```

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gui-mcp": {
      "command": "path/to/gui-mcp"
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
PORT=3000 ./target/release/gui-mcp
```

## Platforms

- **Windows** — full support (Win32 API for window management + accessibility)
- **Linux** — display, input, OCR (X11 for window management)
- **macOS** — display, input, OCR (AppKit/CoreGraphics for window management)

## License

MIT