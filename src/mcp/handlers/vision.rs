#![allow(dead_code)]
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use serde_json::{json, Value};

use crate::error::GuiError;
use crate::gui::types::Region;
use crate::gui::vision::ImageMatch;
use crate::gui::GuiClient;
use crate::protocol::mcp::ToolResult;

use super::u32_arg;

/// Default minimum NCC correlation. 0.8 matches rustautogui's own examples:
/// high enough to reject noise, low enough to tolerate minor anti-aliasing
/// or scaling differences between the template and the live screen.
const DEFAULT_PRECISION: f32 = 0.8;
const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const MAX_TIMEOUT_MS: u64 = 300_000; // 5 minutes

pub(crate) async fn handle_find_image(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let template = parse_template(args)?;
    let precision = parse_precision(args)?;
    let region = parse_region(args)?;

    let matches = client
        .find_image(template, precision, region)
        .await
        .map_err(vision_error)?;
    Ok(match_result(&matches, None))
}

pub(crate) async fn handle_wait_for_image(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let template = parse_template(args)?;
    let precision = parse_precision(args)?;
    let region = parse_region(args)?;
    let timeout_ms = parse_timeout_ms(args)?;

    let (matches, waited_ms) = client
        .wait_for_image(template, precision, region, timeout_ms)
        .await
        .map_err(vision_error)?;
    Ok(match_result(&matches, Some(waited_ms)))
}

/// Map backend errors to handler messages. The unavailable-capability case
/// gets a build-specific message on purpose: the dispatch layer appends
/// "This is a permanent error — do not retry." to any message containing
/// "not supported", which would be wrong here — template matching IS
/// available in local-GUI builds, only this backend lacks it.
fn vision_error(e: GuiError) -> String {
    match e {
        GuiError::UnsupportedCapability(_) => {
            "Template matching is not available in this build: the active GUI backend does not provide the vision capability."
                .to_string()
        }
        e => e.to_string(),
    }
}

fn match_result(matches: &[ImageMatch], waited_ms: Option<u64>) -> ToolResult {
    let mut out = json!({
        "found": !matches.is_empty(),
        "count": matches.len(),
        "matches": matches
            .iter()
            .map(|m| json!({ "x": m.x, "y": m.y, "confidence": m.confidence }))
            .collect::<Vec<_>>(),
        "note": "Match coordinates are the CENTER of the template. On Windows only the main monitor is searched; on Linux all monitors are searched.",
    });
    if let Some(ms) = waited_ms {
        out["waited_ms"] = json!(ms);
    }
    ToolResult::text(out.to_string())
}

/// Accept exactly one of `template_base64` (preferred — matches what fs_read
/// returns) or `template_path`. The bytes must be an ENCODED image
/// (PNG/JPEG/...): everything in this system that carries image bytes
/// (Screenshot.data, fs_read output) carries encoded bytes, which is what
/// `image::load_from_memory` (used by rustautogui) requires. Raw pixel data
/// is rejected here with an explicit error instead of failing opaquely
/// downstream.
fn parse_template(args: &Value) -> Result<Vec<u8>, String> {
    let b64 = args.get("template_base64").and_then(Value::as_str);
    let path = args.get("template_path").and_then(Value::as_str);
    let raw = match (b64, path) {
        (Some(_), Some(_)) => {
            return Err(
                "provide exactly one of template_base64 or template_path, not both".to_string(),
            )
        }
        (Some(b), None) => BASE64_STANDARD
            .decode(b)
            .map_err(|e| format!("template_base64 is not valid base64: {e}"))?,
        (None, Some(p)) => {
            std::fs::read(p).map_err(|e| format!("failed to read template_path '{p}': {e}"))?
        }
        (None, None) => {
            return Err("missing required argument: template_base64 or template_path".to_string())
        }
    };
    if raw.is_empty() {
        return Err("template image is empty".to_string());
    }
    image::load_from_memory(&raw).map_err(|e| {
        format!("template is not a decodable image (provide PNG/JPEG encoded bytes — base64 of raw pixel data is not accepted): {e}")
    })?;
    Ok(raw)
}

fn parse_precision(args: &Value) -> Result<f32, String> {
    match args.get("precision") {
        None | Some(Value::Null) => Ok(DEFAULT_PRECISION),
        Some(v) => {
            let p = v
                .as_f64()
                .ok_or_else(|| "precision must be a number in (0.0, 1.0]".to_string())?
                as f32;
            if !p.is_finite() || p <= 0.0 || p > 1.0 {
                return Err(format!("precision must be > 0.0 and <= 1.0, got {p}"));
            }
            Ok(p)
        }
    }
}

/// Search region — all-or-nothing, like screen_capture_region.
fn parse_region(args: &Value) -> Result<Option<Region>, String> {
    const KEYS: [&str; 4] = ["x", "y", "width", "height"];
    if KEYS.iter().all(|k| args.get(k).is_none()) {
        return Ok(None);
    }
    let width = u32_arg(args, "width")?;
    let height = u32_arg(args, "height")?;
    if width == 0 || height == 0 {
        return Err("search region width and height must be > 0".to_string());
    }
    Ok(Some(Region {
        x: u32_arg(args, "x")?,
        y: u32_arg(args, "y")?,
        width,
        height,
    }))
}

fn parse_timeout_ms(args: &Value) -> Result<u64, String> {
    match args.get("timeout_ms") {
        None | Some(Value::Null) => Ok(DEFAULT_TIMEOUT_MS),
        Some(v) => {
            let t = v
                .as_u64()
                .ok_or_else(|| "timeout_ms must be a non-negative integer".to_string())?;
            if t > MAX_TIMEOUT_MS {
                return Err(format!(
                    "timeout_ms must be <= {MAX_TIMEOUT_MS} (5 minutes), got {t}"
                ));
            }
            Ok(t)
        }
    }
}
