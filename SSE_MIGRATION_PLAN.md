# SSE Migration Plan: gui-mcp

## 1. Goal

Enable gui-mcp to run as a remote-capable HTTP server (SSE transport) while maintaining backward compatibility with stdio transport.

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     gui-mcp (dual mode)                     │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐     ┌─────────────────────────────┐   │
│  │  Transport Layer │     │      Application Layer      │   │
│  ├─────────────────┤     ├─────────────────────────────┤   │
│  │  stdio (default)│────▶│  MCP Protocol Handler      │   │
│  │  http (optional) │────▶│  - initialize              │   │
│  │                  │     │  - tools/list              │   │
│  │                  │     │  - tools/call              │   │
│  ├─────────────────┤     ├─────────────────────────────┤   │
│  │  Session Manager │     │  GUI Operations (Local)    │   │
│  │  - UUID per req │     │  - screen capture           │   │
│  │  - 30min TTL    │     │  - mouse/keyboard          │   │
│  │  - per-session  │     │  - ocr, clipboard, etc     │   │
│  │    config       │     │                             │   │
│  └─────────────────┘     └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

MCP Client (stdio):  gui-mcp                    # current
MCP Client (HTTP):  http://host:port/mcp       # new
```

## 3. Transport Modes

### Mode A: Stdio (Default, Current)
- Reads JSON-RPC from stdin
- Writes responses to stdout
- No session management
- No network exposure

### Mode B: HTTP/SSE (New)
- `POST /mcp` — JSON-RPC requests
- `DELETE /mcp` — terminate session
- `GET /health` — health check
- Session-based: each client gets UUID
- Per-session SSH-like config via headers (not needed for GUI, but reserve for consistency)

## 4. New Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `--transport` | `stdio` | Transport mode: `stdio` or `http` |
| `--port` | `3000` | HTTP listen port (http mode) |
| `--host` | `0.0.0.0` | HTTP bind address (http mode) |
| `--max-sessions` | `100` | Max concurrent sessions |
| `--session-ttl` | `1800` | Session TTL in seconds (30min) |

## 5. File Changes

### New Files

| File | Purpose |
|------|---------|
| `src/mcp/session.rs` | Session management (UUID, TTL, storage) |
| `src/mcp/http_transport.rs` | HTTP server, endpoints, CORS |

### Modified Files

| File | Changes |
|------|---------|
| `src/config.rs` | Add transport, port, host, session options |
| `src/mcp/server.rs` | Add transport selection, route to http handler |
| `src/main.rs` | Pass new config to server |

## 6. Session Manager Spec (`session.rs`)

```rust
pub struct Session {
    pub id: String,          // UUID v4
    pub created_at: i64,     // unix timestamp (seconds)
    pub last_active: i64,    // unix timestamp
}

pub struct SessionManager {
    sessions: HashMap<String, Session>,
    max_sessions: usize,
    ttl_seconds: u64,
    // ... mutex-protected
}

impl SessionManager {
    pub fn create() -> Session;
    pub fn get_or_create(session_id: Option<String>) -> Session;
    pub fn touch(session_id: &str);
    pub fn remove(session_id: &str);
    pub fn cleanup_expired();
}
```

## 7. HTTP Transport Spec (`http_transport.rs`)

```rust
pub fn run_http_server(port: u16, host: &str, session_mgr: SessionManager)

Endpoints:
- GET  /health        → {"status": "ok", "server": "gui-mcp", "version": "x.x.x"}
- POST /mcp           → JSON-RPC request body, returns JSON-RPC response
- DELETE /mcp         → Remove session (requires Mcp-Session-Id header)

Headers:
- Mcp-Session-Id: (optional) session UUID, created if not provided
- Access-Control-* : CORS headers

Response:
- 200: JSON-RPC response
- 202: Accepted (notification, no response)
- 400: JSON error
- 404: Not found
- 500: Internal error
```

## 8. Implementation Order

### Phase 1: Infrastructure ✅ DONE
1. Add new config options to `config.rs` — DONE
2. Create `session.rs` — session management — DONE

### Phase 2: HTTP Transport ✅ DONE
3. Create `http_transport.rs` — HTTP endpoints — DONE

### Phase 3: Integration ✅ DONE
4. Update `server.rs` — transport router — DONE
5. Update `main.rs` — pass config, start appropriate transport — (config passed, uses new run())

### Phase 4: Verification ✅ DONE
6. Test stdio mode still works - works normally (backward compatible)
7. Test HTTP mode works with curl/MCP client:
   - `GET /health` → `{"status":"ok","server":"gui-mcp","version":"0.1.0"}`
   - `POST /mcp` initialize → returns protocol capabilities
   - `POST /mcp` tools/list → returns 26 tools
   - Session headers work (`Mcp-Session-Id`)

## 9. Backward Compatibility

- Default: `--transport stdio` behaves exactly like current gui-mcp
- No breaking changes to existing users
- CLI args unchanged unless explicitly using new options

## 10. Security Notes (from ssh-mcp-sse reference)

- Validate all header inputs
- Sanitize paths if any user input used
- Consider rate limiting for HTTP mode
- CORS should be configurable (default: restricted, not open `*`)

---

*Plan created for gui-mcp SSE migration*