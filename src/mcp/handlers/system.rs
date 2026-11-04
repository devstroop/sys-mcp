#![allow(dead_code)]
use crate::gui::GuiClient;
use crate::mcp::handlers::{json_pretty, opt_str_array, str_arg, u64_arg};
use crate::protocol::mcp::ToolResult;
use crate::sysinfo::ProcessManager;
use crate::system::{LogLevel, LogViewer, NetworkManager, ServiceManager, SystemMonitor};
use serde_json::Value;

pub(crate) async fn handle_system_info(client: &GuiClient) -> Result<ToolResult, String> {
    let info = client.system_info().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(json_pretty(&info)?))
}

// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_wait(args: &Value) -> Result<ToolResult, String> {
    let ms = args.get("ms").and_then(Value::as_u64).unwrap_or(500);
    let ms = ms.min(30000); // Cap at 30 seconds
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    Ok(ToolResult::text(format!("Waited {ms}ms.")))
}

pub(crate) async fn handle_process_list(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    // list_processes shells out with blocking Command::output(); keep it off
    // the async runtime.
    let mut processes = tokio::task::spawn_blocking(ProcessManager::list_processes)
        .await
        .map_err(|e| format!("Process list task failed: {e}"))?;
    processes.truncate(limit);

    Ok(ToolResult::text(json_pretty(&processes)?))
}

pub(crate) async fn handle_process_kill(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let pid = u64_arg(args, "pid")? as u32;

    tokio::task::spawn_blocking(move || ProcessManager::kill_process(pid))
        .await
        .map_err(|e| format!("Kill process task failed: {e}"))??;

    Ok(ToolResult::text(format!("Killed process {}", pid)))
}

pub(crate) async fn handle_process_info(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let pid = u64_arg(args, "pid")? as u32;

    let info = tokio::task::spawn_blocking(move || ProcessManager::get_process_info(pid))
        .await
        .map_err(|e| format!("Process info task failed: {e}"))??;

    Ok(ToolResult::text(json_pretty(&info)?))
}

pub(crate) async fn handle_process_start(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let command = str_arg(args, "command")?.to_string();
    let args = opt_str_array(args, "args")?;

    let command_name = command.clone();
    let pid = tokio::task::spawn_blocking(move || ProcessManager::start_process(&command, args))
        .await
        .map_err(|e| format!("Start process task failed: {e}"))??;

    Ok(ToolResult::text(
        serde_json::json!({
            "message": format!("Started process {}", command_name),
            "pid": pid
        })
        .to_string(),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// Service Manager
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_service_list(_client: &GuiClient) -> Result<ToolResult, String> {
    let services = tokio::task::spawn_blocking(ServiceManager::list_services)
        .await
        .map_err(|e| format!("Service list task failed: {e}"))?;
    Ok(ToolResult::text(json_pretty(&services)?))
}

pub(crate) async fn handle_service_start(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?.to_string();
    let name_arg = name.clone();
    tokio::task::spawn_blocking(move || ServiceManager::start_service(&name_arg))
        .await
        .map_err(|e| format!("Start service task failed: {e}"))??;
    Ok(ToolResult::text(format!("Started service: {}", name)))
}

pub(crate) async fn handle_service_stop(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?.to_string();
    let name_arg = name.clone();
    tokio::task::spawn_blocking(move || ServiceManager::stop_service(&name_arg))
        .await
        .map_err(|e| format!("Stop service task failed: {e}"))??;
    Ok(ToolResult::text(format!("Stopped service: {}", name)))
}
pub(crate) async fn handle_service_status(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?.to_string();
    let status = tokio::task::spawn_blocking(move || ServiceManager::service_status(&name))
        .await
        .map_err(|e| format!("Service status task failed: {e}"))??;
    Ok(ToolResult::text(json_pretty(&status)?))
}

// ═══════════════════════════════════════════════════════════════════════════
// Network Tools
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_network_info(_client: &GuiClient) -> Result<ToolResult, String> {
    let info = tokio::task::spawn_blocking(NetworkManager::get_info)
        .await
        .map_err(|e| format!("Network info task failed: {e}"))??;
    Ok(ToolResult::text(json_pretty(&info)?))
}

pub(crate) async fn handle_network_connections(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    // list_connections shells out with blocking Command::output(); keep it
    // off the async runtime and honor the declared `limit` argument.
    let mut connections = tokio::task::spawn_blocking(NetworkManager::list_connections)
        .await
        .map_err(|e| format!("Network connections task failed: {e}"))?;
    connections.truncate(limit);
    Ok(ToolResult::text(json_pretty(&connections)?))
}
// ═══════════════════════════════════════════════════════════════════════════
// System Monitoring
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_system_stats(_client: &GuiClient) -> Result<ToolResult, String> {
    let stats = tokio::task::spawn_blocking(SystemMonitor::get_stats)
        .await
        .map_err(|e| format!("System stats task failed: {e}"))??;
    Ok(ToolResult::text(json_pretty(&stats)?))
}
pub(crate) async fn handle_disk_usage(_client: &GuiClient) -> Result<ToolResult, String> {
    let usage = tokio::task::spawn_blocking(SystemMonitor::get_disk_usage)
        .await
        .map_err(|e| format!("Disk usage task failed: {e}"))?;
    Ok(ToolResult::text(json_pretty(&usage)?))
}

// ═══════════════════════════════════════════════════════════════════════════
// Log Viewer
// ═══════════════════════════════════════════════════════════════════════════

/// Longest `system_logs` request the tool will serve. Above this the call is
/// refused rather than silently capped: returning fewer entries than were asked
/// for is indistinguishable from a short log.
const MAX_LOG_COUNT: u64 = 1000;

pub(crate) async fn handle_system_logs(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let count = match args.get("count") {
        None | Some(Value::Null) => 50,
        Some(v) => match v.as_u64() {
            Some(0) | None => {
                return Err(format!(
                    "invalid argument: 'count' must be a whole number between 1 and {MAX_LOG_COUNT}, got {v}"
                ))
            }
            Some(n) if n > MAX_LOG_COUNT => {
                return Err(format!(
                    "invalid argument: 'count' = {n} exceeds the {MAX_LOG_COUNT}-entry limit for system_logs"
                ))
            }
            Some(n) => n as usize,
        },
    };

    // Absent means every level. It used to mean "Information", which returned a
    // reassuring list of informational events while hiding every error.
    let level = match args.get("level") {
        None | Some(Value::Null) => None,
        Some(Value::String(raw)) => Some(LogLevel::parse(raw).ok_or_else(|| format!(
            "invalid argument: 'level' = {raw:?} is not a known severity. Use Critical, Error, Warning, Information or Verbose, or omit 'level' to read them all."
        ))?),
        Some(v) => return Err(format!("invalid argument: 'level' must be a string, got {v}")),
    };

    let log = match args.get("log") {
        None | Some(Value::Null) => None,
        Some(Value::String(raw)) => Some(raw.clone()),
        Some(v) => return Err(format!("invalid argument: 'log' must be a string, got {v}")),
    };

    // Composed before the call moves its arguments into the blocking closure,
    // so the empty-result message can name the filter that produced it.
    let filter_text = format!(
        "{}{}",
        match &level {
            Some(level) => format!("level {} ", level.as_str()),
            None => "every level ".to_string(),
        },
        match &log {
            Some(log) => format!("in the '{log}' log"),
            None => "in the platform log".to_string(),
        }
    );

    let logs = tokio::task::spawn_blocking(move || LogViewer::get_system_logs(count, level, log))
        .await
        .map_err(|e| format!("System logs task failed: {e}"))??;
    if logs.is_empty() {
        // Say so in words: an empty array alone cannot tell "the log is clean"
        // from "the filter matched nothing".
        return Ok(ToolResult::text(format!("No entries for {filter_text}.")));
    }
    Ok(ToolResult::text(json_pretty(&logs)?))
}
