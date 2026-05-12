use std::path::PathBuf;

use image::GenericImageView;
use serde::{Deserialize, Serialize};
use tract_onnx::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub label: String,
    pub confidence: f32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub cx: i32,
    pub cy: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub detections: Vec<Detection>,
    pub image_width: u32,
    pub image_height: u32,
}

#[allow(dead_code)]
const COCO_LABELS: &[&str] = &[
    "person",
    "bicycle",
    "car",
    "motorcycle",
    "airplane",
    "bus",
    "train",
    "truck",
    "boat",
    "traffic light",
    "fire hydrant",
    "stop sign",
    "parking meter",
    "bench",
    "bird",
    "cat",
    "dog",
    "horse",
    "sheep",
    "cow",
    "elephant",
    "bear",
    "zebra",
    "giraffe",
    "backpack",
    "umbrella",
    "handbag",
    "tie",
    "suitcase",
    "frisbee",
    "skis",
    "snowboard",
    "sports ball",
    "kite",
    "baseball bat",
    "baseball glove",
    "skateboard",
    "surfboard",
    "tennis racket",
    "bottle",
    "wine glass",
    "cup",
    "fork",
    "knife",
    "spoon",
    "bowl",
    "banana",
    "apple",
    "sandwich",
    "orange",
    "broccoli",
    "carrot",
    "hot dog",
    "pizza",
    "donut",
    "cake",
    "chair",
    "couch",
    "potted plant",
    "bed",
    "dining table",
    "toilet",
    "tv",
    "laptop",
    "mouse",
    "remote",
    "keyboard",
    "cell phone",
    "microwave",
    "oven",
    "toaster",
    "sink",
    "refrigerator",
    "book",
    "clock",
    "vase",
    "scissors",
    "teddy bear",
    "hair drier",
    "toothbrush",
];

const MODEL_URL: &str =
    "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx";

const MODEL_INPUT_SIZE: u32 = 640;
const CONFIDENCE_THRESHOLD: f32 = 0.25;
const NMS_THRESHOLD: f32 = 0.45;

fn models_dir() -> PathBuf {
    let mut dir = dirs_next::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("sys-mcp");
    dir.push("models");
    dir
}

fn model_path() -> PathBuf {
    models_dir().join("yolov8n.onnx")
}

fn ensure_model() -> Result<PathBuf, String> {
    let path = model_path();
    if path.exists() {
        return Ok(path);
    }

    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create model dir: {e}"))?;

    log::info!("downloading YOLOv8n model from {MODEL_URL}");
    let resp = ureq::get(MODEL_URL)
        .call()
        .map_err(|e| format!("failed to download YOLO model: {e}"))?;

    let mut body = resp.into_body();
    let mut file = std::fs::File::create(&path)
        .map_err(|e| format!("failed to create {}: {}", path.display(), e))?;
    std::io::copy(&mut body.as_reader(), &mut file)
        .map_err(|e| format!("failed to write model: {e}"))?;

    log::info!("YOLOv8n model downloaded: {}", path.display());
    Ok(path)
}

/// Preprocess image: letterbox to 640x640, normalize to [0,1], return CHW tensor.
fn preprocess_image(img: &image::DynamicImage) -> Result<(Tensor, f32, f32), String> {
    let (orig_w, orig_h) = (img.width() as f32, img.height() as f32);
    let scale = (MODEL_INPUT_SIZE as f32 / orig_w).min(MODEL_INPUT_SIZE as f32 / orig_h);
    let new_w = (orig_w * scale) as u32;
    let new_h = (orig_h * scale) as u32;

    let resized = img.resize_exact(
        new_w.max(1),
        new_h.max(1),
        image::imageops::FilterType::Triangle,
    );

    let mut data = vec![0.0f32; (MODEL_INPUT_SIZE * MODEL_INPUT_SIZE * 3) as usize];

    for y in 0..new_h.min(MODEL_INPUT_SIZE) {
        for x in 0..new_w.min(MODEL_INPUT_SIZE) {
            let pixel = resized.get_pixel(x, y);
            let idx = (y * MODEL_INPUT_SIZE + x) as usize;
            data[idx] = pixel[0] as f32 / 255.0;
            data[idx + (MODEL_INPUT_SIZE * MODEL_INPUT_SIZE) as usize] = pixel[1] as f32 / 255.0;
            data[idx + (2 * MODEL_INPUT_SIZE * MODEL_INPUT_SIZE) as usize] =
                pixel[2] as f32 / 255.0;
        }
    }

    let tensor = tensor1(&data)
        .into_shape(&[
            1usize,
            3,
            MODEL_INPUT_SIZE as usize,
            MODEL_INPUT_SIZE as usize,
        ])
        .map_err(|e| format!("reshape input: {e}"))?;

    Ok((tensor, scale, orig_w))
}

fn non_max_suppression(
    mut boxes: Vec<(f32, f32, f32, f32, f32, usize)>,
    iou_threshold: f32,
) -> Vec<(f32, f32, f32, f32, f32, usize)> {
    boxes.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));
    let mut keep = Vec::new();
    while !boxes.is_empty() {
        let best = boxes.remove(0);
        keep.push(best);
        boxes.retain(|b| iou(keep.last().unwrap(), b) <= iou_threshold);
    }
    keep
}

fn iou(a: &(f32, f32, f32, f32, f32, usize), b: &(f32, f32, f32, f32, f32, usize)) -> f32 {
    let inter_x1 = a.0.max(b.0);
    let inter_y1 = a.1.max(b.1);
    let inter_x2 = a.2.min(b.2);
    let inter_y2 = a.3.min(b.3);
    let inter = ((inter_x2 - inter_x1).max(0.0)) * ((inter_y2 - inter_y1).max(0.0));
    let area_a = (a.2 - a.0) * (a.3 - a.1);
    let area_b = (b.2 - b.0) * (b.3 - b.1);
    inter / (area_a + area_b - inter + 1e-6)
}

pub fn detect_objects(screenshot: &super::types::Screenshot) -> Result<DetectionResult, String> {
    let model_path = ensure_model()?;

    let model = tract_onnx::onnx()
        .model_for_path(model_path)
        .map_err(|e| format!("load model: {e}"))?
        .with_input_fact(
            0,
            InferenceFact::dt_shape(
                f32::datum_type(),
                tvec!(1i64, 3, MODEL_INPUT_SIZE as i64, MODEL_INPUT_SIZE as i64),
            ),
        )
        .map_err(|e| format!("set input shape: {e}"))?
        .into_optimized()
        .map_err(|e| format!("optimize: {e}"))?
        .into_runnable()
        .map_err(|e| format!("make runnable: {e}"))?;

    let img =
        image::load_from_memory(&screenshot.data).map_err(|e| format!("decode image: {e}"))?;
    let (orig_w, orig_h) = img.dimensions();
    let (input_tensor, scale, _) = preprocess_image(&img)?;

    let result = model
        .run(tvec!(input_tensor.into()))
        .map_err(|e| format!("inference: {e}"))?;

    let output = result[0]
        .to_array_view::<f32>()
        .map_err(|e| format!("output view: {e}"))?;

    let num_detections = output.shape()[2] as usize;
    let num_classes = output.shape()[1] as usize - 4;

    let mut candidates: Vec<(f32, f32, f32, f32, f32, usize)> = Vec::new();

    for i in 0..num_detections {
        let cx = output[[0, 0, i]];
        let cy = output[[0, 1, i]];
        let w = output[[0, 2, i]];
        let h = output[[0, 3, i]];

        if w <= 0.0 || h <= 0.0 {
            continue;
        }

        let mut best_score = 0.0f32;
        let mut best_class = 0usize;
        for c in 0..num_classes {
            let score = output[[0, 4 + c, i]];
            if score > best_score {
                best_score = score;
                best_class = c;
            }
        }

        if best_score < CONFIDENCE_THRESHOLD {
            continue;
        }

        let x1 = cx - w / 2.0;
        let y1 = cy - h / 2.0;
        let x2 = cx + w / 2.0;
        let y2 = cy + h / 2.0;

        candidates.push((x1, y1, x2, y2, best_score, best_class));
    }

    let kept = non_max_suppression(candidates, NMS_THRESHOLD);

    let detections: Vec<Detection> = kept
        .into_iter()
        .map(|(x1, y1, x2, y2, score, class_id)| {
            let x = ((x1 / scale).max(0.0).min(orig_w as f32)) as i32;
            let y = ((y1 / scale).max(0.0).min(orig_h as f32)) as i32;
            let w = (((x2 - x1) / scale).max(1.0).min(orig_w as f32)) as i32;
            let h = (((y2 - y1) / scale).max(1.0).min(orig_h as f32)) as i32;
            Detection {
                label: COCO_LABELS.get(class_id).unwrap_or(&"unknown").to_string(),
                confidence: score,
                x,
                y,
                width: w,
                height: h,
                cx: x + w / 2,
                cy: y + h / 2,
            }
        })
        .collect();

    Ok(DetectionResult {
        detections,
        image_width: orig_w,
        image_height: orig_h,
    })
}
