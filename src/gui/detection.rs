//! On-device object detection using YOLOv8 (stub with mock data for testing)

use std::path::PathBuf;
use std::sync::OnceLock;

use image::GenericImageView;
use serde::{Deserialize, Serialize};

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

const COCO_LABELS: &[&str] = &[
    "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck", "boat",
    "traffic light", "fire hydrant", "stop sign", "parking meter", "bench", "bird", "cat",
    "dog", "horse", "sheep", "cow", "elephant", "bear", "zebra", "giraffe", "backpack",
    "umbrella", "handbag", "tie", "suitcase", "frisbee", "skis", "snowboard", "sports ball",
    "kite", "baseball bat", "baseball glove", "skateboard", "surfboard", "tennis racket",
    "bottle", "wine glass", "cup", "fork", "knife", "spoon", "bowl", "banana", "apple",
    "sandwich", "orange", "broccoli", "carrot", "hot dog", "pizza", "donut", "cake", "chair",
    "couch", "potted plant", "bed", "dining table", "toilet", "tv", "laptop", "mouse",
    "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink", "refrigerator",
    "book", "clock", "vase", "scissors", "teddy bear", "hair drier", "toothbrush",
];

fn models_dir() -> PathBuf {
    let mut dir = dirs_next::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("gui-mcp");
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
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create model dir: {}", e))?;

    let url = "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx";
    log::info!("downloading YOLOv8n model: {}", url);

    // Note: Model download not implemented in stub
    // For now, just use mock detections
    log::info!("YOLOv8n model init (using stub data)");
    Ok(path)
}

static MODEL_INIT: OnceLock<()> = OnceLock::new();

fn init_model() {
    let _ = MODEL_INIT.get_or_init(|| {
        let _ = ensure_model();
    });
}

/// Detect objects in screenshot using YOLOv8
/// 
/// Note: This is a stub implementation that returns mock detections.
/// Real ONNX inference with tract-onnx requires additional setup.
pub fn detect_objects(screenshot: &super::types::Screenshot) -> Result<DetectionResult, String> {
    // Initialize model (just logs, doesn't actually load for stub)
    init_model();

    // Get image dimensions
    let img = image::load_from_memory(&screenshot.data)
        .map_err(|e| format!("decode image: {}", e))?;
    let (width, height) = img.dimensions();

    // Return mock detections for testing
    // In a full implementation, this would run actual YOLOv8 inference
    let detections = vec![
        Detection {
            label: "laptop".to_string(),
            confidence: 0.85,
            x: 100,
            y: 50,
            width: 400,
            height: 250,
            cx: 300,
            cy: 175,
        },
        Detection {
            label: "mouse".to_string(),
            confidence: 0.72,
            x: 520,
            y: 280,
            width: 40,
            height: 50,
            cx: 540,
            cy: 305,
        },
        Detection {
            label: "keyboard".to_string(),
            confidence: 0.68,
            x: 200,
            y: 320,
            width: 300,
            height: 80,
            cx: 350,
            cy: 360,
        },
        Detection {
            label: "cup".to_string(),
            confidence: 0.55,
            x: 600,
            y: 100,
            width: 30,
            height: 40,
            cx: 615,
            cy: 120,
        },
    ];

    Ok(DetectionResult {
        detections,
        image_width: width,
        image_height: height,
    })
}