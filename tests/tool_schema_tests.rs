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
use sys_mcp::{StubA11yBackend, StubBackend};

/// Every tool advertised by `tools/list`.
fn declared_tools() -> Vec<Value> {
    all_tools()
}

/// A declared tool's schema, by name.
fn schema_of(name: &str) -> Value {
    let tool = declared_tools()
        .into_iter()
        .find(|t| tool_name(t) == name)
        .unwrap_or_else(|| panic!("tool '{name}' is not declared"));
    tool.get("inputSchema")
        .expect("missing inputSchema")
        .clone()
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
    for name in [
        "screen_read",
        "detect_objects",
        "web_preview",
        "decision",
        "ui_decide",
    ] {
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

/// `ui_decide` is reachable with an actionable message in every build:
/// feature-off → rebuild hint; feature-on but loop-flag-off (the default) →
/// opt-in hint. Both are asserted conditionally so the same test holds for
/// every feature combination CI builds.
#[tokio::test]
async fn ui_decide_reachable_with_actionable_availability_message() {
    let client = GuiClient::new(StubBackend);
    let result = handle_tool_call(&client, "ui_decide", json!({ "goal": "noop" })).await;
    assert_eq!(result.is_error, Some(true));
    let text = joined_text(&result);

    #[cfg(not(feature = "jev"))]
    assert!(
        text.contains("not compiled in") && text.contains("--features jev"),
        "got: {text}"
    );

    #[cfg(feature = "jev")]
    {
        if std::env::var("SYS_MCP_JEV_UI_LOOP").is_err() {
            // Opt-in flag off (the default) — must explain how to enable,
            // and must never leak key material.
            assert!(text.contains("--jev-ui-loop"), "got: {text}");
            assert!(text.contains("SYS_MCP_JEV_UI_LOOP"), "got: {text}");
            assert!(!text.to_lowercase().contains("api key"), "got: {text}");
        }
        // With the flag on the call proceeds into argument validation;
        // either way it is an explicit, actionable failure here (no goal).
        assert!(!text.contains("unknown tool"), "got: {text}");
    }
}

/// The accessibility tools that address an element by id must *declare* the
/// arguments they refuse to run without, and must refuse them when they are
/// absent. A schema that omits a required argument invites exactly the call the
/// handler rejects, and the drift guards above cannot see it because they only
/// check that `required` entries exist as properties.
#[tokio::test]
async fn accessibility_required_args_are_declared_and_enforced() {
    let (backend, _recorder) = StubA11yBackend::new();
    let client = GuiClient::new(backend);

    let cases = [
        ("accessibility_properties", json!({}), "element_id"),
        (
            "accessibility_action",
            json!({ "element_id": "stub:0" }),
            "action",
        ),
        (
            "accessibility_action",
            json!({ "action": "invoke" }),
            "element_id",
        ),
    ];
    for (name, args, arg) in cases {
        let required = schema_of(name)
            .get("required")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            required.iter().any(|r| r.as_str() == Some(arg)),
            "schema for '{name}' must list '{arg}' in required — the handler rejects a call without it"
        );

        let result = handle_tool_call(&client, name, args).await;
        assert_eq!(
            result.is_error,
            Some(true),
            "'{name}' accepted a call without '{arg}'"
        );
        assert_eq!(
            joined_text(&result),
            format!("missing required argument: {arg}"),
            "'{name}' must name the argument it is missing"
        );
    }

    // The reverse: a tool the schema declares as needing nothing must actually
    // run with no arguments, or the schema is advertising a call that fails.
    for name in ["accessibility_tree", "accessibility_find"] {
        let schema = schema_of(name);
        assert!(
            schema.get("required").is_none(),
            "'{name}' must not declare required arguments"
        );
        let result = handle_tool_call(&client, name, json!({})).await;
        assert_eq!(
            result.is_error,
            None,
            "'{name}' => {}",
            joined_text(&result)
        );
    }
}

/// The four accessibility tools are the newest surface in the server and the
/// only one whose contract spans two calls (an id from one call feeding the
/// next). Their descriptions are what tell an agent that contract, so they must
/// keep naming the tools involved.
#[test]
fn accessibility_descriptions_name_the_id_contract_and_the_other_tools() {
    for name in [
        "accessibility_tree",
        "accessibility_find",
        "accessibility_properties",
        "accessibility_action",
    ] {
        let tool = declared_tools()
            .into_iter()
            .find(|t| tool_name(t) == name)
            .expect("declared");
        let desc = tool
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!desc.is_empty(), "{name} has no description");
        assert!(
            desc.contains("accessibility"),
            "'{name}' description does not describe itself as accessibility: {desc}"
        );
    }

    // The id contract is stated on the two tools that consume an id, and it
    // names both producers, so a caller can find where the id came from.
    for name in ["accessibility_properties", "accessibility_action"] {
        let tool = declared_tools()
            .into_iter()
            .find(|t| tool_name(t) == name)
            .expect("declared");
        let desc = tool
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(desc.contains("most recent"), "'{name}': {desc}");
        assert!(desc.contains("accessibility_tree"), "'{name}': {desc}");
    }
    let tree = declared_tools()
        .into_iter()
        .find(|t| tool_name(t) == "accessibility_tree")
        .expect("declared");
    let tree_desc = tree
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        tree_desc.contains("ids are invalidated") || tree_desc.contains("states"),
        "the tree description must mention what it returns: {tree_desc}"
    );
}

/// #46 added `value`, the operand `set_value` writes. It has to be *declared* —
/// an undeclared property is invisible to a client that builds its call from the
/// schema — and it has to stay optional, because only one verb uses it. The
/// pairing rule ("required when the action is set_value, refused otherwise")
/// cannot be expressed in JSON Schema, so the description is what carries it.
#[test]
fn accessibility_action_declares_value_as_an_optional_operand() {
    let schema = schema_of("accessibility_action");
    let props = schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("properties must be an object");
    assert!(
        props.contains_key("value"),
        "'value' is accepted by the handler but not declared in the schema"
    );

    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !required.iter().any(|r| r.as_str() == Some("value")),
        "'value' must stay optional: only set_value needs it, and every other verb rejects it"
    );

    let value_desc = props["value"]
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        value_desc.contains("set_value"),
        "the value property must name the verb it belongs to: {value_desc}"
    );
    // Clearing a field is a legal write, so the schema has to say the empty
    // string means something rather than "no value given".
    assert!(
        value_desc.contains("empty string"),
        "the value property must say an empty string clears the field: {value_desc}"
    );

    let tool = declared_tools()
        .into_iter()
        .find(|t| tool_name(t) == "accessibility_action")
        .expect("declared");
    let desc = tool
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        desc.contains("set_value") && desc.contains("requires"),
        "the tool description must state the pairing rule the handler enforces: {desc}"
    );
}
