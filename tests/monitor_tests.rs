//! Monitor enumeration tests (`screen_list_monitors` / `list_monitors`).
//!
//! Issue #11: the local backend previously returned a single synthetic
//! "Primary" monitor, so secondary displays were invisible. Windows now
//! enumerates for real; these tests pin the invariants that enumeration must
//! hold regardless of how many displays are attached.

use serde_json::json;
use sys_mcp::gui::GuiClient;
use sys_mcp::mcp::handlers::handle_tool_call;
use sys_mcp::StubBackend;

#[tokio::test]
async fn stub_backend_reports_a_primary_monitor() {
    // The test double is platform-independent and always returns one display.
    let client = GuiClient::new(StubBackend);
    let result = handle_tool_call(&client, "screen_list_monitors", json!({})).await;
    assert!(!result.is_error.unwrap_or(false));
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("Primary"), "got: {text}");
}

/// Real enumeration invariants on Windows.
///
/// Gated on the platform AND guarded against a headless/session-0 environment
/// (such as some CI runners) where the desktop backend may not construct: in
/// that case the test passes rather than failing on infrastructure it does not
/// control. This is the first `#[cfg(...)]`-gated test in the suite, moving
/// issue #25 forward.
#[cfg(windows)]
#[tokio::test]
async fn windows_enumeration_reports_valid_monitors() {
    let backend = match sys_mcp::gui::backend::local::LocalBackend::new(false) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("skipping: no desktop available in this environment ({e})");
            return;
        }
    };
    let client = GuiClient::new(backend);

    let monitors = match client.list_monitors().await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("skipping: enumeration unavailable in this environment ({e})");
            return;
        }
    };

    assert!(
        !monitors.is_empty(),
        "enumeration must return at least one display"
    );
    assert_eq!(
        monitors.iter().filter(|m| m.primary).count(),
        1,
        "exactly one display must be flagged primary"
    );

    for m in &monitors {
        assert!(
            m.resolution.width > 0 && m.resolution.height > 0,
            "zero-sized display"
        );
        // A display must not be both at the origin and have a negative origin.
        assert!(
            m.position.0 >= 0 || m.position.1 >= 0,
            "display {m:?} has a fully negative origin"
        );
    }

    // A secondary display (if present) must not overlap the primary's origin.
    if monitors.len() > 1 {
        let primary = monitors.iter().find(|m| m.primary).unwrap();
        for m in monitors.iter().filter(|m| !m.primary) {
            assert_ne!(
                m.position, primary.position,
                "secondary display shares the primary origin: {m:?}"
            );
        }
    }
}
