use sys_mcp::config::{ServerConfig, TransportMode};

#[test]
fn test_config_defaults() {
    let config = ServerConfig::default();
    assert!(config.web_preview);
    assert!(!config.debug);
    assert_eq!(config.transport, TransportMode::Stdio);
    assert_eq!(config.port, 3000);
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.max_sessions, 100);
    assert_eq!(config.session_ttl_secs, 1800);
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
