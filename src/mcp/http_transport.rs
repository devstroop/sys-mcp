use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, HeaderName, Method, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use crate::config::ServerConfig;
use crate::mcp::session::SessionManagerHandle;
use crate::protocol::mcp::McpRequest;

pub struct HttpServer {
    config: ServerConfig,
    session_mgr: SessionManagerHandle,
    mcp_handler: Arc<Mutex<super::server::McpRequestHandler>>,
}

impl HttpServer {
    pub fn new(
        config: ServerConfig,
        session_mgr: SessionManagerHandle,
        mcp_handler: Arc<Mutex<super::server::McpRequestHandler>>,
    ) -> Self {
        Self {
            config,
            session_mgr,
            mcp_handler,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("failed to parse socket address: {e}"))?;

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers(Any)
            .expose_headers([HeaderName::from_static("mcp-session-id")]);

        let state = HttpState {
            session_mgr: self.session_mgr.clone(),
            mcp_handler: self.mcp_handler.clone(),
        };

        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/mcp", post(mcp_handler))
            .route("/mcp", delete(mcp_delete_handler))
            .route("/mcp", get(mcp_get_handler))
            .layer(cors)
            .with_state(state);

        log::info!("gui-mcp HTTP server listening on http://{}", addr);
        log::info!("MCP endpoint: http://{}/mcp", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await?;

        Ok(())
    }
}

#[derive(Clone)]
struct HttpState {
    session_mgr: SessionManagerHandle,
    mcp_handler: Arc<Mutex<super::server::McpRequestHandler>>,
}

async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "server": "gui-mcp",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn mcp_get_handler() -> impl IntoResponse {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(json!({"error":"Use POST for MCP requests"})),
    )
}

async fn mcp_delete_handler(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let session_id = match headers.get("mcp-session-id") {
        Some(v) => v.to_str().ok(),
        None => {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing Mcp-Session-Id header"})),
            ));
        }
    };

    if let Some(sid) = session_id {
        let mut mgr = state.session_mgr.lock().await;
        if mgr.remove(sid) {
            return Ok((
                StatusCode::OK,
                Json(json!({"message": "Session terminated"})),
            ));
        }
    }

    Ok((
        StatusCode::NOT_FOUND,
        Json(json!({"error": "Session not found"})),
    ))
}

async fn mcp_handler(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<impl IntoResponse, StatusCode> {
    // Cleanup expired sessions periodically
    {
        let mut mgr = state.session_mgr.lock().await;
        mgr.cleanup_expired();
    }

    // Get or create session
    let session_id = headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let session = {
        let mut mgr = state.session_mgr.lock().await;
        mgr.get_or_create(session_id.as_deref())
    };

    // Parse MCP request
    let mcp_request: McpRequest = match serde_json::from_value(body) {
        Ok(req) => req,
        Err(e) => {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("parse error: {e}")})),
            ));
        }
    };

    // Handle MCP request
    let mut handler = state.mcp_handler.lock().await;
    let response = handler.handle_request(&mcp_request).await;

    // Serialize response
    let response_json = match serde_json::to_string(&response) {
        Ok(json) => json,
        Err(e) => {
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("serialization error: {e}")})),
            ));
        }
    };

    let status = if response_json.is_empty() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::OK
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        axum::http::HeaderValue::from_static("mcp-session-id"),
        session.id.to_string().parse().unwrap_or_default(),
    );

    Ok((status, headers, Json(serde_json::from_str::<Value>(&response_json).unwrap_or_default())))
}