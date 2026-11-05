use std::io::Write;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;
use tokio::task::JoinSet;

use crate::config::{ServerConfig, TransportMode};
use crate::gui::GuiClient;
use crate::mcp::handlers::handle_tool_call;
#[cfg(feature = "web-preview")]
use crate::mcp::session::create_session_manager;
use crate::mcp::tools::all_tools;
use crate::protocol::mcp::{McpRequest, McpResponse};

#[cfg(feature = "web-preview")]
use crate::web::server::WebServer;

/// How many requests are handled at the same time before the read loop waits
/// for a slot to free up.
///
/// The defect in `docs/ISSUES.md` #32 is that one slow tool (a full-screen OCR,
/// say) made every other tool on the connection time out behind it. Answering
/// that with an unbounded `spawn` per line would trade the stall for a client
/// that floods stdin and gets a task per line — each of which can hold a
/// screenshot — with nothing pushing back. So the cap is admission control, not
/// a queue: at `MAX_IN_FLIGHT` unfinished requests the loop stops reading until
/// one lands, which puts the pressure on the pipe (what it is for) instead of on
/// this process's memory. 32 is well above the concurrency a single MCP client
/// puts on one session today, so a normal session never sees it.
const MAX_IN_FLIGHT: usize = 32;

pub struct McpRequestHandler {
    client: Arc<GuiClient>,
    /// Written once by [`Self::start_web_preview`] before any request can reach
    /// it, then read per request.
    ///
    /// It is behind a lock rather than a plain field because `handle_request`
    /// takes `&self`, and that is the whole of issue #32: the request path never
    /// mutated the handler, but its `&mut self` signature forced the transport to
    /// hold a `Mutex<McpRequestHandler>` across every `await`, which serialized
    /// the connection behind whichever tool was slowest.
    #[cfg(feature = "web-preview")]
    web_server: std::sync::Mutex<Option<WebServer>>,
}

impl McpRequestHandler {
    pub fn new(client: Arc<GuiClient>) -> Self {
        Self {
            client,
            #[cfg(feature = "web-preview")]
            web_server: std::sync::Mutex::new(None),
        }
    }

    #[cfg(feature = "web-preview")]
    pub async fn start_web_preview(&self, host: &str) {
        match WebServer::start(Arc::clone(&self.client), host).await {
            Ok(ws) => {
                log::info!("web preview: {}", ws.url());
                // A poisoned lock here means a *reader* panicked while holding
                // it. `Option<WebServer>` has no invariant that a panic could
                // have broken, so the value is kept: replacing it with `None`
                // would report "web preview is not running" for a server that is
                // still listening.
                let mut slot = self.web_server.lock().unwrap_or_else(|p| p.into_inner());
                *slot = Some(ws);
            }
            Err(e) => {
                log::error!("failed to start web preview: {e}");
            }
        }
    }

    pub async fn handle_request(&self, req: &McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "initialized" => McpResponse::success(req.id.clone(), json!({})),
            "tools/list" => self.handle_tools_list(req),
            "tools/call" => self.handle_tools_call(req).await,
            // Resources: read-only observations of the host. No side effects.
            "resources/list" => McpResponse::success(req.id.clone(), crate::mcp::resources::list()),
            "resources/read" => self.handle_resource_read(req).await,
            // Prompts: reusable instruction templates the host injects into
            // its own context. Nothing here touches the machine.
            "prompts/list" => McpResponse::success(req.id.clone(), crate::mcp::prompts::list()),
            "prompts/get" => self.handle_prompt_get(req),
            "ping" => McpResponse::success(req.id.clone(), json!({})),
            _ => McpResponse::error(
                req.id.clone(),
                -32601,
                format!("method not found: {}", req.method),
            ),
        }
    }

    fn handle_initialize(&self, req: &McpRequest) -> McpResponse {
        McpResponse::success(
            req.id.clone(),
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": { "subscribe": false, "listChanged": false },
                    "prompts": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "sys-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    fn handle_tools_list(&self, req: &McpRequest) -> McpResponse {
        McpResponse::success(req.id.clone(), json!({ "tools": all_tools() }))
    }

    /// `resources/read` — reads one URI's current value.
    ///
    /// Missing/unknown URIs are protocol errors, so a client can distinguish
    /// "no such resource" from "resource is empty".
    async fn handle_resource_read(&self, req: &McpRequest) -> McpResponse {
        let uri = req
            .params
            .as_ref()
            .and_then(|p| p.get("uri"))
            .and_then(Value::as_str);

        let Some(uri) = uri else {
            return McpResponse::error(req.id.clone(), -32602, "resources/read requires a 'uri'");
        };

        match crate::mcp::resources::read(&self.client, uri).await {
            Ok(contents) => McpResponse::success(req.id.clone(), contents),
            Err(e) => McpResponse::error(req.id.clone(), -32002, e),
        }
    }

    /// `prompts/get` — returns a templated instruction message.
    fn handle_prompt_get(&self, req: &McpRequest) -> McpResponse {
        let params = req.params.as_ref();
        let name = params.and_then(|p| p.get("name")).and_then(Value::as_str);
        let args = params
            .and_then(|p| p.get("arguments"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        let Some(name) = name else {
            return McpResponse::error(req.id.clone(), -32602, "prompts/get requires a 'name'");
        };

        match crate::mcp::prompts::get(name, &args) {
            Ok(prompt) => McpResponse::success(req.id.clone(), prompt),
            Err(e) => McpResponse::error(req.id.clone(), -32602, e),
        }
    }

    async fn handle_tools_call(&self, req: &McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => {
                return McpResponse::error(req.id.clone(), -32602, "missing params");
            }
        };

        let tool_name = match params.get("name").and_then(Value::as_str) {
            Some(n) => n,
            None => {
                return McpResponse::error(req.id.clone(), -32602, "missing tool name");
            }
        };

        // Matches the tool name advertised in tools.rs (web_preview).
        #[cfg(feature = "web-preview")]
        if tool_name == "web_preview" {
            return self.handle_web_preview(req);
        }

        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let result = handle_tool_call(&self.client, tool_name, args).await;

        McpResponse::success(
            req.id.clone(),
            serde_json::to_value(&result).unwrap_or_default(),
        )
    }

    #[cfg(feature = "web-preview")]
    fn handle_web_preview(&self, req: &McpRequest) -> McpResponse {
        use crate::protocol::mcp::{ContentItem, ToolResult};

        let slot = self.web_server.lock().unwrap_or_else(|p| p.into_inner());
        let result = match &*slot {
            Some(ws) => ToolResult {
                content: vec![ContentItem::text(ws.url())],
                is_error: None,
            },
            None => ToolResult::error(
                "web preview is not running — start server with --web-preview flag",
            ),
        };

        McpResponse::success(
            req.id.clone(),
            serde_json::to_value(&result).unwrap_or_default(),
        )
    }
}

pub struct GuiMcpServer {
    #[allow(dead_code)]
    client: Arc<GuiClient>,
    config: ServerConfig,
    /// Shared by reference, never locked. Handling a request takes `&self` (#32),
    /// so each transport hands out clones of this Arc and runs requests at the
    /// same time instead of queueing them behind one mutex.
    handler: Arc<McpRequestHandler>,
}

impl GuiMcpServer {
    pub fn new(client: GuiClient, config: ServerConfig) -> Self {
        let client = Arc::new(client);
        let handler = Arc::new(McpRequestHandler::new(Arc::clone(&client)));
        Self {
            client,
            config,
            handler,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        #[cfg(feature = "web-preview")]
        if self.config.web_preview {
            self.handler.start_web_preview(&self.config.host).await;
        }

        // Always start HTTP MCP transport when web preview is on, so agents
        // can connect remotely even when the primary transport is stdio.
        #[cfg(feature = "web-preview")]
        if self.config.web_preview && self.config.transport != TransportMode::Http {
            let cfg = self.config.clone();
            let handler = self.handler.clone();
            tokio::spawn(async move {
                let session_mgr = create_session_manager(cfg.max_sessions, cfg.session_ttl_secs);
                let http_server =
                    crate::mcp::http_transport::HttpServer::new(cfg, session_mgr, handler);
                if let Err(e) = http_server.run().await {
                    log::error!("MCP HTTP transport failed: {e}");
                }
            });
        }

        match self.config.transport {
            TransportMode::Stdio => {
                self.run_stdio().await?;
            }
            #[cfg(feature = "web-preview")]
            TransportMode::Http => {
                self.run_http().await?;
            }
            // The HTTP transport is built on axum, which only exists when
            // the `web-preview` feature is compiled in.
            #[cfg(not(feature = "web-preview"))]
            TransportMode::Http => {
                anyhow::bail!(
                    "HTTP transport requires the 'web-preview' cargo feature — rebuild with --features web-preview"
                );
            }
        }

        Ok(())
    }

    async fn run_stdio(&self) -> anyhow::Result<()> {
        let reader = BufReader::new(tokio::io::stdin());
        // `Stdout` is written through our own mutex rather than held locked for
        // the whole session: with concurrent handling, a task has to be able to
        // take the line, print it atomically, and let go.
        let sink = Arc::new(Mutex::new(std::io::stdout()));
        serve_lines(Arc::clone(&self.handler), reader, sink).await
    }

    #[cfg(feature = "web-preview")]
    async fn run_http(&self) -> anyhow::Result<()> {
        let session_mgr =
            create_session_manager(self.config.max_sessions, self.config.session_ttl_secs);
        let http_server = crate::mcp::http_transport::HttpServer::new(
            self.config.clone(),
            session_mgr,
            self.handler.clone(),
        );
        http_server.run().await
    }
}

/// Read JSON-RPC lines from `reader`, answer them concurrently, and write each
/// finished response to `sink` as exactly one line.
///
/// Takes the handler and both ends of the connection rather than living on
/// [`GuiMcpServer`] because the loop only ever needed the handler — and so the
/// transport is testable without being the process's own stdin and stdout
/// (`docs/ISSUES.md` #32): the tests below drive it with an in-memory buffer and
/// a `Vec<u8>` sink.
async fn serve_lines<R, W>(
    handler: Arc<McpRequestHandler>,
    reader: R,
    sink: Arc<Mutex<W>>,
) -> anyhow::Result<()>
where
    R: AsyncBufReadExt + Unpin,
    W: Write + Send + 'static,
{
    let mut lines = reader.lines();
    let mut in_flight: JoinSet<()> = JoinSet::new();

    loop {
        let Some(line) = lines.next_line().await? else {
            break;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: McpRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                // Answered inline: no tool ran, so there is nothing to wait for,
                // and keeping malformed input out of the task set means a client
                // spamming bad JSON cannot occupy the in-flight slots.
                emit(
                    &sink,
                    McpResponse::error(None, -32700, format!("parse error: {e}")),
                )
                .await;
                continue;
            }
        };

        // Admission control; see [`MAX_IN_FLIGHT`].
        while in_flight.len() >= MAX_IN_FLIGHT {
            reap(&mut in_flight).await;
        }

        let handler = Arc::clone(&handler);
        let sink = Arc::clone(&sink);
        in_flight.spawn(async move {
            let response = handler.handle_request(&request).await;
            emit(&sink, response).await;
        });
    }

    // EOF is not an answer. A client that writes its requests and closes stdin —
    // a piped script, or the test harness — expects a response for everything it
    // sent, so in-flight work is drained before the transport returns. Without
    // this the process exits with slow requests silently dropped, which is #40's
    // false absence one layer down.
    while !in_flight.is_empty() {
        reap(&mut in_flight).await;
    }

    Ok(())
}

/// Take one finished task out of the set, and report a panic without letting it
/// take the connection down.
///
/// The old code ran every request through `Mutex<McpRequestHandler>`; a panic
/// inside one poisoned that mutex and every later request on the connection
/// failed for the rest of the session. Now a panic costs its own response and
/// nothing else — the trade is that the client gets no answer for that id, which
/// is logged because a silent gap is the thing an agent cannot debug.
async fn reap(in_flight: &mut JoinSet<()>) {
    match in_flight.join_next().await {
        None => {}
        Some(Ok(())) => {}
        Some(Err(e)) => {
            if e.is_panic() {
                log::error!("request task panicked; the connection stays up, but that request has no response");
            } else {
                log::debug!("request task was cancelled");
            }
        }
    }
}

/// Write one response as one line, whole.
///
/// Encoding happens *before* the sink is locked, so a slow or failing encode
/// cannot hold up the other tasks' writes. If the encode fails the caller still
/// gets a JSON-RPC error (`-32603`) instead of silence: an unanswered request is
/// indistinguishable from a hung server. The fallback carries the response's own
/// id, taken from the value that failed to encode, so the client can correlate it.
async fn emit<W: Write + Send + 'static>(sink: &Arc<Mutex<W>>, response: McpResponse) {
    let line = match serde_json::to_string(&response) {
        Ok(line) => line,
        Err(e) => {
            log::error!("failed to serialize response: {e}");
            let id = match &response {
                McpResponse::Success { id, .. } | McpResponse::Error { id, .. } => id.clone(),
            };
            match serde_json::to_string(&McpResponse::error(
                id,
                -32603,
                format!("internal error: could not serialize the response: {e}"),
            )) {
                Ok(line) => line,
                Err(e) => {
                    // Nothing left to say that could be read; this is the one
                    // case where the transport genuinely cannot answer.
                    log::error!("failed to serialize the error response itself: {e}");
                    return;
                }
            }
        }
    };

    let mut out = sink.lock().await;
    if let Err(e) = writeln!(out, "{line}") {
        log::error!("failed to write response: {e}");
    }
    if let Err(e) = out.flush() {
        log::error!("failed to flush stdout: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::GuiClient;
    use crate::StubBackend;
    use serde_json::json;
    use std::future::Future;
    use std::time::{Duration, Instant};

    /// A tool call that takes `ms` of real time and nothing else. `system_wait`
    /// is the slow request these tests need: it is the only handler that awaits
    /// on purpose, it needs no backend at all, so it is the same in the stub as
    /// on a desktop, and it stays inside #47's hermeticity rule (`tests/` cannot
    /// shell out or load a model, and neither can these).
    fn wait_request(id: &str, ms: u64) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": "system_wait", "arguments": { "ms": ms } },
        })
        .to_string()
    }

    /// A request that answers the moment it is read.
    fn ping_request(id: &str) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "method": "ping" }).to_string()
    }

    fn handler() -> Arc<McpRequestHandler> {
        Arc::new(McpRequestHandler::new(Arc::new(GuiClient::new(
            StubBackend,
        ))))
    }

    fn sink() -> Arc<Mutex<Vec<u8>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    /// Every response written so far, one entry per line.
    ///
    /// Reading takes the same lock the writer holds, so a snapshot can never be
    /// a half-written response — which is what makes "the fast answer is already
    /// here and the slow one is not" a claim a test can actually make.
    async fn lines_out(sink: &Arc<Mutex<Vec<u8>>>) -> Vec<Value> {
        let buf = sink.lock().await.clone();
        String::from_utf8(buf)
            .expect("responses are written as UTF-8")
            .lines()
            .map(|l| {
                serde_json::from_str(l)
                    .unwrap_or_else(|e| panic!("line is not one JSON value ({e}): {l}"))
            })
            .collect()
    }

    fn id_of(resp: &Value) -> String {
        resp.get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    async fn ids_out(sink: &Arc<Mutex<Vec<u8>>>) -> Vec<String> {
        (lines_out(sink).await).iter().map(id_of).collect()
    }

    /// Drive `serve_lines` and a probe at the same time.
    ///
    /// Both are awaited together rather than spawning the server: the input is a
    /// borrow, and a task would need `'static`. The probe is what needs the
    /// concurrency — awaiting the server first would let it finish and hide the
    /// ordering the transport is being tested for.
    async fn serve_with_probe<P, T>(
        handler: Arc<McpRequestHandler>,
        input: &[u8],
        sink: Arc<Mutex<Vec<u8>>>,
        probe: P,
    ) -> (anyhow::Result<()>, T)
    where
        P: Future<Output = T>,
    {
        tokio::join!(serve_lines(handler, BufReader::new(input), sink), probe,)
    }

    /// The defect in `docs/ISSUES.md` #32: one slow tool kept the read loop
    /// locked, so every other call on the connection queued behind it and timed
    /// out. Here `ping` follows a 600ms `system_wait` and must answer while that
    /// wait is still running.
    #[tokio::test]
    async fn a_slow_tool_does_not_hold_up_the_next_request() {
        let input = format!("{}\n{}\n", wait_request("slow", 600), ping_request("fast"));
        let sink = sink();

        // Deadline is well under the wait, so a serialized transport cannot make
        // this assertion by being fast; it can only fail by being blocked.
        let probe = async {
            let started = Instant::now();
            loop {
                let ids = ids_out(&sink).await;
                if ids.contains(&"fast".to_string()) {
                    return (ids, started.elapsed());
                }
                assert!(
                    started.elapsed() < Duration::from_millis(300),
                    "ping was still unanswered after {:?} while only a \
                     system_wait was in flight: {ids:?}",
                    started.elapsed()
                );
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        };

        let (served, (early_ids, elapsed)) =
            serve_with_probe(handler(), input.as_bytes(), Arc::clone(&sink), probe).await;
        served.expect("transport ran clean");

        assert!(
            !early_ids.contains(&"slow".to_string()),
            "the wait answered before it could have finished ({elapsed:?})"
        );
        assert!(
            elapsed < Duration::from_millis(600),
            "ping took {elapsed:?}, which is the whole wait: still serialized"
        );

        let ids = ids_out(&sink).await;
        assert!(ids.contains(&"slow".to_string()), "{ids:?}");
        assert_eq!(ids.first().map(String::as_str), Some("fast"));
    }

    /// Closing stdin is not an answer. A piped client writes its requests and
    /// hangs up; everything it asked for still has to come back before the
    /// transport returns, or the slowest request of the batch silently vanishes
    /// when the process exits.
    #[tokio::test]
    async fn eof_drains_the_responses_still_in_flight() {
        let input = format!("{}\n{}\n", wait_request("slow", 200), ping_request("fast"));
        let sink = sink();

        serve_lines(
            handler(),
            BufReader::new(input.as_bytes()),
            Arc::clone(&sink),
        )
        .await
        .expect("transport ran clean");

        let mut ids = ids_out(&sink).await;
        ids.sort();
        assert_eq!(ids, vec!["fast".to_string(), "slow".to_string()]);
    }

    /// Out-of-order responses are only safe because each carries the id it
    /// answers. A response that lost or invented its id would be worse than the
    /// stall: the client would read the wrong answer for the wrong call.
    #[tokio::test]
    async fn every_response_carries_the_id_of_the_request_that_produced_it() {
        let input = format!(
            "{}\n{}\n{}\n",
            wait_request("a", 150),
            ping_request("b"),
            wait_request("c", 50),
        );
        let sink = sink();

        serve_lines(
            handler(),
            BufReader::new(input.as_bytes()),
            Arc::clone(&sink),
        )
        .await
        .expect("transport ran clean");

        let responses = lines_out(&sink).await;
        assert_eq!(responses.len(), 3, "{responses:?}");
        for resp in &responses {
            let id = id_of(resp);
            assert!(!id.is_empty(), "response with no id: {resp}");
            assert!(
                resp.get("error").is_none(),
                "{id} came back as a protocol error: {resp}"
            );
            // `ping` answers with an empty result; each wait answers with text
            // naming its own duration, which is what ties an id to its call.
            match id.as_str() {
                "b" => assert!(resp["result"]["content"].is_null(), "{resp}"),
                "a" | "c" => {
                    let text = resp["result"]["content"][0]["text"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    let ms = if id == "a" {
                        "Waited 150ms"
                    } else {
                        "Waited 50ms"
                    };
                    assert!(text.contains(ms), "{id} answered {text:?}, expected {ms}");
                }
                other => panic!("unexpected id {other:?}"),
            }
        }
    }

    /// A line that is not valid JSON is a parse error, answered on the spot, and
    /// the connection keeps working. Feeding the bad line into the task set
    /// would let a client spamming garbage occupy the in-flight slots.
    #[tokio::test]
    async fn a_malformed_line_is_answered_and_the_loop_keeps_reading() {
        let input = format!("not json\n\n{}\n", ping_request("after"));
        let sink = sink();

        serve_lines(
            handler(),
            BufReader::new(input.as_bytes()),
            Arc::clone(&sink),
        )
        .await
        .expect("transport ran clean");

        let responses = lines_out(&sink).await;
        assert_eq!(responses.len(), 2, "{responses:?}");
        assert_eq!(
            responses[0]["error"]["code"].as_i64(),
            Some(-32700),
            "{}",
            responses[0]
        );
        assert!(responses[0].get("id").map(|v| v.is_null()).unwrap_or(false));
        assert_eq!(id_of(&responses[1]), "after");
    }

    /// Admission control must not become the bug it replaces. Requests past the
    /// cap wait for a free slot, but every one still answers, so a flood slows
    /// down instead of deadlocking or being dropped.
    #[tokio::test]
    async fn more_requests_than_the_admission_cap_all_complete() {
        let count = MAX_IN_FLIGHT + 8;
        let mut input = String::new();
        for i in 0..count {
            input.push_str(&ping_request(&format!("p{i}")));
            input.push('\n');
        }
        let sink = sink();

        serve_lines(
            handler(),
            BufReader::new(input.as_bytes()),
            Arc::clone(&sink),
        )
        .await
        .expect("transport ran clean");

        let ids = ids_out(&sink).await;
        assert_eq!(ids.len(), count, "{} of {count} answered", ids.len());
        for i in 0..count {
            let want = format!("p{i}");
            assert!(ids.contains(&want), "no response for {want}");
        }
    }

    /// The structural half of #32: one handler shared by reference, with no
    /// mutex around it, serves two requests at the same time. Total elapsed is
    /// the evidence — a `&mut self` path would make two parallel 300ms waits
    /// take 600ms however the transport is written.
    #[tokio::test]
    async fn one_shared_handler_overlaps_two_requests() {
        let wait = || McpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!("slow")),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "system_wait",
                "arguments": { "ms": 300 }
            })),
        };
        let (first, second) = (wait(), wait());
        let started = Instant::now();
        // Two clones of one `Arc`, so this is the same handler answering both —
        // the borrow checker is part of the assertion: `&mut self` would not
        // compile here at all.
        let shared = handler();
        let h1 = Arc::clone(&shared);
        let h2 = Arc::clone(&shared);
        let (a, b) = tokio::join!(h1.handle_request(&first), h2.handle_request(&second));
        let elapsed = started.elapsed();

        for resp in [&a, &b] {
            match resp {
                McpResponse::Success { id, result, .. } => {
                    assert_eq!(id.as_ref().and_then(Value::as_str), Some("slow"));
                    assert!(result.get("content").is_some(), "{resp:?}");
                }
                other => panic!("wait answered with a protocol error: {other:?}"),
            }
        }
        assert!(
            elapsed < Duration::from_millis(550),
            "two parallel 300ms waits took {elapsed:?} — the handler is still serializing"
        );
    }
}
