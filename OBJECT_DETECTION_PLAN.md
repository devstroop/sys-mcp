# UI Element Detection Plan

## Goal
Add on-device object detection using YOLO to detect common UI elements (buttons, icons, menus, etc.)

## Architecture
```
Screen → YOLO Model → Detections → MCP Tools
                                → Web Preview overlay
```

## New MCP Tools
- `gui_detect_objects` — detect UI elements on screen, returns bounding boxes + labels
- `gui_click_object` — click a detected object by label/index

## Implementation Steps

### 1. Add Dependencies
- `ort` — ONNX Runtime for Rust (YOLO inference)
- or `tract` — alternative ONNX runtime

### 2. Model
- Use **YOLOv8n** (nano) — ~6MB, fast on CPU
- Download ONNX model on first use
- Input: RGB image, Output: bounding boxes + labels

### 3. Detection Types
Map COCO classes to UI-relevant ones:
- button, mouse, keyboard (from COCO)
- custom: icon, text-field, menu, checkbox, etc. (if model supports)

### 4. Files to Modify/Create
- `src/gui/detection.rs` — YOLO model loading + inference
- `src/gui/mod.rs` — add `detect_objects` to GuiClient
- `src/mcp/tools.rs` — add detection tools
- `src/mcp/handlers.rs` — implement handlers
- `src/web/server.rs` — add overlay toggle for detections

## Implementation Status
- [x] Add dependency to Cargo.toml (ort added, switching to tract)
- [x] Create src/gui/detection.rs — stub with mock data
- [x] Add detect_objects to GuiClient
- [x] Add MCP tools: gui_detect_objects, gui_click_object
- [ ] Implement real YOLO inference with tract
- [ ] Add web preview overlay for detections

## Current: Using tract ONNX runtime (more Rust-native)