//! Tests for the MCP resources and prompts capabilities (ISSUES #15).
//!
//! Drives the real request handler so the capabilities advertised in
//! `initialize` are checked against what the server actually serves.

use serde_json::json;
use sys_mcp::gui::GuiClient;
use sys_mcp::mcp::server::McpRequestHandler;
use sys_mcp::protocol::mcp::McpRequest;
use sys_mcp::StubBackend;

fn handler() -> McpRequestHandler {
    McpRequestHandler::new(std::sync::Arc::new(GuiClient::new(StubBackend)))
}

fn req(method: &str, params: serde_json::Value) -> McpRequest {
    McpRequest {
        jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params: Some(params),
        id: Some(json!("test-1")),
    }
}

fn body(resp: &sys_mcp::protocol::mcp::McpResponse) -> serde_json::Value {
    serde_json::to_value(resp).expect("serialize")
}

#[tokio::test]
async fn initialize_advertises_resources_and_prompts() {
    let mut h = handler();
    let resp = h.handle_request(&req("initialize", json!({}))).await;
    let caps = body(&resp)["result"]["capabilities"].clone();

    assert!(caps.get("tools").is_some(), "tools capability missing");
    assert!(
        caps.get("resources").is_some(),
        "resources capability must be advertised now that the methods are implemented"
    );
    assert!(
        caps.get("prompts").is_some(),
        "prompts capability must be advertised now that the methods are implemented"
    );
}

#[tokio::test]
async fn resources_list_matches_the_catalogue() {
    let mut h = handler();
    let resp = h.handle_request(&req("resources/list", json!({}))).await;
    let resources = body(&resp)["result"]["resources"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    assert_eq!(resources.len(), 5, "expected the five documented resources");
    for r in &resources {
        assert!(r.get("uri").and_then(|v| v.as_str()).is_some(), "{r}");
        assert!(r.get("name").and_then(|v| v.as_str()).is_some(), "{r}");
        assert_eq!(
            r.get("mimeType").and_then(|v| v.as_str()),
            Some("application/json"),
            "{r}"
        );
    }
}

#[tokio::test]
async fn resources_read_returns_an_mcp_contents_envelope() {
    let mut h = handler();
    let resp = h
        .handle_request(&req(
            "resources/read",
            json!({ "uri": "sys-mcp://screen/size" }),
        ))
        .await;

    let contents = body(&resp)["result"]["contents"][0].clone();
    assert_eq!(
        contents.get("uri").and_then(|v| v.as_str()),
        Some("sys-mcp://screen/size")
    );
    // The payload must be actual JSON, not a stringified error.
    let text = contents.get("text").and_then(|v| v.as_str()).expect("text");
    let parsed: serde_json::Value = serde_json::from_str(text).expect("valid JSON payload");
    assert!(parsed.get("width").is_some(), "payload: {parsed}");
}

#[tokio::test]
async fn resources_read_unknown_uri_is_a_protocol_error() {
    let mut h = handler();
    let resp = h
        .handle_request(&req("resources/read", json!({ "uri": "sys-mcp://nope" })))
        .await;

    let v = body(&resp);
    assert!(
        v.get("error").is_some(),
        "unknown URI must error, not return empty"
    );
    let msg = v["error"]["message"].as_str().unwrap_or_default();
    assert!(msg.contains("unknown resource"), "message: {msg}");
    // The error should teach the caller the valid set.
    assert!(msg.contains("sys-mcp://monitors"), "message: {msg}");
}

#[tokio::test]
async fn resources_read_requires_a_uri() {
    let mut h = handler();
    let resp = h.handle_request(&req("resources/read", json!({}))).await;
    assert!(body(&resp).get("error").is_some());
}

#[tokio::test]
async fn prompts_list_matches_the_catalogue() {
    let mut h = handler();
    let resp = h.handle_request(&req("prompts/list", json!({}))).await;
    let prompts = body(&resp)["result"]["prompts"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let names: Vec<_> = prompts
        .iter()
        .filter_map(|p| p.get("name").and_then(|v| v.as_str()))
        .collect();
    assert_eq!(
        names,
        vec!["describe-screen", "find-and-click", "automate-goal"]
    );

    // Every declared argument must be described, and required args marked.
    for p in &prompts {
        for a in p
            .get("arguments")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            assert!(
                a.get("description").is_some(),
                "arg without description: {a}"
            );
            assert!(
                a.get("required").is_some(),
                "arg without required flag: {a}"
            );
        }
    }
}

#[tokio::test]
async fn prompts_get_returns_a_message() {
    let mut h = handler();
    let resp = h
        .handle_request(&req(
            "prompts/get",
            json!({ "name": "describe-screen", "arguments": {} }),
        ))
        .await;

    let msg = body(&resp)["result"]["messages"][0].clone();
    assert_eq!(msg.get("role").and_then(|v| v.as_str()), Some("user"));
    let text = msg["content"]["text"].as_str().expect("prompt text");
    assert!(
        text.contains("screen_read"),
        "prompt should steer to structured reads"
    );
}

#[tokio::test]
async fn prompts_get_requires_declared_arguments() {
    let mut h = handler();

    // Missing required argument is an error, not a silently-empty template.
    let resp = h
        .handle_request(&req(
            "prompts/get",
            json!({ "name": "find-and-click", "arguments": {} }),
        ))
        .await;
    let msg = body(&resp)["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(
        msg.contains("missing required prompt argument"),
        "message: {msg}"
    );

    // With it supplied, the target is interpolated into the instructions.
    let resp = h
        .handle_request(&req(
            "prompts/get",
            json!({ "name": "find-and-click", "arguments": { "target": "the Save button" } }),
        ))
        .await;
    let text = body(&resp)["result"]["messages"][0]["content"]["text"]
        .as_str()
        .expect("text")
        .to_string();
    assert!(
        text.contains("the Save button"),
        "target not interpolated: {text}"
    );
}

#[tokio::test]
async fn prompts_get_unknown_name_lists_the_valid_set() {
    let mut h = handler();
    let resp = h
        .handle_request(&req("prompts/get", json!({ "name": "nope" })))
        .await;
    let msg = body(&resp)["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(msg.contains("unknown prompt"), "message: {msg}");
    assert!(msg.contains("describe-screen"), "message: {msg}");
}
