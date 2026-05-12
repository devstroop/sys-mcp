use std::collections::HashMap;
use sys_mcp::mcp::hub::McpServerConfig;
use sys_mcp::McpHub;

#[test]
fn test_mcp_hub_new() {
    let hub = McpHub::new();
    let servers = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hub.list_servers());
    assert_eq!(servers.len(), 0);
}

#[test]
fn test_mcp_hub_register() {
    let hub = McpHub::new();

    let config = McpServerConfig {
        command: "echo".to_string(),
        args: vec!["test".to_string()],
        env: HashMap::new(),
        transport: "stdio".to_string(),
    };

    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hub.register("test-server".to_string(), config));

    assert!(result.is_ok());

    let servers = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hub.list_servers());
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "test-server");
}

#[test]
fn test_mcp_hub_unregister() {
    let hub = McpHub::new();

    let config = McpServerConfig {
        command: "echo".to_string(),
        args: vec![],
        env: HashMap::new(),
        transport: "stdio".to_string(),
    };

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hub.register("test-server".to_string(), config))
        .unwrap();

    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hub.unregister("test-server"));

    assert!(result.is_ok());

    let servers = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hub.list_servers());
    assert!(servers.is_empty());
}

#[tokio::test]
async fn test_mcp_hub_register_async() {
    let hub = McpHub::new();

    let config = McpServerConfig {
        command: "echo".to_string(),
        args: vec!["test".to_string()],
        env: HashMap::new(),
        transport: "stdio".to_string(),
    };

    let result = hub.register("async-test".to_string(), config).await;
    assert!(result.is_ok());

    let servers = hub.list_servers().await;
    assert_eq!(servers.len(), 1);
}

#[tokio::test]
async fn test_mcp_hub_list_tools_empty() {
    let hub = McpHub::new();
    let tools = hub.list_all_tools().await;
    assert!(tools.is_empty());
}

#[tokio::test]
async fn test_mcp_hub_tool_groups_empty() {
    let hub = McpHub::new();
    let groups = hub.get_tool_groups().await;
    assert!(groups.is_empty());
}

#[test]
fn test_mcp_server_config() {
    let config = McpServerConfig {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "chrome-devtools-mcp@latest".to_string()],
        env: HashMap::new(),
        transport: "stdio".to_string(),
    };

    assert_eq!(config.command, "npx");
    assert_eq!(config.args.len(), 2);
    assert_eq!(config.transport, "stdio");
}
