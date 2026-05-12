//! Server configuration and CLI argument parsing.
//!
//! Parses command-line arguments into a [`ServerConfig`] struct.
//! Supports `--host`, `--hostname`, `--port`, `--transport`, and other flags.

/// Transport mode for MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--version" | "-V" => {
                    println!("gui-mcp {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                "--no-web-preview" => config.web_preview = false,
                "--debug" => config.debug = true,
                "--transport" => {
                    if i + 1 < args.len() {
                        match args[i + 1].parse::<TransportMode>() {
                            Ok(mode) => config.transport = mode,
                            Err(_) => {
                                eprintln!("Unknown transport mode: '{}'", args[i + 1]);
                                eprintln!("Valid options: stdio, http");
                                std::process::exit(1);
                            }
                        }
                        i += 1;
                    }
                }
                "--port" => {
                    if i + 1 < args.len() {
                        if let Ok(port) = args[i + 1].parse() {
                            config.port = port;
                        } else {
                            eprintln!("Invalid port: '{}'", args[i + 1]);
                            std::process::exit(1);
                        }
                        i += 1;
                    }
                }
                "--host" | "--hostname" => {
                    if i + 1 < args.len() {
                        config.host = args[i + 1].clone();
                        i += 1;
                    }
                }
                "--max-sessions" => {
                    if i + 1 < args.len() {
                        if let Ok(max) = args[i + 1].parse() {
                            config.max_sessions = max;
                        } else {
                            eprintln!("Invalid max-sessions: '{}'", args[i + 1]);
                            std::process::exit(1);
                        }
                        i += 1;
                    }
                }
                "--session-ttl" => {
                    if i + 1 < args.len() {
                        if let Ok(ttl) = args[i + 1].parse() {
                            config.session_ttl_secs = ttl;
                        } else {
                            eprintln!("Invalid session-ttl: '{}'", args[i + 1]);
                            std::process::exit(1);
                        }
                        i += 1;
                    }
                }
                unknown => {
                    eprintln!("Unknown argument: '{}'", unknown);
                    eprintln!("Run with --help for usage information.");
                    std::process::exit(1);
                }
            }
            i += 1;
        }

        config
    }
}

fn print_help() {
    println!(
        r#"sys-mcp {} — GUI Automation MCP Server

USAGE:
    sys-mcp [OPTIONS]

OPTIONS:
    --host, --hostname <HOST>    Bind address (default: 0.0.0.0)
    --port <PORT>                MCP HTTP port (default: 3000)
    --transport <stdio|http>     Transport mode (default: stdio)
    --no-web-preview             Disable web preview server
    --max-sessions <N>           Max concurrent sessions (default: 100)
    --session-ttl <SECONDS>      Session timeout in seconds (default: 1800)
    --debug                      Enable debug logging
    --help, -h                   Print this help message
    --version, -V                Print version information

ENVIRONMENT:
    RUST_LOG                     Override log level (e.g. RUST_LOG=trace)
"#,
        env!("CARGO_PKG_VERSION")
    );
}
