use serde_json::json;
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
