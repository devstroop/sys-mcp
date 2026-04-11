use std::io::Write;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::config::ServerConfig;
use crate::gui::GuiClient;
use crate::mcp::handlers::handle_tool_call;
use crate::mcp::tools::all_tools;
use crate::protocol::mcp::{McpRequest, McpResponse};

#[cfg(feature = "web-preview")]
use crate::web::server::WebServer;

pub struct GuiMcpServer {
    client: Arc<GuiClient>,
    #[allow(dead_code)]
    config: ServerConfig,
    #[cfg(feature = "web-preview")]
    web_server: Option<WebServer>,
}

impl GuiMcpServer {
    pub fn new(client: GuiClient, config: ServerConfig) -> Self {
        Self {
            client: Arc::new(client),
            config,
            #[cfg(feature = "web-preview")]
            web_server: None,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Auto-start web preview (local backend is always "connected")
        #[cfg(feature = "web-preview")]
        if self.config.web_preview {
            self.start_web_preview().await;
        }

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

            let response = self.handle_request(&request).await;
            write_response(&mut stdout, &response);
        }

        Ok(())
    }

    async fn handle_request(&mut self, req: &McpRequest) -> McpResponse {
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

        // Handle web_preview directly — needs mutable access to web_server state
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
    async fn start_web_preview(&mut self) {
        match WebServer::start(Arc::clone(&self.client)).await {
            Ok(ws) => {
                log::info!("web preview: {}", ws.url());
                self.web_server = Some(ws);
            }
            Err(e) => {
                log::error!("failed to start web preview: {e}");
            }
        }
    }

    #[cfg(feature = "web-preview")]
    fn handle_web_preview(&self, req: &McpRequest) -> McpResponse {
        use crate::protocol::mcp::{ContentItem, ToolResult};

        let result = match &self.web_server {
            Some(ws) => ToolResult {
                content: vec![ContentItem::text(ws.url())],
                is_error: None,
            },
            None => ToolResult::error("web preview is not running — start server with --web-preview flag"),
        };

        McpResponse::success(
            req.id.clone(),
            serde_json::to_value(&result).unwrap_or_default(),
        )
    }
}

fn write_response(stdout: &mut impl Write, resp: &McpResponse) {
    if let Ok(json) = serde_json::to_string(resp) {
        let _ = writeln!(stdout, "{json}");
        let _ = stdout.flush();
    }
}
