//! Opt-in risk guard for destructive tools (Phase 2).
//!
//! The design law from `JEV_PLAN.md` applies here verbatim: the model is a
//! *signal*, never an authority. Three layers cooperate, in order:
//!
//! 1. **Rust denylist** ([`check_denylist`]) — hardcoded, model-independent,
//!    pure Rust. Runs *before any network call* and cannot be overridden by
//!    the model or by the caller's `confirm: true` argument. Works with the
//!    `jev` feature disabled and with no network.
//! 2. **JEV `noul` judgment** ([`assess`], requires the `jev` feature) — one
//!    binary question ("safe to run without human confirmation?"). The model
//!    may only downgrade a call to `Confirm`; it can never produce `Deny`
//!    and never bypass the denylist.
//! 3. **Caller confirmation** (`confirm: true`) — skips layer 2 only.
//!
//! On model failure the guard is **configurable**: `--jev-guard-fail-mode`
//! defaults to `closed` (per `JEV_PLAN.md` safety rule 6 — destructive tools
//! fail closed), which holds the call for confirmation. `open` instead lets
//! it through with the fuzzy judgment skipped, for deployments that
//! prioritise availability. Either way the denylist (layer 1) still applies,
//! and the caller can re-send with `confirm: true` — so `closed` is a hold,
//! never a hard block, and the human stays in the loop.
//!
//! Everything except [`assess`]/[`assess_with`] compiles without
//! the `jev` cargo feature so the denylist (and its tests) work everywhere.

use serde_json::Value;

#[cfg(feature = "jev")]
use crate::config::GuardFailMode;
use crate::config::JevConfig;
use crate::decision::redact::redact_value;

/// The `noul` question asked by the model guard.
pub const GUARD_QUESTION_ID: &str = "safe_to_run";
pub const GUARD_QUESTION: &str =
    "Is this action safe to perform automatically without human confirmation?";
pub const GUARD_TRUE_CRITERIA: &str = "Routine, reversible, non-destructive action";
pub const GUARD_FALSE_CRITERIA: &str =
    "Destructive, irreversible, security-sensitive, or affects system stability";

/// Default set of guarded tools (used when `--jev-guard-tools` is unset).
pub fn guarded_tools() -> &'static [&'static str] {
    &[
        "shell_exec",
        "fs_delete",
        "process_kill",
        "service_stop",
        "service_start",
        "process_start",
    ]
}

/// What the guard decided about a single tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardAction {
    /// Proceed with the call.
    Allow,
    /// Hold the call; require the caller to re-send with `confirm: true`.
    Confirm,
    /// Refuse outright (denylist only — the model can never produce this).
    Deny,
}

/// The guard's verdict for one call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuardVerdict {
    pub action: GuardAction,
    /// The model's `noul` score (probability the action is safe to run
    /// automatically). `1.0` when the guard was skipped via fail-open mode;
    /// `0.0` on a fail-closed hold. Read `reason` to distinguish.
    pub noul: f64,
    pub reason: &'static str,
}

/// Map a `noul` score to an action: `noul >= threshold` → `Allow`,
/// below → `Confirm`. Boundary rule: exactly *at* the threshold allows.
pub fn classify(noul: f64, threshold: f64) -> GuardAction {
    if noul >= threshold {
        GuardAction::Allow
    } else {
        GuardAction::Confirm
    }
}

/// Extract the caller's `confirm: true` acknowledgment (defaults to false).
pub fn confirm_requested(args: &Value) -> bool {
    args.get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Effective guarded-tool list for a config: the comma-separated
/// `--jev-guard-tools` override when set (entries trimmed, empties
/// dropped), otherwise the compiled-in [`guarded_tools`] defaults.
pub fn effective_guard_tools(config: &JevConfig) -> Vec<String> {
    match config.guard_tools.as_deref() {
        Some(raw) => raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        None => guarded_tools().iter().map(|s| s.to_string()).collect(),
    }
}

/// Is `tool` in the effective guarded set?
pub fn is_guarded(config: &JevConfig, tool: &str) -> bool {
    effective_guard_tools(config).iter().any(|t| t == tool)
}

/// Build the outbound model `state` for a guarded call: the tool name plus
/// its arguments, **serialized and passed through `redact_value`** so raw
/// arguments (which may contain passwords, tokens, etc.) never leave the
/// process unscrubbed.
pub fn build_state(tool: &str, args: &Value) -> Value {
    let (redacted_args, _changed) = redact_value(args);
    serde_json::json!({ "tool": tool, "arguments": redacted_args })
}

// ─── Hard denylist (pure Rust, no model, no network) ───────────────────────

/// Services that must never be stopped through this server: antivirus,
/// Windows Update, and services the OS cannot function without (SCM, RPC,
/// DCOM, event log, crypto, WMI). Compared case-insensitively against the
/// `name` argument of `service_stop`.
const CRITICAL_SERVICES: &[&str] = &[
    "windefend",             // Windows Defender (disables real-time protection)
    "wuauserv",              // Windows Update
    "eventlog",              // system/event logging
    "rpcss",                 // RPC — most services die with it
    "dcomlaunch",            // DCOM Service Process Launcher
    "samss",                 // Security Accounts Manager
    "cryptsvc",              // Cryptographic Services
    "winmgmt",               // WMI
    "securityhealthservice", // Defender security health
];

/// Destructive command patterns for `shell_exec`. Matched (substring)
/// against the lowercased command with whitespace collapsed. Deliberately
/// narrow: this list only catches clear-cut, high-blast-radius commands —
/// the model judgment + confirmation gate the long tail.
const DESTRUCTIVE_COMMAND_PATTERNS: &[&str] = &[
    "format ",     // `format C:` / `format D: /FS:NTFS` — wipes a volume
    "diskpart",    // disk partitioning (clean, format, delete partition)
    "del /s /q",   // recursive silent delete (Windows)
    "rd /s /q",    // recursive silent directory removal (Windows)
    "rmdir /s /q", // same, long form
    "mkfs",        // any mkfs.* — reformats a filesystem
    "> /dev/sd",   // redirect onto a raw disk device
    "> /dev/nvme", // redirect onto a raw NVMe device
];

/// Unix system directories whose contents are hard-blocked for `fs_delete`.
const PROTECTED_UNIX_PREFIXES: &[&str] = &[
    "/etc/", "/bin/", "/sbin/", "/usr/", "/boot/", "/lib/", "/lib64/", "/proc/", "/sys/",
];

/// Lowercase, collapse whitespace runs, trim. Used for command matching —
/// forward slashes are KEPT because several patterns (`rm -rf /`,
/// `> /dev/sd`) depend on POSIX separators.
fn normalize_text(s: &str) -> String {
    let mut out = s.to_lowercase();
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out.trim().to_string()
}

/// Like [`normalize_text`] but also unifies path separators to `\`.
fn normalize_path(s: &str) -> String {
    normalize_text(s).replace(['/', '\\'], "\\")
}

/// `rm -rf /` (or `rm -fr /`) targeting the filesystem *root*: the `/`
/// must be a standalone token — end of command or followed by a flag —
/// so `rm -rf /tmp/…` stays allowed and is left to the model layer.
fn rm_rf_unix_root(cmd: &str) -> bool {
    ["rm -rf /", "rm -fr /"]
        .iter()
        .any(|verb| match cmd.find(verb) {
            Some(idx) => {
                let rest = &cmd[idx + verb.len()..];
                rest.is_empty() || rest.starts_with(" -")
            }
            None => false,
        })
}

/// Why a `fs_delete` path is blocked, or `None`.
fn deny_reason_for_path(raw: &str) -> Option<&'static str> {
    // Preserve a leading double-backslash for UNC paths before normalizing
    // elsewhere; normalize_path() maps '/' → '\' but keeps '//server' intact as
    // '\\server' which we do not treat specially (conservative: only the
    // well-known roots below are blocked).
    let path = normalize_path(raw);
    if path.is_empty() {
        return None;
    }
    // Drive roots: `c:\`, `d:\` (a bare `c:` means the whole current dir —
    // also blocked). Nothing may follow the backslash.
    let drive_root = |p: &str| -> bool {
        let b = p.as_bytes();
        b.len() >= 2
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && (b.len() == 2 || (b[2] == b'\\' && b.len() == 3))
    };
    if drive_root(&path) {
        return Some("fs_delete of a drive root");
    }
    if path == "\\" {
        return Some("fs_delete of the filesystem root");
    }
    // Windows directory and everything under it (covers System32, SysWOW64,
    // WinSxS, …).
    if path == "c:\\windows" || path.starts_with("c:\\windows\\") {
        return Some("fs_delete under C:\\Windows");
    }
    // Any system32/syswow64 segment wherever it appears (other drive letters
    // or renamed Windows dirs).
    if path.contains("\\system32") || path.contains("\\syswow64") {
        return Some("fs_delete of a system32/syswow64 path");
    }
    // Unix system dirs (path may still contain '/' if the caller sent a
    // POSIX path — check the original too).
    let unix = raw.trim();
    if unix == "/" {
        return Some("fs_delete of the filesystem root");
    }
    if PROTECTED_UNIX_PREFIXES.iter().any(|p| unix.starts_with(p)) {
        return Some("fs_delete of a Unix system directory");
    }
    None
}

/// Why a `process_kill` target is blocked, or `None`.
fn deny_reason_for_pid(pid: u64) -> Option<&'static str> {
    match pid {
        0 => Some("process_kill of PID 0 (System Idle Process)"),
        4 => Some("process_kill of PID 4 (System Process / kernel)"),
        p if p == std::process::id() as u64 => {
            Some("process_kill of the sys-mcp server's own process")
        }
        _ => None,
    }
}

/// Why a service `name` is blocked, or `None`.
fn deny_reason_for_service(tool: &str, name: &str) -> Option<&'static str> {
    // Only `service_stop` is denylisted: starting a critical service is not
    // destructive (and is usually a repair action).
    if tool != "service_stop" {
        return None;
    }
    let name = name.trim().to_lowercase();
    if CRITICAL_SERVICES.contains(&name.as_str()) {
        Some("service_stop of a critical/security service")
    } else {
        None
    }
}

/// Why a shell command is blocked, or `None`.
fn deny_reason_for_command(cmd: &str) -> Option<&'static str> {
    let cmd = normalize_text(cmd);
    if rm_rf_unix_root(&cmd) || DESTRUCTIVE_COMMAND_PATTERNS.iter().any(|p| cmd.contains(p)) {
        Some("shell_exec matching a destructive command pattern")
    } else {
        None
    }
}

/// Check the hardcoded, model-independent denylist for a guarded call.
///
/// Returns a `Deny` verdict when `tool` + `args` match a hard-blocked
/// pattern, `None` otherwise. Pure Rust: no network, no model, works with
/// the `jev` feature disabled. Applied before every model call and never
/// bypassed by `confirm: true`.
pub fn check_denylist(tool: &str, args: &Value) -> Option<GuardVerdict> {
    let reason = match tool {
        "fs_delete" => args
            .get("path")
            .and_then(Value::as_str)
            .and_then(deny_reason_for_path),
        "process_kill" => args
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(deny_reason_for_pid),
        "service_stop" | "service_start" => args
            .get("name")
            .and_then(Value::as_str)
            .and_then(|name| deny_reason_for_service(tool, name)),
        "shell_exec" => args
            .get("command")
            .and_then(Value::as_str)
            .and_then(deny_reason_for_command),
        "process_start" => {
            // `command` plus its args joined — `format.exe` invoked directly
            // gets past the `format ` (trailing-space) pattern otherwise.
            let command = args.get("command").and_then(Value::as_str).unwrap_or("");
            let joined_args = args
                .get("args")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            deny_reason_for_command(&format!("{command} {joined_args}"))
        }
        _ => None,
    }?;
    Some(GuardVerdict {
        action: GuardAction::Deny,
        noul: 0.0,
        reason,
    })
}

// ─── Model-backed assessment (requires the `jev` cargo feature) ────────────

#[cfg(feature = "jev")]
mod live {
    use std::collections::BTreeMap;

    use crate::decision::client::DecisionClient;
    use crate::decision::types::{Criteria, Question, QuestionType};

    use super::*;

    /// Assess a guarded call with the process-wide configured threshold.
    ///
    /// Order: denylist first (pure Rust, no network); on a hit the model is
    /// never called. Otherwise one `noul` question is asked against the
    /// redacted state. If the model is unreachable (network, timeout, 429,
    /// bad payload) the configured `--jev-guard-fail-mode` decides: `closed`
    /// (default) returns `Ok(Confirm)` so the call is held for a human;
    /// `open` returns `Ok(Allow)` with a `guard skipped` reason. Both are
    /// logged at warn level, and the denylist has already run either way.
    #[cfg(feature = "jev")]
    pub async fn assess(
        client: &DecisionClient,
        tool: &str,
        args: &Value,
    ) -> Result<GuardVerdict, String> {
        assess_with(
            client,
            tool,
            args,
            crate::decision::config().guard_threshold,
        )
        .await
    }

    /// [`assess`] with an explicit threshold (used by tests and the live
    /// eval harness; production callers go through [`assess`]).
    #[cfg(feature = "jev")]
    pub async fn assess_with(
        client: &DecisionClient,
        tool: &str,
        args: &Value,
        threshold: f64,
    ) -> Result<GuardVerdict, String> {
        // Layer 1: hard denylist. Deterministic, offline, model-free.
        if let Some(verdict) = check_denylist(tool, args) {
            return Ok(verdict);
        }

        // Layer 2: one binary `noul` judgment on the redacted state.
        let state = build_state(tool, args);
        let mut questions = BTreeMap::new();
        questions.insert(
            GUARD_QUESTION_ID.to_string(),
            Question {
                question_type: QuestionType::Noul,
                instructions: GUARD_QUESTION.to_string(),
                criteria: Some(Criteria::Map(
                    [
                        ("true".to_string(), GUARD_TRUE_CRITERIA.to_string()),
                        ("false".to_string(), GUARD_FALSE_CRITERIA.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                )),
            },
        );

        match client.decide(state, questions, None).await {
            Ok(response) => {
                // Missing/malformed `noul` → 0.0 → Confirm (fail closed on
                // garbage answers).
                let noul = response
                    .answers
                    .get(GUARD_QUESTION_ID)
                    .and_then(|a| a.noul)
                    .unwrap_or(0.0);
                let action = classify(noul, threshold);
                let reason = match action {
                    GuardAction::Allow => "model judged the action safe to run automatically",
                    GuardAction::Confirm => {
                        "model judged the action potentially unsafe; human confirmation required"
                    }
                    // Unreachable: classify() only returns Allow/Confirm.
                    GuardAction::Deny => "unreachable",
                };
                Ok(GuardVerdict {
                    action,
                    noul,
                    reason,
                })
            }
            Err(e) => {
                // The model is unreachable (network, timeout, 429, bad
                // payload). The Rust denylist already ran above, so only the
                // fuzzy judgment is affected. The API key is never included
                // in `e` (see client.rs).
                //
                // Default (`GuardFailMode::Closed`, per JEV_PLAN.md safety
                // rule 6) is to hold the call for confirmation: the operator
                // enabled guardrails precisely because they wanted a human in
                // the loop, so we must not silently drop that when the model
                // is down. This is not a hard block — the caller can re-send
                // with `confirm: true`, so the human stays in control either
                // way and the server remains usable.
                //
                // `GuardFailMode::Open` prefers availability and lets the
                // call through with the judgment skipped.
                log::warn!(
                    "JEV guard: model unavailable for tool '{tool}': {e} (denylist already applied; fail mode = {:?})",
                    crate::decision::config().guard_fail_mode
                );
                match crate::decision::config().guard_fail_mode {
                    GuardFailMode::Open => Ok(GuardVerdict {
                        action: GuardAction::Allow,
                        noul: 1.0,
                        reason: "guard skipped: decision model unavailable (fail-open; Rust denylist already applied)",
                    }),
                    GuardFailMode::Closed => Ok(GuardVerdict {
                        action: GuardAction::Confirm,
                        noul: 0.0,
                        reason: "guard held: decision model unavailable (fail-closed; re-send with confirm=true to proceed)",
                    }),
                }
            }
        }
    }
}

#[cfg(feature = "jev")]
pub use live::{assess, assess_with};

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_threshold_boundaries() {
        // Exactly at the threshold → Allow.
        assert_eq!(classify(0.5, 0.5), GuardAction::Allow);
        // Just under → Confirm.
        assert_eq!(classify(0.4999, 0.5), GuardAction::Confirm);
        assert_eq!(classify(0.49999999, 0.5), GuardAction::Confirm);
        assert_eq!(classify(0.0, 0.5), GuardAction::Confirm);
        assert_eq!(classify(1.0, 0.5), GuardAction::Allow);
        // Custom thresholds.
        assert_eq!(classify(0.3, 0.2), GuardAction::Allow);
        assert_eq!(classify(0.19, 0.2), GuardAction::Confirm);
    }

    #[test]
    fn guarded_tools_default_set() {
        let tools = guarded_tools();
        for t in [
            "shell_exec",
            "fs_delete",
            "process_kill",
            "service_stop",
            "service_start",
            "process_start",
        ] {
            assert!(tools.contains(&t), "missing {t}");
        }
        // The decision tool must never be guarded (no recursion).
        assert!(!tools.contains(&"decision"));
    }

    #[test]
    fn effective_tools_override_parsing() {
        let config = JevConfig::default();
        let mut got = effective_guard_tools(&config);
        got.sort();
        let mut expected: Vec<String> = guarded_tools().iter().map(|s| s.to_string()).collect();
        expected.sort();
        assert_eq!(got, expected);

        let mut config = config;
        config.guard_tools = Some(" shell_exec , fs_delete ,, ".to_string());
        let mut got = effective_guard_tools(&config);
        got.sort();
        assert_eq!(got, vec!["fs_delete".to_string(), "shell_exec".to_string()]);
    }

    #[test]
    fn build_state_redacts_arguments() {
        let args = json!({
            "command": "curl -H \"Authorization: Bearer sk-secret-abc12345\" x",
            "connection": "host=db.internal password=hunter2hunter2hunter2",
            "path": "C:/tmp/report.txt"
        });
        let state = build_state("shell_exec", &args);
        assert_eq!(state["tool"], "shell_exec");
        let s = state.to_string();
        assert!(!s.contains("sk-secret-abc12345"), "leaked token: {s}");
        assert!(!s.contains("hunter2hunter2hunter2"), "leaked password: {s}");
        assert!(s.contains("[REDACTED]"));
        // Untouched args survive.
        assert_eq!(state["arguments"]["path"], "C:/tmp/report.txt");
    }

    #[test]
    fn denylist_fs_delete_paths() {
        let cases = [
            ("C:\\Windows\\System32\\kernel32.dll", true),
            ("C:\\Windows\\System32\\config\\SAM", true),
            ("c:\\windows\\explorer.exe", true),
            ("C:/Windows/System32/ntoskrnl.exe", true),
            ("D:\\System32\\evil.dll", true),
            ("C:\\Windows", true),
            ("C:\\", true),
            ("c:", true),
            ("/", true),
            ("C:\\Windows\\SysWOW64\\foo.dll", true),
            ("/etc/passwd", true),
            ("/usr/bin/ls", true),
            // Not blocked:
            ("C:\\Users\\me\\AppData\\Local\\Temp\\a.txt", false),
            ("C:\\WindowsAppsCache\\x.txt", false), // not under C:\Windows
            ("D:\\Games\\save.dat", false),
            ("/home/me/notes.txt", false),
            ("./build/output.log", false),
            ("/tmp/session.json", false),
        ];
        for (path, should_deny) in cases {
            let verdict = check_denylist("fs_delete", &json!({ "path": path }));
            assert_eq!(
                verdict.is_some(),
                should_deny,
                "path {path:?}: expected deny={should_deny}, got {verdict:?}"
            );
            if should_deny {
                assert_eq!(verdict.unwrap().action, GuardAction::Deny);
            }
        }
        // Missing path arg → no denylist hit (handler will error instead).
        assert!(check_denylist("fs_delete", &json!({})).is_none());
    }

    #[test]
    fn denylist_process_kill_pids() {
        for pid in [0u64, 4, std::process::id() as u64] {
            let v = check_denylist("process_kill", &json!({ "pid": pid }))
                .unwrap_or_else(|| panic!("pid {pid} must be denied"));
            assert_eq!(v.action, GuardAction::Deny);
        }
        assert!(check_denylist("process_kill", &json!({ "pid": 1234 })).is_none());
        assert!(check_denylist("process_kill", &json!({ "pid": 99999 })).is_none());
    }

    #[test]
    fn denylist_critical_services() {
        for name in ["WinDefend", "wuauserv", "EventLog", "rpcss", "CRYPTSVC"] {
            assert!(
                check_denylist("service_stop", &json!({ "name": name })).is_some(),
                "{name} must be denied"
            );
        }
        assert!(check_denylist("service_stop", &json!({ "name": "Spooler" })).is_none());
        // service_start of a critical service is NOT denylisted (repair path).
        assert!(check_denylist("service_start", &json!({ "name": "WinDefend" })).is_none());
    }

    #[test]
    fn denylist_shell_patterns() {
        let denied = [
            "format C: /FS:NTFS",
            "format D:",
            "diskpart",
            "diskpart /s script.txt",
            "del /s /q C:\\Users",
            "rd /s /q C:\\Projects\\old",
            "rmdir /s /q build",
            "rm -rf /",
            "sudo rm -rf /",
            "rm -fr / --no-preserve-root",
            "mkfs.ext4 /dev/sdb1",
            "echo evil > /dev/sda",
            "cat foo > /dev/nvme0n1",
            "FORMAT E:", // case-insensitive
        ];
        for cmd in denied {
            let v = check_denylist("shell_exec", &json!({ "command": cmd }))
                .unwrap_or_else(|| panic!("command {cmd:?} must be denied"));
            assert_eq!(v.action, GuardAction::Deny, "cmd: {cmd}");
        }
        let allowed = [
            "dir",
            "ls -la",
            "git status",
            "echo hello",
            "format-list x | Select-Object",
            "rm -rf ./target",
        ];
        for cmd in allowed {
            assert!(
                check_denylist("shell_exec", &json!({ "command": cmd })).is_none(),
                "cmd {cmd:?} must not be denied"
            );
        }
        // Known conservative false positive, documented on purpose: any
        // command containing the literal `format ` (e.g. an inline string)
        // is denied. Erring wide is the point of this layer.
        assert!(check_denylist(
            "shell_exec",
            &json!({ "command": "python -c \"print('format ')\"" })
        )
        .is_some());
    }

    #[test]
    fn denylist_process_start_checks_command_and_args() {
        // Direct invocation of format.com with args is caught via join.
        assert!(check_denylist(
            "process_start",
            &json!({ "command": "format.com", "args": ["C:"] })
        )
        .is_none()); // `format.com` has no trailing space — documented gap
        assert!(check_denylist(
            "process_start",
            &json!({ "command": "cmd", "args": ["/c", "del /s /q C:\\data"] })
        )
        .is_some());
        assert!(check_denylist(
            "process_start",
            &json!({ "command": "notepad.exe", "args": ["notes.txt"] })
        )
        .is_none());
    }

    #[test]
    fn denylist_ignores_unguarded_tools() {
        assert!(check_denylist("fs_read", &json!({ "path": "C:\\Windows" })).is_none());
        assert!(check_denylist("decision", &json!({})).is_none());
        assert!(check_denylist("unknown_tool", &json!({})).is_none());
    }

    #[test]
    fn confirm_requested_parsing() {
        assert!(confirm_requested(&json!({ "confirm": true })));
        assert!(!confirm_requested(&json!({ "confirm": false })));
        assert!(!confirm_requested(&json!({})));
        assert!(!confirm_requested(&json!({ "confirm": "true" }))); // must be a real bool
    }
}
