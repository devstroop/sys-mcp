//! Windows system service management utilities.
//!
//! Provides [`ServiceManager`] for querying and managing Windows services.

use std::process::Command;
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::process::Stdio;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub status: String,
    pub start_type: String,
}

pub struct ServiceManager;

impl ServiceManager {
    pub fn list_services() -> Vec<ServiceInfo> {
        let mut services = Vec::new();

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Get-Service | Select-Object Name, DisplayName, Status, StartType | ConvertTo-Json"
                ])
                .stdout(Stdio::piped())
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(arr) = json.as_array() {
                            for item in arr {
                                let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let display = item.get("DisplayName").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let status = format!("{:?}", item.get("Status").and_then(|v| v.as_str()).unwrap_or("Unknown"));
                                let start_type = format!("{:?}", item.get("StartType").and_then(|v| v.as_str()).unwrap_or("Manual"));

                                services.push(ServiceInfo {
                                    name,
                                    display_name: display,
                                    status,
                                    start_type,
                                });
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(output) = Command::new("systemctl")
                .args(["list-units", "--all", "--type=service", "--no-pager", "--no-legend"])
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    for line in text.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            services.push(ServiceInfo {
                                name: parts[0].to_string(),
                                display_name: parts[0].to_string(),
                                status: parts[3].to_string(),
                                start_type: parts[1].to_string(),
                            });
                        }
                    }
                }
            }
        }

        services
    }

    pub fn start_service(name: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("Start-Service -Name '{}'", name)])
                .output()
                .map_err(|e| e.to_string())?;

            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }

        #[cfg(not(windows))]
        {
            let output = Command::new("systemctl")
                .args(["start", name])
                .output()
                .map_err(|e| e.to_string())?;

            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
    }

    pub fn stop_service(name: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("Stop-Service -Name '{}'", name)])
                .output()
                .map_err(|e| e.to_string())?;

            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }

        #[cfg(not(windows))]
        {
            let output = Command::new("systemctl")
                .args(["stop", name])
                .output()
                .map_err(|e| e.to_string())?;

            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
    }

    pub fn service_status(name: &str) -> Result<ServiceInfo, String> {
        let services = Self::list_services();
        services.into_iter()
            .find(|s| s.name == name)
            .ok_or_else(|| format!("Service not found: {}", name))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub hostname: String,
    pub ip_addresses: Vec<String>,
    pub default_gateway: Option<String>,
    pub dns_servers: Vec<String>,
}

pub struct NetworkManager;

impl NetworkManager {
    pub fn get_info() -> Result<NetworkInfo, String> {
        let mut info = NetworkInfo {
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default(),
            ip_addresses: Vec::new(),
            default_gateway: None,
            dns_servers: Vec::new(),
        };

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Get-NetIPConfiguration | Select-Object InterfaceAlias, IPv4Address, IPv4DefaultGateway, DNSServer | ConvertTo-Json"
                ])
                .stdout(Stdio::piped())
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(arr) = json.as_array() {
                            for item in arr {
                                if let Some(ip) = item.get("IPv4Address").and_then(|v| v.as_array()).and_then(|a| a.first()) {
                                    if let Some(addr) = ip.get("IPAddress").and_then(|v| v.as_str()) {
                                        if !addr.starts_with("169.254") {
                                            info.ip_addresses.push(addr.to_string());
                                        }
                                    }
                                }
                                if let Some(gw) = item.get("IPv4DefaultGateway").and_then(|v| v.as_array()).and_then(|a| a.first()) {
                                    if let Some(addr) = gw.get("NextHop").and_then(|v| v.as_str()) {
                                        info.default_gateway = Some(addr.to_string());
                                    }
                                }
                                if let Some(dns) = item.get("DNSServer").and_then(|v| v.as_array()) {
                                    for server in dns {
                                        if let Some(addr) = server.get("ServerAddresses").and_then(|v| v.as_array()) {
                                            for a in addr {
                                                if let Some(ip) = a.as_str() {
                                                    info.dns_servers.push(ip.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(output) = Command::new("hostname")
                .args(["-I"])
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    info.ip_addresses = text.trim().split_whitespace().map(String::from).collect();
                }
            }

            if let Ok(output) = Command::new("route")
                .args(["-n"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    if line.starts_with("0.0.0.0") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            info.default_gateway = Some(parts[1].to_string());
                        }
                    }
                }
            }

            if let Ok(output) = Command::new("cat")
                .args(["/etc/resolv.conf"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    if line.starts_with("nameserver") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            info.dns_servers.push(parts[1].to_string());
                        }
                    }
                }
            }
        }

        Ok(info)
    }

    pub fn list_connections() -> Vec<serde_json::Value> {
        let mut connections = Vec::new();

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("netstat")
                .args(["-ano"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(4) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let protocol = parts[0].to_string();
                        if protocol == "TCP" || protocol == "UDP" {
                            let local_addr = parts[1].to_string();
                            let state = if parts.len() >= 4 { parts[3].to_string() } else { "-".to_string() };
                            let pid = parts.last().unwrap_or(&"0").parse::<u32>().unwrap_or(0);

                            connections.push(serde_json::json!({
                                "protocol": protocol,
                                "local_address": local_addr,
                                "state": state,
                                "pid": pid
                            }));
                        }
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(output) = Command::new("ss")
                .args(["-tunap"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 6 {
                        connections.push(serde_json::json!({
                            "protocol": parts[0],
                            "local_address": parts[4],
                            "peer_address": parts[5],
                            "state": parts[1]
                        }));
                    }
                }
            }
        }

        connections.truncate(100);
        connections
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub cpu_usage_percent: f32,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub memory_percent: f32,
    pub disk_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_percent: f32,
}

pub struct SystemMonitor;

impl SystemMonitor {
    pub fn get_stats() -> Result<SystemStats, String> {
        let mut stats = SystemStats {
            cpu_usage_percent: 0.0,
            memory_total_bytes: 0,
            memory_used_bytes: 0,
            memory_percent: 0.0,
            disk_total_bytes: 0,
            disk_used_bytes: 0,
            disk_percent: 0.0,
        };

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "$cpu = (Get-Counter '\\Processor(_Total)\\% Processor Time').CounterSamples.CookedValue; $mem = Get-CimInstance Win32_OperatingSystem; $disk = Get-CimInstance Win32_LogicalDisk -Filter \"DeviceID='C:'\"; [PSCustomObject]@{ CPU=$cpu; MemTotal=$mem.TotalVisibleMemorySize*1024; MemFree=$mem.FreePhysicalMemory*1024; DiskTotal=$disk.Size; DiskFree=$disk.FreeSpace } | ConvertTo-Json"
                ])
                .stdout(Stdio::piped())
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        stats.cpu_usage_percent = json.get("CPU").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                        stats.memory_total_bytes = json.get("MemTotal").and_then(|v| v.as_u64()).unwrap_or(0);
                        let mem_free = json.get("MemFree").and_then(|v| v.as_u64()).unwrap_or(0);
                        stats.memory_used_bytes = stats.memory_total_bytes - mem_free;
                        stats.memory_percent = if stats.memory_total_bytes > 0 {
                            (stats.memory_used_bytes as f32 / stats.memory_total_bytes as f32) * 100.0
                        } else {
                            0.0
                        };
                        stats.disk_total_bytes = json.get("DiskTotal").and_then(|v| v.as_u64()).unwrap_or(0);
                        let disk_free = json.get("DiskFree").and_then(|v| v.as_u64()).unwrap_or(0);
                        stats.disk_used_bytes = stats.disk_total_bytes - disk_free;
                        stats.disk_percent = if stats.disk_total_bytes > 0 {
                            (stats.disk_used_bytes as f32 / stats.disk_total_bytes as f32) * 100.0
                        } else {
                            0.0
                        };
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(output) = Command::new("top")
                .args(["-bn1"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    if line.starts_with("%Cpu(s):") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if let Some(idle) = parts.last() {
                            if let Ok(idle_val) = idle.trim_end_matches('i').parse::<f32>() {
                                stats.cpu_usage_percent = 100.0 - idle_val;
                            }
                        }
                    }
                }
            }

            if let Ok(output) = Command::new("free")
                .args(["-b"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts[0] == "Mem:" {
                        stats.memory_total_bytes = parts[1].parse().unwrap_or(0);
                        stats.memory_used_bytes = parts[2].parse().unwrap_or(0);
                        stats.memory_percent = if stats.memory_total_bytes > 0 {
                            (stats.memory_used_bytes as f32 / stats.memory_total_bytes as f32) * 100.0
                        } else {
                            0.0
                        };
                    }
                }
            }

            if let Ok(output) = Command::new("df")
                .args(["-B1", "/"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        stats.disk_total_bytes = parts[1].parse().unwrap_or(0);
                        stats.disk_used_bytes = parts[2].parse().unwrap_or(0);
                        stats.disk_percent = if stats.disk_total_bytes > 0 {
                            (stats.disk_used_bytes as f32 / stats.disk_total_bytes as f32) * 100.0
                        } else {
                            0.0
                        };
                    }
                }
            }
        }

        Ok(stats)
    }

    pub fn get_disk_usage() -> Vec<serde_json::Value> {
        let mut drives = Vec::new();

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Get-Volume | Select-Object DriveLetter, FileSystemLabel, Size, FreeSpace | ConvertTo-Json"
                ])
                .stdout(Stdio::piped())
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        let arr = if json.is_array() { json.as_array().unwrap().clone() } else { vec![json] };
                        for item in arr {
                            if let Some(drive) = item.get("DriveLetter").and_then(|v| v.as_str()) {
                                let total = item.get("Size").and_then(|v| v.as_u64()).unwrap_or(0);
                                let free = item.get("FreeSpace").and_then(|v| v.as_u64()).unwrap_or(0);
                                let used = total.saturating_sub(free);
                                drives.push(serde_json::json!({
                                    "drive": drive,
                                    "total_bytes": total,
                                    "used_bytes": used,
                                    "free_bytes": free,
                                    "percent": if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 }
                                }));
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(output) = Command::new("df")
                .args(["-B1"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 6 {
                        let total: u64 = parts[1].parse().unwrap_or(0);
                        let used: u64 = parts[2].parse().unwrap_or(0);
                        let free: u64 = parts[3].parse().unwrap_or(0);
                        let mount = parts[5].to_string();

                        drives.push(serde_json::json!({
                            "mount": mount,
                            "total_bytes": total,
                            "used_bytes": used,
                            "free_bytes": free,
                            "percent": if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 }
                        }));
                    }
                }
            }
        }

        drives
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub source: String,
    pub level: String,
    pub message: String,
}

pub struct LogViewer;

impl LogViewer {
    pub fn get_system_logs(count: usize, level: Option<String>) -> Vec<LogEntry> {
        let mut entries = Vec::new();
        let level_filter = level.unwrap_or_else(|| "Information".to_string());

        #[cfg(windows)]
        {
            let filter = format!(
                "Get-WinEvent -FilterHashtable @{{LogName='System'; Level={}}} -MaxEvents {} | Select-Object TimeCreated, ProviderName, LevelDisplayName, Message | ConvertTo-Json",
                match level_filter.as_str() {
                    "Error" => "2",
                    "Warning" => "3",
                    "Information" => "4",
                    _ => "4"
                },
                count
            );

            if let Ok(output) = Command::new("powershell")
                .args(["-NoProfile", "-Command", &filter])
                .stdout(Stdio::piped())
                .output()
            {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        let arr = if json.is_array() { json.as_array().unwrap().clone() } else { vec![json] };
                        for item in arr {
                            entries.push(LogEntry {
                                timestamp: item.get("TimeCreated").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                source: item.get("ProviderName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                level: item.get("LevelDisplayName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                message: item.get("Message").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            });
                        }
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            let journal_args = if let Ok(level_num) = match level_filter.as_str() {
                "Error" => Ok::<&str, ()>("err"),
                "Warning" => Ok("warning"),
                _ => Ok("info")
            } {
                vec!["-p", level_num, "-n", &count.to_string()]
            } else {
                vec!["-n", &count.to_string()]
            };

            if let Ok(output) = Command::new("journalctl")
                .args(&journal_args)
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines().take(count) {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        entries.push(LogEntry {
                            timestamp: parts[0].to_string(),
                            source: parts.get(1).unwrap_or(&"").to_string(),
                            level: "Info".to_string(),
                            message: parts[2..].join(" "),
                        });
                    }
                }
            }
        }

        entries
    }
}