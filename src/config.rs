/// Transport mode for MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportMode {
    #[default]
    Stdio,
    Http,
}

impl TransportMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "stdio" => Some(Self::Stdio),
            "http" => Some(Self::Http),
            _ => None,
        }
    }
}

/// Server configuration parsed from CLI arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerConfig {
    pub web_preview: bool,
    pub debug: bool,
    pub transport: TransportMode,
    pub port: u16,
    pub host: String,
    pub max_sessions: usize,
    pub session_ttl_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            web_preview: true,
            debug: false,
            transport: TransportMode::Stdio,
            port: 3000,
            host: "0.0.0.0".to_string(),
            max_sessions: 100,
            session_ttl_secs: 1800,
        }
    }
}

impl ServerConfig {
    pub fn from_args() -> Self {
        let mut config = Self::default();
        let args: Vec<String> = std::env::args().collect();
        let mut i = 1;

        while i < args.len() {
            match args[i].as_str() {
                "--no-web-preview" => config.web_preview = false,
                "--debug" => config.debug = true,
                "--transport" => {
                    if i + 1 < args.len() {
                        if let Some(mode) = TransportMode::from_str(&args[i + 1]) {
                            config.transport = mode;
                        }
                        i += 1;
                    }
                }
                "--port" => {
                    if i + 1 < args.len() {
                        if let Ok(port) = args[i + 1].parse() {
                            config.port = port;
                        }
                        i += 1;
                    }
                }
                "--host" => {
                    if i + 1 < args.len() {
                        config.host = args[i + 1].clone();
                        i += 1;
                    }
                }
                "--max-sessions" => {
                    if i + 1 < args.len() {
                        if let Ok(max) = args[i + 1].parse() {
                            config.max_sessions = max;
                        }
                        i += 1;
                    }
                }
                "--session-ttl" => {
                    if i + 1 < args.len() {
                        if let Ok(ttl) = args[i + 1].parse() {
                            config.session_ttl_secs = ttl;
                        }
                        i += 1;
                    }
                }
                _ => {} // silently ignore unknown args
            }
            i += 1;
        }

        config
    }
}
