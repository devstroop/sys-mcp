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
}

impl Default for ServerConfig {
    fn default() -> Self {
        // clap's Parser provides defaults via arg attributes,
        // but we need to parse empty args to get those defaults.
        Self::try_parse_from(std::iter::once::<String>("".into()))
            .expect("default config should be valid")
    }
}

impl ServerConfig {
    pub fn from_args() -> Self {
        Self::parse()
    }
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
