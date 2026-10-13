//! Tool-schema consistency tests.
//!
//! Guards against schema/dispatch drift, the class of bug that produced
//! issues #21 (descriptions referencing renamed `gui_*` tools, including the
//! nonexistent `gui_delete_dir`) and the `web_preview`/`gui_web_preview`
//! mismatch where an advertised tool could never be called.
//!
//! These assert on *behaviour* (does the dispatcher recognise every advertised
//! tool?) rather than parsing source text, so they survive refactors such as
//! splitting `tools.rs` or renaming handler modules.

use serde_json::{json, Value};
use sys_mcp::gui::GuiClient;
use sys_mcp::mcp::handlers::handle_tool_call;
use sys_mcp::mcp::tools::all_tools;
use sys_mcp::StubBackend;

/// Every tool advertised by `tools/list`.
fn declared_tools() -> Vec<Value> {
    all_tools()
}

fn tool_name(t: &Value) -> String {
    t.get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn joined_text(r: &sys_mcp::protocol::mcp::ToolResult) -> String {
    r.content
        .iter()
        .filter_map(|c| c.text.as_deref())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn tool_names_are_unique() {
    let tools = declared_tools();
    let mut seen = std::collections::HashSet::new();
    for t in &tools {
        let name = tool_name(t);
        assert!(!name.is_empty(), "a tool has an empty name");
        assert!(seen.insert(name.clone()), "duplicate tool name: {name}");
    }
    assert!(!tools.is_empty());
}

#[test]
fn every_tool_has_a_description_and_object_schema() {
    for t in declared_tools() {
        let name = tool_name(&t);
        let desc = t
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(
            !desc.trim().is_empty(),
            "tool '{name}' has no description — agents pick tools by description"
        );

        let schema = t.get("inputSchema").expect("missing inputSchema");
        assert_eq!(
            schema.get("type").and_then(Value::as_str),
            Some("object"),
            "tool '{name}' inputSchema must be an object"
        );
    }
}

#[test]
fn required_args_are_a_subset_of_declared_properties() {
    // A `required` entry with no matching property is a schema violation that
    // clients will reject, and it silently breaks tool calling.
    for t in declared_tools() {
        let name = tool_name(&t);
        let schema = t.get("inputSchema").unwrap();
        let props = match schema.get("properties") {
            Some(Value::Object(m)) => m.keys().cloned().collect::<Vec<_>>(),
            None => Vec::new(),
            other => panic!("tool '{name}': properties must be an object, got {other:?}"),
        };
        if let Some(req) = schema.get("required").and_then(Value::as_array) {
            for r in req {
                let arg = r.as_str().unwrap_or_default();
                assert!(
                    props.iter().any(|p| p == arg),
                    "tool '{name}' requires '{arg}' but never declares it in properties"
                );
            }
        }
    }
}

/// The core drift guard: every advertised tool must be reachable in the
/// dispatcher. If this fails, an agent can discover a tool that always
/// returns "unknown tool" — exactly the `web_preview`/`gui_web_preview` bug.
#[tokio::test]
async fn every_declared_tool_is_dispatchable() {
    let client = GuiClient::new(StubBackend);
    let mut spawned_shells: Vec<String> = Vec::new();

    for t in declared_tools() {
        let name = tool_name(&t);

        // shell_open spawns a real PTY even with empty args, so capture the
        // session id it returns and close it rather than leaking a process
        // per test run.
        let result = handle_tool_call(&client, &name, json!({})).await;
        let text = joined_text(&result);

        assert!(
            !text.contains("unknown tool"),
            "tool '{name}' is advertised in tools/list but the dispatcher does \
             not recognise it — an agent would always get 'unknown tool'"
        );

        if name == "shell_open" {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                if let Some(id) = v.get("session_id").and_then(Value::as_str) {
                    spawned_shells.push(id.to_string());
                }
            }
        }
    }

    for id in spawned_shells {
        let _ = handle_tool_call(&client, "shell_close", json!({ "session_id": id })).await;
    }
}

/// Tools whose handlers are feature-gated still get declared, so an agent can
/// see the full surface; they must fail with an explicit availability message
/// rather than silently vanishing or returning "unknown tool".
#[tokio::test]
async fn feature_gated_tools_stay_visible_and_fail_explicitly() {
    let client = GuiClient::new(StubBackend);
    for name in ["screen_read", "detect_objects", "web_preview", "decision"] {
        let declared = declared_tools().iter().any(|t| tool_name(t) == name);
        assert!(
            declared,
            "tool '{name}' must stay declared even though its feature may be off"
        );

        let result = handle_tool_call(&client, name, json!({})).await;
        let text = joined_text(&result);
        assert!(
            !text.contains("unknown tool"),
            "'{name}' must not report 'unknown tool'"
        );
    }
}
