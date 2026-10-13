use clap::Parser;

/// MCP server for local system automation.
///
/// Provides AI agents with access to screen capture, OCR, mouse/keyboard
/// input, window management, file system, shell, process/service/network
/// management, and system monitoring tools over the MCP protocol.
#[derive(Parser, Debug, Clone)]
#[command(name = "sys-mcp", version, about)]
pub struct ServerConfig {
    /// Bind address for the HTTP transport and web preview server
    #[arg(long, default_value = "127.0.0.1", env = "SYS_MCP_HOST")]
    pub host: String,

    /// Port for the HTTP transport
    #[arg(long, default_value_t = 3000, env = "SYS_MCP_PORT")]
    pub port: u16,

    /// Transport mode (stdio or http)
    #[arg(long, default_value_t = TransportMode::Stdio, value_enum, env = "SYS_MCP_TRANSPORT")]
    pub transport: TransportMode,

    /// Disable the web preview server
    #[arg(long = "no-web-preview", default_value_t = true, action = clap::ArgAction::SetFalse)]
    pub web_preview: bool,

    /// Maximum concurrent sessions
    #[arg(long, default_value_t = 100, env = "SYS_MCP_MAX_SESSIONS")]
    pub max_sessions: usize,

    /// Session timeout in seconds
    #[arg(long, default_value_t = 1800, env = "SYS_MCP_SESSION_TTL")]
    pub session_ttl_secs: u64,

    /// Enable debug logging
    #[arg(long, short)]
    pub debug: bool,

    /// JEV (TypeSafe System One) decision service settings
    #[command(flatten)]
    pub jev: JevConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        // clap's Parser provides defaults via arg attributes,
        // but we need to parse empty args to get those defaults.
        Self::try_parse_from(std::iter::once::<String>("".into()))
            .expect("default config should be valid")
    }
}

/// Settings for the JEV decision endpoint (`decision` tool) and the
/// opt-in destructive-tool guardrails.
///
/// Environment overrides: `JEV_ENDPOINT`, `JEV_API_KEY`, `JEV_MODEL`,
/// `JEV_TIMEOUT_SECS`, `SYS_MCP_JEV_GUARDRAILS`,
/// `SYS_MCP_JEV_GUARD_THRESHOLD`, `SYS_MCP_JEV_GUARD_TOOLS`. The API key
/// is never logged or echoed in tool output.
#[derive(Parser, Debug, Clone, PartialEq)]
pub struct JevConfig {
    /// JEV decision endpoint URL (POST /v1/systemone)
    #[arg(
        long,
        default_value = "http://192.168.2.49:5050/v1/systemone",
        env = "JEV_ENDPOINT"
    )]
    pub endpoint: String,

    /// JEV API key (kept out of logs and tool output)
    #[arg(long, default_value = "", env = "JEV_API_KEY")]
    pub api_key: String,

    /// JEV model name
    #[arg(long, default_value = "jev-1.13-free", env = "JEV_MODEL")]
    pub model: String,

    /// JEV request timeout in seconds
    #[arg(long, default_value_t = 15, env = "JEV_TIMEOUT_SECS")]
    pub timeout_secs: u64,

    /// Gate destructive tools behind a JEV risk check (opt-in; default off).
    /// When enabled, guarded tool calls are assessed by the decision model
    /// and held for confirmation unless the Rust denylist refuses them first.
    ///
    /// Accepts a bare `--jev-guardrails` flag, or any of `1/0`, `true/false`,
    /// `yes/no`, `on/off` (case-insensitive) via the env var — `=1` is the
    /// most natural thing to type and must not abort the process.
    #[arg(long = "jev-guardrails", default_value_t = false, num_args = 0..=1, default_missing_value = "true", value_parser = parse_bool, env = "SYS_MCP_JEV_GUARDRAILS")]
    pub guardrails: bool,

    /// Risk threshold (0.0–1.0) for the JEV guard: a `noul` "safe to run
    /// without confirmation" score below this value holds the call for
    /// confirmation.
    #[arg(long = "jev-guard-threshold", default_value_t = 0.5, value_parser = parse_guard_threshold, env = "SYS_MCP_JEV_GUARD_THRESHOLD")]
    pub guard_threshold: f64,

    /// Comma-separated override list of tools to guard. When unset, the
    /// compiled-in defaults are used (shell_exec, fs_delete, process_kill,
    /// service_stop, service_start, process_start).
    #[arg(long = "jev-guard-tools", env = "SYS_MCP_JEV_GUARD_TOOLS")]
    pub guard_tools: Option<String>,

    /// What to do when the decision model itself is unreachable (network,
    /// timeout, 429). `closed` (the default, per JEV_PLAN.md safety rule 6)
    /// holds the call for confirmation — the Rust denylist still applies
    /// either way, and the caller can always re-send with `confirm: true`,
    /// so this is not a hard block. `open` prioritises availability and
    /// lets the call through with the fuzzy judgment skipped.
    #[arg(long = "jev-guard-fail-mode", default_value_t = GuardFailMode::Closed, value_enum, env = "SYS_MCP_JEV_GUARD_FAIL_MODE")]
    pub guard_fail_mode: GuardFailMode,
}

impl Default for JevConfig {
    fn default() -> Self {
        // Same pattern as ServerConfig: parse empty args so clap resolves
        // env overrides and default values.
        Self::try_parse_from(std::iter::once::<String>("".into()))
            .expect("default JEV config should be valid")
    }
}

impl ServerConfig {
    pub fn from_args() -> Self {
        Self::parse()
    }
}

/// Clap value parser for `--jev-guard-threshold`: a finite f64 in
/// `[0.0, 1.0]`.
fn parse_guard_threshold(s: &str) -> Result<f64, String> {
    let v: f64 = s
        .parse()
        .map_err(|_| format!("invalid threshold '{s}': expected a number between 0.0 and 1.0"))?;
    if v.is_finite() && (0.0..=1.0).contains(&v) {
        Ok(v)
    } else {
        Err(format!(
            "invalid threshold '{s}': expected a value between 0.0 and 1.0"
        ))
    }
}

/// Clap value parser for boolean flags exposed via env vars.
///
/// clap's built-in `ArgAction::SetTrue` only accepts the literal strings
/// `true`/`false` from the environment, so `SYS_MCP_JEV_GUARDRAILS=1` — the
/// most natural way to enable a flag — aborts the whole process with an
/// opaque parse error. Accept the common spellings instead.
fn parse_bool(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(format!(
            "invalid boolean '{other}': expected one of 1/0, true/false, yes/no, on/off"
        )),
    }
}

/// Behaviour when the decision model is unreachable while guardrails are on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum GuardFailMode {
    /// Hold the call for confirmation (default; JEV_PLAN.md safety rule 6).
    #[default]
    Closed,
    /// Allow the call through with the fuzzy judgment skipped.
    Open,
}

/// Transport mode for MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum TransportMode {
    #[default]
    Stdio,
    Http,
}

impl std::str::FromStr for TransportMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdio" => Ok(Self::Stdio),
            "http" => Ok(Self::Http),
            _ => Err(format!("unknown transport mode: {s}")),
        }
    }
}
