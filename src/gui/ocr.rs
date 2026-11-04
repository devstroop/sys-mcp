//! On-device OCR engine using `ocrs` (pure Rust).
//!
//! Adapted from rdp-mcp — reads local screen captures instead of RDP framebuffer.
//! Returns structured text + bounding-box coordinates (~2 KB) instead of multi-MB images.

use std::path::PathBuf;
use std::sync::OnceLock;

use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use rten::Model;
use serde::{Deserialize, Serialize};

use crate::error::GuiError;
use crate::gui::types::Screenshot;

// ─── Public result types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrLine {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub words: Vec<OcrWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrWord {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Center x — use this directly for `gui_click`.
    pub cx: i32,
    /// Center y — use this directly for `gui_click`.
    pub cy: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrMatch {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Center x — use this for `gui_click`.
    pub cx: i32,
    /// Center y — use this for `gui_click`.
    pub cy: i32,
    pub line_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub lines: Vec<OcrLine>,
    pub screen_width: u32,
    pub screen_height: u32,
}

// ─── Engine singleton ──────────────────────────────────────────────────────

static OCR_ENGINE: OnceLock<Result<OcrEngine, String>> = OnceLock::new();

fn models_dir() -> PathBuf {
    let mut dir = dirs_next::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("sys-mcp");
    dir.push("models");
    dir
}

fn model_path(name: &str) -> PathBuf {
    models_dir().join(name)
}

fn ensure_model(name: &str) -> Result<PathBuf, GuiError> {
    let path = model_path(name);
    if path.exists() {
        return Ok(path);
    }

    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| {
        GuiError::OcrError(format!("failed to create model dir {}: {e}", dir.display()))
    })?;

    let url = format!("https://ocrs-models.s3-accelerate.amazonaws.com/{name}");
    log::info!("downloading OCR model: {url} -> {}", path.display());

    // Atomic + verified + retried: a partial download can no longer leave a
    // truncated file behind that `path.exists()` would then treat as valid.
    crate::download::download_verified(&url, &path, &format!("OCR model {name}"))
        .map_err(GuiError::OcrError)?;

    log::info!("OCR model downloaded: {}", path.display());
    Ok(path)
}

fn get_engine() -> Result<&'static OcrEngine, GuiError> {
    let result = OCR_ENGINE.get_or_init(|| {
        let det_path = ensure_model("text-detection.onnx").map_err(|e| e.to_string())?;
        let rec_path = ensure_model("text-recognition.onnx").map_err(|e| e.to_string())?;

        let detection_model =
            Model::load_file(det_path).map_err(|e| format!("load detection model: {e}"))?;
        let recognition_model =
            Model::load_file(rec_path).map_err(|e| format!("load recognition model: {e}"))?;

        OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection_model),
            recognition_model: Some(recognition_model),
            ..Default::default()
        })
        .map_err(|e| format!("init OCR engine: {e}"))
    });

    match result {
        Ok(engine) => Ok(engine),
        Err(e) => Err(GuiError::OcrError(format!("OCR engine init failed: {e}"))),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn iou(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> f32 {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;

    let ax1 = ax;
    let ay1 = ay;
    let ax2 = ax + aw;
    let ay2 = ay + ah;

    let bx1 = bx;
    let by1 = by;
    let bx2 = bx + bw;
    let by2 = by + bh;

    let inter_w = (ax2.min(bx2) - ax1.max(bx1)).max(0);
    let inter_h = (ay2.min(by2) - ay1.max(by1)).max(0);
    let inter_area = inter_w * inter_h;

    let a_area = aw * ah;
    let b_area = bw * bh;
    let union_area = a_area + b_area - inter_area;

    if union_area == 0 {
        0.0
    } else {
        inter_area as f32 / union_area as f32
    }
}

fn boxes_overlap_vertically(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    let (_, ay, _, ah) = a;
    let (_, by, _, bh) = b;
    let overlap = (ay + ah).min(by + bh) - ay.max(by);
    let min_h = ah.min(bh);
    overlap > 0 && (overlap as f32 / min_h as f32) > 0.3
}

/// Merge overlapping or adjacent lines that belong to the same visual text line.
fn merge_lines(lines: Vec<OcrLine>, iou_threshold: f32) -> Vec<OcrLine> {
    if lines.len() < 2 {
        return lines;
    }

    let mut merged: Vec<OcrLine> = Vec::with_capacity(lines.len());

    for line in lines {
        let mut candidate = line;
        let mut readded = false;

        for existing in merged.iter_mut() {
            let a = (existing.x, existing.y, existing.width, existing.height);
            let b = (candidate.x, candidate.y, candidate.width, candidate.height);

            let overlap = iou(a, b) > iou_threshold
                || (boxes_overlap_vertically(a, b) && (b.0 - (a.0 + a.2)).abs() < a.2.max(b.2) / 2);

            if overlap {
                let x1 = existing.x.min(candidate.x);
                let y1 = existing.y.min(candidate.y);
                let x2 = (existing.x + existing.width).max(candidate.x + candidate.width);
                let y2 = (existing.y + existing.height).max(candidate.y + candidate.height);

                existing.text.push(' ');
                existing.text.push_str(&candidate.text);
                existing.x = x1;
                existing.y = y1;
                existing.width = x2 - x1;
                existing.height = y2 - y1;

                existing.words.append(&mut candidate.words);

                readded = true;
                break;
            }
        }

        if !readded {
            merged.push(candidate);
        }
    }

    merged
}

// ─── Public API ────────────────────────────────────────────────────────────

/// Run OCR on a screenshot and return structured text with positions.
pub fn read_screen(screenshot: &Screenshot) -> Result<OcrResult, GuiError> {
    let engine = get_engine()?;

    // Screenshot data is PNG-encoded bytes from save_screenshot
    let img = image::load_from_memory(&screenshot.data)
        .map_err(|e| GuiError::OcrError(format!("decode screenshot for OCR: {e}")))?
        .into_rgb8();

    let img_source = ImageSource::from_bytes(img.as_raw(), img.dimensions())
        .map_err(|e| GuiError::OcrError(format!("prepare OCR input: {e}")))?;

    let ocr_input = engine
        .prepare_input(img_source)
        .map_err(|e| GuiError::OcrError(format!("OCR prepare: {e}")))?;

    let word_rects = engine
        .detect_words(&ocr_input)
        .map_err(|e| GuiError::OcrError(format!("OCR detect: {e}")))?;

    let line_rects = engine.find_text_lines(&ocr_input, &word_rects);

    let line_texts = engine
        .recognize_text(&ocr_input, &line_rects)
        .map_err(|e| GuiError::OcrError(format!("OCR recognize: {e}")))?;

    let mut lines = Vec::new();

    for line_opt in &line_texts {
        let Some(line) = line_opt else { continue };
        let text = line.to_string();
        if text.trim().is_empty() || text.len() <= 1 {
            continue;
        }

        let line_rect = line.bounding_rect();
        let mut words = Vec::new();

        for word in line.words() {
            let word_text = word.to_string();
            if word_text.trim().is_empty() {
                continue;
            }
            let wr = word.bounding_rect();
            words.push(OcrWord {
                text: word_text,
                x: wr.left(),
                y: wr.top(),
                width: wr.width(),
                height: wr.height(),
                cx: wr.left() + wr.width() / 2,
                cy: wr.top() + wr.height() / 2,
            });
        }

        lines.push(OcrLine {
            text,
            x: line_rect.left(),
            y: line_rect.top(),
            width: line_rect.width(),
            height: line_rect.height(),
            words,
        });
    }

    let lines = merge_lines(lines, 0.5);

    let merged_text: String = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(OcrResult {
        text: merged_text,
        lines,
        screen_width: screenshot.width,
        screen_height: screenshot.height,
    })
}

/// Move OCR output from crop-local coordinates into screen coordinates.
///
/// [`read_screen`] reports boxes relative to whatever image it is handed, so a
/// region capture yields coordinates local to the crop. `screen_read` advertises
/// `cx`/`cy` as `mouse_click` targets — which are screen coordinates — so the
/// region origin has to be added back before the numbers leave the handler.
/// Without this, every region-based click landed up and left of its target by
/// exactly (x, y). See ISSUES.md #33.
pub fn translate_to_screen(result: &mut OcrResult, origin: (i32, i32)) {
    let (ox, oy) = origin;
    for line in &mut result.lines {
        line.x += ox;
        line.y += oy;
        for word in &mut line.words {
            word.x += ox;
            word.y += oy;
            word.cx += ox;
            word.cy += oy;
        }
    }
}

/// Search for text on screen and return matches with click coordinates.
pub fn find_text(screenshot: &Screenshot, query: &str) -> Result<Vec<OcrMatch>, GuiError> {
    let result = read_screen(screenshot)?;
    let query_lower = query.to_lowercase();

    let mut matches = Vec::new();

    for line in &result.lines {
        for word in &line.words {
            if word.text.to_lowercase().contains(&query_lower) {
                matches.push(OcrMatch {
                    text: word.text.clone(),
                    x: word.x,
                    y: word.y,
                    width: word.width,
                    height: word.height,
                    cx: word.cx,
                    cy: word.cy,
                    line_text: line.text.clone(),
                });
            }
        }

        if query.contains(' ') && line.text.to_lowercase().contains(&query_lower) {
            if let Some(start_idx) = line.text.to_lowercase().find(&query_lower) {
                let char_width = if !line.text.is_empty() {
                    line.width as f32 / line.text.len() as f32
                } else {
                    10.0
                };
                let match_x = line.x + (start_idx as f32 * char_width) as i32;
                let match_width = (query.len() as f32 * char_width) as i32;

                matches.push(OcrMatch {
                    text: query.to_string(),
                    x: match_x,
                    y: line.y,
                    width: match_width,
                    height: line.height,
                    cx: match_x + match_width / 2,
                    cy: line.y + line.height / 2,
                    line_text: line.text.clone(),
                });
            }
        }
    }

    Ok(matches)
}

/// Compress a screenshot to JPEG for efficient transport.
/// Returns base64-encoded JPEG data.
pub fn compress_screenshot(
    screenshot: &Screenshot,
    quality: u8,
    scale: f32,
) -> Result<(String, u32, u32), GuiError> {
    let img = image::load_from_memory(&screenshot.data)
        .map_err(|e| GuiError::OcrError(format!("decode screenshot: {e}")))?;

    let (new_w, new_h) = if (scale - 1.0).abs() > 0.01 {
        let w = (screenshot.width as f32 * scale) as u32;
        let h = (screenshot.height as f32 * scale) as u32;
        (w, h)
    } else {
        (screenshot.width, screenshot.height)
    };

    let resized = if new_w != screenshot.width || new_h != screenshot.height {
        image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Triangle)
    } else {
        img.to_rgba8()
    };

    let rgb_img: image::RgbImage = image::DynamicImage::ImageRgba8(resized).into_rgb8();

    let mut jpeg_buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_buf, quality);
    image::ImageEncoder::write_image(
        encoder,
        rgb_img.as_raw(),
        new_w,
        new_h,
        image::ExtendedColorType::Rgb8,
    )
    .map_err(|e| GuiError::OcrError(format!("JPEG encode: {e}")))?;

    let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &jpeg_buf);

    Ok((base64_data, new_w, new_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One line at (10,20) 100x12 holding a word centred at (35,26), in the
    /// coordinate space a 420x340 crop would produce.
    fn crop_local_result() -> OcrResult {
        OcrResult {
            text: "hello world".to_string(),
            lines: vec![OcrLine {
                text: "hello world".to_string(),
                x: 10,
                y: 20,
                width: 100,
                height: 12,
                words: vec![OcrWord {
                    text: "hello".to_string(),
                    x: 20,
                    y: 20,
                    width: 30,
                    height: 12,
                    cx: 35,
                    cy: 26,
                }],
            }],
            screen_width: 420,
            screen_height: 340,
        }
    }

    #[test]
    fn region_origin_is_added_to_every_coordinate() {
        let mut result = crop_local_result();
        translate_to_screen(&mut result, (300, 400));

        let line = &result.lines[0];
        assert_eq!((line.x, line.y), (310, 420));

        let word = &line.words[0];
        assert_eq!((word.x, word.y), (320, 420));
        assert_eq!((word.cx, word.cy), (335, 426));

        // The click target must be inside the region it was read from, which is
        // the invariant that broke when crop-local coordinates leaked out.
        assert!(word.cx >= 300 && word.cy >= 400);
    }

    #[test]
    fn translation_moves_centres_and_boxes_consistently() {
        let mut result = crop_local_result();
        translate_to_screen(&mut result, (300, 400));

        let word = &result.lines[0].words[0];
        // cx/cy stay the centre of the moved box, not an independent offset.
        assert_eq!(word.cx, word.x + word.width / 2);
        assert_eq!(word.cy, word.y + word.height / 2);
    }

    #[test]
    fn extents_are_not_shifted() {
        let mut result = crop_local_result();
        translate_to_screen(&mut result, (300, 400));

        let line = &result.lines[0];
        assert_eq!((line.width, line.height), (100, 12));
        let word = &line.words[0];
        assert_eq!((word.width, word.height), (30, 12));
    }

    #[test]
    fn zero_origin_leaves_full_screen_reads_untouched() {
        let mut result = crop_local_result();
        let before = result.clone();
        translate_to_screen(&mut result, (0, 0));
        assert_eq!(result.lines[0].words[0].cx, before.lines[0].words[0].cx);
        assert_eq!(result.lines[0].x, before.lines[0].x);
    }
}
