//! MCP **resources**: read-only observations of the local machine.
//!
//! Tools are commands an agent invokes; resources are state it can read. Every
//! resource here is a non-mutating snapshot backed by an existing `GuiClient`
//! call, so reading one has no side effects and costs nothing beyond the call
//! itself. An agent can inspect the machine before choosing tools, and can
//! re-read a resource to detect change, without burning a tool round-trip.

use serde_json::{json, Value};

use crate::gui::GuiClient;

/// Static resource catalogue: `(uri, name, description)`.
pub const RESOURCES: &[(&str, &str, &str)] = &[
    (
        "sys-mcp://system/info",
        "System information",
        "OS, version, hostname and screen size of the host.",
    ),
    (
        "sys-mcp://screen/size",
        "Screen resolution",
        "Primary display resolution in pixels.",
    ),
    (
        "sys-mcp://monitors",
        "Attached monitors",
        "Every attached display with position, resolution and which is primary.",
    ),
    (
        "sys-mcp://windows",
        "Open windows",
        "Titles, ids and states of the currently open top-level windows.",
    ),
    (
        "sys-mcp://window/active",
        "Active window",
        "The window that currently has focus, with its title, id and bounds.",
    ),
];

/// `resources/list` payload.
pub fn list() -> Value {
    let resources: Vec<Value> = RESOURCES
        .iter()
        .map(|(uri, name, description)| {
            json!({
                "uri": uri,
                "name": name,
                "description": description,
                "mimeType": "application/json",
            })
        })
        .collect();
    json!({ "resources": resources })
}

/// `resources/read` for one URI.
///
/// Unknown URIs return an error naming the supported set, so a mis-typed URI
/// is self-correcting rather than a silent empty read.
pub async fn read(client: &GuiClient, uri: &str) -> Result<Value, String> {
    let value = match uri {
        "sys-mcp://system/info" => {
            let info = client.system_info().await.map_err(|e| e.to_string())?;
            serde_json::to_value(info).map_err(|e| e.to_string())?
        }
        "sys-mcp://screen/size" => {
            let r = client.get_screen_size().await.map_err(|e| e.to_string())?;
            json!({ "width": r.width, "height": r.height })
        }
        "sys-mcp://monitors" => {
            let ms = client.list_monitors().await.map_err(|e| e.to_string())?;
            serde_json::to_value(ms).map_err(|e| e.to_string())?
        }
        "sys-mcp://windows" => {
            let ws = client.list_windows().await.map_err(|e| e.to_string())?;
            serde_json::to_value(ws).map_err(|e| e.to_string())?
        }
        "sys-mcp://window/active" => {
            let w = client
                .get_active_window()
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(w).map_err(|e| e.to_string())?
        }
        other => {
            let known: Vec<&str> = RESOURCES.iter().map(|(u, _, _)| *u).collect();
            return Err(format!(
                "unknown resource '{other}'; known resources: {}",
                known.join(", ")
            ));
        }
    };

    // MCP resources/read envelope: contents[] of { uri, mimeType, text }.
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": value.to_string(),
        }]
    }))
}
