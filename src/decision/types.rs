//! Serde types for the JEV `/v1/systemone` request/response contract.
//!
//! Request: `{ model, state, questions }` where each question is
//! `{ type: noul|choice|score, instructions, criteria? }`.
//! Response: `{ answers, model, usage }` where each answer's `type` field
//! determines which payload fields are meaningful.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::decision::redact::redact_value;

/// The three JEV question types. Serialized lowercase: `noul`, `choice`,
/// `score`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionType {
    /// Binary truth question. Answer: `{ noul: 0..1 }`.
    Noul,
    /// Pick one option from a bounded set. Answer: `{ choice, probabilities, confidence }`.
    Choice,
    /// Ordered rubric. Answer: `{ score, probabilities, confidence, legend }`.
    Score,
}

/// Per-question criteria. `noul`/`choice` use an object
/// (option name → description); `score` uses an ordered array whose
/// indices become the numeric scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Criteria {
    Map(BTreeMap<String, String>),
    Scale(Vec<String>),
}

/// One question in a decision request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Question {
    #[serde(rename = "type")]
    pub question_type: QuestionType,
    pub instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Criteria>,
}

/// POST `/v1/systemone` request body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRequest {
    pub model: String,
    /// Any JSON value — a plain string or structured state.
    pub state: Value,
    /// Question id → question. Evaluated in parallel upstream.
    pub questions: BTreeMap<String, Question>,
}

/// One typed answer. The `type` field says which payload is meaningful:
/// `noul` → `noul`; `choice` → `choice`, `probabilities`, `confidence`;
/// `score` → `score`, `probabilities`, `confidence`, `legend`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    #[serde(rename = "type")]
    pub answer_type: QuestionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noul: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Option/legend-index (as string) → probability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probabilities: Option<HashMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// For `score` answers: legend index (as string) → label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legend: Option<HashMap<String, String>>,
}

/// Token usage reported by the gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
}

/// Successful `200` response body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionResponse {
    pub answers: HashMap<String, Answer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Error envelope body (`{"error": {...}}`), returned with non-200 status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionErrorEnvelope {
    pub error: DecisionError,
}

/// Contents of the `error` object in a non-200 response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionError {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default, rename = "type")]
    pub error_type: Option<String>,
}

/// A validated, redacted decision input ready to be turned into a
/// [`DecisionRequest`].
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDecision {
    pub model: Option<String>,
    /// Redacted state (string leaves scrubbed via `redact_value`).
    pub state: Value,
    pub questions: BTreeMap<String, Question>,
    /// True when redaction changed the state.
    pub redacted: bool,
}

/// Validate raw tool arguments for the `decision` tool.
///
/// Accepts `state` (string or arbitrary JSON), `questions` (object as
/// described in the module docs), and optional `model` override. Applies
/// secret redaction to the state. Errors follow the handler conventions:
/// `missing required argument: <key>` for absent args, `invalid argument:
/// <detail>` for malformed ones.
pub fn parse_and_validate(args: &Value) -> Result<ValidatedDecision, String> {
    let raw_state = args
        .get("state")
        .ok_or_else(|| "missing required argument: state".to_string())?;

    let model = match args.get("model") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.trim().is_empty() => {
            return Err("invalid argument: model must be a non-empty string".to_string());
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return Err("invalid argument: model must be a string".to_string()),
    };

    let questions_obj = match args.get("questions") {
        None => return Err("missing required argument: questions".to_string()),
        Some(Value::Object(m)) if m.is_empty() => {
            return Err(
                "invalid argument: questions must contain at least one question".to_string(),
            );
        }
        Some(Value::Object(m)) => m.clone(),
        Some(_) => {
            return Err(
                "invalid argument: questions must be an object mapping question ids to questions"
                    .to_string(),
            );
        }
    };

    let mut questions = BTreeMap::new();
    for (id, raw) in &questions_obj {
        let q = validate_question(id, raw)?;
        questions.insert(id.clone(), q);
    }

    let (state, redacted) = redact_value(raw_state);

    Ok(ValidatedDecision {
        model,
        state,
        questions,
        redacted,
    })
}

fn validate_question(id: &str, raw: &Value) -> Result<Question, String> {
    let obj = raw
        .as_object()
        .ok_or_else(|| format!("invalid argument: question '{id}' must be an object"))?;

    let question_type = match obj.get("type") {
        None => return Err(format!("missing required argument: {id}.type")),
        Some(Value::String(t)) => match t.as_str() {
            "noul" => QuestionType::Noul,
            "choice" => QuestionType::Choice,
            "score" => QuestionType::Score,
            other => {
                return Err(format!(
                    "invalid argument: question '{id}' has unknown type '{other}' (expected noul, choice, or score)"
                ))
            }
        },
        Some(_) => return Err(format!("invalid argument: {id}.type must be a string")),
    };

    let instructions = match obj.get("instructions") {
        None => return Err(format!("missing required argument: {id}.instructions")),
        Some(Value::String(s)) if s.trim().is_empty() => {
            return Err(format!(
                "invalid argument: question '{id}' instructions must be a non-empty string"
            ));
        }
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(format!(
                "invalid argument: question '{id}' instructions must be a string"
            ))
        }
    };

    let criteria_raw = obj.get("criteria");
    let criteria = match question_type {
        QuestionType::Noul => match criteria_raw {
            None | Some(Value::Null) => None,
            Some(v @ Value::Object(_)) => Some(parse_criteria_map(v)),
            Some(_) => {
                return Err(format!(
                    "invalid argument: question '{id}' (noul) criteria must be an object with 'true'/'false' keys"
                ))
            }
        },
        QuestionType::Choice => match criteria_raw {
            Some(v) if v.as_object().is_some_and(|m| m.len() >= 2) => Some(parse_criteria_map(v)),
            Some(Value::Object(map)) => {
                return Err(format!(
                    "invalid argument: question '{id}' (choice) requires at least 2 criteria options, got {}",
                    map.len()
                ))
            }
            _ => {
                return Err(format!(
                    "invalid argument: question '{id}' (choice) requires at least 2 criteria options as an object of option name -> description"
                ))
            }
        },
        QuestionType::Score => match criteria_raw {
            Some(Value::Array(items)) if !items.is_empty() => {
                if !items.iter().all(|v| v.is_string()) {
                    return Err(format!(
                        "invalid argument: question '{id}' (score) criteria entries must be strings"
                    ));
                }
                let labels = items
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect();
                Some(Criteria::Scale(labels))
            }
            Some(Value::Array(_)) => {
                return Err(format!(
                    "invalid argument: question '{id}' (score) criteria must be a non-empty array"
                ))
            }
            _ => {
                return Err(format!(
                    "invalid argument: question '{id}' (score) criteria must be a non-empty array of legend labels"
                ))
            }
        },
    };

    Ok(Question {
        question_type,
        instructions,
        criteria,
    })
}

fn parse_criteria_map(value: &Value) -> Criteria {
    let map = value
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.as_str().map(str::to_string).unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    Criteria::Map(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn question_type_serde_lowercase() {
        assert_eq!(
            serde_json::to_value(QuestionType::Noul).unwrap(),
            json!("noul")
        );
        assert_eq!(
            serde_json::to_value(QuestionType::Choice).unwrap(),
            json!("choice")
        );
        assert_eq!(
            serde_json::to_value(QuestionType::Score).unwrap(),
            json!("score")
        );
    }

    #[test]
    fn criteria_untagged_object_vs_array() {
        let map = Criteria::Map(BTreeMap::from([("a".to_string(), "desc".to_string())]));
        assert_eq!(serde_json::to_value(&map).unwrap(), json!({"a": "desc"}));
        let scale = Criteria::Scale(vec!["Low".into(), "High".into()]);
        assert_eq!(
            serde_json::to_value(&scale).unwrap(),
            json!(["Low", "High"])
        );
    }
}
