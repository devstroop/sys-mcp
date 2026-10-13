use clap::Parser;
use sys_mcp::config::{JevConfig, ServerConfig, TransportMode};

#[test]
fn test_config_defaults() {
    let config = ServerConfig::default();
    assert!(config.web_preview);
    assert!(!config.debug);
    assert_eq!(config.transport, TransportMode::Stdio);
    assert_eq!(config.port, 3000);
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.max_sessions, 100);
    assert_eq!(config.session_ttl_secs, 1800);
}

#[test]
fn test_jev_config_defaults() {
    // Guard against ambient JEV_*/SYS_MCP_JEV_* env vars in the test
    // environment.
    if [
        "JEV_ENDPOINT",
        "JEV_API_KEY",
        "JEV_MODEL",
        "JEV_TIMEOUT_SECS",
        "SYS_MCP_JEV_GUARDRAILS",
        "SYS_MCP_JEV_GUARD_THRESHOLD",
        "SYS_MCP_JEV_GUARD_TOOLS",
    ]
    .iter()
    .any(|k| std::env::var(k).is_ok())
    {
        return;
    }
    let jev = JevConfig::default();
    assert_eq!(jev.endpoint, "http://192.168.2.49:5050/v1/systemone");
    assert_eq!(jev.api_key, "");
    assert_eq!(jev.model, "jev-1.13-free");
    assert_eq!(jev.timeout_secs, 15);

    // Guardrails are opt-in: off by default, 0.5 threshold, default tool set.
    assert!(!jev.guardrails);
    assert!((jev.guard_threshold - 0.5).abs() < f64::EPSILON);
    assert_eq!(jev.guard_tools, None);

    // Flattened into ServerConfig.
    let config = ServerConfig::default();
    assert_eq!(config.jev.model, "jev-1.13-free");
    assert!(!config.jev.guardrails);
}

#[test]
fn test_jev_guardrails_cli_overrides() {
    // Guardrails enabled + threshold + tool list via CLI flags.
    // (CLI flags win over env in clap, so only the second half below needs
    // the ambient-env guard.)
    let config = ServerConfig::try_parse_from([
        "sys-mcp",
        "--jev-guardrails",
        "--jev-guard-threshold",
        "0.75",
        "--jev-guard-tools",
        "shell_exec, fs_delete",
    ])
    .expect("args should parse");
    assert!(config.jev.guardrails);
    assert!((config.jev.guard_threshold - 0.75).abs() < f64::EPSILON);
    assert_eq!(
        config.jev.guard_tools.as_deref(),
        Some("shell_exec, fs_delete")
    );

    // Default remains off (skip when the ambient environment sets the
    // guard env vars).
    if [
        "SYS_MCP_JEV_GUARDRAILS",
        "SYS_MCP_JEV_GUARD_THRESHOLD",
        "SYS_MCP_JEV_GUARD_TOOLS",
    ]
    .iter()
    .any(|k| std::env::var(k).is_ok())
    {
        return;
    }
    let config = ServerConfig::try_parse_from(["sys-mcp"]).expect("args should parse");
    assert!(!config.jev.guardrails);
    assert!((config.jev.guard_threshold - 0.5).abs() < f64::EPSILON);
    assert_eq!(config.jev.guard_tools, None);
}

#[test]
fn test_jev_guard_threshold_out_of_range_rejected() {
    let result = ServerConfig::try_parse_from(["sys-mcp", "--jev-guard-threshold", "1.5"]);
    assert!(result.is_err(), "threshold > 1.0 must be rejected");
    let result = ServerConfig::try_parse_from(["sys-mcp", "--jev-guard-threshold", "-0.1"]);
    assert!(result.is_err(), "threshold < 0.0 must be rejected");
}

#[test]
fn test_transport_mode_from_str() {
    assert_eq!(
        "stdio".parse::<TransportMode>().ok(),
        Some(TransportMode::Stdio)
    );
    assert_eq!(
        "http".parse::<TransportMode>().ok(),
        Some(TransportMode::Http)
    );
    assert_eq!(
        "STDIO".parse::<TransportMode>().ok(),
        Some(TransportMode::Stdio)
    );
    assert_eq!(
        "HTTP".parse::<TransportMode>().ok(),
        Some(TransportMode::Http)
    );
    assert!("unknown".parse::<TransportMode>().is_err());
}

#[test]
fn test_transport_mode_debug() {
    let mode = TransportMode::Stdio;
    assert_eq!(format!("{mode:?}"), "Stdio");
}
