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

    let resp = ureq::get(&url)
        .call()
        .map_err(|e| GuiError::OcrError(format!("failed to download OCR model {name}: {e}")))?;

    let mut body = resp.into_body();
    let mut file = std::fs::File::create(&path)
        .map_err(|e| GuiError::OcrError(format!("failed to create {}: {e}", path.display())))?;
    std::io::copy(&mut body.as_reader(), &mut file)
        .map_err(|e| GuiError::OcrError(format!("failed to write model {name}: {e}")))?;

    log::info!("OCR model downloaded: {}", path.display());
    Ok(path)
}

fn get_engine() -> Result<&'static OcrEngine, GuiError> {
    let result = OCR_ENGINE.get_or_init(|| {
        let det_path = ensure_model("text-detection.rten").map_err(|e| e.to_string())?;
        let rec_path = ensure_model("text-recognition.rten").map_err(|e| e.to_string())?;

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

    let mut full_text = String::new();
    let mut lines = Vec::new();

    for line_opt in &line_texts {
        let Some(line) = line_opt else { continue };
        let text = line.to_string();
        if text.trim().is_empty() || text.len() <= 1 {
            continue;
        }

        if !full_text.is_empty() {
            full_text.push('\n');
        }
        full_text.push_str(&text);

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

    Ok(OcrResult {
        text: full_text,
        lines,
        screen_width: screenshot.width,
        screen_height: screenshot.height,
    })
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
