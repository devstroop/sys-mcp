//! Windows system service management utilities.
//!
//! Provides [`ServiceManager`] for querying and managing Windows services.

use serde::{Deserialize, Serialize};
use std::process::Command;

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

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("systemctl")
                .args([
                    "list-units",
                    "--all",
                    "--type=service",
                    "--no-pager",
                    "--no-legend",
                ])
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

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("launchctl").args(["list"]).output() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    for line in text.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            let status = match parts[0] {
                                "-" => "Running".to_string(),
                                "1" => "Error".to_string(),
                                _ => "Running".to_string(),
                            };
                            services.push(ServiceInfo {
                                name: parts[2].to_string(),
                                display_name: parts[2].to_string(),
                                status,
                                start_type: "N/A".to_string(),
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
            let output = Command::new("sc")
                .args(["start", name])
                .output()
                .map_err(|e| e.to_string())?;

            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }

        #[cfg(target_os = "linux")]
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

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("launchctl")
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
            let output = Command::new("sc")
                .args(["stop", name])
                .output()
                .map_err(|e| e.to_string())?;

            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }

        #[cfg(target_os = "linux")]
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

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("launchctl")
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
        services
            .into_iter()
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

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("hostname").args(["-I"]).output() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    info.ip_addresses = text.split_whitespace().map(String::from).collect();
                }
            }

            if let Ok(output) = Command::new("route").args(["-n"]).output() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    if line.starts_with("0.0.0.0") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            info.default_gateway = Some(parts[1].to_string());
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("ipconfig").args(["getifaddr", "en0"]).output() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    let ip = text.trim().to_string();
                    if !ip.is_empty() {
                        info.ip_addresses.push(ip);
                    }
                }
            }

            if let Ok(output) = Command::new("netstat").args(["-rn"]).output() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    if line.starts_with("default") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            info.default_gateway = Some(parts[1].to_string());
                        }
                    }
                }
            }
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            if let Ok(output) = Command::new("cat").args(["/etc/resolv.conf"]).output() {
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
            if let Ok(output) = Command::new("netstat").args(["-ano"]).output() {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(4) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let protocol = parts[0].to_string();
                        if protocol == "TCP" || protocol == "UDP" {
                            let local_addr = parts[1].to_string();
                            let state = if parts.len() >= 4 {
                                parts[3].to_string()
                            } else {
                                "-".to_string()
                            };
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

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("ss").args(["-tunap"]).output() {
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

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("netstat")
                .args(["-an", "-p", "tcp", "-p", "udp"])
                .output()
            {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let proto = parts[0].to_string();
                        if (proto == "tcp" || proto == "tcp4" || proto == "tcp6")
                            || (proto == "udp" || proto == "udp4" || proto == "udp6")
                        {
                            connections.push(serde_json::json!({
                                "protocol": proto,
                                "local_address": parts[3],
                                "peer_address": parts[4],
                                "state": if proto.starts_with("tcp") { parts[5].to_string() } else { "-".to_string() }
                            }));
                        }
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

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("top").args(["-bn1"]).output() {
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

            if let Ok(output) = Command::new("free").args(["-b"]).output() {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts[0] == "Mem:" {
                        stats.memory_total_bytes = parts[1].parse().unwrap_or(0);
                        stats.memory_used_bytes = parts[2].parse().unwrap_or(0);
                        stats.memory_percent = if stats.memory_total_bytes > 0 {
                            (stats.memory_used_bytes as f32 / stats.memory_total_bytes as f32)
                                * 100.0
                        } else {
                            0.0
                        };
                    }
                }
            }

            if let Ok(output) = Command::new("df").args(["-B1", "/"]).output() {
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

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("top").args(["-l", "1", "-n", "0"]).output() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    if line.contains("CPU usage:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for (i, part) in parts.iter().enumerate() {
                            if part.ends_with("%user") || *part == "%user" {
                                if let Some(pct) = parts.get(i.saturating_sub(1)) {
                                    stats.cpu_usage_percent =
                                        pct.trim_end_matches('%').parse().unwrap_or(0.0);
                                }
                                break;
                            }
                        }
                    }
                }
            }

            if let Ok(output) = Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    stats.memory_total_bytes = text.trim().parse().unwrap_or(0);
                }
            }

            if let Ok(output) = Command::new("vm_stat").output() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    let page_size: u64 = 4096;
                    let mut pages_active: u64 = 0;
                    let mut pages_wired: u64 = 0;
                    let mut pages_compressed: u64 = 0;
                    for line in text.lines() {
                        if let Some(val) = line
                            .strip_prefix("Pages active:")
                            .or_else(|| line.strip_prefix("Pages active:"))
                        {
                            pages_active = val.trim().trim_end_matches('.').parse().unwrap_or(0);
                        }
                        if let Some(val) = line
                            .strip_prefix("Pages wired down:")
                            .or_else(|| line.strip_prefix("Pages wired down:"))
                        {
                            pages_wired = val.trim().trim_end_matches('.').parse().unwrap_or(0);
                        }
                        if let Some(val) = line
                            .strip_prefix("Pages occupied by compressor:")
                            .or_else(|| line.strip_prefix("Pages occupied by compressor:"))
                        {
                            pages_compressed =
                                val.trim().trim_end_matches('.').parse().unwrap_or(0);
                        }
                    }
                    stats.memory_used_bytes =
                        (pages_active + pages_wired + pages_compressed) * page_size;
                    if stats.memory_total_bytes > 0 {
                        stats.memory_percent = (stats.memory_used_bytes as f32
                            / stats.memory_total_bytes as f32)
                            * 100.0;
                    }
                }
            }

            if let Ok(output) = Command::new("df").args(["-k", "/"]).output() {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let total_kb: u64 = parts[1].parse().unwrap_or(0);
                        let used_kb: u64 = parts[2].parse().unwrap_or(0);
                        stats.disk_total_bytes = total_kb * 1024;
                        stats.disk_used_bytes = used_kb * 1024;
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

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("df").args(["-B1"]).output() {
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

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("df").args(["-k"]).output() {
                for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 6 {
                        let total_kb: u64 = parts[1].parse().unwrap_or(0);
                        let used_kb: u64 = parts[2].parse().unwrap_or(0);
                        let free_kb: u64 = parts[3].parse().unwrap_or(0);
                        let mount = parts[5].to_string();

                        drives.push(serde_json::json!({
                            "mount": mount,
                            "total_bytes": total_kb * 1024,
                            "used_bytes": used_kb * 1024,
                            "free_bytes": free_kb * 1024,
                            "percent": if total_kb > 0 { (used_kb as f64 / total_kb as f64) * 100.0 } else { 0.0 }
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
    /// Platform-native timestamp. Windows reports ISO 8601 local time, Linux
    /// reports seconds since the Unix epoch, macOS reports whatever
    /// `log show --style compact` printed. Not yet a single format — see
    /// `docs/ISSUES.md` #41.
    pub timestamp: String,
    pub source: String,
    /// Empty when the platform's log format carries no per-entry severity
    /// (macOS compact output, and Linux records without a `PRIORITY` field).
    /// It is never filled in with the level the caller asked for.
    pub level: String,
    pub message: String,
}

/// Severity filter for [`LogViewer::get_system_logs`].
///
/// The names are the Windows event-viewer labels, because Windows is the
/// platform whose log records genuinely carry per-entry severity. The other
/// platforms map onto their own vocabulary ([`LogLevel::journald_priority`],
/// [`LogLevel::apple_level`]); a level a platform cannot express is refused
/// rather than quietly rounded to the nearest supported word, which would
/// return a filtered result the caller cannot tell apart from the one they asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Critical,
    Error,
    Warning,
    Information,
    Verbose,
}

impl LogLevel {
    /// Case-insensitive, and tolerant of the abbreviations the platforms use
    /// (`err`, `warn`, `info`, `debug`) so a caller that writes `"error"` in
    /// lowercase gets its errors instead of an argument error.
    pub fn parse(raw: &str) -> Option<LogLevel> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "critical" | "fatal" => Some(Self::Critical),
            "error" | "err" => Some(Self::Error),
            "warning" | "warn" => Some(Self::Warning),
            "information" | "info" => Some(Self::Information),
            "verbose" | "debug" => Some(Self::Verbose),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "Critical",
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Information => "Information",
            Self::Verbose => "Verbose",
        }
    }

    /// `Get-WinEvent -FilterHashtable` `Level` value.
    // Each platform mapper is referenced from inside that platform's `cfg`
    // block, so the other two builds see it as unused. The mappings are
    // exercised by tests on every platform.
    #[allow(dead_code)]
    fn windows_level(self) -> &'static str {
        match self {
            Self::Critical => "1",
            Self::Error => "2",
            Self::Warning => "3",
            Self::Information => "4",
            Self::Verbose => "5",
        }
    }

    /// `journalctl -p` priority name. journald treats `-p` as a *threshold*:
    /// this priority and everything more severe.
    #[allow(dead_code)]
    fn journald_priority(self) -> &'static str {
        match self {
            Self::Critical => "crit",
            Self::Error => "err",
            Self::Warning => "warning",
            Self::Information => "info",
            Self::Verbose => "debug",
        }
    }

    /// Map a journald `PRIORITY` number back to the reported level name.
    #[allow(dead_code)]
    fn from_journald_priority(raw: &str) -> String {
        match raw.trim() {
            "0" | "1" | "2" => Self::Critical.as_str(),
            "3" => Self::Error.as_str(),
            "4" => Self::Warning.as_str(),
            "5" | "6" => Self::Information.as_str(),
            "7" => Self::Verbose.as_str(),
            other => return other.to_string(),
        }
        .to_string()
    }

    /// `log show --level` accepts debug / info / error / fault only, and
    /// treats them as a threshold. `Warning` has no equivalent, so a caller on
    /// macOS is told instead of being handed error-and-above entries labelled
    /// as if they were the warning they asked for.
    #[allow(dead_code)]
    fn apple_level(self) -> Option<&'static str> {
        match self {
            Self::Critical => Some("fault"),
            Self::Error => Some("error"),
            Self::Warning => None,
            Self::Information => Some("info"),
            Self::Verbose => Some("debug"),
        }
    }
}

/// Longest message a single entry may carry into the tool result. An event-log
/// message can be many kilobytes, and 50 of those is a context overflow; the
/// marker makes the cut visible rather than pretending the message was short.
const MAX_LOG_MESSAGE_CHARS: usize = 2000;

fn cap_field(value: String, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value;
    }
    let kept: String = value.chars().take(limit).collect();
    format!("{kept}\u{2026}[message truncated at {limit} chars]")
}

/// One-line version for stderr text going into an error message.
fn cap_line(value: &str) -> String {
    let flat = value.replace(['\r', '\n'], " ");
    let trimmed = flat.trim();
    if trimmed.chars().count() <= 400 {
        return trimmed.to_string();
    }
    format!("{}\u{2026}", trimmed.chars().take(400).collect::<String>())
}

/// Windows event-log names arrive straight from the caller and are interpolated
/// into a PowerShell hashtable literal, so the character set is the injection
/// boundary: `'` is what breaks out of the quoted value. Real log names
/// (`System`, `Application`, `Security`, `Microsoft-Windows-Kernel-PnP/Config`,
/// `Windows PowerShell`) only need letters, digits, `_`, `-`, `.`, `/` and a
/// space, so anything else is refused.
#[cfg(windows)]
fn is_event_log_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ' '))
}

/// The PowerShell script behind a Windows event-log query, built apart from the
/// execution so a test can assert its exact text: the brace nesting of
/// `-FilterHashtable @{…}` inside a format string is easy to get wrong, and a
/// malformed script fails as a PowerShell parse error rather than as log data.
#[cfg(windows)]
fn windows_log_query(log_name: &str, level: Option<LogLevel>, count: usize) -> String {
    let mut script = format!("Get-WinEvent -FilterHashtable @{{LogName='{log_name}'");
    // `Level` is omitted when the caller named no level, so "no level" means
    // every level — including Error and Warning, the entries a log query is
    // called for. Defaulting to Information made the tool hide failures.
    if let Some(level) = level {
        script.push_str(&format!("; Level={}", level.windows_level()));
    }
    script.push_str(&format!(
        "}} -MaxEvents {count} | Select-Object @{{n='TimeCreated';e={{$_.TimeCreated.ToString('o')}}}}, ProviderName, LevelDisplayName, Message | ConvertTo-Json"
    ));
    script
}

pub struct LogViewer;

impl LogViewer {
    /// Read the platform event log, newest entries first.
    ///
    /// `Err` means the query could not be run or reported a failure. An empty
    /// `Vec` now means exactly one thing: the filter matched no entries. It used
    /// to mean that as well as "PowerShell is not on PATH", "the filter was
    /// rejected", and "the response did not parse" — a diagnostic tool reporting
    /// a clean log when it had failed to read one.
    pub fn get_system_logs(
        count: usize,
        level: Option<LogLevel>,
        log: Option<String>,
    ) -> Result<Vec<LogEntry>, String> {
        let mut entries = Vec::new();

        #[cfg(windows)]
        {
            let log_name = match log {
                None => "System".to_string(),
                Some(raw) if is_event_log_name(&raw) => raw,
                Some(raw) => {
                    return Err(format!(
                        "invalid argument: 'log' is not a usable Windows event log name: {raw:?}. Expected e.g. 'System', 'Application', 'Security', 'Setup', or 'Microsoft-Windows-…/…'"
                    ))
                }
            };

            let script = windows_log_query(&log_name, level, count);
            let output = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .stdout(Stdio::piped())
                .output()
                .map_err(|e| format!("could not run powershell to read '{log_name}': {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                // `Get-WinEvent` reports "nothing matched" by writing an error
                // record and exiting non-zero with empty stdout. That is a true
                // empty answer, not a failure; everything else must surface.
                if stdout.trim().is_empty() && stderr.contains("No events were found") {
                    return Ok(entries);
                }
                return Err(format!(
                    "event-log query on '{log_name}' failed ({}): {}",
                    output.status,
                    cap_line(&stderr)
                ));
            }

            let trimmed = stdout.trim();
            if trimmed.is_empty() {
                return Ok(entries);
            }
            let json: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
                format!(
                    "could not parse the event-log response ({e}); it began: {}",
                    cap_line(trimmed)
                )
            })?;

            // `ConvertTo-Json` emits a bare object when a single event matched
            // and an array otherwise.
            let items: Vec<&serde_json::Value> = match &json {
                serde_json::Value::Array(arr) => arr.iter().collect(),
                single => vec![single],
            };
            for item in items {
                entries.push(LogEntry {
                    timestamp: item
                        .get("TimeCreated")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    source: item
                        .get("ProviderName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    level: item
                        .get("LevelDisplayName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    message: cap_field(
                        item.get("Message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        MAX_LOG_MESSAGE_CHARS,
                    ),
                });
            }
        }

        #[cfg(target_os = "linux")]
        {
            if log.is_some() {
                return Err(
                    "invalid argument: 'log' names a Windows event log. The Linux journal is one store; this tool does not expose unit filtering, so drop 'log' to read the whole journal."
                        .to_string(),
                );
            }
            let count_str = count.to_string();
            let mut journal_args: Vec<&str> =
                vec!["--no-pager", "--quiet", "-n", &count_str, "-o", "json"];
            if let Some(level) = level {
                journal_args.extend(["-p", level.journald_priority()]);
            }

            let output = Command::new("journalctl")
                .args(&journal_args)
                .output()
                .map_err(|e| format!("could not run journalctl: {e}"))?;
            if !output.status.success() {
                return Err(format!(
                    "journalctl failed ({}): {}",
                    output.status,
                    cap_line(&String::from_utf8_lossy(&output.stderr))
                ));
            }

            // `-o json` rather than the default text format: the text line
            // carries no severity, and the previous code labelled every entry
            // "Info" because that was the filter's name, not the record's level.
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let item: serde_json::Value = serde_json::from_str(line)
                    .map_err(|e| format!("could not parse a journal record ({e}): {line}"))?;

                let raw_ts = match item.get("__REALTIME_TIMESTAMP") {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    _ => String::new(),
                };
                // Microseconds since the epoch, rendered as decimal seconds so
                // the value stays exact (an f64 would drop the sub-microsecond
                // digits at this magnitude). Anything unparseable is passed
                // through rather than dropped.
                let timestamp = match raw_ts.parse::<u64>() {
                    Ok(micros) => format!("{}.{:06}", micros / 1_000_000, micros % 1_000_000),
                    Err(_) => raw_ts,
                };

                let level_name = match item.get("PRIORITY") {
                    Some(serde_json::Value::String(s)) => LogLevel::from_journald_priority(s),
                    Some(serde_json::Value::Number(n)) => {
                        LogLevel::from_journald_priority(&n.to_string())
                    }
                    _ => String::new(),
                };

                // `MESSAGE` is a string, or an array of byte values when the
                // record is not valid UTF-8.
                let message = match item.get("MESSAGE") {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Array(bytes)) => {
                        let raw: Vec<u8> = bytes
                            .iter()
                            .filter_map(|b| b.as_u64().and_then(|n| u8::try_from(n).ok()))
                            .collect();
                        String::from_utf8_lossy(&raw).into_owned()
                    }
                    _ => String::new(),
                };

                let source = item
                    .get("SYSLOG_IDENTIFIER")
                    .and_then(|v| v.as_str())
                    .or_else(|| item.get("_COMM").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();

                entries.push(LogEntry {
                    timestamp,
                    source,
                    level: level_name,
                    message: cap_field(message, MAX_LOG_MESSAGE_CHARS),
                });
            }
            // journalctl emits oldest-first; the Windows event log and this
            // tool's description both promise newest-first.
            entries.reverse();
        }

        #[cfg(target_os = "macos")]
        {
            if log.is_some() {
                return Err(
                    "invalid argument: 'log' names a Windows event log. The macOS unified log is queried with a predicate, which this tool does not expose; drop 'log' to read the whole log."
                        .to_string(),
                );
            }
            let mut log_args: Vec<String> = vec![
                "show".into(),
                "--last".into(),
                "1h".into(),
                "--style".into(),
                "compact".into(),
            ];
            if let Some(level) = level {
                let word = level.apple_level().ok_or_else(|| format!(
                    "invalid argument: 'level' = '{}' cannot be applied to the macOS unified log, which filters on debug, info, error and fault only. Omit 'level' to read every entry.",
                    level.as_str()
                ))?;
                log_args.extend(["--level".into(), word.into()]);
            }

            let output = Command::new("log")
                .args(&log_args)
                .output()
                .map_err(|e| format!("could not run 'log': {e}"))?;
            if !output.status.success() {
                return Err(format!(
                    "log show failed ({}): {}",
                    output.status,
                    cap_line(&String::from_utf8_lossy(&output.stderr))
                ));
            }

            let mut lines: Vec<&str> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect();
            lines.reverse();
            for line in lines.into_iter().take(count) {
                let parts: Vec<&str> = line.splitn(4, ' ').collect();
                if parts.len() < 4 {
                    continue;
                }
                entries.push(LogEntry {
                    timestamp: parts[0].to_string(),
                    source: parts[2].to_string(),
                    // `--style compact` prints no severity per line, so there is
                    // nothing honest to report; `--style json` would carry it.
                    level: String::new(),
                    message: cap_field(parts[3..].join(" "), MAX_LOG_MESSAGE_CHARS),
                });
            }
        }

        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        {
            let _ = (count, level, log);
            return Err("system_logs is not implemented for this platform.".to_string());
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_names_parse_case_insensitively_and_with_platform_abbreviations() {
        // A caller that writes `"error"` means the Windows label `"Error"`, and
        // one that writes `"err"` is using journald's word. Rejecting either
        // would be an argument error over a name collision, not a real mistake.
        for (raw, want) in [
            ("Critical", LogLevel::Critical),
            ("fatal", LogLevel::Critical),
            (" ERROR ", LogLevel::Error),
            ("err", LogLevel::Error),
            ("warn", LogLevel::Warning),
            ("Warning", LogLevel::Warning),
            ("info", LogLevel::Information),
            ("Information", LogLevel::Information),
            ("debug", LogLevel::Verbose),
            ("Verbose", LogLevel::Verbose),
        ] {
            assert_eq!(LogLevel::parse(raw), Some(want), "parse({raw:?})");
        }

        // Anything else is refused rather than defaulted: the old code sent an
        // unrecognized level to `Level=4`, so a request for "Critical" silently
        // returned Information events.
        for raw in ["", "severe", "informational", "warnerr", "0"] {
            assert_eq!(LogLevel::parse(raw), None, "parse({raw:?}) must reject");
        }
    }

    #[test]
    fn level_names_map_to_each_platforms_own_vocabulary() {
        assert_eq!(LogLevel::Critical.windows_level(), "1");
        assert_eq!(LogLevel::Error.windows_level(), "2");
        assert_eq!(LogLevel::Warning.windows_level(), "3");
        assert_eq!(LogLevel::Information.windows_level(), "4");
        assert_eq!(LogLevel::Verbose.windows_level(), "5");

        assert_eq!(LogLevel::Critical.journald_priority(), "crit");
        assert_eq!(LogLevel::Error.journald_priority(), "err");
        assert_eq!(LogLevel::Verbose.journald_priority(), "debug");

        // journald emits its own numbers; read them back as the tool's names.
        assert_eq!(LogLevel::from_journald_priority("3"), "Error");
        assert_eq!(LogLevel::from_journald_priority("4"), "Warning");
        assert_eq!(LogLevel::from_journald_priority("6"), "Information");
        assert_eq!(LogLevel::from_journald_priority("7"), "Verbose");
        // An out-of-range priority is passed through, not coerced to a name.
        assert_eq!(LogLevel::from_journald_priority("9"), "9");

        // The macOS log has no warning level, so asking for one must not be
        // answered with the nearest supported word.
        assert_eq!(LogLevel::Error.apple_level(), Some("error"));
        assert_eq!(LogLevel::Critical.apple_level(), Some("fault"));
        assert_eq!(LogLevel::Warning.apple_level(), None);
    }

    #[cfg(windows)]
    #[test]
    fn event_log_names_are_the_injection_boundary() {
        // `log` reaches PowerShell inside a single-quoted hashtable value, so
        // the allowlist is what stands between a name and a script.
        for good in [
            "System",
            "Application",
            "Security",
            "Setup",
            "Windows PowerShell",
            "Microsoft-Windows-Kernel-PnP/Config",
            "ServiceModel-Host",
        ] {
            assert!(is_event_log_name(good), "{good:?} is a real log name");
        }
        let too_long = "x".repeat(129);
        for bad in [
            "",
            "Syste'm",
            "System'; Get-Process; #",
            "System\"",
            "Sys`tem",
            "Sys\tem",
            "System;Level=1",
            "Sy stem)",
            too_long.as_str(),
        ] {
            assert!(!is_event_log_name(bad), "{bad:?} must be refused");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_query_omits_the_level_clause_when_none_is_requested() {
        // The bug this issue is about: with no `level`, the old script always
        // emitted `Level=4`, so a bare `system_logs` call hid every Error and
        // Warning entry.
        let all = windows_log_query("System", None, 50);
        assert!(all.contains("LogName='System'"), "{all}");
        assert!(
            !all.contains("Level="),
            "no level must mean every level: {all}"
        );
        assert!(all.contains("-MaxEvents 50"), "{all}");

        let errors = windows_log_query("Application", Some(LogLevel::Error), 5);
        assert!(errors.contains("LogName='Application'"), "{errors}");
        assert!(errors.contains("; Level=2"), "{errors}");
        assert!(errors.contains("-MaxEvents 5"), "{errors}");

        // Brace nesting is what makes or breaks the hashtable syntax, and a
        // malformed script fails as a PowerShell parse error rather than as
        // log data, so assert it here instead of discovering it in the field.
        for script in [all, errors] {
            assert_eq!(
                script.matches('{').count(),
                script.matches('}').count(),
                "unbalanced hashtable braces: {script}"
            );
            assert!(script.ends_with("| ConvertTo-Json"), "{script}");
        }
    }

    #[test]
    fn long_messages_are_cut_with_a_visible_marker() {
        let short = cap_field("disk failure predicted".to_string(), 2000);
        assert_eq!(short, "disk failure predicted");

        let long = cap_field(
            "x".repeat(MAX_LOG_MESSAGE_CHARS + 500),
            MAX_LOG_MESSAGE_CHARS,
        );
        assert!(long.starts_with(&"x".repeat(MAX_LOG_MESSAGE_CHARS)));
        assert!(long.contains("message truncated"), "{long}");

        // Character count, not byte count, so a provider message containing
        // multi-byte text is not cut through the middle of a code point.
        let text = cap_field(
            "é".repeat(MAX_LOG_MESSAGE_CHARS + 10),
            MAX_LOG_MESSAGE_CHARS,
        );
        assert!(
            text.starts_with(&"é".repeat(MAX_LOG_MESSAGE_CHARS)),
            "the cut must land on a code-point boundary"
        );
        assert!(!text.contains('\u{FFFD}'), "no replacement char: {text}");
        assert!(text.contains("message truncated"), "{text}");
    }

    #[test]
    fn command_stderr_collapses_to_one_bounded_line() {
        let noisy = "Get-WinEvent : \r\nCannot find \n a path.\n";
        let line = cap_line(noisy);
        assert!(!line.contains('\n') && !line.contains('\r'), "{line:?}");
        assert!(line.starts_with("Get-WinEvent :"), "{line}");

        let huge = cap_line(&"e".repeat(5000));
        assert!(huge.chars().count() <= 401, "{}", huge.chars().count());
    }
}
