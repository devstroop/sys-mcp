use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub config: McpServerConfig,
    pub status: McpServerStatus,
    pub tools: Vec<McpTool>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum McpServerStatus {
    Discovered,
    Starting,
    Running,
    Stopped,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGroup {
    pub name: String,
    pub description: String,
    pub tools: Vec<McpTool>,
}

pub struct McpHub {
    servers: Arc<RwLock<HashMap<String, McpServerInfo>>>,
    #[allow(clippy::type_complexity)]
    processes: Arc<RwLock<HashMap<String, Arc<Mutex<Option<Child>>>>>>,
    discovery_paths: Vec<PathBuf>,
}

impl Default for McpHub {
    fn default() -> Self {
        Self::new()
    }
}

impl McpHub {
    pub fn new() -> Self {
        let mut discovery_paths = vec![];

        if let Ok(home) = std::env::var("HOME") {
            discovery_paths.push(PathBuf::from(&home).join(".mcp"));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            discovery_paths.push(PathBuf::from(&user_profile).join(".mcp"));
        }

        discovery_paths.push(PathBuf::from("."));
        discovery_paths.push(PathBuf::from("./.mcp"));

        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
            discovery_paths,
        }
    }

    pub fn add_discovery_path(&mut self, path: PathBuf) {
        self.discovery_paths.push(path);
    }

    pub async fn discover(&self) -> Vec<McpServerInfo> {
        let mut discovered = Vec::new();

        for path in &self.discovery_paths {
            if !path.exists() {
                continue;
            }

            let entries = match std::fs::read_dir(path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.extension().is_some_and(|e| e == "json") {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(mcp_servers) =
                                config.get("mcpServers").and_then(|v| v.as_object())
                            {
                                for (name, server_config) in mcp_servers {
                                    let cmd = server_config
                                        .get("command")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let args: Vec<String> = server_config
                                        .get("args")
                                        .and_then(|v| v.as_array())
                                        .map(|arr| {
                                            arr.iter()
                                                .filter_map(|v| v.as_str().map(String::from))
                                                .collect()
                                        })
                                        .unwrap_or_default();

                                    if !cmd.is_empty() {
                                        discovered.push(McpServerInfo {
                                            name: name.clone(),
                                            config: McpServerConfig {
                                                command: cmd,
                                                args,
                                                env: HashMap::new(),
                                                transport: "stdio".to_string(),
                                            },
                                            status: McpServerStatus::Discovered,
                                            tools: vec![],
                                            categories: vec!["discovered".to_string()],
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        discovered
    }

    pub async fn register(
        &self,
        name: String,
        config: McpServerConfig,
    ) -> Result<McpServerInfo, String> {
        let mut servers = self.servers.write().await;

        let info = McpServerInfo {
            name: name.clone(),
            config,
            status: McpServerStatus::Discovered,
            tools: vec![],
            categories: vec![],
        };

        servers.insert(name, info.clone());
        Ok(info)
    }

    pub async fn unregister(&self, name: &str) -> Result<(), String> {
        self.stop_server(name).await?;
        let mut servers = self.servers.write().await;
        servers.remove(name);
        Ok(())
    }

    pub async fn start_server(&self, name: &str) -> Result<Vec<McpTool>, String> {
        let config = {
            let servers = self.servers.read().await;
            servers.get(name).cloned()
        };

        let config = config.ok_or_else(|| format!("Server not found: {}", name))?;

        let mut processes = self.processes.write().await;
        if processes.contains_key(name) {
            return Err(format!("Server already running: {}", name));
        }

        let mut cmd = Command::new(&config.config.command);
        cmd.args(&config.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &config.config.env {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", config.config.command, e))?;

        let child_lock = Arc::new(Mutex::new(Some(child)));
        processes.insert(name.to_string(), child_lock);

        {
            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(name) {
                server.status = McpServerStatus::Running;
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let tools = self.list_tools(name).await?;
        Ok(tools)
    }

    pub async fn stop_server(&self, name: &str) -> Result<(), String> {
        let mut processes = self.processes.write().await;
        if let Some(child_lock) = processes.remove(name) {
            let mut child_guard = child_lock.lock().await;
            if let Some(mut child) = child_guard.take() {
                let _ = child.kill().await;
            }
        }

        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(name) {
            server.status = McpServerStatus::Stopped;
            server.tools.clear();
        }

        Ok(())
    }

    pub async fn list_servers(&self) -> Vec<McpServerInfo> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    pub async fn list_tools(&self, name: &str) -> Result<Vec<McpTool>, String> {
        let servers = self.servers.read().await;
        let server = servers
            .get(name)
            .ok_or_else(|| format!("Server not found: {}", name))?;

        if server.status != McpServerStatus::Running {
            return Ok(vec![]);
        }

        Ok(server.tools.clone())
    }

    pub async fn list_all_tools(&self) -> Vec<McpTool> {
        let servers = self.servers.read().await;
        let mut all_tools = Vec::new();

        for server in servers.values() {
            if server.status == McpServerStatus::Running {
                all_tools.extend(server.tools.clone());
            }
        }

        all_tools
    }

    pub async fn get_tool_groups(&self) -> Vec<ToolGroup> {
        let servers = self.servers.read().await;
        let mut groups: HashMap<String, Vec<McpTool>> = HashMap::new();

        for server in servers.values() {
            if server.status != McpServerStatus::Running {
                continue;
            }

            for tool in &server.tools {
                groups
                    .entry(tool.category.clone())
                    .or_default()
                    .push(tool.clone());
            }
        }

        groups
            .into_iter()
            .map(|(name, tools)| ToolGroup {
                name: name.clone(),
                description: format!("Tools from {}", name),
                tools,
            })
            .collect()
    }

    pub async fn execute_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let servers = self.servers.read().await;
        let server = servers
            .get(server_name)
            .ok_or_else(|| format!("Server not found: {}", server_name))?;

        if server.status != McpServerStatus::Running {
            return Err(format!("Server not running: {}", server_name));
        }

        drop(servers);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": args
            }
        });

        let response = self.send_jsonrpc(server_name, request).await?;
        Ok(response)
    }

    async fn send_jsonrpc(
        &self,
        server_name: &str,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let processes = self.processes.read().await;
        let child_lock = processes
            .get(server_name)
            .ok_or_else(|| format!("No process for: {}", server_name))?;

        let mut child_guard = child_lock.lock().await;
        let child = child_guard
            .as_mut()
            .ok_or_else(|| format!("Process died: {}", server_name))?;

        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| format!("No stdin for: {}", server_name))?;
        let stdout = child
            .stdout
            .as_mut()
            .ok_or_else(|| format!("No stdout for: {}", server_name))?;

        let request_str = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(stdout);

        stdin
            .write_all(format!("{}\n", request_str).as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stdin.flush().await.map_err(|e| e.to_string())?;

        let mut response = String::new();
        reader
            .read_line(&mut response)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::from_str(&response).map_err(|e| format!("Invalid JSON response: {}", e))
    }
}

pub async fn discover_npm_mcp_servers() -> Vec<McpServerInfo> {
    let output = Command::new("npm")
        .args(["list", "-g", "--json", "--depth=0"])
        .output()
        .await;

    if let Ok(output) = output {
        if output.status.success() {
            if let Ok(json) =
                serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout))
            {
                let mut servers = Vec::new();

                if let Some(dependencies) = json.get("dependencies").and_then(|v| v.as_object()) {
                    for (name, _) in dependencies {
                        if name.contains("mcp") || name.contains("-mcp") {
                            servers.push(McpServerInfo {
                                name: name.clone(),
                                config: McpServerConfig {
                                    command: "npx".to_string(),
                                    args: vec!["-y".to_string(), format!("{}@latest", name)],
                                    env: HashMap::new(),
                                    transport: "stdio".to_string(),
                                },
                                status: McpServerStatus::Discovered,
                                tools: vec![],
                                categories: vec!["npm".to_string()],
                            });
                        }
                    }
                }

                return servers;
            }
        }
    }

    vec![]
}
