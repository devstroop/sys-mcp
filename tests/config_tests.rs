use clap::Parser;
use sys_mcp::config::{GuardFailMode, JevConfig, ServerConfig, TransportMode};

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
        "SYS_MCP_JEV_GUARD_FAIL_MODE",
        "SYS_MCP_JEV_UI_LOOP",
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

    // Guardrails are opt-in: off by default, 0.5 threshold, default tool
    // set, and fail CLOSED when the model is unreachable (JEV_PLAN.md
    // safety rule 6).
    assert!(!jev.guardrails);
    assert!((jev.guard_threshold - 0.5).abs() < f64::EPSILON);
    assert_eq!(jev.guard_tools, None);
    assert_eq!(jev.guard_fail_mode, GuardFailMode::Closed);

    // The ui_decide action loop is opt-in too — this one drives real input,
    // so default-off matters even more than for guardrails.
    assert!(!jev.ui_loop);

    // Flattened into ServerConfig.
    let config = ServerConfig::default();
    assert_eq!(config.jev.model, "jev-1.13-free");
    assert!(!config.jev.guardrails);
    assert!(!config.jev.ui_loop);
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
fn test_jev_ui_loop_cli_override_and_default_off() {
    // Bare flag enables the action loop.
    let config = ServerConfig::try_parse_from(["sys-mcp", "--jev-ui-loop"]).expect("parse");
    assert!(config.jev.ui_loop);

    // Same value parser as --jev-guardrails, so every common boolean
    // spelling works — exercised via CLI (not std::env::set_var) because
    // cargo runs tests in parallel and env mutation would race.
    for truthy in ["1", "true", "TRUE", "yes", "on"] {
        let config =
            ServerConfig::try_parse_from(["sys-mcp", "--jev-ui-loop", truthy]).expect("parse");
        assert!(config.jev.ui_loop, "expected true for {truthy:?}");
    }
    for falsy in ["0", "false", "FALSE", "no", "off"] {
        let config =
            ServerConfig::try_parse_from(["sys-mcp", "--jev-ui-loop", falsy]).expect("parse");
        assert!(!config.jev.ui_loop, "expected false for {falsy:?}");
    }

    // Garbage is a clean error, not a silent enable.
    let result = ServerConfig::try_parse_from(["sys-mcp", "--jev-ui-loop", "maybe"]);
    assert!(result.is_err(), "invalid boolean must be rejected");

    // Default is off, unless the ambient environment opted in.
    if std::env::var("SYS_MCP_JEV_UI_LOOP").is_ok() {
        return;
    }
    let config = ServerConfig::try_parse_from(["sys-mcp"]).expect("args should parse");
    assert!(!config.jev.ui_loop, "ui loop must default to off");
}

#[test]
fn test_jev_guardrails_flag_accepts_common_boolean_spellings() {
    // A bare flag still works.
    let config = ServerConfig::try_parse_from(["sys-mcp", "--jev-guardrails"]).expect("parse");
    assert!(config.jev.guardrails);

    // clap's SetTrue action only accepted literal "true"/"false" from env, so
    // the natural SYS_MCP_JEV_GUARDRAILS=1 aborted the whole process. The
    // env and CLI paths share this value_parser, so exercising it via CLI
    // covers both — and unlike std::env::set_var it is safe under cargo's
    // parallel test threads.
    for truthy in ["1", "true", "TRUE", "yes", "on"] {
        let config =
            ServerConfig::try_parse_from(["sys-mcp", "--jev-guardrails", truthy]).expect("parse");
        assert!(config.jev.guardrails, "expected true for {truthy:?}");
    }
    for falsy in ["0", "false", "FALSE", "no", "off"] {
        let config =
            ServerConfig::try_parse_from(["sys-mcp", "--jev-guardrails", falsy]).expect("parse");
        assert!(!config.jev.guardrails, "expected false for {falsy:?}");
    }

    // Garbage is a clean error, not a process abort.
    let result = ServerConfig::try_parse_from(["sys-mcp", "--jev-guardrails", "maybe"]);
    assert!(result.is_err(), "invalid boolean must be rejected");
}

#[test]
fn test_jev_guard_threshold_out_of_range_rejected() {
    let result = ServerConfig::try_parse_from(["sys-mcp", "--jev-guard-threshold", "1.5"]);
    assert!(result.is_err(), "threshold > 1.0 must be rejected");
    let result = ServerConfig::try_parse_from(["sys-mcp", "--jev-guard-threshold", "-0.1"]);
    assert!(result.is_err(), "threshold < 0.0 must be rejected");
}

#[test]
fn test_jev_guard_fail_mode_cli_override_and_default() {
    // Default is Closed (destructive tools fail closed per JEV_PLAN.md).
    if std::env::var("SYS_MCP_JEV_GUARD_FAIL_MODE").is_ok() {
        return;
    }
    let config = ServerConfig::try_parse_from(["sys-mcp"]).expect("args parse");
    assert_eq!(config.jev.guard_fail_mode, GuardFailMode::Closed);

    // Explicit closed is accepted.
    let config = ServerConfig::try_parse_from(["sys-mcp", "--jev-guard-fail-mode", "closed"])
        .expect("args parse");
    assert_eq!(config.jev.guard_fail_mode, GuardFailMode::Closed);

    // Opting into availability-first behaviour.
    let config = ServerConfig::try_parse_from(["sys-mcp", "--jev-guard-fail-mode", "open"])
        .expect("args parse");
    assert_eq!(config.jev.guard_fail_mode, GuardFailMode::Open);

    // Anything else is rejected rather than silently defaulting.
    let result = ServerConfig::try_parse_from(["sys-mcp", "--jev-guard-fail-mode", "maybe"]);
    assert!(result.is_err(), "unknown fail mode must be rejected");
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
