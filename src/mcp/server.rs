use std::io::Write;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

use crate::config::{ServerConfig, TransportMode};
use crate::gui::GuiClient;
use crate::mcp::handlers::handle_tool_call;
use crate::mcp::session::create_session_manager;
use crate::mcp::tools::all_tools;
use crate::protocol::mcp::{McpRequest, McpResponse};

#[cfg(feature = "web-preview")]
use crate::web::server::WebServer;

pub struct McpRequestHandler {
    client: Arc<GuiClient>,
    #[cfg(feature = "web-preview")]
    web_server: Option<WebServer>,
}

impl McpRequestHandler {
    pub fn new(client: Arc<GuiClient>) -> Self {
        Self {
            client,
            #[cfg(feature = "web-preview")]
            web_server: None,
        }
    }

    #[cfg(feature = "web-preview")]
    pub async fn start_web_preview(&mut self, host: &str) {
        match WebServer::start(Arc::clone(&self.client), host).await {
            Ok(ws) => {
                log::info!("web preview: {}", ws.url());
                self.web_server = Some(ws);
            }
            Err(e) => {
                log::error!("failed to start web preview: {e}");
            }
        }
    }

    pub async fn handle_request(&mut self, req: &McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "initialized" => McpResponse::success(req.id.clone(), json!({})),
            "tools/list" => self.handle_tools_list(req),
            "tools/call" => self.handle_tools_call(req).await,
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
                    "tools": {}
                },
                "serverInfo": {
                    "name": "gui-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    fn handle_tools_list(&self, req: &McpRequest) -> McpResponse {
        McpResponse::success(req.id.clone(), json!({ "tools": all_tools() }))
    }

    async fn handle_tools_call(&mut self, req: &McpRequest) -> McpResponse {
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

        #[cfg(feature = "web-preview")]
        if tool_name == "gui_web_preview" {
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

        let result = match &self.web_server {
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
    handler: Arc<Mutex<McpRequestHandler>>,
}

impl GuiMcpServer {
    pub fn new(client: GuiClient, config: ServerConfig) -> Self {
        let client = Arc::new(client);
        let handler = Arc::new(Mutex::new(McpRequestHandler::new(Arc::clone(&client))));
        Self {
            client,
            config,
            handler,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        #[cfg(feature = "web-preview")]
        if self.config.web_preview {
            self.handler
                .lock()
                .await
                .start_web_preview(&self.config.host)
                .await;
        }

        match self.config.transport {
            TransportMode::Stdio => {
                self.run_stdio().await?;
            }
            TransportMode::Http => {
                self.run_http().await?;
            }
        }

        Ok(())
    }

    async fn run_stdio(&self) -> anyhow::Result<()> {
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();
        let mut stdout = std::io::stdout().lock();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            let request: McpRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let resp = McpResponse::error(None, -32700, format!("parse error: {e}"));
                    write_response(&mut stdout, &resp);
                    continue;
                }
            };

            let mut handler = self.handler.lock().await;
            let response = handler.handle_request(&request).await;
            write_response(&mut stdout, &response);
        }

        Ok(())
    }

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

fn write_response(stdout: &mut impl Write, resp: &McpResponse) {
    match serde_json::to_string(resp) {
        Ok(json) => {
            if let Err(e) = writeln!(stdout, "{json}") {
                log::error!("failed to write response: {e}");
            }
            if let Err(e) = stdout.flush() {
                log::error!("failed to flush stdout: {e}");
            }
        }
        Err(e) => log::error!("failed to serialize response: {e}"),
    }
}
