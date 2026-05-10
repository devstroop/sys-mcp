#[cfg(feature = "web-preview")]
use std::sync::Arc;

#[cfg(feature = "web-preview")]
use axum::extract::{Query, State};
#[cfg(feature = "web-preview")]
use axum::http::StatusCode;
#[cfg(feature = "web-preview")]
use axum::response::{Html, IntoResponse, Response};
#[cfg(feature = "web-preview")]
use axum::Json;
#[cfg(feature = "web-preview")]
use axum::routing::get;
#[cfg(feature = "web-preview")]
use axum::Router;
#[cfg(feature = "web-preview")]
use serde::Deserialize;
#[cfg(feature = "web-preview")]
use tokio::net::TcpListener;

#[cfg(feature = "web-preview")]
use crate::gui::GuiClient;
#[cfg(feature = "web-preview")]
use crate::gui::types::*;

// ─── Auth guard ─────────────────────────────────────────────────────────────

#[cfg(feature = "web-preview")]
#[derive(Deserialize)]
pub struct TokenQuery {
    pub token: Option<String>,
}

#[cfg(feature = "web-preview")]
pub struct WebState {
    pub client: Arc<GuiClient>,
    pub token: String,
}

#[cfg(feature = "web-preview")]
type SharedState = Arc<WebState>;

#[cfg(feature = "web-preview")]
fn check_token(state: &WebState, query: &TokenQuery) -> Result<(), Response> {
    match &query.token {
        Some(t) if t == &state.token => Ok(()),
        _ => Err((StatusCode::UNAUTHORIZED, "invalid or missing token").into_response()),
    }
}

// ─── Web server lifecycle ───────────────────────────────────────────────────

#[cfg(feature = "web-preview")]
pub struct WebServer {
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub port: u16,
    pub token: String,
}

#[cfg(feature = "web-preview")]
impl WebServer {
    pub async fn start(client: Arc<GuiClient>) -> anyhow::Result<Self> {
        let token = uuid::Uuid::new_v4().to_string();
        let state = Arc::new(WebState {
            client,
            token: token.clone(),
        });

        let app = Router::new()
            .route("/", get(page_index))
            .route("/screenshot", get(screenshot_feed))
            .route("/screenshot.png", get(screenshot_png))
            .route("/api/ocr", get(api_ocr))
            .route("/api/click", get(api_click))
            .route("/api/type", get(api_type))
            .route("/api/key", get(api_key))
            .route("/api/scroll", get(api_scroll))
            .with_state(state);

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
        });

        log::info!("web preview server started on http://127.0.0.1:{port}/?token={token}");

        Ok(Self {
            shutdown_tx: Some(shutdown_tx),
            port,
            token,
        })
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/?token={}", self.port, self.token)
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(feature = "web-preview")]
impl Drop for WebServer {
    fn drop(&mut self) {
        self.stop();
    }
}

// ─── Routes ─────────────────────────────────────────────────────────────────

#[cfg(feature = "web-preview")]
async fn page_index(
    State(state): State<SharedState>,
    Query(q): Query<TokenQuery>,
) -> Result<Html<String>, Response> {
    check_token(&state, &q)?;
    let token = &state.token;
    Ok(Html(format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, user-scalable=no">
<title>GUI MCP — Local Desktop</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  html, body {{ background: #000; width: 100%; height: 100%; overflow: hidden; }}
  #viewport {{ width: 100vw; height: 100vh; display: flex; align-items: center; justify-content: center; overflow: hidden; position: relative; }}
  #screen {{ max-width: 100vw; max-height: 100vh; width: auto; height: auto; object-fit: contain; cursor: crosshair; display: block; }}
  #text-overlay {{ position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; overflow: hidden; }}
  #text-overlay .word {{ position: absolute; color: #00ff00; font-size: 12px; font-family: monospace; background: rgba(0,0,0,0.6); padding: 1px 3px; border-radius: 2px; white-space: nowrap; }}
  #overlay {{ position: fixed; top: 0; right: 0; z-index: 90; display: flex; align-items: center; gap: 6px; padding: 4px 10px; background: rgba(10,10,10,0.75); backdrop-filter: blur(8px); border-bottom-left-radius: 8px; font: 11px/1.2 system-ui, sans-serif; color: #aaa; opacity: 0; transition: opacity 0.25s; pointer-events: none; }}
  #overlay:hover, body:hover #overlay {{ opacity: 1; pointer-events: auto; }}
  #overlay label {{ cursor: pointer; white-space: nowrap; user-select: none; }}
  #overlay select {{ background: #1a1a1a; border: 1px solid #333; color: #ccc; padding: 1px 4px; border-radius: 3px; font-size: 10px; }}
  #coords {{ position: fixed; bottom: 4px; right: 8px; font: 11px/1 monospace; color: #444; z-index: 80; pointer-events: none; }}
  #toast {{ position: fixed; bottom: 28px; right: 8px; background: rgba(20,20,20,0.85); border: 1px solid #333; padding: 4px 12px; border-radius: 5px; font: 11px system-ui; color: #4fc3f7; opacity: 0; transition: opacity 0.25s; pointer-events: none; z-index: 100; }}
  #toast.show {{ opacity: 1; }}
  #type-overlay {{ position: fixed; top: 50%; left: 50%; transform: translate(-50%,-50%); background: #111; border: 1px solid #4fc3f7; padding: 10px 14px; border-radius: 8px; display: none; z-index: 110; min-width: 280px; }}
  #type-overlay input {{ background: #1a1a1a; border: 1px solid #333; color: #fff; padding: 6px 10px; width: 100%; border-radius: 4px; font: 13px monospace; }}
  #type-overlay .hint {{ font-size: 10px; color: #555; margin-top: 3px; }}
</style>
</head>
<body>
<div id="overlay">
  <label><input type="checkbox" id="auto-refresh" checked onchange="toggleAutoRefresh()"> Auto</label>
  <select id="refresh-interval" onchange="updateInterval()">
    <option value="500">0.5s</option>
    <option value="1000" selected>1s</option>
    <option value="2000">2s</option>
    <option value="5000">5s</option>
  </select>
  <label><input type="checkbox" id="click-mode" checked> Click</label>
  <label><input type="checkbox" id="show-text" onchange="toggleText()"> Text</label>
</div>

<div id="viewport">
  <img id="screen" src="/screenshot.png?token={token}&_t=0" alt="Screen"
       onclick="handleClick(event)" onmousemove="trackMouse(event)" oncontextmenu="handleRightClick(event)">
  <div id="text-overlay"></div>
</div>

<div id="coords">-</div>
<div id="toast"></div>
<div id="type-overlay">
  <input id="type-input" placeholder="Type and press Enter..." onkeydown="handleTypeKey(event)">
  <div class="hint">Enter = send | Esc = cancel | Ctrl+Enter = key combo</div>
</div>

<script>
const TOKEN = "{token}";
const IMG = document.getElementById("screen");
const TEXT_OVERLAY = document.getElementById("text-overlay");
let refreshTimer = null;
let interval = 1000;
let showText = false;

function refreshScreenshot() {{
  const next = new Image();
  next.onload = function() {{ IMG.src = this.src; scheduleRefresh(); }};
  next.onerror = function() {{ scheduleRefresh(); }};
  next.src = `/screenshot.png?token=${{TOKEN}}&_t=${{Date.now()}}`;
}}
function scheduleRefresh() {{
  clearTimeout(refreshTimer);
  if (document.getElementById("auto-refresh").checked) refreshTimer = setTimeout(refreshScreenshot, interval);
}}
function toggleAutoRefresh() {{ scheduleRefresh(); if (document.getElementById("auto-refresh").checked) refreshScreenshot(); }}
function updateInterval() {{ interval = parseInt(document.getElementById("refresh-interval").value); }}
function toggleText() {{ showText = document.getElementById("show-text").checked; TEXT_OVERLAY.innerHTML = ""; if (showText) fetchOcr(); }}
async function fetchOcr() {{
  try {{
    const resp = await fetch(`/api/ocr?token=${{TOKEN}}`);
    const data = await resp.json();
    if (!data.lines) return;
    // Wait for image to be fully loaded
    if (!IMG.complete || IMG.naturalWidth === 0) {{
      IMG.onload = fetchOcr;
      return;
    }}
    // Get image position within viewport
    const imgRect = IMG.getBoundingClientRect();
    const viewRect = document.getElementById("viewport").getBoundingClientRect();
    const offsetX = imgRect.left - viewRect.left;
    const offsetY = imgRect.top - viewRect.top;
    const scaleX = imgRect.width / data.screen_width;
    const scaleY = imgRect.height / data.screen_height;
    console.log("OCR: screen=" + data.screen_width + "x" + data.screen_height + " img=" + imgRect.width + "x" + imgRect.height + " offset=" + offsetX + "," + offsetY);
    TEXT_OVERLAY.innerHTML = "";
    for (const line of data.lines) {{
      for (const word of line.words || []) {{
        const el = document.createElement("span");
        el.className = "word";
        el.textContent = word.text;
        el.style.left = Math.round(offsetX + word.x * scaleX) + "px";
        el.style.top = Math.round(offsetY + word.y * scaleY) + "px";
        TEXT_OVERLAY.appendChild(el);
      }}
    }}
  }} catch(e) {{ console.log("OCR error:", e); }}
}}
refreshTimer = setTimeout(refreshScreenshot, interval);

function getImageCoords(e) {{
  const rect = IMG.getBoundingClientRect();
  return {{ x: Math.round((e.clientX - rect.left) * IMG.naturalWidth / rect.width), y: Math.round((e.clientY - rect.top) * IMG.naturalHeight / rect.height) }};
}}
function trackMouse(e) {{ const c = getImageCoords(e); document.getElementById("coords").textContent = `${{c.x}},${{c.y}}`; }}
function toast(msg) {{ const el = document.getElementById("toast"); el.textContent = msg; el.classList.add("show"); setTimeout(() => el.classList.remove("show"), 1200); }}

async function handleClick(e) {{
  if (!document.getElementById("click-mode").checked) return;
  e.preventDefault();
  const c = getImageCoords(e);
  const btn = e.button === 2 ? "right" : "left";
  toast(`click ${{c.x}},${{c.y}} ${{btn}}`);
  await fetch(`/api/click?token=${{TOKEN}}&x=${{c.x}}&y=${{c.y}}&button=${{btn}}`);
  setTimeout(refreshScreenshot, 400);
}}
function handleRightClick(e) {{ if (document.getElementById("click-mode").checked) {{ e.preventDefault(); handleClick(e); }} }}

document.addEventListener("keydown", async (e) => {{
  if (document.getElementById("type-overlay").style.display === "block") return;
  if (e.target.tagName === "INPUT" || e.target.tagName === "SELECT") return;
  if (e.key === "t" || e.key === "T") {{ e.preventDefault(); document.getElementById("type-overlay").style.display = "block"; document.getElementById("type-input").value = ""; document.getElementById("type-input").focus(); return; }}
  const keyMap = {{ "Enter":"return","Escape":"escape","Tab":"tab","Backspace":"backspace","Delete":"delete","ArrowUp":"up","ArrowDown":"down","ArrowLeft":"left","ArrowRight":"right"," ":"space" }};
  const mapped = keyMap[e.key];
  if (mapped) {{ e.preventDefault(); let c = []; if (e.ctrlKey) c.push("ctrl"); if (e.altKey) c.push("alt"); if (e.shiftKey) c.push("shift"); c.push(mapped); await fetch(`/api/key?token=${{TOKEN}}&key=${{encodeURIComponent(c.join("+"))}}`); setTimeout(refreshScreenshot, 400); }}
}});

async function handleTypeKey(e) {{
  if (e.key === "Escape") {{ document.getElementById("type-overlay").style.display = "none"; return; }}
  if (e.key === "Enter") {{ e.preventDefault(); const text = document.getElementById("type-input").value; document.getElementById("type-overlay").style.display = "none"; if (text) {{ if (e.ctrlKey) await fetch(`/api/key?token=${{TOKEN}}&key=${{encodeURIComponent(text)}}`); else await fetch(`/api/type?token=${{TOKEN}}&text=${{encodeURIComponent(text)}}`); toast(`typed: ${{text}}`); setTimeout(refreshScreenshot, 400); }} }}
}}

document.getElementById("viewport").addEventListener("wheel", async (e) => {{
  if (!document.getElementById("click-mode").checked) return;
  e.preventDefault();
  const c = getImageCoords(e);
  const dir = e.deltaY > 0 ? "down" : "up";
  await fetch(`/api/scroll?token=${{TOKEN}}&x=${{c.x}}&y=${{c.y}}&direction=${{dir}}&amount=${{Math.min(Math.ceil(Math.abs(e.deltaY)/50),10)}}`);
  setTimeout(refreshScreenshot, 300);
}}, {{ passive: false }});
</script>
</body>
</html>"##,
        token = token,
    )))
}

#[cfg(feature = "web-preview")]
async fn screenshot_png(
    State(state): State<SharedState>,
    Query(q): Query<TokenQuery>,
) -> Result<Response, Response> {
    check_token(&state, &q)?;

    let screenshot = state.client.screenshot().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("screenshot error: {e}")).into_response()
    })?;

    Ok((
        StatusCode::OK,
        [
            ("content-type", "image/png"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
        ],
        screenshot.data,
    )
        .into_response())
}

#[cfg(feature = "web-preview")]
async fn api_ocr(
    State(state): State<SharedState>,
    Query(q): Query<TokenQuery>,
) -> Result<Json<serde_json::Value>, Response> {
    check_token(&state, &q)?;

    let ocr_result = state.client.read_screen(None).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("OCR error: {e}")).into_response()
    })?;

    let json_value = serde_json::to_value(ocr_result).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("JSON error: {e}")).into_response()
    })?;

    Ok(Json(json_value))
}

#[cfg(feature = "web-preview")]
async fn screenshot_feed(
    State(state): State<SharedState>,
    Query(q): Query<TokenQuery>,
) -> Result<Html<String>, Response> {
    check_token(&state, &q)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let token = &state.token;
    Ok(Html(format!(
        r#"<img id="screen" src="/screenshot.png?token={token}&_t={ts}" alt="Screen"
             onclick="handleClick(event)" onmousemove="trackMouse(event)" oncontextmenu="handleRightClick(event)">"#
    )))
}

#[cfg(feature = "web-preview")]
#[derive(Deserialize)]
struct ClickParams {
    token: Option<String>,
    x: u32,
    y: u32,
    button: Option<String>,
}

#[cfg(feature = "web-preview")]
async fn api_click(
    State(state): State<SharedState>,
    Query(p): Query<ClickParams>,
) -> Result<&'static str, Response> {
    check_token(&state, &TokenQuery { token: p.token })?;
    let button = match p.button.as_deref() {
        Some("right") => MouseButton::Right,
        Some("middle") => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    state.client.click(p.x, p.y, button).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
    })?;
    Ok("ok")
}

#[cfg(feature = "web-preview")]
#[derive(Deserialize)]
struct TypeParams {
    token: Option<String>,
    text: String,
}

#[cfg(feature = "web-preview")]
async fn api_type(
    State(state): State<SharedState>,
    Query(p): Query<TypeParams>,
) -> Result<&'static str, Response> {
    check_token(&state, &TokenQuery { token: p.token })?;
    state.client.type_text(&p.text).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
    })?;
    Ok("ok")
}

#[cfg(feature = "web-preview")]
#[derive(Deserialize)]
struct KeyParams {
    token: Option<String>,
    key: String,
}

#[cfg(feature = "web-preview")]
async fn api_key(
    State(state): State<SharedState>,
    Query(p): Query<KeyParams>,
) -> Result<&'static str, Response> {
    check_token(&state, &TokenQuery { token: p.token })?;
    if p.key.contains('+') {
        let keys: Vec<String> = p.key.split('+').map(String::from).collect();
        state.client.key_combo(&keys).await
    } else {
        state.client.press_key(&p.key).await
    }
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok("ok")
}

#[cfg(feature = "web-preview")]
#[derive(Deserialize)]
struct ScrollParams {
    token: Option<String>,
    x: u32,
    y: u32,
    direction: String,
    amount: Option<i32>,
}

#[cfg(feature = "web-preview")]
async fn api_scroll(
    State(state): State<SharedState>,
    Query(p): Query<ScrollParams>,
) -> Result<&'static str, Response> {
    check_token(&state, &TokenQuery { token: p.token })?;
    let direction = match p.direction.as_str() {
        "up" => ScrollDirection::Up,
        "down" => ScrollDirection::Down,
        "left" => ScrollDirection::Left,
        "right" => ScrollDirection::Right,
        _ => ScrollDirection::Down,
    };
    state
        .client
        .scroll(p.x, p.y, direction, p.amount.unwrap_or(3))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok("ok")
}
