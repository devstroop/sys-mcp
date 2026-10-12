#![allow(dead_code)]
use crate::gui::GuiClient;
use crate::mcp::handlers::{str_arg, u64_arg};
use crate::protocol::mcp::ToolResult;
use crate::sysinfo::ProcessManager;
use crate::system::{LogViewer, NetworkManager, ServiceManager, SystemMonitor};
use serde_json::Value;

pub(crate) async fn handle_system_info(client: &GuiClient) -> Result<ToolResult, String> {
    let info = client.system_info().await.map_err(|e| e.to_string())?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&info).unwrap_or_default(),
    ))
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
    let mut processes = tokio::task::spawn_blocking(move || ProcessManager::list_processes())
        .await
        .map_err(|e| format!("Process list task failed: {e}"))?;
    processes.truncate(limit);

    Ok(ToolResult::text(
        serde_json::to_string_pretty(&processes).unwrap_or("[]".to_string()),
    ))
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

    Ok(ToolResult::text(
        serde_json::to_string_pretty(&info).unwrap_or("{}".to_string()),
    ))
}

pub(crate) async fn handle_process_start(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let command = str_arg(args, "command")?.to_string();
    let args: Vec<String> = args
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

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
    let services = tokio::task::spawn_blocking(move || ServiceManager::list_services())
        .await
        .map_err(|e| format!("Service list task failed: {e}"))?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&services).unwrap_or("[]".to_string()),
    ))
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
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&status).unwrap_or("{}".to_string()),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// Network Tools
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_network_info(_client: &GuiClient) -> Result<ToolResult, String> {
    let info = tokio::task::spawn_blocking(move || NetworkManager::get_info())
        .await
        .map_err(|e| format!("Network info task failed: {e}"))??;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&info).unwrap_or("{}".to_string()),
    ))
}

pub(crate) async fn handle_network_connections(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    // list_connections shells out with blocking Command::output(); keep it
    // off the async runtime and honor the declared `limit` argument.
    let mut connections = tokio::task::spawn_blocking(move || NetworkManager::list_connections())
        .await
        .map_err(|e| format!("Network connections task failed: {e}"))?;
    connections.truncate(limit);
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&connections).unwrap_or("[]".to_string()),
    ))
}
// ═══════════════════════════════════════════════════════════════════════════
// System Monitoring
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_system_stats(_client: &GuiClient) -> Result<ToolResult, String> {
    let stats = tokio::task::spawn_blocking(move || SystemMonitor::get_stats())
        .await
        .map_err(|e| format!("System stats task failed: {e}"))??;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&stats).unwrap_or("{}".to_string()),
    ))
}
pub(crate) async fn handle_disk_usage(_client: &GuiClient) -> Result<ToolResult, String> {
    let usage = tokio::task::spawn_blocking(move || SystemMonitor::get_disk_usage())
        .await
        .map_err(|e| format!("Disk usage task failed: {e}"))?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&usage).unwrap_or("[]".to_string()),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// Log Viewer
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) async fn handle_system_logs(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let level = args.get("level").and_then(|v| v.as_str()).map(String::from);

    let logs = tokio::task::spawn_blocking(move || LogViewer::get_system_logs(count, level))
        .await
        .map_err(|e| format!("System logs task failed: {e}"))?;
    Ok(ToolResult::text(
        serde_json::to_string_pretty(&logs).unwrap_or("[]".to_string()),
    ))
}
