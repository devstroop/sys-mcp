#![allow(dead_code)]
#[cfg(feature = "detection")]
use crate::gui::types::*;
use crate::gui::GuiClient;
#[cfg(feature = "detection")]
use crate::protocol::mcp::ContentItem;
use crate::protocol::mcp::ToolResult;
use serde_json::Value;

/// Message returned when the binary was built without the `detection` feature.
/// The tools stay advertised in `tools/list`; failing here rather than
/// gating the dispatch arm keeps them reachable, so an agent gets an
/// actionable error instead of the dispatcher's generic "unknown tool".
#[cfg(not(feature = "detection"))]
const DETECTION_DISABLED: &str =
    "Object detection is not available in this build — rebuild with `--features detection`.";

#[cfg(feature = "detection")]
pub(crate) async fn handle_detect_objects(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let min_confidence = args
        .get("min_confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.3) as f32;

    let filter_labels: Option<Vec<String>> =
        args.get("labels").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    let result = client.detect_objects().await.map_err(|e| e.to_string())?;

    let mut detections: Vec<_> = result.detections;

    // Filter by confidence
    detections.retain(|d| d.confidence >= min_confidence);

    // Filter by labels if specified
    if let Some(labels) = &filter_labels {
        let labels_lower: Vec<String> = labels.iter().map(|l| l.to_lowercase()).collect();
        detections.retain(|d| {
            labels_lower
                .iter()
                .any(|l| d.label.to_lowercase().contains(l))
        });
    }

    if detections.is_empty() {
        return Ok(ToolResult::text(
            "No objects detected. Try lowering min_confidence or checking what's on screen.",
        ));
    }

    // Format output
    let mut output = String::from("Detected objects:\n");
    for (i, det) in detections.iter().enumerate() {
        output.push_str(&format!(
            "{}: {} (conf: {:.2}) at {},{} size {}x{}\n",
            i, det.label, det.confidence, det.x, det.y, det.width, det.height
        ));
    }
    output.push_str("\nUse detect_click_object with label and index to click.");

    Ok(ToolResult {
        content: vec![ContentItem::text(output)],
        is_error: None,
    })
}

#[cfg(feature = "detection")]
pub(crate) async fn handle_click_object(
    client: &GuiClient,
    args: &Value,
) -> Result<ToolResult, String> {
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .ok_or("missing 'label' argument")?;

    let index = args.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let result = client.detect_objects().await.map_err(|e| e.to_string())?;

    // Filter by label
    let matches: Vec<_> = result
        .detections
        .iter()
        .filter(|d| d.label.to_lowercase().contains(&label.to_lowercase()))
        .collect();

    if matches.is_empty() {
        return Err(format!("No objects found with label '{}'", label));
    }

    if index >= matches.len() {
        return Err(format!(
            "Index {} out of range (found {} objects)",
            index,
            matches.len()
        ));
    }

    let target = matches[index];
    client
        .click(target.cx as u32, target.cy as u32, MouseButton::Left)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ToolResult::text(format!(
        "Clicked {} at ({}, {})",
        target.label, target.cx, target.cy
    )))
}

// Feature-off fallbacks. Mirrors the pattern used by `ocr` and `decision`:
// the tool stays in `tools/list` and fails with an actionable message rather
// than disappearing from the dispatcher as an "unknown tool".
#[cfg(not(feature = "detection"))]
pub(crate) async fn handle_detect_objects(
    _client: &GuiClient,
    _args: &Value,
) -> Result<ToolResult, String> {
    Err(DETECTION_DISABLED.to_string())
}

#[cfg(not(feature = "detection"))]
pub(crate) async fn handle_click_object(
    _client: &GuiClient,
    _args: &Value,
) -> Result<ToolResult, String> {
    Err(DETECTION_DISABLED.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// System handlers
// ═══════════════════════════════════════════════════════════════════════════
