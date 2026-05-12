#![allow(dead_code)]
use crate::gui::GuiClient;
use crate::mcp::handlers::str_arg;
use crate::protocol::mcp::ToolResult;
use crate::terminal::{PtyManager, TerminalHandle};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use uuid::Uuid;

static SHELL_SESSIONS: std::sync::LazyLock<Arc<RwLock<HashMap<String, SessionState>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

static PTY_MANAGER: std::sync::LazyLock<Arc<PtyManager>> =
    std::sync::LazyLock::new(|| Arc::new(PtyManager::new()));

struct SessionState {
    handle: TerminalHandle,
    output_buffer: Arc<Mutex<Vec<u8>>>,
}

pub(crate) async fn handle_shell_exec(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let command = str_arg(args, "command")?;
    let cwd = args.get("cwd").and_then(Value::as_str);

    let session_id = format!("exec_{}", Uuid::new_v4());
    let output_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let output_buffer_clone = output_buffer.clone();

    let handle = PTY_MANAGER
        .spawn(
            session_id.clone(),
            80,
            24,
            cwd.map(String::from),
            vec![],
            move |data| {
                let mut buf = output_buffer_clone.lock().unwrap();
                buf.extend(data);
            },
        )
        .await
        .map_err(|e| format!("Failed to spawn terminal: {}", e))?;

    // Send command with newline
    handle
        .input_tx
        .send(format!("{}\n", command).into_bytes())
        .await
        .map_err(|e| format!("Failed to send command: {}", e))?;

    // Close the terminal (signals EOF to the process)
    let _ = PTY_MANAGER.close(&session_id).await;

    // Give the process a moment to finish and flush output
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Get output
    let output = spawn_blocking_get_buffer(output_buffer).await;
    let output_str = String::from_utf8(output).unwrap_or_default();

    // Clean up ANSI escape codes for display
    let cleaned = strip_ansi_codes(&output_str);

    Ok(ToolResult::text(cleaned))
}

pub(crate) async fn handle_shell_open(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let cwd = args.get("cwd").and_then(Value::as_str).map(String::from);
    let cols = args.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
    let rows = args.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;

    let session_id = Uuid::new_v4().to_string();
    let output_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let output_buffer_clone = output_buffer.clone();

    let handle = PTY_MANAGER
        .spawn(session_id.clone(), cols, rows, cwd, vec![], move |data| {
            let mut buf = output_buffer_clone.lock().unwrap();
            buf.extend(data);
        })
        .await
        .map_err(|e| format!("Failed to open shell: {}", e))?;

    let state = SessionState {
        handle,
        output_buffer,
    };

    SHELL_SESSIONS
        .write()
        .map_err(|e| format!("sessions poisoned: {}", e))?
        .insert(session_id.clone(), state);

    Ok(ToolResult::text(serde_json::json!({
        "session_id": session_id,
        "message": "Shell session opened. Use gui_shell_write to send commands, gui_shell_read to get output, gui_shell_close to close."
    }).to_string()))
}

pub(crate) async fn handle_shell_write(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let session_id = str_arg(args, "session_id")?;
    let input = str_arg(args, "input")?;
    let data = input.as_bytes().to_vec();
    let len = data.len();

    let input_tx = {
        let sessions = SHELL_SESSIONS
            .read()
            .map_err(|e| format!("sessions poisoned: {}", e))?;
        sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?
            .handle
            .input_tx
            .clone()
    };

    input_tx
        .send(data)
        .await
        .map_err(|e| format!("Failed to write to shell: {}", e))?;

    Ok(ToolResult::text(format!(
        "Sent {} bytes to session {}",
        len, session_id
    )))
}

pub(crate) async fn handle_shell_read(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let session_id = str_arg(args, "session_id")?;

    let output_buffer = {
        let sessions = SHELL_SESSIONS
            .read()
            .map_err(|e| format!("sessions poisoned: {}", e))?;
        sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?
            .output_buffer
            .clone()
    };

    let output = spawn_blocking_get_buffer(output_buffer).await;
    let output_str = String::from_utf8(output).unwrap_or_default();
    let cleaned = strip_ansi_codes(&output_str);

    Ok(ToolResult::text(cleaned))
}

pub(crate) async fn handle_shell_close(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let session_id = str_arg(args, "session_id")?;

    PTY_MANAGER
        .close(session_id)
        .await
        .map_err(|e| format!("Failed to close shell: {}", e))?;

    SHELL_SESSIONS
        .write()
        .map_err(|e| format!("sessions poisoned: {}", e))?
        .remove(session_id);

    Ok(ToolResult::text(format!("Closed session {}", session_id)))
}

/// Helper to read a buffer from a std::sync::Mutex inside an async context.
pub(crate) async fn handle_shell_list(_client: &GuiClient) -> Result<ToolResult, String> {
    let sessions = SHELL_SESSIONS
        .read()
        .map_err(|e| format!("sessions poisoned: {}", e))?;
    let ids: Vec<String> = sessions.keys().cloned().collect();

    Ok(ToolResult::text(
        serde_json::to_string_pretty(&ids).unwrap_or("[]".to_string()),
    ))
}

async fn spawn_blocking_get_buffer(buf: Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    let buf_clone = buf.clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = buf_clone.lock().unwrap();
        std::mem::take(&mut *guard)
    })
    .await
    .unwrap_or_default()
}

fn strip_ansi_codes(s: &str) -> String {
    // Remove ANSI escape sequences using a simple state machine.
    // Handles CSI sequences (most common), and strips everything from ESC
    // until a known terminator letter.
    let mut result = String::with_capacity(s.len());
    let mut skip = false;

    for c in s.chars() {
        if c == '\u{1B}' {
            skip = true;
        } else if skip {
            // Terminator bytes for CSI and other escape sequences
            if (c.is_ascii_alphabetic() || c == '@' || c == '`')
                && (c == 'm'
                    || c == 'H'
                    || c == 'J'
                    || c == 'K'
                    || c == 'A'
                    || c == 'B'
                    || c == 'C'
                    || c == 'D'
                    || c == 'P'
                    || c == 'S'
                    || c == 'T'
                    || c == 'f')
            {
                skip = false;
            }
        } else {
            result.push(c);
        }
    }

    result
}
