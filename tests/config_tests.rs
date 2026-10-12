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
    // Guard against ambient JEV_* env vars in the test environment.
    if ["JEV_ENDPOINT", "JEV_API_KEY", "JEV_MODEL", "JEV_TIMEOUT_SECS"]
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

    // Flattened into ServerConfig.
    let config = ServerConfig::default();
    assert_eq!(config.jev.model, "jev-1.13-free");
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
