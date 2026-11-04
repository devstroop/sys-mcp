use crate::gui::GuiClient;
use crate::mcp::handlers::{json_pretty, opt_str_array, str_arg};
use crate::mcp::hub::McpHub;
use crate::protocol::mcp::ToolResult;
use serde_json::Value;
use std::sync::Arc;

static MCP_HUB: std::sync::LazyLock<Arc<McpHub>> =
    std::sync::LazyLock::new(|| Arc::new(McpHub::new()));

pub(crate) async fn handle_mcp_discover(_client: &GuiClient) -> Result<ToolResult, String> {
    let file_based = MCP_HUB.discover().await;
    let npm_based = crate::mcp::hub::discover_npm_mcp_servers().await;

    let mut all_discovered: Vec<crate::mcp::hub::McpServerInfo> = file_based;
    for server in npm_based {
        if !all_discovered.iter().any(|s| s.name == server.name) {
            all_discovered.push(server);
        }
    }

    Ok(ToolResult::text(json_pretty(&all_discovered)?))
}

pub(crate) async fn handle_mcp_list(_client: &GuiClient) -> Result<ToolResult, String> {
    let servers = MCP_HUB.list_servers().await;
    Ok(ToolResult::text(json_pretty(&servers)?))
}

pub(crate) async fn handle_mcp_register(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    let command = str_arg(args, "command")?;

    let args = opt_str_array(args, "args")?;

    let config = crate::mcp::hub::McpServerConfig {
        command: command.to_string(),
        args,
        env: std::collections::HashMap::new(),
        transport: "stdio".to_string(),
    };

    let info = MCP_HUB.register(name.to_string(), config).await?;
    Ok(ToolResult::text(json_pretty(&info)?))
}

pub(crate) async fn handle_mcp_unregister(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    MCP_HUB.unregister(name).await?;
    Ok(ToolResult::text(format!(
        "Unregistered MCP server: {}",
        name
    )))
}

pub(crate) async fn handle_mcp_start(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    let tools = MCP_HUB.start_server(name).await?;
    Ok(ToolResult::text(
        serde_json::json!({
            "message": format!("Started MCP server: {}", name),
            "tools_count": tools.len(),
            "tools": tools
        })
        .to_string(),
    ))
}

pub(crate) async fn handle_mcp_stop(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let name = str_arg(args, "name")?;
    MCP_HUB.stop_server(name).await?;
    Ok(ToolResult::text(format!("Stopped MCP server: {}", name)))
}

pub(crate) async fn handle_mcp_tools(_client: &GuiClient) -> Result<ToolResult, String> {
    let tools = MCP_HUB.list_all_tools().await;
    Ok(ToolResult::text(json_pretty(&tools)?))
}

pub(crate) async fn handle_mcp_tool_groups(_client: &GuiClient) -> Result<ToolResult, String> {
    let groups = MCP_HUB.get_tool_groups().await;
    Ok(ToolResult::text(json_pretty(&groups)?))
}

pub(crate) async fn handle_mcp_exec(
    _client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let server = str_arg(args, "server")?;
    let tool = str_arg(args, "tool")?;
    let tool_args = args.get("args").cloned().unwrap_or(Value::Null);

    let result = MCP_HUB.execute_tool(server, tool, tool_args).await?;
    Ok(ToolResult::text(result.to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════
// Process Manager
// ═══════════════════════════════════════════════════════════════════════════
