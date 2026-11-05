//! On-device OCR engine using `ocrs` (pure Rust).
//!
//! Adapted from rdp-mcp — reads local screen captures instead of RDP framebuffer.
//! Returns structured text + bounding-box coordinates (~2 KB) instead of multi-MB images.

use std::path::PathBuf;
use std::sync::OnceLock;

use async_trait::async_trait;
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
    /// Center x — use this directly for `mouse_click`.
    pub cx: i32,
    /// Center y — use this directly for `mouse_click`.
    pub cy: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrMatch {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Center x — use this for `mouse_click`.
    pub cx: i32,
    /// Center y — use this for `mouse_click`.
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
    let img = image::load_from_memory(&screenshot.data)
        .map_err(|e| GuiError::OcrError(format!("decode screenshot for OCR: {e}")))?
        .into_rgb8();
    ocr_rgb(&img)
}

/// The inference half of [`read_screen`], on already-decoded pixels.
fn ocr_rgb(img: &image::RgbImage) -> Result<OcrResult, GuiError> {
    let engine = get_engine()?;

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
        screen_width: img.width(),
        screen_height: img.height(),
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
    Ok(match_lines(&result, query))
}

/// Match a query against an OCR result that has already been read.
///
/// Split out of [`find_text`] so a caller that has the text in hand — the
/// native-scale sweep below, or a test — can apply exactly the same matching
/// rules instead of a second copy that might drift.
pub fn match_lines(result: &OcrResult, query: &str) -> Vec<OcrMatch> {
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

    matches
}

// ─── Small-text search strategy ────────────────────────────────────────────

/// The text-detection model's fixed working size: **600 wide × 800 tall**, read
/// from the shipped `text-detection.onnx` input tensor (`[1, 1, 800, 600]`).
///
/// `ocrs` gives every image to that one tensor. An image *smaller* than it is
/// padded with black, so its glyphs keep their pixel size; an image *larger* is
/// resized to it, and the resize ignores the aspect ratio — a 1920-wide screen is
/// squeezed to 0.31x horizontally while being shrunk only 0.74x vertically.
/// That asymmetry is issue #34: the same pixels are readable at 11px in a
/// window and invisible at 3px after a whole-screen squeeze.
///
/// Measured on a real 1920x1080 desktop capture, over 12 text lines that a tight
/// crop reads correctly: a single whole-screen pass reads **5**, a 1200-wide
/// window reads **7**, and windows no larger than this working size read
/// **12/12**. Run through [`OcrCapability::read_screen_thorough`] on the same
/// capture, the sweep returned **179** lines where one whole-screen pass returned
/// **71**.
pub const DETECT_WORKING_SIZE: (u32, u32) = (600, 800);

/// Overlap between adjacent search windows.
///
/// Not cosmetic: a line that straddles a window boundary is read as a fragment or
/// lost entirely. Counting distinct line texts across the sweep of the same
/// capture, the 600x800 grid with zero overlap returned 108 lines and this grid
/// returned 132.
pub const SEARCH_WINDOW_OVERLAP: u32 = 96;

/// One window of a screen that native-scale OCR is run over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchWindow {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Start positions along one axis, so a `len`-pixel span is covered by
/// `window`-wide windows that overlap by `SEARCH_WINDOW_OVERLAP`.
///
/// The last position is pulled back to `len - window` rather than left to run
/// off the edge: a trailing 60px sliver would cost a whole inference pass to
/// read a strip no line is fully inside.
fn axis_positions(len: u32, window: u32) -> Vec<u32> {
    if len <= window {
        return vec![0];
    }
    let step = window.saturating_sub(SEARCH_WINDOW_OVERLAP).max(1);
    let last = len - window;
    let mut out: Vec<u32> = Vec::new();
    let mut start = 0u32;
    loop {
        // Pulling the start back to `last` can land it on a position already
        // covered; that means the frame ends here, so the sweep stops rather
        // than paying for a duplicate pass.
        start = start.min(last);
        if out.last().copied() == Some(start) {
            return out;
        }
        out.push(start);
        if start + window >= len {
            return out;
        }
        start += step;
    }
}

/// The windows a native-scale read of a `width`x`height` image needs.
///
/// A frame that already fits [`DETECT_WORKING_SIZE`] is returned as one window:
/// it is padded rather than rescaled, so its text is already at native scale and
/// splitting it would only add cost. That is why `screen_read` with a small
/// `region` is the precise lookup in issue #34, and why a region read costs no
/// more under the thorough strategy.
pub fn search_windows(width: u32, height: u32) -> Vec<SearchWindow> {
    let (win_w, win_h) = DETECT_WORKING_SIZE;
    let xs = axis_positions(width, win_w.min(width));
    let ys = axis_positions(height, win_h.min(height));
    let mut out = Vec::with_capacity(xs.len() * ys.len());
    for &y in &ys {
        for &x in &xs {
            out.push(SearchWindow {
                x,
                y,
                width: win_w.min(width - x),
                height: win_h.min(height - y),
            });
        }
    }
    out
}

/// Fold the per-window reads into one result in screen coordinates.
///
/// Coordinates come back window-local, so each line is shifted by its window's
/// origin. A line the overlap made two windows read is kept once: the duplicate
/// test is the same text *and* a centre within the overlap distance, which is
/// what distinguishes a repeated line from the same words appearing twice on
/// screen (a column of "1"s in a ledger is the second case, and dropping it
/// would invent absence in the merge that #34 is about removing).
pub fn merge_window_results(parts: &[(SearchWindow, OcrResult)], frame: (u32, u32)) -> OcrResult {
    let mut kept: Vec<OcrLine> = Vec::new();

    for (win, result) in parts {
        for line in &result.lines {
            let mut line = line.clone();
            line.x += win.x as i32;
            line.y += win.y as i32;
            for word in &mut line.words {
                word.x += win.x as i32;
                word.y += win.y as i32;
                word.cx += win.x as i32;
                word.cy += win.y as i32;
            }

            let cx = line.x + line.width / 2;
            let cy = line.y + line.height / 2;
            let duplicate = kept.iter().any(|k| {
                k.text == line.text
                    && (k.x + k.width / 2 - cx).abs() <= SEARCH_WINDOW_OVERLAP as i32
                    && (k.y + k.height / 2 - cy).abs() <= SEARCH_WINDOW_OVERLAP as i32
            });
            if !duplicate {
                kept.push(line);
            }
        }
    }

    kept.sort_by_key(|l| (l.y, l.x));

    let text = kept
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    OcrResult {
        text,
        lines: kept,
        screen_width: frame.0,
        screen_height: frame.1,
    }
}

/// Re-encode one window of a decoded frame as a screenshot, so the window goes
/// through the same [`OcrCapability::read_screen`] path a caller would use.
fn window_shot(img: &image::RgbImage, win: &SearchWindow) -> Result<Screenshot, GuiError> {
    let crop = image::imageops::crop_imm(img, win.x, win.y, win.width, win.height).to_image();
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    image::DynamicImage::ImageRgb8(crop)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| GuiError::OcrError(format!("encode OCR window: {e}")))?;
    Ok(Screenshot {
        data: buf,
        width: win.width,
        height: win.height,
    })
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

// ═══════════════════════════════════════════════════════════════════════════
// OCR capability seam — the surface the screen_read/find_text handlers call
// ═══════════════════════════════════════════════════════════════════════════

/// The OCR surface the `screen_read` and `find_text` handlers need, as a
/// capability object rather than a free call into [`read_screen`].
///
/// Those handlers used to run [`read_screen`]/[`find_text`] directly on a
/// `spawn_blocking`, so a `StubBackend` test that exercised them still pulled the
/// ONNX engine online: the first call downloads `text-detection.onnx` and
/// `text-recognition.onnx` off the network and loads them into memory
/// (`docs/ISSUES.md` #47). Making it a trait lets the test backend hand back a
/// fixed [`OcrResult`] with no model, no download and no inference.
///
/// The screenshot is captured through the backend before it reaches here, so the
/// bytes are already hermetic in tests — only the inference itself needed the
/// seam. Both methods take an owned [`Screenshot`] because `spawn_blocking`
/// requires `'static` input.
///
/// [`read_screen_thorough`] and [`search_text`] are **provided** methods: the
/// strategy (issue #34) is geometry and merging, so it belongs to the seam, not
/// to each backend. A backend only supplies the single-image primitives, and the
/// test stub therefore exercises the same sweep the desktop does.
#[async_trait]
pub trait OcrCapability: Send + Sync {
    async fn read_screen(&self, screenshot: Screenshot) -> Result<OcrResult, GuiError>;
    async fn find_text(
        &self,
        screenshot: Screenshot,
        query: String,
    ) -> Result<Vec<OcrMatch>, GuiError>;

    /// Read the frame at the detector's native scale, however many passes that
    /// takes. See [`search_windows`] for when one pass is already native scale.
    async fn read_screen_thorough(&self, screenshot: Screenshot) -> Result<OcrResult, GuiError> {
        let windows = search_windows(screenshot.width, screenshot.height);
        if windows.len() <= 1 {
            return self.read_screen(screenshot).await;
        }

        // Cropping needs pixels. A backend whose screenshot carries none — the
        // test stub's `data: vec![]` — cannot be cropped, and the sweep runs the
        // same windows with the same geometry over the uncropped bytes. A real
        // capture is always PNG, and an undecodable one still fails inside
        // [`OcrCapability::read_screen`] with the same message it gives today.
        let decoded = image::load_from_memory(&screenshot.data)
            .ok()
            .map(|img| img.into_rgb8());

        let mut parts = Vec::with_capacity(windows.len());
        for win in &windows {
            let shot = match &decoded {
                Some(img) => window_shot(img, win)?,
                None => screenshot.clone(),
            };
            parts.push((*win, self.read_screen(shot).await?));
        }
        Ok(merge_window_results(
            &parts,
            (screenshot.width, screenshot.height),
        ))
    }

    /// Look for `query`, and report how far the search reached.
    ///
    /// The coverage figure is part of the answer, not decoration: "no text
    /// matching X" from one squashed whole-screen pass and "no text matching X"
    /// from a sweep of nine native-scale windows are different claims, and the
    /// caller cannot tell them apart from the empty list alone.
    async fn search_text(
        &self,
        screenshot: Screenshot,
        query: String,
        strategy: FindStrategy,
    ) -> Result<TextSearch, GuiError> {
        let matches = self.find_text(screenshot.clone(), query.clone()).await?;
        if !matches.is_empty() || strategy == FindStrategy::SinglePass {
            return Ok(TextSearch {
                matches,
                scope: SearchScope::SinglePass,
            });
        }

        let window_count = search_windows(screenshot.width, screenshot.height).len();
        if window_count <= 1 {
            // Already native scale: a second pass over the same pixels could not
            // see anything the first one missed.
            return Ok(TextSearch {
                matches,
                scope: SearchScope::Windows {
                    count: window_count.max(1),
                },
            });
        }

        let read = self.read_screen_thorough(screenshot).await?;
        Ok(TextSearch {
            matches: match_lines(&read, &query),
            scope: SearchScope::Windows {
                count: window_count,
            },
        })
    }
}

/// How hard a text lookup looks before it reports the text absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindStrategy {
    /// One pass over the whole image. Fast, and measurably blind to small text
    /// (`docs/ISSUES.md` #34): 5 of 12 lines that a tight crop reads.
    SinglePass,
    /// One pass, then a native-scale sweep of [`search_windows`] if it found
    /// nothing. The extra passes are paid only on a miss — which is exactly the
    /// answer that must not be a lie.
    EscalateOnMiss,
}

/// What a text search actually covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    SinglePass,
    Windows { count: usize },
}

impl std::fmt::Display for SearchScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchScope::SinglePass => write!(f, "one whole-image pass"),
            SearchScope::Windows { count } => {
                write!(
                    f,
                    "{count} native-scale window{}",
                    if *count == 1 { "" } else { "s" }
                )
            }
        }
    }
}

/// A text lookup and the coverage that produced it.
#[derive(Debug, Clone)]
pub struct TextSearch {
    pub matches: Vec<OcrMatch>,
    pub scope: SearchScope,
}

/// The production [`OcrCapability`]: runs the real `ocrs` engine off the async
/// runtime, exactly as the handlers did before the seam existed.
pub struct LocalOcr;

static LOCAL_OCR: LocalOcr = LocalOcr;

impl LocalOcr {
    /// Shared instance handed out by reference; the engine is a process-wide
    /// `OnceLock` singleton, so there is nothing to own per client.
    pub fn shared() -> &'static LocalOcr {
        &LOCAL_OCR
    }
}

#[async_trait]
impl OcrCapability for LocalOcr {
    async fn read_screen(&self, screenshot: Screenshot) -> Result<OcrResult, GuiError> {
        tokio::task::spawn_blocking(move || read_screen(&screenshot))
            .await
            .map_err(|e| GuiError::OcrError(format!("OCR task failed: {e}")))?
    }

    async fn find_text(
        &self,
        screenshot: Screenshot,
        query: String,
    ) -> Result<Vec<OcrMatch>, GuiError> {
        tokio::task::spawn_blocking(move || find_text(&screenshot, &query))
            .await
            .map_err(|e| GuiError::OcrError(format!("OCR task failed: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    // ─── Small-text search strategy (issue #34) ───────────────────────────
    //
    // Geometry, merging and matching only — no engine, no model, no screen
    // capture, per the hermetic rule in `docs/ISSUES.md` #47. The recall figures
    // these strategies buy (5/12 → 12/12) can only be measured against real
    // pixels, and were, in a throwaway probe against a live 1920x1080 capture.

    fn word(text: &str, x: i32, y: i32, w: i32, h: i32) -> OcrWord {
        OcrWord {
            text: text.to_string(),
            x,
            y,
            width: w,
            height: h,
            cx: x + w / 2,
            cy: y + h / 2,
        }
    }

    fn line(text: &str, x: i32, y: i32, w: i32, h: i32) -> OcrLine {
        OcrLine {
            text: text.to_string(),
            x,
            y,
            width: w,
            height: h,
            words: vec![word(text, x, y, w, h)],
        }
    }

    fn read_of(width: u32, height: u32, lines: Vec<OcrLine>) -> OcrResult {
        let text = lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        OcrResult {
            text,
            lines,
            screen_width: width,
            screen_height: height,
        }
    }

    fn shot(width: u32, height: u32) -> Screenshot {
        Screenshot {
            data: Vec::new(),
            width,
            height,
        }
    }

    /// Start positions along one axis, read back out of a window grid.
    fn distinct_starts(windows: &[SearchWindow], by_x: bool) -> Vec<u32> {
        let mut v: Vec<u32> = windows
            .iter()
            .map(|w| if by_x { w.x } else { w.y })
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    #[test]
    fn a_full_hd_frame_sweeps_the_grid_the_measurement_found_readable() {
        let windows = search_windows(1920, 1080);
        assert_eq!(
            windows.len(),
            8,
            "4 columns x 2 rows at 600x800/96px overlap"
        );
        assert_eq!(distinct_starts(&windows, true), vec![0, 504, 1008, 1320]);
        assert_eq!(distinct_starts(&windows, false), vec![0, 280]);

        for win in &windows {
            // Every window has to fit the detector's working size, or it gets
            // resized and the sweep buys nothing.
            assert!(win.width <= DETECT_WORKING_SIZE.0, "{win:?}");
            assert!(win.height <= DETECT_WORKING_SIZE.1, "{win:?}");
            // And stay inside the frame: a crop past the edge is clipped by
            // `crop_imm`, which would silently leave a band unread.
            assert!(win.x + win.width <= 1920, "{win:?} runs off the right edge");
            assert!(
                win.y + win.height <= 1080,
                "{win:?} runs off the bottom edge"
            );
        }
        assert!(windows.iter().any(|w| w.x + w.width == 1920));
        assert!(windows.iter().any(|w| w.y + w.height == 1080));
    }

    #[test]
    fn the_grid_covers_every_pixel_on_screens_of_any_size() {
        let covers = |len: u32, spans: Vec<(u32, u32)>| {
            let mut spans = spans;
            spans.sort_unstable();
            assert_eq!(spans[0].0, 0, "the grid must start at the edge, not inset");
            let mut reach = 0u32;
            for (start, end) in spans {
                assert!(start <= reach, "uncovered band at {start}");
                reach = reach.max(end);
            }
            assert_eq!(reach, len, "the grid stops short of the far edge");
        };

        for (w, h) in [
            (1920u32, 1080u32),
            (2560, 1440),
            (1366, 768),
            (800, 600),
            (601, 600),
            (640, 480),
        ] {
            let windows = search_windows(w, h);
            assert!(!windows.is_empty(), "{w}x{h} has no windows at all");
            covers(
                w,
                windows
                    .iter()
                    .map(|win| (win.x, win.x + win.width))
                    .collect(),
            );
            covers(
                h,
                windows
                    .iter()
                    .map(|win| (win.y, win.y + win.height))
                    .collect(),
            );
        }
    }

    #[test]
    fn a_frame_that_already_fits_the_working_size_is_one_window() {
        // Padding rather than rescaling, so a tight region is already at native
        // scale: this is why the thorough strategy costs a region read nothing.
        for (w, h) in [(420u32, 140u32), (600, 800), (599, 799), (100, 100)] {
            let windows = search_windows(w, h);
            assert_eq!(windows.len(), 1, "{w}x{h} should not be split");
            assert_eq!(
                (
                    windows[0].x,
                    windows[0].y,
                    windows[0].width,
                    windows[0].height
                ),
                (0, 0, w, h),
                "{w}x{h} must cover the frame exactly"
            );
        }
    }

    #[test]
    fn the_last_window_pulls_back_to_the_edge_instead_of_leaving_a_sliver() {
        // 1000px with a 600px window: stepping 504 would put the second window
        // past the end, and clipping it to 40px wide would leave a 40px band of
        // the screen unsearched while still costing a full inference pass.
        assert_eq!(axis_positions(1000, 600), vec![0, 400]);
        assert_eq!(axis_positions(600, 600), vec![0]);
        assert_eq!(axis_positions(601, 600), vec![0, 1]);
        assert_eq!(axis_positions(0, 600), vec![0]);
        assert_eq!(axis_positions(1080, 800), vec![0, 280]);
    }

    #[test]
    fn window_reads_come_back_in_screen_coordinates() {
        let win = SearchWindow {
            x: 504,
            y: 280,
            width: 600,
            height: 800,
        };
        let merged = merge_window_results(
            &[(win, read_of(600, 800, vec![line("Submit", 10, 20, 60, 12)]))],
            (1920, 1080),
        );

        assert_eq!((merged.screen_width, merged.screen_height), (1920, 1080));
        let l = &merged.lines[0];
        assert_eq!((l.x, l.y), (514, 300));
        assert_eq!((l.width, l.height), (60, 12), "extents are not shifted");
        let word = &l.words[0];
        assert_eq!(
            (word.cx, word.cy),
            (544, 306),
            "the click target moves with its box"
        );
        assert_eq!(word.cx, word.x + word.width / 2);
        assert_eq!(word.cy, word.y + word.height / 2);
    }

    #[test]
    fn a_line_the_overlap_read_twice_is_kept_once() {
        let left = SearchWindow {
            x: 0,
            y: 0,
            width: 600,
            height: 800,
        };
        let right = SearchWindow {
            x: 504,
            y: 0,
            width: 600,
            height: 800,
        };
        // The same screen line as each window sees it: 520px in both spaces.
        let parts = vec![
            (
                left,
                read_of(600, 800, vec![line("Monthly Register", 520, 40, 60, 12)]),
            ),
            (
                right,
                read_of(600, 800, vec![line("Monthly Register", 16, 40, 60, 12)]),
            ),
        ];

        let merged = merge_window_results(&parts, (1920, 1080));
        assert_eq!(merged.lines.len(), 1, "text: {}", merged.text);
        assert_eq!(merged.lines[0].x, 520);
    }

    #[test]
    fn the_same_words_apart_are_not_deduplicated() {
        // Collapsing on text alone would delete a ledger column of "1"s — the
        // merge inventing the very absence #34 is about removing.
        let win = SearchWindow {
            x: 0,
            y: 0,
            width: 600,
            height: 800,
        };
        let lines = vec![
            line("1", 40, 100, 12, 12),
            line("1", 40, 300, 12, 12),
            line("1", 40, 500, 12, 12),
        ];

        let merged = merge_window_results(&[(win, read_of(600, 800, lines))], (600, 800));
        assert_eq!(merged.lines.len(), 3);
        assert_eq!(merged.text, "1\n1\n1");
    }

    #[test]
    fn merged_lines_come_back_in_reading_order() {
        let right = SearchWindow {
            x: 504,
            y: 0,
            width: 600,
            height: 800,
        };
        let left = SearchWindow {
            x: 0,
            y: 0,
            width: 600,
            height: 800,
        };
        let lower = SearchWindow {
            x: 0,
            y: 280,
            width: 600,
            height: 800,
        };
        let parts = vec![
            (
                right,
                read_of(600, 800, vec![line("top right", 10, 10, 60, 12)]),
            ),
            (
                lower,
                read_of(600, 800, vec![line("bottom", 10, 10, 60, 12)]),
            ),
            (
                left,
                read_of(600, 800, vec![line("top left", 10, 10, 60, 12)]),
            ),
        ];

        let merged = merge_window_results(&parts, (1920, 1080));
        let texts: Vec<&str> = merged.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["top left", "top right", "bottom"]);
        assert_eq!(
            merged.text, "top left\ntop right\nbottom",
            "the text block follows the line order"
        );
    }

    #[test]
    fn match_lines_reports_word_positions_for_a_single_word_query() {
        let mut l = line("Voucher No", 100, 40, 100, 12);
        l.words = vec![
            word("Voucher", 100, 40, 60, 12),
            word("No", 170, 40, 30, 12),
        ];
        let result = read_of(1920, 1080, vec![l]);

        // One hit, not two: the line also contains "no", but the line-level
        // branch is gated on the query holding a space.
        let hits = match_lines(&result, "no");
        assert_eq!(hits.len(), 1);
        assert_eq!(
            (hits[0].text.as_str(), hits[0].cx, hits[0].cy),
            ("No", 185, 46)
        );
        assert_eq!(hits[0].line_text, "Voucher No");
    }

    #[test]
    fn match_lines_interpolates_a_phrase_across_the_line() {
        let mut l = line("Voucher No", 100, 40, 100, 12);
        l.words = vec![
            word("Voucher", 100, 40, 60, 12),
            word("No", 170, 40, 30, 12),
        ];
        let result = read_of(1920, 1080, vec![l]);

        // No single word holds "her No", so the position comes from the offset
        // within the line scaled by its average char width (100px / 10 chars).
        let hits = match_lines(&result, "her No");
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].x, hits[0].width), (140, 60));
        assert_eq!(hits[0].cx, 170);
        assert_eq!(
            hits[0].text, "her No",
            "the match carries the query, not the whole line"
        );
    }

    /// A capability that answers from fixed geometry and counts the passes it is
    /// asked for, so the sweep and the escalation are testable without an engine
    /// (`docs/ISSUES.md` #47).
    struct ScriptedOcr {
        passes: AtomicUsize,
        hit: bool,
    }

    #[async_trait]
    impl OcrCapability for ScriptedOcr {
        async fn read_screen(&self, screenshot: Screenshot) -> Result<OcrResult, GuiError> {
            self.passes.fetch_add(1, Ordering::SeqCst);
            // One line per pass, named by the window that produced it, so a
            // merged result shows which geometry ran instead of just how many.
            let text = format!("{}x{}", screenshot.width, screenshot.height);
            Ok(read_of(
                screenshot.width,
                screenshot.height,
                vec![line(&text, 10, 20, 40, 12)],
            ))
        }

        async fn find_text(
            &self,
            _screenshot: Screenshot,
            query: String,
        ) -> Result<Vec<OcrMatch>, GuiError> {
            self.passes.fetch_add(1, Ordering::SeqCst);
            if !self.hit {
                return Ok(Vec::new());
            }
            Ok(vec![OcrMatch {
                cx: 50,
                cy: 46,
                height: 12,
                line_text: query.clone(),
                text: query,
                width: 40,
                x: 30,
                y: 40,
            }])
        }
    }

    impl ScriptedOcr {
        fn new(hit: bool) -> Self {
            Self {
                passes: AtomicUsize::new(0),
                hit,
            }
        }

        fn passes(&self) -> usize {
            self.passes.load(Ordering::SeqCst)
        }
    }

    #[tokio::test]
    async fn a_thorough_read_costs_one_pass_per_window_and_keeps_them_all() {
        let ocr = ScriptedOcr::new(true);
        let result = ocr.read_screen_thorough(shot(1920, 1080)).await.unwrap();

        assert_eq!(
            ocr.passes(),
            8,
            "the sweep runs the grid, not a fixed tile count"
        );
        assert_eq!(
            result.lines.len(),
            8,
            "a line per window survived the merge"
        );
        assert_eq!((result.screen_width, result.screen_height), (1920, 1080));
        let xs: Vec<i32> = result.lines.iter().map(|l| l.x).collect();
        assert!(xs.contains(&1330), "last column is 1320 + 10: {xs:?}");
    }

    #[tokio::test]
    async fn a_frame_that_fits_the_working_size_is_never_split() {
        let ocr = ScriptedOcr::new(true);
        let full = ocr.read_screen_thorough(shot(600, 800)).await.unwrap();
        assert_eq!(ocr.passes(), 1, "already native scale");
        assert_eq!(full.lines.len(), 1);

        let region = ocr.read_screen_thorough(shot(420, 140)).await.unwrap();
        assert_eq!(
            ocr.passes(),
            2,
            "a region read costs one pass under either strategy"
        );
        assert_eq!(region.lines.len(), 1);
    }

    #[tokio::test]
    async fn a_hit_on_the_first_pass_never_pays_for_the_sweep() {
        let ocr = ScriptedOcr::new(true);
        let found = ocr
            .search_text(
                shot(1920, 1080),
                "File".into(),
                FindStrategy::EscalateOnMiss,
            )
            .await
            .unwrap();

        assert_eq!(found.matches.len(), 1);
        assert_eq!(found.scope, SearchScope::SinglePass);
        assert_eq!(found.scope.to_string(), "one whole-image pass");
        assert_eq!(ocr.passes(), 1, "the extra passes are only for a miss");
    }

    #[tokio::test]
    async fn a_miss_escalates_to_the_native_scale_sweep_and_says_so() {
        let ocr = ScriptedOcr::new(false);
        let missed = ocr
            .search_text(
                shot(1920, 1080),
                "Invoice".into(),
                FindStrategy::EscalateOnMiss,
            )
            .await
            .unwrap();

        assert!(missed.matches.is_empty());
        assert_eq!(missed.scope, SearchScope::Windows { count: 8 });
        assert_eq!(missed.scope.to_string(), "8 native-scale windows");
        assert_eq!(
            ocr.passes(),
            9,
            "one pass for the lookup, eight for the sweep"
        );
    }

    #[tokio::test]
    async fn the_single_strategy_reports_the_shallow_search_it_ran() {
        let ocr = ScriptedOcr::new(false);
        let missed = ocr
            .search_text(shot(1920, 1080), "Invoice".into(), FindStrategy::SinglePass)
            .await
            .unwrap();

        assert_eq!(missed.scope, SearchScope::SinglePass);
        assert_eq!(ocr.passes(), 1, "the opt-out must not sweep");
    }

    #[tokio::test]
    async fn a_miss_on_an_already_native_frame_does_not_re_read_it() {
        let ocr = ScriptedOcr::new(false);
        let missed = ocr
            .search_text(
                shot(420, 140),
                "Invoice".into(),
                FindStrategy::EscalateOnMiss,
            )
            .await
            .unwrap();

        // A second pass over the same pixels at the same scale cannot see more,
        // so escalating here would only cost time.
        assert_eq!(missed.scope, SearchScope::Windows { count: 1 });
        assert_eq!(missed.scope.to_string(), "1 native-scale window");
        assert_eq!(ocr.passes(), 1);
    }
}
