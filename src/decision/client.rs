//! Async HTTP client for the JEV `/v1/systemone` endpoint.
//!
//! Only compiled with the `jev` cargo feature (pulls in `reqwest`).
//! Maps upstream failures to distinct, actionable error strings and never
//! includes the API key in any message.

use std::time::Duration;

use serde_json::Value;

use crate::config::JevConfig;
use crate::decision::types::{DecisionErrorEnvelope, DecisionRequest, DecisionResponse, Question};

/// Client for the JEV decision endpoint.
#[derive(Clone)]
pub struct DecisionClient {
    http: reqwest::Client,
    endpoint: String,
    api_key: String,
    default_model: String,
    timeout: Duration,
}

impl DecisionClient {
    /// Build a client from configuration. The timeout is applied per request.
    pub fn new(config: &JevConfig) -> Result<Self, String> {
        let timeout = Duration::from_secs(config.timeout_secs.max(1));
        // Header-based auth only; no URL/query ever carries the key.
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("failed to build JEV HTTP client: {e}"))?;
        Ok(Self {
            http,
            endpoint: config.endpoint.clone(),
            api_key: config.api_key.clone(),
            default_model: config.model.clone(),
            timeout,
        })
    }

    /// The model used when a request does not override it.
    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    /// Evaluate `state` against `questions`.
    ///
    /// `model` overrides the configured default when set. Errors are
    /// human-readable and actionable; the API key never appears in them.
    /// Send a decision request, recording it in the cost ledger under the
    /// generic `decision` tag.
    pub async fn decide(
        &self,
        state: Value,
        questions: std::collections::BTreeMap<String, Question>,
        model: Option<&str>,
    ) -> Result<DecisionResponse, String> {
        self.decide_tag(state, questions, model, "decision").await
    }

    /// [`decide`] with an explicit ledger tag, so callers can tell apart what
    /// spent the tokens (guard checks, ranking batches, direct decisions).
    ///
    /// Every call — success or failure — is recorded with its token usage and
    /// latency, because a decision layer that quietly costs more than it saves
    /// is exactly what the ledger exists to expose.
    pub async fn decide_tag(
        &self,
        state: Value,
        questions: std::collections::BTreeMap<String, Question>,
        model: Option<&str>,
        tag: &str,
    ) -> Result<DecisionResponse, String> {
        let started = std::time::Instant::now();
        let model_name = model.unwrap_or(&self.default_model).to_string();

        match self.decide_inner(state, questions, &model_name).await {
            Ok(response) => {
                // `usage` is optional in the contract; absent means the
                // gateway did not report it, not that the call was free.
                let (in_tok, out_tok) = response
                    .usage
                    .as_ref()
                    .map(|u| (u.input_tokens, u.output_tokens))
                    .unwrap_or((0, 0));
                crate::decision::stats::record(
                    tag,
                    &model_name,
                    in_tok,
                    out_tok,
                    started.elapsed(),
                    true,
                );
                Ok(response)
            }
            Err(e) => {
                // Failures still cost latency and often tokens upstream, so
                // they are recorded too — with zeroes for usage because the
                // response never arrived.
                crate::decision::stats::record(tag, &model_name, 0, 0, started.elapsed(), false);
                Err(e)
            }
        }
    }

    async fn decide_inner(
        &self,
        state: Value,
        questions: std::collections::BTreeMap<String, Question>,
        model: &str,
    ) -> Result<DecisionResponse, String> {
        let body = DecisionRequest {
            model: model.to_string(),
            state,
            questions,
        };

        let response = self
            .http
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| self.describe_send_error(e))?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            return Err(match retry_after {
                Some(secs) => format!(
                    "JEV rate limit exceeded (429). Retry after {secs}s, or reduce request frequency."
                ),
                None => {
                    "JEV rate limit exceeded (429). Retry later, or reduce request frequency."
                        .to_string()
                }
            });
        }

        if !status.is_success() {
            let status_code = status.as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(match serde_json::from_str::<DecisionErrorEnvelope>(&text) {
                Ok(env) => {
                    let code = env.error.code.unwrap_or_default();
                    let msg = env.error.message.unwrap_or_default();
                    match status_code {
                        400 => format!(
                            "JEV rejected the request as invalid (400, {code}): {msg}. Check question instructions/criteria shapes."
                        ),
                        401 | 403 => format!(
                            "JEV authentication failed ({status_code}, {code}): {msg}. Check the JEV API key configuration (value not shown)."
                        ),
                        _ => format!("JEV request failed ({status_code}, {code}): {msg}"),
                    }
                }
                Err(_) => {
                    format!(
                        "JEV request failed with HTTP {status_code}: {}",
                        truncate(&text, 200)
                    )
                }
            });
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("failed to read JEV response body: {e}"))?;
        serde_json::from_str::<DecisionResponse>(&text)
            .map_err(|e| format!("failed to parse JEV response: {e}"))
    }

    /// Map a transport-level reqwest error to an actionable message.
    /// Never includes the API key (reqwest errors don't, but be explicit).
    fn describe_send_error(&self, e: reqwest::Error) -> String {
        if e.is_timeout() {
            format!(
                "JEV request timed out after {}s. The endpoint may be unreachable; check --jev-endpoint / JEV_ENDPOINT.",
                self.timeout.as_secs()
            )
        } else if e.is_connect() {
            format!(
                "failed to connect to JEV endpoint. Check that the gateway is running and --jev-endpoint / JEV_ENDPOINT is correct: {e}"
            )
        } else {
            format!("JEV request failed at the transport layer: {e}")
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}
