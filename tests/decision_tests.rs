//! Offline tests for the JEV `decision` tool: request/response serde
//! shapes, redaction, handler argument validation, and the feature-gated
//! dispatch path. No network access — response fixtures are recorded from
//! the live gateway contract. The single live test lives at the bottom,
//! gated behind `JEV_LIVE_TESTS=1` and `#[ignore]`.

use serde_json::{json, Value};
use sys_mcp::decision::redact::{redact, redact_value};
use sys_mcp::decision::types::{
    parse_and_validate, DecisionErrorEnvelope, DecisionRequest, DecisionResponse, Question,
    QuestionType,
};
use sys_mcp::gui::GuiClient;
use sys_mcp::mcp::handlers::handle_tool_call;
use sys_mcp::StubBackend;

// ─── Fixtures ──────────────────────────────────────────────────────────────

/// Recorded noul answer shape (gateway contract).
const NOUL_RESPONSE: &str = r#"{
    "answers": { "is_urgent": { "type": "noul", "noul": 0.96 } },
    "model": "jev-1.13-free",
    "usage": { "input_tokens": 120, "output_tokens": 8 }
}"#;

/// Recorded choice answer shape.
const CHOICE_RESPONSE: &str = r#"{
    "answers": {
        "route": {
            "type": "choice",
            "choice": "optA",
            "probabilities": { "optA": 0.9, "optB": 0.1 },
            "confidence": 0.95
        }
    },
    "model": "jev-1.13-free",
    "usage": { "input_tokens": 140, "output_tokens": 12 }
}"#;

/// Recorded score answer shape (probabilities/legend keys are strings).
const SCORE_RESPONSE: &str = r#"{
    "answers": {
        "severity": {
            "type": "score",
            "score": 1.98,
            "probabilities": { "0": 0.0, "1": 0.5, "2": 0.5 },
            "confidence": 0.9,
            "legend": { "0": "Low", "1": "Med", "2": "High" }
        }
    },
    "model": "jev-1.13-free",
    "usage": { "input_tokens": 290, "output_tokens": 23 }
}"#;

/// Recorded mixed response with all three answer types in one payload.
const MIXED_RESPONSE: &str = r#"{
    "answers": {
        "is_urgent": { "type": "noul", "noul": 0.96 },
        "route": {
            "type": "choice",
            "choice": "optA",
            "probabilities": { "optA": 0.9, "optB": 0.1 },
            "confidence": 0.95
        },
        "severity": {
            "type": "score",
            "score": 1.98,
            "probabilities": { "0": 0.0, "1": 0.5, "2": 0.5 },
            "confidence": 0.9,
            "legend": { "0": "Low", "1": "Med", "2": "High" }
        }
    },
    "model": "jev-1.13-free",
    "usage": { "input_tokens": 290, "output_tokens": 23 }
}"#;

/// Recorded error envelope (returned with non-200 status).
const ERROR_RESPONSE: &str =
    r#"{"error":{"code":"invalid_api_key","message":"Unauthorized","type":"auth_error"}}"#;

fn noul_question() -> Value {
    json!({
        "type": "noul",
        "instructions": "Does this request require urgent attention?"
    })
}

fn choice_question() -> Value {
    json!({
        "type": "choice",
        "instructions": "Route this ticket.",
        "criteria": { "optA": "Backend issue", "optB": "Frontend issue" }
    })
}

fn score_question() -> Value {
    json!({
        "type": "score",
        "instructions": "Rate severity.",
        "criteria": ["Low", "Med", "High"]
    })
}

// ─── Request serialization ─────────────────────────────────────────────────

#[test]
fn request_serializes_noul_question() {
    let args = json!({
        "state": "My payments have failed for three days.",
        "questions": { "is_urgent": noul_question() }
    });
    let v = parse_and_validate(&args).unwrap();
    let req = DecisionRequest {
        model: "jev-1.13-free".into(),
        state: v.state,
        questions: v.questions,
    };
    let value = serde_json::to_value(&req).unwrap();
    assert_eq!(value["model"], "jev-1.13-free");
    assert_eq!(value["state"], "My payments have failed for three days.");
    assert_eq!(value["questions"]["is_urgent"]["type"], "noul");
    assert_eq!(
        value["questions"]["is_urgent"]["instructions"],
        "Does this request require urgent attention?"
    );
    // noul without criteria must not serialize a criteria key.
    assert!(value["questions"]["is_urgent"].get("criteria").is_none());
}

#[test]
fn request_serializes_choice_question_with_map_criteria() {
    let args = json!({ "state": "x", "questions": { "route": choice_question() } });
    let v = parse_and_validate(&args).unwrap();
    let req = DecisionRequest {
        model: "jev-1.13-free".into(),
        state: v.state,
        questions: v.questions,
    };
    let value = serde_json::to_value(&req).unwrap();
    assert_eq!(value["questions"]["route"]["type"], "choice");
    assert_eq!(
        value["questions"]["route"]["criteria"],
        json!({ "optA": "Backend issue", "optB": "Frontend issue" })
    );
}

#[test]
fn request_serializes_score_question_with_array_criteria() {
    let args = json!({ "state": "x", "questions": { "sev": score_question() } });
    let v = parse_and_validate(&args).unwrap();
    let req = DecisionRequest {
        model: "jev-1.13-free".into(),
        state: v.state,
        questions: v.questions,
    };
    let value = serde_json::to_value(&req).unwrap();
    assert_eq!(value["questions"]["sev"]["type"], "score");
    assert_eq!(
        value["questions"]["sev"]["criteria"],
        json!(["Low", "Med", "High"])
    );
}

#[test]
fn request_roundtrips_through_question_type() {
    // The typed Question must survive serde round-trip for all three types.
    for raw in [noul_question(), choice_question(), score_question()] {
        let q: Question = serde_json::from_value(raw.clone()).unwrap();
        let back = serde_json::to_value(&q).unwrap();
        assert_eq!(back["type"], raw["type"]);
        assert_eq!(back["instructions"], raw["instructions"]);
        assert_eq!(back["criteria"], raw["criteria"]);
    }
}

// ─── Response deserialization ──────────────────────────────────────────────

#[test]
fn deserializes_noul_answer() {
    let resp: DecisionResponse = serde_json::from_str(NOUL_RESPONSE).unwrap();
    let a = resp.answers.get("is_urgent").unwrap();
    assert_eq!(a.answer_type, QuestionType::Noul);
    assert_eq!(a.noul, Some(0.96));
    assert!(a.choice.is_none() && a.score.is_none());
    assert_eq!(resp.model.as_deref(), Some("jev-1.13-free"));
    assert_eq!(resp.usage.unwrap().input_tokens, 120);
}

#[test]
fn deserializes_choice_answer() {
    let resp: DecisionResponse = serde_json::from_str(CHOICE_RESPONSE).unwrap();
    let a = resp.answers.get("route").unwrap();
    assert_eq!(a.answer_type, QuestionType::Choice);
    assert_eq!(a.choice.as_deref(), Some("optA"));
    assert_eq!(a.confidence, Some(0.95));
    let probs = a.probabilities.as_ref().unwrap();
    assert_eq!(probs.get("optA"), Some(&0.9));
    assert_eq!(probs.get("optB"), Some(&0.1));
}

#[test]
fn deserializes_score_answer_with_string_keyed_legend() {
    let resp: DecisionResponse = serde_json::from_str(SCORE_RESPONSE).unwrap();
    let a = resp.answers.get("severity").unwrap();
    assert_eq!(a.answer_type, QuestionType::Score);
    assert_eq!(a.score, Some(1.98));
    let probs = a.probabilities.as_ref().unwrap();
    assert_eq!(probs.get("0"), Some(&0.0));
    assert_eq!(probs.get("2"), Some(&0.5));
    let legend = a.legend.as_ref().unwrap();
    assert_eq!(legend.get("1").map(String::as_str), Some("Med"));
}

#[test]
fn deserializes_mixed_response_preserving_branchable_fields() {
    let resp: DecisionResponse = serde_json::from_str(MIXED_RESPONSE).unwrap();
    assert_eq!(resp.answers.len(), 3);
    // Probabilities/confidence must not be flattened away — an agent must
    // be able to branch on them.
    assert!(resp.answers["route"].probabilities.is_some());
    assert!(resp.answers["route"].confidence.is_some());
    assert!(resp.answers["severity"].legend.is_some());
    assert!(resp.answers["is_urgent"].noul.is_some());
    let usage = resp.usage.unwrap();
    assert_eq!((usage.input_tokens, usage.output_tokens), (290, 23));
}

// ─── Error shape ───────────────────────────────────────────────────────────

#[test]
fn deserializes_error_envelope() {
    let env: DecisionErrorEnvelope = serde_json::from_str(ERROR_RESPONSE).unwrap();
    assert_eq!(env.error.code.as_deref(), Some("invalid_api_key"));
    assert_eq!(env.error.message.as_deref(), Some("Unauthorized"));
    assert_eq!(env.error.error_type.as_deref(), Some("auth_error"));
}

// ─── Redaction ─────────────────────────────────────────────────────────────

#[test]
fn redact_scrubs_multiple_secret_shapes() {
    let cases = [
        // sk- bearer-style token
        ("key is sk-abcdef123456 done", "key is [REDACTED] done"),
        // password= pair
        (
            "connect with password=hunter2 now",
            "connect with password=[REDACTED] now",
        ),
        // token= pair
        ("token=ghp_abc123def456&x=1", "token=[REDACTED]&x=1"),
        // secret= pair (colon form)
        ("secret: topsecret99", "secret: [REDACTED]"),
        // api_key= pair
        ("api_key=AKIA1234567890ABCD", "api_key=[REDACTED]"),
        // Bearer token in an Authorization header dump
        (
            "Authorization: Bearer eyJhbGciOi.payload.sig",
            "Authorization: [REDACTED]",
        ),
        // 64-char hex blob
        (
            "hash 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef end",
            "hash [REDACTED] end",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(redact(input), expected, "input: {input}");
    }
}

#[test]
fn redact_json_object_form_masks_value_keeps_structure() {
    let input = r#"{"user":"alice","password":"hunter2","port":8080}"#;
    let out = redact(input);
    assert!(out.contains(r#""password":"[REDACTED]""#), "got: {out}");
    assert!(out.contains(r#""user":"alice""#), "got: {out}");
    // Must remain valid JSON with the same shape.
    let parsed: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["port"], 8080);
}

#[test]
fn redact_leaves_non_secrets_untouched() {
    for input in [
        "",
        "The deploy failed with exit code 1",
        "user=alice role=admin",        // no secret key names
        "tokenizer=fast",               // "tokenizer" does not end with a secret key suffix
        "short sk-x",                   // sk body too short
        "https://example.com/path?q=1", // URL
        "password:",                    // separator, empty value
    ] {
        assert_eq!(redact(input), input, "input: {input}");
    }
}

#[test]
fn redact_is_idempotent() {
    let input = "password=hunter2 and sk-abcdef123456 and averyveryverylongbase64==";
    let once = redact(input);
    let twice = redact(&once);
    assert_eq!(once, twice);
}

#[test]
fn redact_value_walks_nested_state_and_reports_change() {
    let state = json!({
        "action": "shell_exec",
        "detail": { "cmd": "curl -H 'Authorization: Bearer sk-abcdef123456' x" },
        "logins": ["password=foo", "plain text"],
        "count": 3
    });
    let (scrubbed, changed) = redact_value(&state);
    assert!(changed);
    let s = scrubbed.to_string();
    assert!(!s.contains("sk-abcdef123456"), "token leaked: {s}");
    assert!(!s.contains("password=foo"), "password leaked: {s}");
    assert_eq!(scrubbed["count"], 3);
    // Non-secret state is unchanged and reported unchanged.
    let (same, changed) = redact_value(&json!({"a": "b"}));
    assert!(!changed);
    assert_eq!(same, json!({"a": "b"}));
}

// ─── Handler argument validation ───────────────────────────────────────────

fn validation_error(args: Value) -> String {
    parse_and_validate(&args).unwrap_err()
}

#[test]
fn validate_missing_state() {
    let err = validation_error(json!({ "questions": { "q": noul_question() } }));
    assert_eq!(err, "missing required argument: state");
}

#[test]
fn validate_missing_questions() {
    let err = validation_error(json!({ "state": "hello" }));
    assert_eq!(err, "missing required argument: questions");
}

#[test]
fn validate_empty_questions() {
    let err = validation_error(json!({ "state": "hello", "questions": {} }));
    assert_eq!(
        err,
        "invalid argument: questions must contain at least one question"
    );
}

#[test]
fn validate_questions_not_an_object() {
    let err = validation_error(json!({ "state": "hello", "questions": [noul_question()] }));
    assert!(err.contains("questions must be an object"), "got: {err}");
}

#[test]
fn validate_choice_requires_two_criteria() {
    let err = validation_error(json!({
        "state": "x",
        "questions": {
            "q": { "type": "choice", "instructions": "pick", "criteria": { "only": "one" } }
        }
    }));
    assert!(err.contains("at least 2 criteria"), "got: {err}");

    let err = validation_error(json!({
        "state": "x",
        "questions": { "q": { "type": "choice", "instructions": "pick" } }
    }));
    assert!(err.contains("at least 2 criteria"), "got: {err}");
}

#[test]
fn validate_score_requires_non_empty_array_criteria() {
    let err = validation_error(json!({
        "state": "x",
        "questions": {
            "q": { "type": "score", "instructions": "rate", "criteria": { "not": "an array" } }
        }
    }));
    assert!(err.contains("non-empty array"), "got: {err}");

    let err = validation_error(json!({
        "state": "x",
        "questions": {
            "q": { "type": "score", "instructions": "rate", "criteria": [] }
        }
    }));
    assert!(err.contains("non-empty array"), "got: {err}");
}

#[test]
fn validate_unknown_type_and_missing_instructions() {
    let err = validation_error(json!({
        "state": "x",
        "questions": { "q": { "type": "yesno", "instructions": "?" } }
    }));
    assert!(err.contains("unknown type 'yesno'"), "got: {err}");

    let err = validation_error(json!({
        "state": "x",
        "questions": { "q": { "type": "noul" } }
    }));
    assert_eq!(err, "missing required argument: q.instructions");

    let err = validation_error(json!({
        "state": "x",
        "questions": { "q": { "type": "noul", "instructions": "   " } }
    }));
    assert!(err.contains("non-empty"), "got: {err}");
}

#[test]
fn validate_accepts_arbitrary_json_state_and_sets_redacted_flag() {
    // Structured (non-string) state is accepted.
    let v = parse_and_validate(&json!({
        "state": { "cmd": "ls", "n": 1 },
        "questions": { "q": noul_question() }
    }))
    .unwrap();
    assert!(!v.redacted);
    assert_eq!(v.state["cmd"], "ls");

    // A secret inside string state flips the redacted flag.
    let v = parse_and_validate(&json!({
        "state": "deploy with password=hunter2",
        "questions": { "q": noul_question() }
    }))
    .unwrap();
    assert!(v.redacted);
    assert_eq!(v.state, "deploy with password=[REDACTED]");

    // Model override passes through.
    let v = parse_and_validate(&json!({
        "state": "x",
        "questions": { "q": noul_question() },
        "model": "jev-1.13"
    }))
    .unwrap();
    assert_eq!(v.model.as_deref(), Some("jev-1.13"));
}

// ─── Tool schema & dispatch ────────────────────────────────────────────────

#[test]
fn decision_tool_is_advertised() {
    let tools = sys_mcp::mcp::tools::all_tools();
    let decision = tools
        .iter()
        .find(|t| t["name"] == "decision")
        .expect("decision tool must be listed");
    let schema = &decision["inputSchema"];
    assert_eq!(schema["required"], json!(["state", "questions"]));
    let props = schema["properties"].as_object().unwrap();
    assert!(props.contains_key("state"));
    assert!(props.contains_key("questions"));
    assert!(props.contains_key("model"));
}

#[tokio::test]
async fn decision_dispatch_smoke() {
    // Without the `jev` feature this must fail with a clean, actionable
    // message; with it, it must not panic on valid args (validation runs
    // first, so an invalid request never reaches the network).
    let client = GuiClient::new(StubBackend);
    let result = handle_tool_call(
        &client,
        "decision",
        json!({ "state": "x", "questions": {} }),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");

    #[cfg(not(feature = "jev"))]
    assert!(
        text.contains("not compiled in") && text.contains("--features jev"),
        "got: {text}"
    );

    // Argument validation applies regardless of feature flags: empty
    // questions is rejected before any client is constructed.
    #[cfg(feature = "jev")]
    assert!(text.contains("at least one question"), "got: {text}");
}

#[cfg(not(feature = "jev"))]
#[tokio::test]
async fn decision_feature_disabled_returns_clean_error() {
    let client = GuiClient::new(StubBackend);
    let result = handle_tool_call(
        &client,
        "decision",
        json!({
            "state": "hello",
            "questions": { "ok": { "type": "noul", "instructions": "Is this reachable?" } }
        }),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("not compiled in"), "got: {text}");
    assert!(text.contains("jev"), "got: {text}");
    // The feature-disabled error must never mention a key.
    assert!(!text.to_lowercase().contains("api key"), "got: {text}");
}

// ─── Live gateway test (ignored by default) ────────────────────────────────

/// Single live call against the local gateway. Never runs by default:
/// `#[ignore]` plus the `JEV_LIVE_TESTS=1` env gate. Free tier is rate
/// limited (~1 req/min) — keep this to ONE request.
///
/// Run with:
///   JEV_LIVE_TESTS=1 cargo test --features jev --test decision_tests -- --ignored --nocapture
#[cfg(feature = "jev")]
#[tokio::test]
#[ignore = "live gateway test; requires JEV_LIVE_TESTS=1 and --features jev"]
async fn live_gateway_noul_smoke() {
    use sys_mcp::decision::client::DecisionClient;

    if std::env::var("JEV_LIVE_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipping live gateway test: JEV_LIVE_TESTS != 1");
        return;
    }

    let client = DecisionClient::new(&sys_mcp::decision::config())
        .expect("client should build from default config");
    let mut questions = std::collections::BTreeMap::new();
    questions.insert(
        "is_urgent".to_string(),
        serde_json::from_value(noul_question()).unwrap(),
    );
    let response = client
        .decide(
            json!("My payments have failed for three days and I am losing sales. Please help now."),
            questions,
            None,
        )
        .await
        .expect("live gateway call");

    let answer = response.answers.get("is_urgent").expect("answer present");
    assert_eq!(answer.answer_type, QuestionType::Noul);
    let noul = answer.noul.expect("noul value");
    assert!((0.0..=1.0).contains(&noul), "noul out of range: {noul}");
    let usage = response.usage.expect("usage present");
    assert!(usage.input_tokens > 0, "expected non-zero input tokens");
}
