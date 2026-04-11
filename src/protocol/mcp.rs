use serde::{Deserialize, Serialize};
use serde_json::Value;

// ─── JSON-RPC transport ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum McpResponse {
    Success {
        jsonrpc: String,
        id: Option<Value>,
        result: Value,
    },
    Error {
        jsonrpc: String,
        id: Option<Value>,
        error: McpError,
    },
}

impl McpResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self::Success {
            jsonrpc: "2.0".into(),
            id,
            result,
        }
    }

    pub fn error(id: Option<Value>, code: i64, message: impl Into<String>) -> Self {
        Self::Error {
            jsonrpc: "2.0".into(),
            id,
            error: McpError {
                code,
                message: message.into(),
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct McpError {
    pub code: i64,
    pub message: String,
}

// ─── Tool result payload ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub content: Vec<ContentItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentItem {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

impl ContentItem {
    pub fn text(t: impl Into<String>) -> Self {
        Self {
            content_type: "text".into(),
            text: Some(t.into()),
            mime_type: None,
            data: None,
        }
    }

    pub fn image_base64(mime: &str, data: impl Into<String>) -> Self {
        Self {
            content_type: "image".into(),
            text: None,
            mime_type: Some(mime.into()),
            data: Some(data.into()),
        }
    }
}

impl ToolResult {
    pub fn text(t: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem::text(t)],
            is_error: None,
        }
    }

    pub fn image(mime: &str, data_base64: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem::image_base64(mime, data_base64)],
            is_error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem::text(msg)],
            is_error: Some(true),
        }
    }
}
