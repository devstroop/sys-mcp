//! System process and resource information utilities.
//!
//! Provides [`ProcessManager`] for listing running processes with CPU and memory usage.

use std::process::Command;

#[cfg(windows)]
use std::process::Stdio;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub status: String,
}

pub struct ProcessManager;

impl ProcessManager {
    pub fn list_processes() -> Vec<ProcessInfo> {
        let mut processes = Vec::new();

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Get-Process | Select-Object Id, ProcessName, CPU, WorkingSet64, Responding | ConvertTo-Json"
                ])
                .stdout(Stdio::piped())
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(arr) = json.as_array() {
                            for item in arr {
                                let pid = item.get("Id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                let name = item.get("ProcessName").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let cpu = item.get("CPU").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                let mem = item.get("WorkingSet64").and_then(|v| v.as_u64()).unwrap_or(0);
                                let responding = item.get("Responding").and_then(|v| v.as_bool()).unwrap_or(true);

                                processes.push(ProcessInfo {
                                    pid,
                                    name,
                                    cpu_usage: cpu,
                                    memory_bytes: mem,
                                    status: if responding { "Running".to_string() } else { "Not Responding".to_string() },
                                });
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(output) = Command::new("ps")
                .args(["-eo", "pid,pcpu,pmem,comm"])
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    for line in text.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            if let Ok(pid) = parts[0].parse::<u32>() {
                                processes.push(ProcessInfo {
                                    pid,
                                    name: parts[3..].join(" "),
                                    cpu_usage: parts[1].parse().unwrap_or(0.0),
                                    memory_bytes: 0,
                                    status: "Running".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        processes.sort_by(|a, b| match (a.cpu_usage.is_nan(), b.cpu_usage.is_nan()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => b
                .cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal),
        });
        processes.truncate(100);
        processes
    }

    pub fn kill_process(pid: u32) -> Result<(), String> {
        #[cfg(windows)]
        {
            let output = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output()
                .map_err(|e| e.to_string())?;

            if output.status.success() {
                Ok(())
            } else {
                let err = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to kill process: {}", err))
            }
        }

        #[cfg(not(windows))]
        {
            let output = Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output()
                .map_err(|e| e.to_string())?;

            if output.status.success() {
                Ok(())
            } else {
                let err = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to kill process: {}", err))
            }
        }
    }

    pub fn get_process_info(pid: u32) -> Result<ProcessInfo, String> {
        let processes = Self::list_processes();
        processes
            .into_iter()
            .find(|p| p.pid == pid)
            .ok_or_else(|| format!("Process not found: {}", pid))
    }

    pub fn start_process(command: &str, args: Vec<String>) -> Result<u32, String> {
        #[cfg(windows)]
        {
            let mut cmd = Command::new(command);
            cmd.args(&args);

            let child = cmd.spawn().map_err(|e| format!("Failed to spawn: {}", e))?;
            Ok(child.id())
        }

        #[cfg(not(windows))]
        {
            let mut cmd = Command::new(command);
            cmd.args(&args);

            let child = cmd.spawn().map_err(|e| format!("Failed to spawn: {}", e))?;
            Ok(child.id())
        }
    }
}
