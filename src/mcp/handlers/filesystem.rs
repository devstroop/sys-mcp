#![allow(dead_code)]
use crate::gui::GuiClient;
use crate::mcp::handlers::str_arg;
use crate::mcp::handlers::ui;
use crate::protocol::mcp::ToolResult;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) async fn handle_read_file(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let path = Path::new(path);

    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.is_dir() {
        return Err(format!(
            "Path is a directory, not a file: {}",
            path.display()
        ));
    }

    // Limit file size to 10MB to prevent memory issues
    if metadata.len() > 10_000_000 {
        return Err(format!(
            "File too large ({} bytes). Max size is 10MB.",
            metadata.len()
        ));
    }

    let data = fs::read(path).map_err(|e| e.to_string())?;
    let base64 = ui::base64_encode(&data);

    Ok(ToolResult::text(base64))
}

pub(crate) async fn handle_write_file(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let content = str_arg(args, "content")?;

    let path = Path::new(path);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
    }

    let data = ui::base64_decode(content)?;
    fs::write(path, data).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(ToolResult::text(format!("Written to {}", path.display())))
}

pub(crate) async fn handle_list_dir(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let path = args.get("path").and_then(Value::as_str).unwrap_or(".");
    let path = Path::new(path);

    if !path.exists() {
        return Err(format!("Directory not found: {}", path.display()));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }

    let mut entries: Vec<serde_json::Value> = Vec::new();

    let read_dir = fs::read_dir(path).map_err(|e| e.to_string())?;

    for entry in read_dir {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata().map_err(|e| e.to_string())?;

        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        entries.push(serde_json::json!({
            "name": file_name,
            "type": file_type,
            "size": metadata.len(),
            "modified": modified
        }));
    }

    // Sort by name
    entries.sort_by(|a, b| {
        let a_is_dir = a.get("type").and_then(|t| t.as_str()) == Some("directory");
        let b_is_dir = b.get("type").and_then(|t| t.as_str()) == Some("directory");
        if a_is_dir != b_is_dir {
            b_is_dir.cmp(&a_is_dir)
        } else {
            a.get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .cmp(b.get("name").and_then(|n| n.as_str()).unwrap_or(""))
        }
    });

    Ok(ToolResult::text(
        serde_json::to_string_pretty(&entries).unwrap_or("[]".to_string()),
    ))
}

pub(crate) async fn handle_file_exists(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let path = Path::new(path);

    let exists = path.exists();
    let file_type = if !exists {
        "none"
    } else if path.is_dir() {
        "directory"
    } else if path.is_file() {
        "file"
    } else {
        "other"
    };

    Ok(ToolResult::text(
        serde_json::json!({
            "exists": exists,
            "type": file_type
        })
        .to_string(),
    ))
}

pub(crate) async fn handle_delete_file(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let path = Path::new(path);

    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    if path.is_dir() {
        return Err("Path is a directory, not a file.".to_string());
    }

    fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))?;

    Ok(ToolResult::text(format!("Deleted {}", path.display())))
}

pub(crate) async fn handle_create_dir(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let path = str_arg(args, "path")?;
    let path = Path::new(path);

    fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))?;

    Ok(ToolResult::text(format!("Created {}", path.display())))
}

// ═══════════════════════════════════════════════════════════════════════════
// Base64 helpers
// ═══════════════════════════════════════════════════════════════════════════
