use serde_json::{json, Value};
use std::fs;
use sys_mcp::gui::GuiClient;
use sys_mcp::mcp::handlers::handle_tool_call;
use sys_mcp::{StubA11yBackend, StubA11yRecorder, StubBackend, StubSystem};

fn test_client() -> GuiClient {
    GuiClient::new(StubBackend)
}

#[tokio::test]
async fn test_gui_screenshot() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_capture", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    assert!(
        !result.content.is_empty(),
        "expected at least one content item"
    );
}

#[tokio::test]
async fn test_gui_screenshot_region() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "screen_capture_region",
        json!({"x": 0, "y": 0, "width": 100, "height": 100}),
    )
    .await;
    assert!(!result.is_error.unwrap_or(false));
    assert!(
        !result.content.is_empty(),
        "expected at least one content item"
    );
}

#[tokio::test]
async fn test_gui_screenshot_region_missing_args() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_capture_region", json!({"x": 0})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_get_screen_size() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_size", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert_eq!(text, "1920x1080");
}

#[tokio::test]
async fn test_gui_list_monitors() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_list_monitors", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Primary"), "got: {text}");
}

#[tokio::test]
async fn test_gui_click() {
    let client = test_client();
    let result = handle_tool_call(&client, "mouse_click", json!({"x": 100, "y": 200})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("(100, 200)"), "got: {text}");
}

#[tokio::test]
async fn test_gui_click_missing_y() {
    let client = test_client();
    let result = handle_tool_call(&client, "mouse_click", json!({"x": 100})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_double_click() {
    let client = test_client();
    let result = handle_tool_call(&client, "mouse_double_click", json!({"x": 50, "y": 60})).await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_mouse_move() {
    let client = test_client();
    let result = handle_tool_call(&client, "mouse_move", json!({"x": 10, "y": 20})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("(10, 20)"));
}

#[tokio::test]
async fn test_gui_mouse_position() {
    let client = test_client();
    let result = handle_tool_call(&client, "mouse_position", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(result.content[0].text.as_deref().unwrap_or(""), "(0, 0)");
}

#[tokio::test]
async fn test_gui_drag() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "mouse_drag",
        json!({"from_x": 0, "from_y": 0, "to_x": 100, "to_y": 100}),
    )
    .await;
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(
        result.content[0].text.as_deref().unwrap_or(""),
        "Drag complete."
    );
}

#[tokio::test]
async fn test_gui_drag_missing_from_x() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "mouse_drag",
        json!({"from_y": 0, "to_x": 100, "to_y": 100}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_scroll() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "mouse_scroll",
        json!({"x": 0, "y": 0, "direction": "down"}),
    )
    .await;
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(result.content[0].text.as_deref().unwrap_or(""), "Scrolled.");
}

#[tokio::test]
async fn test_gui_scroll_bad_direction() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "mouse_scroll",
        json!({"x": 0, "y": 0, "direction": "sideways"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_type_text() {
    let client = test_client();
    let result = handle_tool_call(&client, "keyboard_type", json!({"text": "hello world"})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("11"), "expected 11 chars, got: {text}");
}

#[tokio::test]
async fn test_gui_press_key_simple() {
    let client = test_client();
    let result = handle_tool_call(&client, "keyboard_press", json!({"key": "return"})).await;
    assert!(!result.is_error.unwrap_or(false));
    assert!(result.content[0]
        .text
        .as_deref()
        .unwrap_or("")
        .contains("return"));
}

#[tokio::test]
async fn test_gui_press_key_combo() {
    let client = test_client();
    let result = handle_tool_call(&client, "keyboard_press", json!({"key": "ctrl+c"})).await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_press_key_missing_key() {
    let client = test_client();
    let result = handle_tool_call(&client, "keyboard_press", json!({})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_get_clipboard() {
    let client = test_client();
    let result = handle_tool_call(&client, "clipboard_get", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_set_clipboard() {
    let client = test_client();
    let result = handle_tool_call(&client, "clipboard_set", json!({"text": "test"})).await;
    assert!(!result.is_error.unwrap_or(false));
    assert_eq!(
        result.content[0].text.as_deref().unwrap_or(""),
        "Clipboard set."
    );
}

#[tokio::test]
async fn test_gui_list_windows() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_list", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_get_active_window_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_active", json!({})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_focus_window_by_id() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_focus", json!({"window_id": 1})).await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_focus_window_no_args() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_focus", json!({})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_move_resize_window() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "window_move_resize",
        json!({"window_id": 1, "x": 10, "y": 20}),
    )
    .await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_window_action_minimize() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "window_action",
        json!({"window_id": 1, "action": "minimize"}),
    )
    .await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_window_action_bad_action() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "window_action",
        json!({"window_id": 1, "action": "explode"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_find_windows() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_find", json!({"query": "Firefox"})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("No windows"), "got: {text}");
}

#[tokio::test]
async fn test_gui_system_info() {
    let client = test_client();
    let result = handle_tool_call(&client, "system_info", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("screen"), "got: {text}");
}

#[tokio::test]
async fn test_gui_wait() {
    let client = test_client();
    let result = handle_tool_call(&client, "system_wait", json!({"ms": 1})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("1ms"), "got: {text}");
}

#[tokio::test]
async fn test_unknown_tool() {
    let client = test_client();
    let result = handle_tool_call(&client, "nonexistent_tool", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("unknown tool"), "got: {text}");
}

#[tokio::test]
async fn test_gui_click_with_right_button() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "mouse_click",
        json!({"x": 100, "y": 200, "button": "right"}),
    )
    .await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_screenshot_region_missing_width() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_capture_region", json!({"x": 0, "y": 0})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_double_click_missing_args() {
    let client = test_client();
    let result = handle_tool_call(&client, "mouse_double_click", json!({"x": 50})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_scroll_missing_direction() {
    let client = test_client();
    let result = handle_tool_call(&client, "mouse_scroll", json!({"x": 0, "y": 0})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_type_text_empty() {
    let client = test_client();
    let result = handle_tool_call(&client, "keyboard_type", json!({"text": ""})).await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_press_key_combo_multi_modifier() {
    let client = test_client();
    let result =
        handle_tool_call(&client, "keyboard_press", json!({"key": "ctrl+alt+delete"})).await;
    assert!(!result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn test_gui_move_resize_window_no_args() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_move_resize", json!({})).await;
    assert_eq!(result.is_error, Some(true));
}

#[tokio::test]
async fn test_gui_screenshot_content_format() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_capture", json!({})).await;

    // Should have at least a text item describing the image
    let has_text = result.content.iter().any(|c| c.content_type == "text");
    assert!(has_text, "expected text description");
}

// ═══════════════════════════════════════════════════════════════════════════
// Argument validation for destructive / under-tested tools
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_fs_read_missing_path_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "fs_read", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: path"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_fs_write_missing_path_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "fs_write", json!({"content": "aGVsbG8="})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: path"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_fs_write_missing_content_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "fs_write", json!({"path": "unused.txt"})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: content"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_fs_delete_missing_path_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "fs_delete", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: path"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_fs_create_dir_missing_path_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "fs_create_dir", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: path"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_process_kill_missing_pid_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "process_kill", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: pid"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_process_info_missing_pid_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "process_info", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: pid"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_process_start_missing_command_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "process_start", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: command"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_service_start_missing_name_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "service_start", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: name"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_service_stop_missing_name_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "service_stop", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: name"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_service_status_missing_name_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "service_status", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: name"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_shell_write_missing_session_id_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "shell_write", json!({"input": "ls"})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: session_id"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_shell_write_missing_input_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "shell_write", json!({"session_id": "s1"})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: input"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_shell_read_missing_session_id_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "shell_read", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: session_id"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_shell_close_missing_session_id_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "shell_close", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: session_id"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_shell_exec_missing_command_errors() {
    let client = test_client();
    // Validation happens before any PTY is spawned, so nothing executes.
    let result = handle_tool_call(&client, "shell_exec", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: command"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_window_focus_missing_id_and_title_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_focus", json!({"neither": true})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Provide window_id or title"), "got: {text}");
}

#[tokio::test]
async fn test_window_move_resize_missing_window_id_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_move_resize", json!({"x": 10, "y": 20})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: window_id"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_window_action_missing_window_id_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_action", json!({"action": "close"})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: window_id"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_window_action_missing_action_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "window_action", json!({"window_id": 1})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: action"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_mouse_drag_missing_to_y_errors() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "mouse_drag",
        json!({"from_x": 0, "from_y": 0, "to_x": 100}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: to_y"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_keyboard_type_missing_text_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "keyboard_type", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: text"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_clipboard_set_missing_text_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "clipboard_set", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("missing required argument: text"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_screen_find_text_missing_query_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_find_text", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");

    // Both legs are asserted so the test holds in a `--no-default-features`
    // build, where the handler never reaches argument validation: the tool is
    // declared but not compiled, and saying so is the correct answer.
    #[cfg(feature = "ocr")]
    assert!(
        text.contains("missing required argument: query"),
        "got: {text}"
    );

    #[cfg(not(feature = "ocr"))]
    assert!(text.contains("OCR not available"), "got: {text}");
}

#[tokio::test]
async fn test_no_such_tool_returns_error() {
    let client = test_client();
    let result = handle_tool_call(&client, "no_such_tool", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("unknown tool: no_such_tool"), "got: {text}");
}

// ═══════════════════════════════════════════════════════════════════════════
// Destructive tool failure paths — safe, nothing real is touched
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_fs_delete_nonexistent_file_errors() {
    let client = test_client();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does_not_exist.txt");
    let result = handle_tool_call(
        &client,
        "fs_delete",
        json!({"path": missing.to_str().unwrap()}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("File not found"), "got: {text}");
}

#[tokio::test]
async fn test_fs_delete_directory_errors() {
    let client = test_client();
    let dir = tempfile::tempdir().unwrap();
    let result = handle_tool_call(
        &client,
        "fs_delete",
        json!({"path": dir.path().to_str().unwrap()}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("is a directory"), "got: {text}");
    // Directory must be untouched
    assert!(dir.path().exists());
}

#[tokio::test]
async fn test_fs_read_nonexistent_file_errors() {
    let client = test_client();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.txt");
    let result = handle_tool_call(
        &client,
        "fs_read",
        json!({"path": missing.to_str().unwrap()}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("File not found"), "got: {text}");
}

#[tokio::test]
async fn test_process_kill_nonexistent_pid_errors() {
    let client = test_client();
    // 999999999 exceeds the max PID on Windows/Linux, so no real process
    // can be targeted — taskkill/kill fails and the handler reports the error.
    let result = handle_tool_call(&client, "process_kill", json!({"pid": 999999999})).await;
    assert_eq!(result.is_error, Some(true));
    assert!(!result.content.is_empty(), "expected an error message");
}

#[tokio::test]
async fn test_process_info_nonexistent_pid_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "process_info", json!({"pid": 999999999})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Process not found"), "got: {text}");
}

#[tokio::test]
async fn test_process_start_nonexistent_command_errors() {
    let client = test_client();
    // Spawn of a command that cannot exist fails — nothing is executed.
    let result = handle_tool_call(
        &client,
        "process_start",
        json!({"command": "sys_mcp_definitely_not_a_real_binary_xyz"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Failed to spawn"), "got: {text}");
}

// ─── Optional array arguments are validated, not absorbed (issue #40) ──────

#[tokio::test]
async fn test_process_start_rejects_non_array_args() {
    let client = test_client();
    // `args` given as a string instead of an array: the old code read
    // `as_array()` -> None -> `unwrap_or_default()` and silently started the
    // process with *no* arguments, then reported success. Rejected now, and
    // before any spawn (the command below cannot exist either, so nothing can
    // run even if the ordering regressed).
    let result = handle_tool_call(
        &client,
        "process_start",
        json!({"command": "sys_mcp_definitely_not_a_real_binary_xyz", "args": "--version"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("'args' must be an array of strings"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_process_start_rejects_non_string_elements() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "process_start",
        json!({"command": "sys_mcp_definitely_not_a_real_binary_xyz", "args": ["ok", 7]}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("'args' must contain only strings"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_mcp_register_rejects_non_array_args() {
    let client = test_client();
    // Registration only writes to the in-process hub, so a *valid* call here
    // cannot spawn anything; the invalid one must fail before that write.
    let invalid = handle_tool_call(
        &client,
        "mcp_register",
        json!({"name": "args-validation-probe", "command": "npx", "args": {"a": 1}}),
    )
    .await;
    assert_eq!(invalid.is_error, Some(true));
    let text = invalid.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("'args' must be an array of strings"),
        "got: {text}"
    );
    let info = handle_tool_call(&client, "mcp_list", json!({})).await;
    let listed = info.content[0].text.as_deref().unwrap_or("");
    assert!(
        !listed.contains("args-validation-probe"),
        "a rejected registration must not reach the hub: {listed}"
    );
}

// ─── system_logs argument handling (issue #41) ────────────────────────────

#[tokio::test]
async fn test_system_logs_rejects_unusable_count() {
    let client = test_client();
    for args in [
        json!({"count": 0}),
        json!({"count": 1001}),
        json!({"count": "50"}),
        json!({"count": -3}),
    ] {
        let result = handle_tool_call(&client, "system_logs", args.clone()).await;
        assert_eq!(result.is_error, Some(true), "{args} must be refused");
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(
            text.contains("'count'"),
            "{args} should be a count error, got: {text}"
        );
    }
}

#[tokio::test]
async fn test_system_logs_rejects_unknown_level_instead_of_defaulting_it() {
    let client = test_client();
    // The old code sent any unrecognized level to `Level=4`, so a request for
    // Critical came back as a list of Information events.
    let result = handle_tool_call(&client, "system_logs", json!({"level": "severe"})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("'level'"), "got: {text}");
    assert!(
        text.contains("omit 'level' to read them all"),
        "the error must teach the caller the default, got: {text}"
    );
}

#[tokio::test]
async fn test_system_logs_reads_the_application_log() {
    let client = test_client();
    // Answered by `StubSystem` now (issue #47): this used to shell out to
    // PowerShell and could only assert that *some* query succeeded. Against the
    // fixture every claim below is deterministic, and the machine-dependent part
    // moved to `real_os_windows_application_log_is_readable`.
    let result = handle_tool_call(
        &client,
        "system_logs",
        json!({"count": 5, "level": "Error", "log": "Application"}),
    )
    .await;
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert_eq!(result.is_error, None, "a valid query must not fail: {text}");
    let parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("system_logs must return a JSON array ({e}): {text}"));
    let entries = parsed
        .as_array()
        .unwrap_or_else(|| panic!("expected an array of log entries: {text}"));
    assert!(!entries.is_empty(), "expected entries, got: {text}");
    for entry in entries {
        let timestamp = entry["timestamp"].as_str().unwrap_or("");
        assert!(
            timestamp.contains('T') && timestamp.contains('+'),
            "timestamp must be ISO 8601, got {timestamp:?} in: {text}"
        );
        assert_eq!(
            entry["level"].as_str().unwrap_or(""),
            "Error",
            "the requested level must be the level reported: {text}"
        );
        assert!(
            !entry["source"].as_str().unwrap_or("").is_empty(),
            "ProviderName must be populated: {text}"
        );
    }
    // The filter reached the probe rather than being dropped in the handler.
    assert!(
        StubSystem::shared().log_requests().contains(&(
            5,
            Some("Error".to_string()),
            Some("Application".to_string())
        )),
        "the probe never saw this request"
    );
}

#[tokio::test]
async fn test_system_logs_without_a_level_is_not_an_information_only_view() {
    let client = test_client();
    // The default used to be `Level=4`: a bare call returned informational
    // events and hid every error on the machine. The fixture carries a mixed
    // set, so the bare call must come back with the Error entry in it — the
    // claim this test was always making, now provable instead of tolerated.
    let result = handle_tool_call(&client, "system_logs", json!({"count": 5})).await;
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert_eq!(result.is_error, None, "a bare query must not fail: {text}");
    let parsed: serde_json::Value =
        serde_json::from_str(text).expect("expected a JSON array of entries");
    let entries = parsed.as_array().expect("expected an array");
    assert!(
        entries
            .iter()
            .any(|entry| entry["level"].as_str() == Some("Error")),
        "a bare read must include severities above Information: {text}"
    );
}

#[tokio::test]
async fn test_system_logs_refuses_a_log_name_that_could_escape_the_filter() {
    let client = test_client();
    // No longer `cfg(windows)`: `StubSystem` enforces the same name rule as the
    // PowerShell path (issue #47), so the injection boundary is guarded on every
    // platform instead of only where the real query can run.
    let result = handle_tool_call(
        &client,
        "system_logs",
        json!({"log": "Application'; Get-Process; #"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("not a usable Windows event log name"),
        "got: {text}"
    );
}

#[cfg(not(windows))]
#[tokio::test]
#[ignore = "real-OS probe (issue #47): reads the machine's journal / unified log. Run: cargo test --all-targets -- --ignored real_os"]
async fn test_system_logs_names_the_platform_it_is_on_when_log_is_unsupported() {
    let client = test_client();
    // `log` is a Windows concept; ignoring it would return the platform default
    // and read as if the caller's log choice had been honored.
    let result = handle_tool_call(&client, "system_logs", json!({"log": "Application"})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Windows event log"), "got: {text}");
}

#[tokio::test]
async fn test_service_start_nonexistent_service_errors() {
    let client = test_client();
    // A service with this name cannot exist, so `sc start`/`systemctl start`
    // fails without starting anything.
    let result = handle_tool_call(
        &client,
        "service_start",
        json!({"name": "sys_mcp_nonexistent_service_xyz"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    assert!(!result.content.is_empty(), "expected an error message");
}

#[tokio::test]
async fn test_service_stop_nonexistent_service_errors() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "service_stop",
        json!({"name": "sys_mcp_nonexistent_service_xyz"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    assert!(!result.content.is_empty(), "expected an error message");
}

#[tokio::test]
async fn test_service_status_nonexistent_service_errors() {
    let client = test_client();
    // Read-only: lists services then reports "not found".
    let result = handle_tool_call(
        &client,
        "service_status",
        json!({"name": "sys_mcp_nonexistent_service_xyz"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Service not found"), "got: {text}");
}

#[tokio::test]
async fn test_shell_write_unknown_session_errors() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "shell_write",
        json!({"session_id": "no-such-session", "input": "ls"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Session not found"), "got: {text}");
}

#[tokio::test]
async fn test_shell_read_unknown_session_errors() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "shell_read",
        json!({"session_id": "no-such-session"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Session not found"), "got: {text}");
}

#[tokio::test]
async fn test_shell_close_unknown_session_errors() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "shell_close",
        json!({"session_id": "no-such-session"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("not found"), "got: {text}");
}

// ═══════════════════════════════════════════════════════════════════════════
// Safe success paths (read-only ops, or writes confined to a temp dir)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_fs_write_read_roundtrip_tempfile() {
    let client = test_client();
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("roundtrip.txt");
    let path = file_path.to_str().unwrap();

    // fs_write expects base64-encoded content
    let write = handle_tool_call(
        &client,
        "fs_write",
        json!({"path": path, "content": "aGVsbG8="}),
    )
    .await;
    assert!(!write.is_error.unwrap_or(false));
    let text = write.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Written to"), "got: {text}");
    assert_eq!(fs::read(&file_path).unwrap(), b"hello");

    let read = handle_tool_call(&client, "fs_read", json!({"path": path})).await;
    assert!(!read.is_error.unwrap_or(false));
    let text = read.content[0].text.as_deref().unwrap_or("");
    assert_eq!(text, "aGVsbG8=", "fs_read should return base64 content");
}

#[tokio::test]
async fn test_fs_list_tempdir_success() {
    let client = test_client();
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("probe.txt"), b"x").unwrap();

    let result = handle_tool_call(
        &client,
        "fs_list",
        json!({"path": dir.path().to_str().unwrap()}),
    )
    .await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("probe.txt"), "got: {text}");
    assert!(text.contains("\"file\""), "got: {text}");
}

#[tokio::test]
async fn test_fs_exists_reports_true_and_false() {
    let client = test_client();
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("present.txt");
    fs::write(&file_path, b"x").unwrap();

    let present = handle_tool_call(
        &client,
        "fs_exists",
        json!({"path": file_path.to_str().unwrap()}),
    )
    .await;
    assert!(!present.is_error.unwrap_or(false));
    let text = present.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("\"exists\":true"), "got: {text}");
    assert!(text.contains("\"file\""), "got: {text}");

    let missing = dir.path().join("absent.txt");
    let absent = handle_tool_call(
        &client,
        "fs_exists",
        json!({"path": missing.to_str().unwrap()}),
    )
    .await;
    assert!(!absent.is_error.unwrap_or(false));
    let text = absent.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("\"exists\":false"), "got: {text}");
    assert!(text.contains("\"none\""), "got: {text}");
}

#[tokio::test]
async fn test_fs_create_dir_and_delete_in_tempdir_success() {
    let client = test_client();
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("nested").join("child");

    let create = handle_tool_call(
        &client,
        "fs_create_dir",
        json!({"path": sub.to_str().unwrap()}),
    )
    .await;
    assert!(!create.is_error.unwrap_or(false));
    assert!(sub.is_dir(), "directory should have been created");

    // Delete a file (not the directory) to exercise the success path safely
    let file_path = sub.join("victim.txt");
    fs::write(&file_path, b"bye").unwrap();
    let delete = handle_tool_call(
        &client,
        "fs_delete",
        json!({"path": file_path.to_str().unwrap()}),
    )
    .await;
    assert!(!delete.is_error.unwrap_or(false));
    let text = delete.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Deleted"), "got: {text}");
    assert!(!file_path.exists(), "file should have been deleted");
    assert!(sub.is_dir(), "parent directory must not be deleted");
}

#[tokio::test]
async fn test_shell_list_returns_session_list() {
    let client = test_client();
    // No sessions are ever opened in tests, so this must come back empty.
    let result = handle_tool_call(&client, "shell_list", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    let sessions: serde_json::Value =
        serde_json::from_str(text).expect("shell_list should return a JSON array");
    assert!(sessions.as_array().is_some(), "got: {text}");
}

#[tokio::test]
async fn test_process_list_returns_results() {
    let client = test_client();
    // Read-only: shells out to powershell/ps to enumerate processes.
    let result = handle_tool_call(&client, "process_list", json!({"limit": 5})).await;
    assert!(!result.is_error.unwrap_or(false));
    assert!(!result.content.is_empty(), "expected process list content");
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.starts_with('['), "expected a JSON array, got: {text}");
}

#[tokio::test]
async fn test_service_list_returns_results() {
    let client = test_client();
    // Read-only: shells out to powershell/systemctl to enumerate services.
    let result = handle_tool_call(&client, "service_list", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    assert!(!result.content.is_empty(), "expected service list content");
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.starts_with('['), "expected a JSON array, got: {text}");
}

#[tokio::test]
async fn test_network_info_returns_results() {
    let client = test_client();
    // Read-only: queries interface configuration via powershell.
    let result = handle_tool_call(&client, "network_info", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    assert!(!result.content.is_empty(), "expected network info content");
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("hostname"), "got: {text}");
}

#[tokio::test]
async fn test_network_connections_no_required_args() {
    let client = test_client();
    // ACTUAL behavior: network_connections has no required arguments
    // (limit is optional, defaulting to 50) — it runs a read-only netstat.
    let result = handle_tool_call(&client, "network_connections", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    assert!(
        !result.content.is_empty(),
        "expected connection list content"
    );
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.starts_with('['), "expected a JSON array, got: {text}");
}

// ═══════════════════════════════════════════════════════════════════════════
// system_wait boundary / clamp behavior
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test(start_paused = true)]
async fn test_system_wait_missing_ms_defaults_to_500() {
    let client = test_client();
    // Paused clock: the default 500ms sleep completes instantly.
    let result = handle_tool_call(&client, "system_wait", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert_eq!(text, "Waited 500ms.");
}

#[tokio::test(start_paused = true)]
async fn test_system_wait_over_cap_clamped_to_30000ms() {
    let client = test_client();
    // handler does `ms.min(30000)` before sleeping and reports the clamped
    // value. Paused clock makes the 30s sleep complete instantly.
    let result = handle_tool_call(&client, "system_wait", json!({"ms": 99999})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert_eq!(text, "Waited 30000ms.");
}

#[tokio::test(start_paused = true)]
async fn test_system_wait_ignores_non_numeric_ms() {
    let client = test_client();
    // Non-numeric ms falls back to the 500ms default rather than erroring.
    let result = handle_tool_call(&client, "system_wait", json!({"ms": "soon"})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert_eq!(text, "Waited 500ms.");
}

// ── Template matching (vision_find_image / vision_wait_for_image) ─────────

/// Encode a small solid-color PNG as base64 — valid ENCODED image bytes,
/// which is what both tools require (raw pixel data is rejected).
fn tiny_png_b64() -> String {
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([200u8, 40, 40, 255]));
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buf.into_inner())
}

#[test]
fn test_vision_tools_registered_in_tools_list() {
    let tools = sys_mcp::mcp::tools::all_tools();
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"vision_find_image"), "got: {names:?}");
    assert!(names.contains(&"vision_wait_for_image"), "got: {names:?}");
}

#[tokio::test]
async fn test_vision_find_image_missing_template() {
    let client = test_client();
    let result = handle_tool_call(&client, "vision_find_image", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("template_base64 or template_path"),
        "got: {text}"
    );
}

#[tokio::test]
async fn test_vision_find_image_template_path_not_found() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "vision_find_image",
        json!({"template_path": "Z:\\does\\not\\exist.png"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("failed to read template_path"), "got: {text}");
}

#[tokio::test]
async fn test_vision_find_image_invalid_base64() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "vision_find_image",
        json!({"template_base64": "!!!not-base64!!!"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("not valid base64"), "got: {text}");
}

#[tokio::test]
async fn test_vision_find_image_rejects_raw_pixels() {
    let client = test_client();
    // Base64 that decodes fine but whose bytes are NOT an encoded image —
    // e.g. an agent base64-ing raw RGBA pixel data instead of a PNG.
    let raw_pixels =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, vec![7u8; 64]);
    let result = handle_tool_call(
        &client,
        "vision_find_image",
        json!({"template_base64": raw_pixels}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("not a decodable image"), "got: {text}");
}

#[tokio::test]
async fn test_vision_find_image_rejects_both_sources() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "vision_find_image",
        json!({"template_base64": tiny_png_b64(), "template_path": "C:\\x.png"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("not both"), "got: {text}");
}

#[tokio::test]
async fn test_vision_find_image_precision_out_of_range() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "vision_find_image",
        json!({"template_base64": tiny_png_b64(), "precision": 1.5}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("precision"), "got: {text}");
}

#[tokio::test]
async fn test_vision_find_image_precision_non_numeric() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "vision_find_image",
        json!({"template_base64": tiny_png_b64(), "precision": "high"}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("precision"), "got: {text}");
}

#[tokio::test]
async fn test_vision_find_image_partial_region_rejected() {
    let client = test_client();
    // x without y/width/height — region args are all-or-nothing.
    let result = handle_tool_call(
        &client,
        "vision_find_image",
        json!({"template_base64": tiny_png_b64(), "x": 10}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument"), "got: {text}");
}

#[tokio::test]
async fn test_vision_find_image_unavailable_on_stub_backend() {
    let client = test_client();
    // StubBackend has no VisionCapability — valid input must produce the
    // clear "not available in this build" error. It must NOT be flagged as
    // a permanent error: local-GUI builds do provide template matching.
    let result = handle_tool_call(
        &client,
        "vision_find_image",
        json!({"template_base64": tiny_png_b64()}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("not available in this build"), "got: {text}");
    assert!(!text.contains("permanent error"), "got: {text}");
}

#[tokio::test]
async fn test_vision_wait_for_image_unavailable_on_stub_backend() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "vision_wait_for_image",
        json!({"template_base64": tiny_png_b64()}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("not available in this build"), "got: {text}");
}

#[tokio::test]
async fn test_vision_wait_for_image_timeout_too_large() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "vision_wait_for_image",
        json!({"template_base64": tiny_png_b64(), "timeout_ms": 999_999}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("timeout_ms must be <="), "got: {text}");
}

#[tokio::test]
async fn test_vision_wait_for_image_negative_timeout() {
    let client = test_client();
    let result = handle_tool_call(
        &client,
        "vision_wait_for_image",
        json!({"template_base64": tiny_png_b64(), "timeout_ms": -5}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("non-negative integer"), "got: {text}");
}

// ═══════════════════════════════════════════════════════════════════════════
// screen_read argument validation
//
// These only exercise arguments that must be refused, and `screen_read` checks
// them before it captures a screen or loads an OCR model — so no test here can
// reach the model download. That ordering is part of the contract.
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "ocr")]
#[tokio::test]
async fn test_screen_read_rejects_half_a_region() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_read", json!({ "x": 10, "y": 20 })).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("all four of x, y, width and height"),
        "got: {text}"
    );
    assert!(text.contains("missing: width, height"), "got: {text}");
}

#[cfg(feature = "ocr")]
#[tokio::test]
async fn test_screen_read_rejects_unknown_detail_and_zero_sized_region() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_read", json!({ "detail": "txt" })).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("full, lines, text"), "got: {text}");

    let result = handle_tool_call(
        &client,
        "screen_read",
        json!({ "x": 0, "y": 0, "width": 0, "height": 50 }),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("positive width and height"), "got: {text}");
}

// ═══════════════════════════════════════════════════════════════════════════
// Accessibility tools — issue #43
//
// Driven through `StubA11yBackend`, which serves a fixed tree and the id-cache
// contract the platform backends implement. The fixture's depth-first ids are:
//   stub:0 window "Stub Window"
//   stub:1 menu bar "Menu"        stub:2 menu item "File"
//   stub:3 checkbox "Trial Balance"
//   stub:4 edit "Voucher No"
//   stub:5 separator (no name)
//   stub:6 button "OK (F2)"
//   stub:7 table "Day Book"       stub:8 text "Row 1"
// ═══════════════════════════════════════════════════════════════════════════

fn a11y_client() -> (GuiClient, StubA11yRecorder) {
    let (backend, recorder) = StubA11yBackend::new();
    (GuiClient::new(backend), recorder)
}

/// The tool response body as JSON. `accessibility_tree` / `accessibility_properties`
/// return a bare document; `accessibility_find` prefixes a human summary line,
/// so the JSON starts at the first newline for those.
fn tool_json(result: &sys_mcp::protocol::mcp::ToolResult) -> Value {
    let text = result.content[0].text.as_deref().unwrap_or("");
    let body = match text.find('\n') {
        Some(pos) if text.starts_with("Found ") => &text[pos + 1..],
        _ => text,
    };
    serde_json::from_str(body).unwrap_or_else(|e| panic!("not JSON ({e}): {text}"))
}

/// Every id in the tree, depth-first, in the order the walk minted them.
fn all_ids(node: &Value) -> Vec<String> {
    let mut ids = vec![node
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()];
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            ids.extend(all_ids(child));
        }
    }
    ids
}

/// First node whose name matches, searched depth-first.
fn find_by_name<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    if node.get("name").and_then(Value::as_str) == Some(name) {
        return Some(node);
    }
    node.get("children")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|c| find_by_name(c, name))
}

#[tokio::test]
async fn test_accessibility_tools_fail_as_unsupported_without_a_backend() {
    // The premise of #43, kept pinned: the plain stub has no accessibility
    // implementation, so the capability error path is the only reachable one.
    // Arguments are filled in because the handlers validate them first — with
    // empty arguments this would report a missing argument instead.
    let client = test_client();
    let args = json!({ "element_id": "stub:0", "action": "invoke", "query": "any" });
    for name in [
        "accessibility_tree",
        "accessibility_find",
        "accessibility_properties",
        "accessibility_action",
    ] {
        let result = handle_tool_call(&client, name, args.clone()).await;
        assert_eq!(result.is_error, Some(true), "'{name}' must fail here");
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(
            text.contains("capability not supported: accessibility"),
            "'{name}' => {text}"
        );
    }
}

#[tokio::test]
async fn test_accessibility_tree_returns_the_fixture_with_resolvable_ids() {
    let (client, _recorder) = a11y_client();
    let result = handle_tool_call(&client, "accessibility_tree", json!({})).await;
    assert_eq!(result.is_error, None, "got: {:?}", result.content);
    let tree = tool_json(&result);

    assert_eq!(tree["id"], json!("stub:0"));
    assert_eq!(tree["role"], json!("window"));
    assert_eq!(tree["name"], json!("Stub Window"));
    assert_eq!(tree["children"].as_array().unwrap().len(), 6);

    // Ids are minted depth-first and handed to the caller, which is the only
    // reason a later `accessibility_properties` call can resolve them.
    let expected: Vec<String> = (0..9).map(|i| format!("stub:{i}")).collect();
    assert_eq!(all_ids(&tree), expected);

    let button = find_by_name(&tree, "OK (F2)").expect("button in tree");
    assert_eq!(button["role"], json!("button"));
    assert_eq!(button["cx"], json!(680), "640 + 80/2");
    assert_eq!(button["cy"], json!(572), "560 + 24/2");
    assert!(button["actions"]
        .as_array()
        .unwrap()
        .contains(&json!("invoke")));

    let checkbox = find_by_name(&tree, "Trial Balance").expect("checkbox in tree");
    assert_eq!(checkbox["value"], json!("1"));
    assert!(checkbox["states"]
        .as_array()
        .unwrap()
        .contains(&json!("checked")));

    let edit = find_by_name(&tree, "Voucher No").expect("edit in tree");
    assert_eq!(edit["value"], json!("12060623"));
    assert!(edit["description"]
        .as_str()
        .unwrap_or("")
        .starts_with("Voucher number"));

    // An element with no name must serialize the key as null rather than
    // dropping it, or a caller cannot tell "unnamed" from "not returned".
    let separator = find_by_name(&tree, "Nonexistent Name");
    assert!(separator.is_none());
    let separator = &tree["children"][3];
    assert_eq!(separator["role"], json!("separator"));
    assert_eq!(separator["name"], json!(null));
}

#[tokio::test]
async fn test_accessibility_tree_max_depth_trims_children_and_the_id_space() {
    let (client, _recorder) = a11y_client();

    let result = handle_tool_call(&client, "accessibility_tree", json!({ "max_depth": 0 })).await;
    let root = tool_json(&result);
    assert_eq!(root["id"], json!("stub:0"));
    assert_eq!(root["children"].as_array().unwrap().len(), 0);

    let result = handle_tool_call(&client, "accessibility_tree", json!({ "max_depth": 1 })).await;
    let tree = tool_json(&result);
    assert_eq!(tree["children"].as_array().unwrap().len(), 6);
    // Depth 1 reaches the window's own children but no grandchildren: the menu
    // item under "Menu" and the text under "Day Book" are not walked, so they
    // are not issued ids either.
    let expected: Vec<String> = (0..7).map(|i| format!("stub:{i}")).collect();
    assert_eq!(all_ids(&tree), expected);

    // `max_depth` reaching the backend at all is what this asserts: with the
    // default (no argument) the same fixture yields nine ids, not seven.
    let result = handle_tool_call(&client, "accessibility_tree", json!({})).await;
    assert_eq!(all_ids(&tool_json(&result)).len(), 9);
}

#[tokio::test]
async fn test_accessibility_find_matches_name_and_role_separately() {
    let (client, _recorder) = a11y_client();

    let result = handle_tool_call(&client, "accessibility_find", json!({ "query": "trial" })).await;
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.starts_with("Found 1 element(s)"), "got: {text}");
    let matches = tool_json(&result);
    assert_eq!(matches[0]["name"], json!("Trial Balance"));
    assert_eq!(matches[0]["role"], json!("checkbox"));

    // A `role` filter reads the role only, so it finds the unnamed separator
    // that the name query above could never match.
    let result = handle_tool_call(
        &client,
        "accessibility_find",
        json!({ "role": "separator" }),
    )
    .await;
    let matches = tool_json(&result);
    assert_eq!(matches.as_array().unwrap().len(), 1);
    assert_eq!(matches[0]["name"], json!(null));

    // And the reverse: a name query must not match on role text.
    let result = handle_tool_call(
        &client,
        "accessibility_find",
        json!({ "query": "separator" }),
    )
    .await;
    assert_eq!(
        result.content[0].text.as_deref().unwrap_or(""),
        "No matching UI elements found."
    );

    // Substring, case-insensitive, on both fields at once.
    let result = handle_tool_call(
        &client,
        "accessibility_find",
        json!({ "query": "OK (", "role": "BUTTON" }),
    )
    .await;
    assert_eq!(matches_count(&result), 1);

    // The ids a find returned belong to the walk it just ran, so they resolve.
    let id = tool_json(&result)[0]["id"].as_str().unwrap().to_string();
    let result = handle_tool_call(
        &client,
        "accessibility_properties",
        json!({ "element_id": id }),
    )
    .await;
    assert_eq!(result.is_error, None, "id {id} should still resolve");
    assert_eq!(tool_json(&result)["name"], json!("OK (F2)"));
}

fn matches_count(result: &sys_mcp::protocol::mcp::ToolResult) -> usize {
    if result.content[0]
        .text
        .as_deref()
        .unwrap_or("")
        .starts_with("No matching")
    {
        return 0;
    }
    tool_json(result).as_array().unwrap().len()
}

#[tokio::test]
async fn test_accessibility_properties_returns_one_element_without_its_subtree() {
    let (client, _recorder) = a11y_client();
    let _ = handle_tool_call(&client, "accessibility_tree", json!({})).await;

    // "Menu" is the menu bar; in the full walk it carries the "File" item.
    let result = handle_tool_call(
        &client,
        "accessibility_properties",
        json!({ "element_id": "stub:1" }),
    )
    .await;
    assert_eq!(result.is_error, None);
    let node = tool_json(&result);
    assert_eq!(
        node["id"],
        json!("stub:1"),
        "the id must come back unchanged"
    );
    assert_eq!(node["name"], json!("Menu"));
    assert_eq!(
        node["children"].as_array().unwrap().len(),
        0,
        "properties must not re-walk the subtree (issue #39)"
    );
}

#[tokio::test]
async fn test_accessibility_properties_requires_an_element_id() {
    let (client, _recorder) = a11y_client();
    let result = handle_tool_call(&client, "accessibility_properties", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(
        result.content[0].text.as_deref().unwrap_or(""),
        "missing required argument: element_id"
    );

    let result = handle_tool_call(
        &client,
        "accessibility_properties",
        json!({ "element_id": 3 }),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(
        result.content[0].text.as_deref().unwrap_or(""),
        "missing required argument: element_id"
    );
}

#[tokio::test]
async fn test_accessibility_ids_are_invalidated_by_the_next_walk() {
    // The contract the tools advertise: an id resolves only until the next tree
    // walk. It used to be untestable because no stub served a tree at all.
    let (client, _recorder) = a11y_client();
    let _ = handle_tool_call(&client, "accessibility_tree", json!({})).await;
    let deep = handle_tool_call(
        &client,
        "accessibility_properties",
        json!({ "element_id": "stub:8" }),
    )
    .await;
    assert_eq!(deep.is_error, None, "stub:8 exists after the full walk");

    // A shallower walk re-mints the id space and never reaches the ninth node.
    let _ = handle_tool_call(&client, "accessibility_tree", json!({ "max_depth": 1 })).await;
    let stale = handle_tool_call(
        &client,
        "accessibility_properties",
        json!({ "element_id": "stub:8" }),
    )
    .await;
    assert_eq!(stale.is_error, Some(true));
    let text = stale.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("not in the current accessibility cache"),
        "{text}"
    );
    assert!(
        text.contains("call accessibility_tree or accessibility_find again"),
        "the error must say how to get a usable id: {text}"
    );

    // An id that was never issued fails the same way, and a fresh find walks the
    // whole tree again, so the deep id is live once more.
    let never = handle_tool_call(
        &client,
        "accessibility_properties",
        json!({ "element_id": "stub:99" }),
    )
    .await;
    assert_eq!(never.is_error, Some(true));
    let _ = handle_tool_call(&client, "accessibility_find", json!({ "query": "Row" })).await;
    let again = handle_tool_call(
        &client,
        "accessibility_properties",
        json!({ "element_id": "stub:8" }),
    )
    .await;
    assert_eq!(again.is_error, None, "a new walk re-issues the ids");
}

#[tokio::test]
async fn test_accessibility_action_reaches_the_backend_with_the_requested_verb() {
    let (client, recorder) = a11y_client();
    let _ = handle_tool_call(&client, "accessibility_tree", json!({})).await;

    let result = handle_tool_call(
        &client,
        "accessibility_action",
        json!({ "element_id": "stub:6", "action": "Invoke" }),
    )
    .await;
    assert_eq!(result.is_error, None, "got: {:?}", result.content);
    assert_eq!(
        result.content[0].text.as_deref().unwrap_or(""),
        "Invoked 'Invoke' on element stub:6."
    );

    // The response text alone cannot prove the backend was called — a handler
    // can echo an argument it never used. The recorder is the other side.
    assert_eq!(
        recorder.invocations(),
        vec![("stub:6".to_string(), "Invoke".to_string())]
    );

    let _ = handle_tool_call(
        &client,
        "accessibility_action",
        json!({ "element_id": "stub:3", "action": "toggle" }),
    )
    .await;
    assert_eq!(
        recorder.invocations(),
        vec![
            ("stub:6".to_string(), "Invoke".to_string()),
            ("stub:3".to_string(), "toggle".to_string()),
        ],
        "the verb is passed through unchanged; alias normalization is platform code"
    );
}

#[tokio::test]
async fn test_accessibility_action_records_nothing_when_the_call_is_refused() {
    let (client, recorder) = a11y_client();
    let _ = handle_tool_call(&client, "accessibility_tree", json!({})).await;

    let result = handle_tool_call(
        &client,
        "accessibility_action",
        json!({ "element_id": "stub:6" }),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(
        result.content[0].text.as_deref().unwrap_or(""),
        "missing required argument: action"
    );

    let result = handle_tool_call(
        &client,
        "accessibility_action",
        json!({ "action": "invoke" }),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(
        result.content[0].text.as_deref().unwrap_or(""),
        "missing required argument: element_id"
    );

    // A live element id plus an unknown verb does reach the backend, but an
    // unresolvable id must not be invoked at all.
    let result = handle_tool_call(
        &client,
        "accessibility_action",
        json!({ "element_id": "stub:42", "action": "invoke" }),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    assert!(
        result.content[0]
            .text
            .as_deref()
            .unwrap_or("")
            .contains("not in the current accessibility cache"),
        "got: {:?}",
        result.content
    );
    assert!(
        recorder.invocations().is_empty(),
        "a refused action must not have reached the backend: {:?}",
        recorder.invocations()
    );
}
