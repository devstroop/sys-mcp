use serde_json::{json, Value};

pub fn all_tools() -> Vec<Value> {
    vec![
        // ── Display ─────────────────────────────────────────────────────
        json!({
            "name": "gui_screenshot",
            "description": "Capture the local screen as a compressed JPEG image. For reading text content, PREFER gui_read_screen instead — it returns structured text with clickable coordinates and is much smaller.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gui_screenshot_region",
            "description": "Capture a rectangular region of the local screen.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "X coordinate of top-left corner." },
                    "y": { "type": "integer", "description": "Y coordinate of top-left corner." },
                    "width": { "type": "integer", "description": "Width in pixels." },
                    "height": { "type": "integer", "description": "Height in pixels." }
                },
                "required": ["x", "y", "width", "height"]
            }
        }),
        json!({
            "name": "gui_get_screen_size",
            "description": "Get the current screen resolution.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gui_list_monitors",
            "description": "List all connected monitors with resolution and position.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── OCR — Screen Reading ────────────────────────────────────────
        json!({
            "name": "gui_read_screen",
            "description": "Read all text on screen via on-device OCR. Returns structured text with word-level bounding boxes and click coordinates (cx, cy). PREFER THIS over gui_screenshot for understanding screen content — it returns ~2KB instead of a multi-MB image. First call downloads OCR models (~15MB, cached). Optionally specify a region to OCR only part of the screen.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "Region X (optional — omit for full screen)." },
                    "y": { "type": "integer", "description": "Region Y." },
                    "width": { "type": "integer", "description": "Region width." },
                    "height": { "type": "integer", "description": "Region height." },
                    "detail": {
                        "type": "string",
                        "enum": ["full", "lines", "text"],
                        "description": "Response detail level. 'text' = plain text only (smallest), 'lines' = text per line with bounding boxes (no words), 'full' = lines + word-level coordinates (default).",
                        "default": "full"
                    }
                }
            }
        }),
        json!({
            "name": "gui_find_text",
            "description": "Search for specific text on screen via OCR. Returns matching words/phrases with center (cx, cy) coordinates ready for gui_click. Use this to find buttons, links, menu items, or any text element.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Text to search for (case-insensitive substring match)." }
                },
                "required": ["query"]
            }
        }),
        // ── Mouse ───────────────────────────────────────────────────────
        json!({
            "name": "gui_click",
            "description": "Click at a position on the local screen.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "X coordinate." },
                    "y": { "type": "integer", "description": "Y coordinate." },
                    "button": {
                        "type": "string",
                        "enum": ["left", "right", "middle"],
                        "description": "Mouse button.",
                        "default": "left"
                    }
                },
                "required": ["x", "y"]
            }
        }),
        json!({
            "name": "gui_double_click",
            "description": "Double-click at a position on the local screen.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "x": { "type": "integer" },
                    "y": { "type": "integer" }
                },
                "required": ["x", "y"]
            }
        }),
        json!({
            "name": "gui_mouse_move",
            "description": "Move the mouse cursor to a position without clicking.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "x": { "type": "integer" },
                    "y": { "type": "integer" }
                },
                "required": ["x", "y"]
            }
        }),
        json!({
            "name": "gui_mouse_position",
            "description": "Get the current mouse cursor position.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gui_drag",
            "description": "Click-drag from one position to another.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from_x": { "type": "integer" },
                    "from_y": { "type": "integer" },
                    "to_x": { "type": "integer" },
                    "to_y": { "type": "integer" },
                    "button": { "type": "string", "enum": ["left", "right", "middle"], "default": "left" }
                },
                "required": ["from_x", "from_y", "to_x", "to_y"]
            }
        }),
        json!({
            "name": "gui_scroll",
            "description": "Scroll at a position on the local screen.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "x": { "type": "integer" },
                    "y": { "type": "integer" },
                    "direction": { "type": "string", "enum": ["up", "down", "left", "right"] },
                    "amount": { "type": "integer", "description": "Number of scroll notches.", "default": 3 }
                },
                "required": ["x", "y", "direction"]
            }
        }),
        // ── Keyboard ────────────────────────────────────────────────────
        json!({
            "name": "gui_type_text",
            "description": "Type a string of text on the local machine.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to type." }
                },
                "required": ["text"]
            }
        }),
        json!({
            "name": "gui_press_key",
            "description": "Press a key or key combination. Examples: 'return', 'tab', 'ctrl+c', 'ctrl+alt+delete', 'f5'. Use '+' to combine modifier keys.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Key name or combo (e.g. 'ctrl+s', 'return', 'f1')." }
                },
                "required": ["key"]
            }
        }),
        // ── Clipboard ───────────────────────────────────────────────────
        json!({
            "name": "gui_get_clipboard",
            "description": "Read the current clipboard text.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gui_set_clipboard",
            "description": "Set the clipboard text.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"]
            }
        }),
        // ── Window Management ───────────────────────────────────────────
        json!({
            "name": "gui_list_windows",
            "description": "List all visible windows with id, title, position, size, and state.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gui_get_active_window",
            "description": "Get the currently focused/active window.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gui_focus_window",
            "description": "Bring a window to the foreground and give it focus. Provide either window_id (from gui_list_windows) or title (substring match).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "window_id": { "type": "integer", "description": "Window ID from gui_list_windows." },
                    "title": { "type": "string", "description": "Window title substring to search for." }
                }
            }
        }),
        json!({
            "name": "gui_move_resize_window",
            "description": "Move and/or resize a window. Provide window_id and any combination of x, y, width, height.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "window_id": { "type": "integer", "description": "Window ID." },
                    "x": { "type": "integer", "description": "New X position." },
                    "y": { "type": "integer", "description": "New Y position." },
                    "width": { "type": "integer", "description": "New width." },
                    "height": { "type": "integer", "description": "New height." }
                },
                "required": ["window_id"]
            }
        }),
        json!({
            "name": "gui_window_action",
            "description": "Perform a window state action: minimize, maximize, restore, or close.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "window_id": { "type": "integer", "description": "Window ID." },
                    "action": {
                        "type": "string",
                        "enum": ["minimize", "maximize", "restore", "close"],
                        "description": "Action to perform."
                    }
                },
                "required": ["window_id", "action"]
            }
        }),
        json!({
            "name": "gui_find_windows",
            "description": "Search for windows by title substring. Returns matching windows with ids for use with other window tools.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Title substring to search for (case-insensitive)." }
                },
                "required": ["query"]
            }
        }),
        // ── Accessibility (Phase 6 — not yet wired up) ────────────────
        // gui_accessibility_tree and gui_find_ui_element are kept in
        // handlers.rs but hidden from tools/list until backend is ready.

        // ── Template Matching (Phase 7 — not yet implemented) ──────────
        // gui_find_image and gui_wait_for_image are kept in handlers.rs
        // but hidden from tools/list until handlers are wired up.

        // ── Utility ────────────────────────────────────────────────────
        json!({
            "name": "gui_wait",
            "description": "Wait for a specified duration. Useful between focus + type sequences or to wait for UI transitions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ms": { "type": "integer", "description": "Duration to wait in milliseconds (max 30000).", "default": 500 }
                }
            }
        }),
        json!({
            "name": "gui_scroll_to_text",
            "description": "Scroll the screen until specific text is found via OCR, or give up after max_scrolls. Useful for finding off-screen content like buttons, links, or sections. Returns the match coordinates when found.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Text to search for (case-insensitive substring match)." },
                    "direction": { "type": "string", "enum": ["up", "down"], "description": "Scroll direction.", "default": "down" },
                    "max_scrolls": { "type": "integer", "description": "Maximum number of scroll attempts before giving up.", "default": 10 },
                    "scroll_amount": { "type": "integer", "description": "Number of scroll notches per attempt.", "default": 3 },
                    "x": { "type": "integer", "description": "X coordinate to scroll at (defaults to screen center)." },
                    "y": { "type": "integer", "description": "Y coordinate to scroll at (defaults to screen center)." }
                },
                "required": ["query"]
            }
        }),
        // ── Web Preview ─────────────────────────────────────────────────
        json!({
            "name": "gui_web_preview",
            "description": "Get a live web preview URL for the local screen. Open this in a browser to view and interact with the desktop remotely.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Object Detection ────────────────────────────────────────────
        json!({
            "name": "gui_detect_objects",
            "description": "Detect objects on screen using YOLOv8. Returns bounding boxes with labels for common objects (person, laptop, mouse, keyboard, chair, cup, bottle, etc.). Useful for 'click the laptop' or 'click the cup' automation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "min_confidence": {
                        "type": "number",
                        "description": "Minimum confidence threshold (0.0-1.0).",
                        "default": 0.3
                    },
                    "labels": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter to only these labels (e.g. [\"laptop\", \"mouse\"]). Omit for all."
                    }
                }
            }
        }),
        json!({
            "name": "gui_click_object",
            "description": "Click an object by label. Use gui_detect_objects first to see available objects, then specify the label and index (0-based) to click.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "label": { "type": "string", "description": "Object label (e.g. \"laptop\", \"cup\")" },
                    "index": { "type": "integer", "description": "Which object to click if multiple match (0 = first).", "default": 0 }
                },
                "required": ["label"]
            }
        }),
        // ── System Info ─────────────────────────────────────────────────
        json!({
            "name": "gui_system_info",
            "description": "Get system information: OS, screen size, and available capabilities. Call this first to understand what the local GUI server supports.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── File System ──────────────────────────────────────────────────
        json!({
            "name": "gui_read_file",
            "description": "Read a file and return its contents as base64. Use this to read binary files (images, executables, archives) or large text files. For small text files, gui_get_clipboard may be simpler.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file to read." }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "gui_write_file",
            "description": "Write base64-encoded content to a file. Use this to write binary files (images, executables, archives) or large text files. The content must be base64-encoded. Creates parent directories if they don't exist.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to write the file to." },
                    "content": { "type": "string", "description": "Base64-encoded file content." }
                },
                "required": ["path", "content"]
            }
        }),
        json!({
            "name": "gui_list_dir",
            "description": "List contents of a directory. Returns files and subdirectories with names, sizes, and modification times.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the directory to list.", "default": "." }
                }
            }
        }),
        json!({
            "name": "gui_file_exists",
            "description": "Check if a file or directory exists.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to check." }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "gui_delete_file",
            "description": "Delete a file. Does not delete directories. Use gui_delete_dir for directories.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file to delete." }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "gui_create_dir",
            "description": "Create a directory (and parent directories if needed).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path of the directory to create." }
                },
                "required": ["path"]
            }
        }),
        // ── Shell/Terminal ─────────────────────────────────────────────────
        json!({
            "name": "gui_shell_exec",
            "description": "Execute a shell command and return its output. Opens a temporary terminal, runs the command, captures output, and closes. Use for single commands like 'dir', 'ls', 'git status'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to execute." },
                    "cwd": { "type": "string", "description": "Working directory for the command (optional)." }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "gui_shell_open",
            "description": "Open a new interactive shell session. Returns a session_id that can be used with gui_shell_write and gui_shell_close. Useful for multi-step operations like 'cd project && npm install'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Initial working directory (optional, defaults to current directory)." },
                    "cols": { "type": "integer", "description": "Terminal columns (default 80).", "default": 80 },
                    "rows": { "type": "integer", "description": "Terminal rows (default 24).", "default": 24 }
                }
            }
        }),
        json!({
            "name": "gui_shell_write",
            "description": "Write input to an open shell session. Use the session_id from gui_shell_open. Send commands followed by newline to execute.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from gui_shell_open." },
                    "input": { "type": "string", "description": "Input to send to the shell (append newline to execute)." }
                },
                "required": ["session_id", "input"]
            }
        }),
        json!({
            "name": "gui_shell_read",
            "description": "Read output from an open shell session. Returns any new output since last read. Use in a loop to capture command results.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from gui_shell_open." }
                },
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "gui_shell_close",
            "description": "Close an open shell session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": { "type": "string", "description": "Session ID from gui_shell_open." }
                },
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "gui_shell_list",
            "description": "List all open shell sessions.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── MCP Hub (MCP Server Passthrough/Tunnel) ──────────────────────────
        json!({
            "name": "mcp_discover",
            "description": "Discover MCP servers on the local machine. Scans ~/.mcp/, .mcp/ directories, and npm global packages for MCP servers. Returns list of available MCP servers that can be registered.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "mcp_list",
            "description": "List all registered MCP servers and their status (discovered, running, stopped). Shows which MCP servers are available and active.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "mcp_register",
            "description": "Register a new MCP server to be managed by this hub. Provide the command and arguments to start the MCP server (e.g., npx, python, node).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Unique name for this MCP server." },
                    "command": { "type": "string", "description": "Command to run the MCP server (e.g., 'npx', 'python', 'node')." },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Arguments for the command (e.g., ['-y', 'chrome-devtools-mcp@latest'])" }
                },
                "required": ["name", "command"]
            }
        }),
        json!({
            "name": "mcp_unregister",
            "description": "Unregister an MCP server. Stops it if running and removes it from the registry.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Name of the MCP server to unregister." }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "mcp_start",
            "description": "Start a registered MCP server. This activates the MCP server and loads its tools. Once started, you can use mcp_exec to call tools from this server.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Name of the MCP server to start." }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "mcp_stop",
            "description": "Stop a running MCP server. This deactivates the server but keeps it registered. Use mcp_start to reactivate.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Name of the MCP server to stop." }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "mcp_tools",
            "description": "List all tools available from running MCP servers. Returns tools grouped by category with descriptions and input schemas. Use this to see what's available.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "mcp_tool_groups",
            "description": "Get MCP tools organized by category/groups (like YouTrack pattern). Returns tools grouped by their category (e.g., browser, filesystem, etc.) for easier discovery.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "mcp_exec",
            "description": "Execute a tool from a running MCP server. Specify the server name, tool name, and arguments. This is a passthrough - the request is forwarded to the MCP server and response is returned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "server": { "type": "string", "description": "Name of the MCP server (from mcp_list)." },
                    "tool": { "type": "string", "description": "Name of the tool to execute (from mcp_tools)." },
                    "args": { "type": "object", "description": "Arguments for the tool (optional)." }
                },
                "required": ["server", "tool"]
            }
        }),
        // ── Process Manager ───────────────────────────────────────────────────
        json!({
            "name": "process_list",
            "description": "List running processes on the system. Returns top processes sorted by CPU usage (up to 100). Shows PID, name, CPU%, memory, and status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Maximum number of processes to return (default 50).", "default": 50 }
                }
            }
        }),
        json!({
            "name": "process_kill",
            "description": "Kill a process by its PID. Use process_list to find the PID first. Force kills the process immediately.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pid": { "type": "integer", "description": "Process ID to kill." }
                },
                "required": ["pid"]
            }
        }),
        json!({
            "name": "process_info",
            "description": "Get detailed information about a specific process by PID. Returns process name, CPU usage, memory usage, and status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pid": { "type": "integer", "description": "Process ID to get info for." }
                },
                "required": ["pid"]
            }
        }),
        json!({
            "name": "process_start",
            "description": "Start a new process. Provide the command and optional arguments. Returns the PID of the started process.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to execute." },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Arguments for the command (optional)." }
                },
                "required": ["command"]
            }
        }),
        // ── Service Manager ───────────────────────────────────────────────────
        json!({
            "name": "service_list",
            "description": "List all system services/daemons. Shows name, display name, status, and start type for each service.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "service_start",
            "description": "Start a system service/daemon by name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Service name to start." }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "service_stop",
            "description": "Stop a system service/daemon by name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Service name to stop." }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "service_status",
            "description": "Get detailed status of a specific service.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Service name to query." }
                },
                "required": ["name"]
            }
        }),
        // ── Network Tools ─────────────────────────────────────────────────────
        json!({
            "name": "network_info",
            "description": "Get network configuration information including hostname, IP addresses, default gateway, and DNS servers.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "network_connections",
            "description": "List active network connections (TCP/UDP). Shows protocol, local address, remote address, state, and process ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Maximum connections to return (default 50).", "default": 50 }
                }
            }
        }),
        // ── System Monitoring ─────────────────────────────────────────────────
        json!({
            "name": "system_stats",
            "description": "Get current system resource usage: CPU percentage, memory total/used/percent, disk total/used/percent.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "disk_usage",
            "description": "Get disk space information for all drives/mount points. Shows total, used, free space and percentage for each.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Log Viewer ──────────────────────────────────────────────────────
        json!({
            "name": "system_logs",
            "description": "Read system/event logs. Shows recent log entries with timestamp, source, level, and message. Filter by level (Error, Warning, Information).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "count": { "type": "integer", "description": "Number of log entries to return (default 50).", "default": 50 },
                    "level": { "type": "string", "enum": ["Error", "Warning", "Information"], "description": "Filter by log level (optional)." }
                }
            }
        }),
    ]
}
