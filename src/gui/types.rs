use serde::{Deserialize, Serialize};

// ─── Display ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Screenshot {
    /// PNG-encoded image bytes, as produced by the backend and consumed by
    /// every downstream caller (`image::load_from_memory`, and base64-encoded
    /// verbatim as `image/png` in tool responses). This is NOT raw pixel
    /// data — decoding it yourself would be wrong.
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Split a key chord like `Ctrl+Shift+S` into its normalized key names.
///
/// Normalizes case and trims whitespace around each key so `Ctrl + S` and
/// `ctrl+s` produce the same vector. Every entry point that accepts a chord
/// must use this: the MCP handler and the web preview previously normalized
/// differently, so `Ctrl + S` worked over MCP but sent `["ctrl ", " s"]`
/// (and failed) over the web API.
pub fn split_key_combo(key: &str) -> Vec<String> {
    key.to_lowercase()
        .split('+')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub primary: bool,
    pub resolution: Resolution,
    pub position: (i32, i32),
}

// ─── Input ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

// ─── Window ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_focused: bool,
    pub process_name: Option<String>,
    pub process_id: Option<u32>,
}

// ─── Accessibility ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityNode {
    pub id: String,
    pub role: String,
    pub name: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Center x — use for gui_click.
    pub cx: Option<i32>,
    /// Center y — use for gui_click.
    pub cy: Option<i32>,
    pub children: Vec<AccessibilityNode>,
    pub actions: Vec<String>,
    pub states: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityQuery {
    pub name: Option<String>,
    pub role: Option<String>,
    pub window_id: Option<u64>,
    pub max_depth: Option<u32>,
}

// ─── Template Matching ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMatchResult {
    pub x: u32,
    pub y: u32,
    pub cx: u32,
    pub cy: u32,
    pub confidence: f32,
}

// ─── System Info ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub hostname: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub capabilities: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::split_key_combo;

    #[test]
    fn split_normalizes_case_and_whitespace() {
        // The regression this guards: the web preview used to skip both
        // lowercasing and trimming, so "Ctrl + S" produced ["ctrl ", " s"]
        // and failed, while the MCP handler handled it fine.
        assert_eq!(split_key_combo("Ctrl+Shift+S"), vec!["ctrl", "shift", "s"]);
        assert_eq!(split_key_combo("ctrl + s"), vec!["ctrl", "s"]);
        assert_eq!(split_key_combo(" CTRL+S "), vec!["ctrl", "s"]);
        assert_eq!(split_key_combo("s"), vec!["s"]);
    }

    #[test]
    fn split_drops_empty_segments() {
        // A stray or doubled '+' must not yield an empty key name, which
        // would reach the backend and fail validation.
        assert_eq!(split_key_combo("ctrl++s"), vec!["ctrl", "s"]);
        assert_eq!(split_key_combo("+"), Vec::<String>::new());
    }
}
