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
                    "height": { "type": "integer", "description": "Region height." }
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

        // ── Accessibility ───────────────────────────────────────────────
        json!({
            "name": "gui_accessibility_tree",
            "description": "Get the UI element accessibility tree for a window. Returns element hierarchy with ids, roles, names, bounds, and clickable coordinates. Requires the 'accessibility' feature.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "window_id": { "type": "integer", "description": "Window ID (omit for focused window)." },
                    "max_depth": { "type": "integer", "description": "Maximum tree depth (default: 5).", "default": 5 }
                }
            }
        }),
        json!({
            "name": "gui_find_ui_element",
            "description": "Search for a UI element by name and/or role in the accessibility tree. Returns matching elements with clickable coordinates (cx, cy).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Element name to search for." },
                    "role": { "type": "string", "description": "Element role filter (e.g. 'button', 'text', 'edit')." },
                    "window_id": { "type": "integer", "description": "Window ID (omit for focused window)." }
                },
                "required": ["query"]
            }
        }),

        // ── Template Matching ───────────────────────────────────────────
        json!({
            "name": "gui_find_image",
            "description": "Find an image template on screen. Returns match locations with center coordinates for clicking. Supports GPU-accelerated matching with OpenCL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "template_base64": { "type": "string", "description": "Base64-encoded PNG/JPEG image to find on screen." },
                    "precision": { "type": "number", "description": "Match precision 0.0-1.0 (default 0.8).", "default": 0.8 },
                    "region": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "integer" },
                            "y": { "type": "integer" },
                            "width": { "type": "integer" },
                            "height": { "type": "integer" }
                        },
                        "description": "Optional screen region to search within."
                    },
                    "match_mode": {
                        "type": "string",
                        "enum": ["segmented", "fft"],
                        "description": "Matching algorithm.",
                        "default": "segmented"
                    }
                },
                "required": ["template_base64"]
            }
        }),
        json!({
            "name": "gui_wait_for_image",
            "description": "Wait for an image template to appear on screen (polling with timeout). Returns match location when found, or error on timeout.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "template_base64": { "type": "string", "description": "Base64-encoded PNG/JPEG image to wait for." },
                    "precision": { "type": "number", "description": "Match precision 0.0-1.0 (default 0.8).", "default": 0.8 },
                    "timeout": { "type": "integer", "description": "Timeout in seconds (default 30).", "default": 30 },
                    "region": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "integer" },
                            "y": { "type": "integer" },
                            "width": { "type": "integer" },
                            "height": { "type": "integer" }
                        },
                        "description": "Optional screen region to search within."
                    }
                },
                "required": ["template_base64"]
            }
        }),

        // ── Web Preview ─────────────────────────────────────────────────
        json!({
            "name": "gui_web_preview",
            "description": "Get a live web preview URL for the local screen. Open this in a browser to view and interact with the desktop remotely.",
            "inputSchema": { "type": "object", "properties": {} }
        }),

        // ── System Info ─────────────────────────────────────────────────
        json!({
            "name": "gui_system_info",
            "description": "Get system information: OS, screen size, and available capabilities. Call this first to understand what the local GUI server supports.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
    ]
}
