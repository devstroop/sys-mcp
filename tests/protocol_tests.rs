use serde_json::{json, Value};
use sys_mcp::protocol::mcp::{ContentItem, McpRequest, McpResponse, ToolResult};

#[test]
fn test_mcp_request_deserialize() {
    let raw = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    let req: McpRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.id, Some(Value::Number(1.into())));
    assert_eq!(req.method, "tools/list");
    assert!(req.params.is_some());
}

#[test]
fn test_mcp_request_deserialize_no_params() {
    let raw = r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#;
    let req: McpRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.method, "ping");
    assert!(req.params.is_none());
}

#[test]
fn test_mcp_response_success() {
    let resp = McpResponse::success(Some(json!(1)), json!({"tools": []}));
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);
    assert_eq!(json["result"]["tools"], json!([]));
    assert!(json.get("error").is_none());
}

#[test]
fn test_mcp_response_error() {
    let resp = McpResponse::error(Some(json!(1)), -32601, "method not found");
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);
    assert_eq!(json["error"]["code"], -32601);
    assert_eq!(json["error"]["message"], "method not found");
    assert!(json.get("result").is_none());
}

#[test]
fn test_mcp_response_null_id() {
    let resp = McpResponse::error(None, -32700, "parse error");
    let json = serde_json::to_value(&resp).unwrap();
    assert!(json.get("id").unwrap().is_null());
}

#[test]
fn test_content_item_text() {
    let item = ContentItem::text("hello");
    assert_eq!(item.content_type, "text");
    assert_eq!(item.text.as_deref(), Some("hello"));
    assert!(item.mime_type.is_none());
    assert!(item.data.is_none());
}

#[test]
fn test_content_item_image_base64() {
    let item = ContentItem::image_base64("image/jpeg", "abc123");
    assert_eq!(item.content_type, "image");
    assert_eq!(item.mime_type.as_deref(), Some("image/jpeg"));
    assert_eq!(item.data.as_deref(), Some("abc123"));
}

#[test]
fn test_tool_result_text() {
    let result = ToolResult::text("hello");
    assert_eq!(result.content.len(), 1);
    assert_eq!(result.content[0].content_type, "text");
    assert_eq!(result.content[0].text.as_deref(), Some("hello"));
    assert_eq!(result.is_error, None);
}

#[test]
fn test_tool_result_image() {
    let result = ToolResult::image("image/png", "base64data");
    let item = &result.content[0];
    assert_eq!(item.content_type, "image");
    assert_eq!(item.mime_type.as_deref(), Some("image/png"));
    assert_eq!(item.data.as_deref(), Some("base64data"));
}

#[test]
fn test_tool_result_error() {
    let result = ToolResult::error("something went wrong");
    assert_eq!(result.is_error, Some(true));
    assert_eq!(
        result.content[0].text.as_deref(),
        Some("something went wrong")
    );
}

#[test]
fn test_mcp_response_serialize_roundtrip() {
    let resp = McpResponse::success(Some(json!(42)), json!({"status": "ok"}));
    let json_str = serde_json::to_string(&resp).unwrap();
    let parsed: Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["id"], 42);
    assert_eq!(parsed["result"]["status"], "ok");
}

#[test]
fn test_tool_result_serialize_has_is_error_false_when_none() {
    let result = ToolResult::text("ok");
    let json = serde_json::to_value(&result).unwrap();
    // is_error should be absent (skip_serializing_if) when None
    assert!(
        json.get("is_error").is_none(),
        "is_error should be skipped when None"
    );
}

#[test]
fn test_tool_result_serialize_has_is_error_true_when_error() {
    let result = ToolResult::error("fail");
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["is_error"], true);
}

#[test]
fn test_mcp_request_with_string_id() {
    let raw = r#"{"jsonrpc":"2.0","id":"req-1","method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#;
    let req: McpRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.id, Some(Value::String("req-1".to_string())));
}

#[test]
fn test_mcp_response_with_string_id_roundtrip() {
    let resp = McpResponse::success(Some(json!("abc")), json!({}));
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["id"], "abc");
}
