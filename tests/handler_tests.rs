use serde_json::json;
use std::fs;
use sys_mcp::gui::GuiClient;
use sys_mcp::mcp::handlers::handle_tool_call;
use sys_mcp::StubBackend;

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
    assert!(text.contains("missing required argument: path"), "got: {text}");
}

#[tokio::test]
async fn test_fs_write_missing_path_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "fs_write", json!({"content": "aGVsbG8="})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: path"), "got: {text}");
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
    assert!(text.contains("missing required argument: path"), "got: {text}");
}

#[tokio::test]
async fn test_fs_create_dir_missing_path_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "fs_create_dir", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: path"), "got: {text}");
}

#[tokio::test]
async fn test_process_kill_missing_pid_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "process_kill", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: pid"), "got: {text}");
}

#[tokio::test]
async fn test_process_info_missing_pid_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "process_info", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: pid"), "got: {text}");
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
    assert!(text.contains("missing required argument: name"), "got: {text}");
}

#[tokio::test]
async fn test_service_stop_missing_name_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "service_stop", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: name"), "got: {text}");
}

#[tokio::test]
async fn test_service_status_missing_name_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "service_status", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: name"), "got: {text}");
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
    assert!(
        text.contains("Provide window_id or title"),
        "got: {text}"
    );
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
    assert!(text.contains("missing required argument: to_y"), "got: {text}");
}

#[tokio::test]
async fn test_keyboard_type_missing_text_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "keyboard_type", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: text"), "got: {text}");
}

#[tokio::test]
async fn test_clipboard_set_missing_text_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "clipboard_set", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: text"), "got: {text}");
}

#[tokio::test]
async fn test_screen_find_text_missing_query_errors() {
    let client = test_client();
    let result = handle_tool_call(&client, "screen_find_text", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("missing required argument: query"), "got: {text}");
}

#[tokio::test]
async fn test_no_such_tool_returns_error() {
    let client = test_client();
    let result = handle_tool_call(&client, "no_such_tool", json!({})).await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("unknown tool: no_such_tool"),
        "got: {text}"
    );
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
    let write = handle_tool_call(&client, "fs_write", json!({"path": path, "content": "aGVsbG8="}))
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

    let result = handle_tool_call(&client, "fs_list", json!({"path": dir.path().to_str().unwrap()}))
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

    let create = handle_tool_call(&client, "fs_create_dir", json!({"path": sub.to_str().unwrap()}))
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
    assert!(!result.content.is_empty(), "expected connection list content");
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
