pub mod accessibility;
pub mod decision;
pub mod detection;
pub mod filesystem;
pub mod mcp_hub;
pub mod ocr;
pub mod shell;
pub mod system;
pub mod ui;
pub mod vision;
pub mod window;

use serde_json::Value;

use crate::gui::types::*;
use crate::gui::GuiClient;
use crate::protocol::mcp::ToolResult;

pub async fn handle_tool_call(client: &GuiClient, tool_name: &str, args: Value) -> ToolResult {
    // Opt-in JEV risk guard (requires the `jev` cargo feature *and*
    // --jev-guardrails / SYS_MCP_JEV_GUARDRAILS). With either missing this
    // compiles away / returns immediately — zero behavior change.
    #[cfg(feature = "jev")]
    if let Some(held) = guard_gate(tool_name, &args).await {
        return held;
    }

    let result = match tool_name {
        // Display
        "screen_capture" => ui::handle_screenshot(client).await,
        "screen_capture_region" => ui::handle_screenshot_region(client, &args).await,
        "screen_size" => ui::handle_get_screen_size(client).await,
        "screen_list_monitors" => ui::handle_list_monitors(client).await,

        // OCR
        "screen_read" => ocr::handle_read_screen(client, &args).await,
        "screen_find_text" => ocr::handle_find_text(client, &args).await,

        // Mouse
        "mouse_click" => ui::handle_click(client, &args).await,
        "mouse_double_click" => ui::handle_double_click(client, &args).await,
        "mouse_move" => ui::handle_mouse_move(client, &args).await,
        "mouse_position" => ui::handle_mouse_position(client).await,
        "mouse_drag" => ui::handle_drag(client, &args).await,
        "mouse_scroll" => ui::handle_scroll(client, &args).await,

        // Keyboard
        "keyboard_type" => ui::handle_type_text(client, &args).await,
        "keyboard_press" => ui::handle_press_key(client, &args).await,

        // Clipboard
        "clipboard_get" => ui::handle_get_clipboard(client).await,
        "clipboard_set" => ui::handle_set_clipboard(client, &args).await,

        // Window Management
        "window_list" => window::handle_list_windows(client).await,
        "window_active" => window::handle_get_active_window(client).await,
        "window_focus" => window::handle_focus_window(client, &args).await,
        "window_move_resize" => window::handle_move_resize_window(client, &args).await,
        "window_action" => window::handle_window_action(client, &args).await,
        "window_find" => window::handle_find_windows(client, &args).await,

        // Accessibility
        "accessibility_tree" => accessibility::handle_accessibility_tree(client, &args).await,
        "accessibility_find" => accessibility::handle_find_ui_element(client, &args).await,

        // Template matching
        "vision_find_image" => vision::handle_find_image(client, &args).await,
        "vision_wait_for_image" => vision::handle_wait_for_image(client, &args).await,

        // Object detection. The arms are ungated (unlike the handlers' real
        // bodies) so the tools stay reachable when the `detection` feature is
        // off — otherwise an agent sees them in tools/list and gets a
        // confusing "unknown tool" from the dispatcher.
        "detect_objects" => detection::handle_detect_objects(client, &args).await,
        "detect_click_object" => detection::handle_click_object(client, &args).await,

        // JEV decision (feature-gated inside the handler so the tool stays
        // advertised and fails with a clear message when not compiled in)
        "decision" => decision::handle_decision(client, &args).await,
        // Phase 3: relevance-rank large text blocks. Never guarded (see below).
        "decision_rank" => decision::handle_decision_rank(client, &args).await,
        // Phase 5: local cost ledger. Reads no model and mutates nothing on
        // the machine, so it is always available.
        "decision_stats" => decision::handle_decision_stats(client, &args).await,

        // Utility
        "system_wait" => system::handle_wait(&args).await,
        "screen_scroll_to_text" => ocr::handle_scroll_to_text(client, &args).await,

        // Web preview. When the `web-preview` feature is ON, `server.rs`
        // intercepts this name and serves the running preview URL before
        // dispatch reaches here. When it is OFF this arm is the only one
        // that can match, so it must return an actionable error rather than
        // letting the tool fall through as "unknown tool".
        "web_preview" => {
            #[cfg(feature = "web-preview")]
            {
                // Unreachable in a web-preview build (server.rs intercepts),
                // but keep the message correct if that ever changes.
                Err("web preview is not running — start the server with the web-preview feature enabled".to_string())
            }
            #[cfg(not(feature = "web-preview"))]
            {
                Err("Web preview is not available in this build — rebuild with `--features web-preview`.".to_string())
            }
        }

        // System
        "system_info" => system::handle_system_info(client).await,

        // File System
        "fs_read" => filesystem::handle_read_file(client, &args).await,
        "fs_write" => filesystem::handle_write_file(client, &args).await,
        "fs_list" => filesystem::handle_list_dir(client, &args).await,
        "fs_exists" => filesystem::handle_file_exists(client, &args).await,
        "fs_delete" => filesystem::handle_delete_file(client, &args).await,
        "fs_create_dir" => filesystem::handle_create_dir(client, &args).await,

        // Shell/Terminal
        "shell_exec" => shell::handle_shell_exec(client, &args).await,
        "shell_open" => shell::handle_shell_open(client, &args).await,
        "shell_write" => shell::handle_shell_write(client, &args).await,
        "shell_read" => shell::handle_shell_read(client, &args).await,
        "shell_close" => shell::handle_shell_close(client, &args).await,
        "shell_list" => shell::handle_shell_list(client).await,

        // MCP Hub
        "mcp_discover" => mcp_hub::handle_mcp_discover(client).await,
        "mcp_list" => mcp_hub::handle_mcp_list(client).await,
        "mcp_register" => mcp_hub::handle_mcp_register(client, &args).await,
        "mcp_unregister" => mcp_hub::handle_mcp_unregister(client, &args).await,
        "mcp_start" => mcp_hub::handle_mcp_start(client, &args).await,
        "mcp_stop" => mcp_hub::handle_mcp_stop(client, &args).await,
        "mcp_tools" => mcp_hub::handle_mcp_tools(client).await,
        "mcp_tool_groups" => mcp_hub::handle_mcp_tool_groups(client).await,
        "mcp_exec" => mcp_hub::handle_mcp_exec(client, &args).await,

        // Process Manager
        "process_list" => system::handle_process_list(client, &args).await,
        "process_kill" => system::handle_process_kill(client, &args).await,
        "process_info" => system::handle_process_info(client, &args).await,
        "process_start" => system::handle_process_start(client, &args).await,

        // Service Manager
        "service_list" => system::handle_service_list(client).await,
        "service_start" => system::handle_service_start(client, &args).await,
        "service_stop" => system::handle_service_stop(client, &args).await,
        "service_status" => system::handle_service_status(client, &args).await,

        // Network Tools
        "network_info" => system::handle_network_info(client).await,
        "network_connections" => system::handle_network_connections(client, &args).await,

        // System Monitoring
        "system_stats" => system::handle_system_stats(client).await,
        "disk_usage" => system::handle_disk_usage(client).await,

        // Log Viewer
        "system_logs" => system::handle_system_logs(client, &args).await,

        _ => Err(format!("unknown tool: {tool_name}")),
    };

    match result {
        Ok(tr) => tr,
        Err(e) => {
            let msg = if e.contains("not supported") || e.contains("not yet implemented") {
                format!("{e}. This is a permanent error — do not retry.")
            } else {
                e
            };
            ToolResult::error(msg)
        }
    }
}

// ─── JEV risk guard (opt-in; `jev` cargo feature) ─────────────────────────

/// Pre-dispatch gate for guarded tools. Returns `Some(result)` when the
/// call must not proceed (held for confirmation, or denied by the Rust
/// denylist), `None` to dispatch normally.
///
/// Layer order (see `decision::guard` docs): hardcoded Rust denylist first
/// (never bypassed, model-free, offline), then the caller's `confirm: true`
/// acknowledgment (bypasses only the model check), then the JEV `noul`
/// judgment. Model failure fails *open* — a decision model must not become
/// an availability single point of failure — but the denylist still applies.
#[cfg(feature = "jev")]
async fn guard_gate(tool_name: &str, args: &Value) -> Option<ToolResult> {
    use crate::decision::client::DecisionClient;
    use crate::decision::guard::{self, GuardAction};

    let config = crate::decision::config();
    if !config.guardrails {
        return None; // guardrails off (the default) — behave exactly as before
    }
    // The decision tools are never guarded. `decision` and `decision_rank`
    // would recurse if they were; `decision_stats` reads local bookkeeping
    // only. None of them mutate the machine.
    if matches!(tool_name, "decision" | "decision_rank" | "decision_stats") {
        return None;
    }
    if !guard::is_guarded(&config, tool_name) {
        return None;
    }

    // Layer 1: hard Rust denylist. Applies before any network call and is
    // never bypassed by `confirm: true` or by the model.
    if let Some(verdict) = guard::check_denylist(tool_name, args) {
        return Some(ToolResult::error(format!(
            "Refused by the JEV guard denylist: {tool_name} matched a hard-blocked destructive pattern ({}). This is deterministic Rust policy and cannot be overridden with confirm=true.",
            verdict.reason
        )));
    }

    // Layer 3 (caller confirmation) skips the model judgment entirely — no
    // network call, no score. The denylist above still applied.
    if guard::confirm_requested(args) {
        log::debug!(
            "JEV guard: confirm=true supplied by caller for {tool_name}; skipping model judgment"
        );
        return None;
    }

    // Layer 2: one `noul` judgment against the redacted state.
    let client = match DecisionClient::new(&config) {
        Ok(c) => c,
        Err(e) => {
            // Fail open (denylist already applied).
            log::warn!("JEV guard skipped for {tool_name}: failed to build decision client: {e}");
            return None;
        }
    };
    match guard::assess(&client, tool_name, args).await {
        Ok(verdict) => match verdict.action {
            GuardAction::Allow => None,
            GuardAction::Confirm => Some(ToolResult::error(format!(
                "Action held for confirmation by the JEV guard: {tool_name} was scored potentially unsafe to run automatically (noul={:.2}, threshold={:.2}; {}). Nothing was executed. If this action is intended, re-call {tool_name} with the same arguments plus \"confirm\": true; otherwise drop the call.",
                verdict.noul, config.guard_threshold, verdict.reason
            ))),
            // Defensive: `assess` never returns Deny (only the denylist can,
            // and it already ran above).
            GuardAction::Deny => Some(ToolResult::error(format!(
                "Refused by the JEV guard: {tool_name} ({})",
                verdict.reason
            ))),
        },
        Err(e) => {
            // `assess` fail-opens internally; this arm is defensive. Do not
            // block the user on guard plumbing errors.
            log::warn!("JEV guard assess failed for {tool_name}: {e} (failing open)");
            None
        }
    }
}

// ─── Argument helpers ─────────────────────────────────────────────────────

pub(crate) fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

pub(crate) fn u32_arg(args: &Value, key: &str) -> Result<u32, String> {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

pub(crate) fn u64_arg(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

pub(crate) fn opt_u32(args: &Value, key: &str, default: u32) -> u32 {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .unwrap_or(default)
}

pub(crate) fn parse_button(args: &Value) -> MouseButton {
    match args.get("button").and_then(Value::as_str).unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    }
}
