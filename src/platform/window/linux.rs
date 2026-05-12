use std::io::Read;

use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

use crate::error::GuiError;
use crate::gui::types::*;

atom_manager! {
    pub AtomCollection: AtomCollectionCookie {
        NET_ACTIVE_WINDOW,
        NET_CLIENT_LIST,
        NET_CLOSE_WINDOW,
        NET_CURRENT_DESKTOP,
        NET_FRAME_EXTENTS,
        NET_MOVERESIZE_WINDOW,
        NET_WM_DESKTOP,
        NET_WM_NAME,
        NET_WM_PID,
        NET_WM_STATE,
        NET_WM_STATE_FOCUSED,
        NET_WM_STATE_HIDDEN,
        NET_WM_STATE_MAXIMIZED_HORZ,
        NET_WM_STATE_MAXIMIZED_VERT,
        NET_WM_WINDOW_TYPE,
        NET_WM_WINDOW_TYPE_NORMAL,
        NET_WM_WINDOW_TYPE_DIALOG,
        UTF8_STRING,
        WM_DELETE_WINDOW,
        WM_PROTOCOLS,
        WM_STATE,
    }
}

pub struct PlatformWindowManager {
    conn: RustConnection,
    screen_num: usize,
    atoms: AtomCollection,
}

    impl PlatformWindowManager {
    pub fn new() -> Result<Self, GuiError> {
        let (conn, screen_num) = RustConnection::connect(None)
            .map_err(|e| GuiError::PlatformError(format!("X11 connection failed: {e}")))?;

        let atoms = AtomCollection::new(&conn)
            .map_err(|e| GuiError::PlatformError(format!("X11 atom init failed: {e}")))?
            .reply()
            .map_err(|e| GuiError::PlatformError(format!("X11 atom reply failed: {e}")))?;

        Ok(Self {
            conn,
            screen_num,
            atoms,
        })
    }

    fn screen(&self) -> &xproto::Screen {
        &self.conn.setup().roots[self.screen_num]
    }

    fn root(&self) -> u32 {
        self.screen().root
    }

    fn read_property(&self, window: u32, atom: u32) -> Result<Vec<u8>, GuiError> {
        let reply = self
            .conn
            .get_property(false, window, atom, AtomEnum::ANY, 0, u32::MAX)
            .map_err(|e| GuiError::PlatformError(format!("X11 get_property failed: {e}")))?
            .reply()
            .map_err(|e| GuiError::PlatformError(format!("X11 get_property reply failed: {e}")))?;

        Ok(reply.value)
    }

    fn read_string_property(&self, window: u32, atom: u32) -> Result<Option<String>, GuiError> {
        let data = self.read_property(window, atom)?;
        if data.is_empty() {
            return Ok(None);
        }
        // Strip trailing null bytes
        let trimmed = data
            .iter()
            .take_while(|&&b| b != 0)
            .copied()
            .collect::<Vec<_>>();
        String::from_utf8(trimmed)
            .map(Some)
            .map_err(|_| GuiError::PlatformError("Invalid UTF-8 in X11 property".into()))
    }

    fn read_card32_property(&self, window: u32, atom: u32) -> Result<Vec<u32>, GuiError> {
        let data = self.read_property(window, atom)?;
        Ok(data
            .chunks_exact(4)
            .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    }

    pub fn get_window_title(&self, window_id: u64) -> Result<String, GuiError> {
        let wid = window_id as u32;

        // Try _NET_WM_NAME (UTF-8) first, fall back to WM_NAME
        if let Some(name) = self
            .read_string_property(wid, self.atoms.NET_WM_NAME)
            .map_err(|_| GuiError::PlatformError("Failed to read _NET_WM_NAME".into()))?
        {
            if !name.is_empty() {
                return Ok(name);
            }
        }

        // Fall back to WM_NAME
        if let Some(name) = self
            .read_string_property(wid, AtomEnum::WM_NAME.into())
            .map_err(|_| GuiError::PlatformError("Failed to read WM_NAME".into()))?
        {
            if !name.is_empty() {
                return Ok(name);
            }
        }

        Ok(String::new())
    }

    fn is_window_visible(&self, window_id: u32) -> Result<bool, GuiError> {
        let attr = self
            .conn
            .get_window_attributes(window_id)
            .map_err(|e| GuiError::PlatformError(format!("X11 get_window_attributes: {e}")))?
            .reply()
            .map_err(|e| GuiError::PlatformError(format!("X11 get_window_attributes: {e}")))?;

        if attr.map_state == xproto::MapState::UNMAPPED {
            return Ok(false);
        }

        // Check _NET_WM_STATE for HIDDEN
        if let Ok(states) = self.read_card32_property(window_id, self.atoms.NET_WM_STATE) {
            if states.contains(&self.atoms.NET_WM_STATE_HIDDEN) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn get_process_name(&self, window_id: u32) -> Option<String> {
        // Try WM_CLASS first (common for X11 apps)
        if let Ok(Some(class)) =
            self.read_string_property(window_id, AtomEnum::WM_CLASS.into())
        {
            // WM_CLASS is "instance\0class" -- take the second part
            let parts: Vec<&str> = class.split('\0').collect();
            let name = parts.last().unwrap_or(&"").to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }

        // Try _NET_WM_PID and read /proc/PID/comm
        if let Ok(pids) = self.read_card32_property(window_id, self.atoms.NET_WM_PID) {
            if let Some(&pid) = pids.first() {
                if pid > 0 {
                    let mut buf = String::new();
                    if std::fs::File::open(format!("/proc/{pid}/comm"))
                        .and_then(|mut f| f.read_to_string(&mut buf))
                        .is_ok()
                    {
                        return Some(buf.trim().to_string());
                    }
                }
            }
        }

        None
    }

    fn get_pid(&self, window_id: u32) -> Option<u32> {
        if let Ok(pids) = self.read_card32_property(window_id, self.atoms.NET_WM_PID) {
            return pids.first().copied().filter(|&p| p > 0);
        }
        None
    }

    fn send_ewmh_message(
        &self,
        window: u32,
        message_type: u32,
        data: [u32; 5],
    ) -> Result<(), GuiError> {
        let event = xproto::ClientMessageEvent::new(
            32,
            window,
            message_type,
            xproto::ClientMessageData::from(data),
        );

        self.conn
            .send_event(false, self.root(), xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY, event)
            .map_err(|e| GuiError::PlatformError(format!("X11 send_event failed: {e}")))?
            .check()
            .map_err(|e| GuiError::PlatformError(format!("X11 send_event check failed: {e}")))?;

        self.conn
            .flush()
            .map_err(|e| GuiError::PlatformError(format!("X11 flush failed: {e}")))?;

        Ok(())
    }

    pub fn list_windows(&self) -> Result<Vec<WindowInfo>, GuiError> {
        let mut windows = Vec::new();

        // Query _NET_CLIENT_LIST from root window
        let client_list = self
            .read_card32_property(self.root(), self.atoms.NET_CLIENT_LIST)
            .unwrap_or_default();

        let active = self
            .read_card32_property(self.root(), self.atoms.NET_ACTIVE_WINDOW)
            .ok()
            .and_then(|v| v.first().copied())
            .unwrap_or(0);

        for &wid in &client_list {
            if wid == self.root() {
                continue;
            }

            if !self.is_window_visible(wid).unwrap_or(false) {
                continue;
            }

            let title = self.get_window_title(wid as u64).unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let geometry = self
                .conn
                .get_geometry(wid)
                .ok()
                .and_then(|c| c.reply().ok());

            let (x, y, w, h) = geometry
                .map(|g| (g.x as i32, g.y as i32, g.width as u32, g.height as u32))
                .unwrap_or((0, 0, 0, 0));

            let is_minimized = self
                .read_card32_property(wid, self.atoms.NET_WM_STATE)
                .ok()
                .map(|states| {
                    states.contains(&self.atoms.NET_WM_STATE_HIDDEN)
                })
                .unwrap_or(false);

            let is_maximized = self
                .read_card32_property(wid, self.atoms.NET_WM_STATE)
                .ok()
                .map(|states| {
                    states.contains(&self.atoms.NET_WM_STATE_MAXIMIZED_VERT)
                        || states.contains(&self.atoms.NET_WM_STATE_MAXIMIZED_HORZ)
                })
                .unwrap_or(false);

            // Check WM_STATE for IconicState (minimized)
            let is_minimized = is_minimized
                || self
                    .read_property(wid, self.atoms.WM_STATE)
                    .ok()
                    .map(|data| {
                        data.len() >= 4 && u32::from_ne_bytes([data[0], data[1], data[2], data[3]]) == 3
                    })
                    .unwrap_or(false);

            let is_focused = wid == active;

            let process_name = self.get_process_name(wid);
            let process_id = self.get_pid(wid);

            windows.push(WindowInfo {
                id: wid as u64,
                title,
                x,
                y,
                width: w,
                height: h,
                is_minimized,
                is_maximized,
                is_focused,
                process_name,
                process_id,
            });
        }

        Ok(windows)
    }

    pub fn get_active_window(&self) -> Result<WindowInfo, GuiError> {
        let active = self
            .read_card32_property(self.root(), self.atoms.NET_ACTIVE_WINDOW)
            .map_err(|_| GuiError::PlatformError("Failed to get active window".into()))?
            .first()
            .copied()
            .ok_or(GuiError::PlatformError("No active window".into()))?;

        if active == 0 || active == self.root() {
            return Err(GuiError::PlatformError("No active window".into()));
        }

        let all = self.list_windows()?;
        all.into_iter()
            .find(|w| w.id == active as u64)
            .ok_or(GuiError::PlatformError("Active window not found in client list".into()))
    }

    pub fn focus_window(&self, window_id: u64) -> Result<(), GuiError> {
        let wid = window_id as u32;
        self.send_ewmh_message(
            wid,
            self.atoms.NET_ACTIVE_WINDOW,
            [2, 0, 0, 0, 0], // 2 = pager request
        )
    }

    pub fn move_window(&self, window_id: u64, x: i32, y: i32) -> Result<(), GuiError> {
        let wid = window_id as u32;
        // _NET_MOVERESIZE_WINDOW: source=1 (application), gravity=static (1),
        // x,y set, width/height unchanged
        let flags: u32 = 0b0000_0001_0000_0000_0000_0000_0000_0001; // bit 0 = x, bit 8 = y
        self.send_ewmh_message(
            wid,
            self.atoms.NET_MOVERESIZE_WINDOW,
            [flags, x as u32, y as u32, 0, 0],
        )
    }

    pub fn resize_window(&self, window_id: u64, width: u32, height: u32) -> Result<(), GuiError> {
        let wid = window_id as u32;
        // _NET_MOVERESIZE_WINDOW: source=1, gravity=static,
        // width/height set, x/y unchanged
        let flags: u32 = 0b0000_0001_0001_0000_0000_0000_0000_0000; // bit 16 = width, bit 24 = height
        self.send_ewmh_message(
            wid,
            self.atoms.NET_MOVERESIZE_WINDOW,
            [flags, 0, 0, width, height],
        )
    }

    pub fn minimize_window(&self, window_id: u64) -> Result<(), GuiError> {
        let wid = window_id as u32;
        // _NET_WM_STATE: action=0 (remove), first=HIDDEN, second=0
        self.send_ewmh_message(
            wid,
            self.atoms.NET_WM_STATE,
            [0, self.atoms.NET_WM_STATE_HIDDEN, 0, 0, 1],
        )
    }

    pub fn maximize_window(&self, window_id: u64) -> Result<(), GuiError> {
        let wid = window_id as u32;
        // _NET_WM_STATE: action=1 (add), first=MAXIMIZED_VERT, second=MAXIMIZED_HORZ
        self.send_ewmh_message(
            wid,
            self.atoms.NET_WM_STATE,
            [
                1,
                self.atoms.NET_WM_STATE_MAXIMIZED_VERT,
                self.atoms.NET_WM_STATE_MAXIMIZED_HORZ,
                0,
                1,
            ],
        )
    }

    pub fn restore_window(&self, window_id: u64) -> Result<(), GuiError> {
        let wid = window_id as u32;
        // Remove HIDDEN and MAXIMIZED states
        self.send_ewmh_message(
            wid,
            self.atoms.NET_WM_STATE,
            [
                0,
                self.atoms.NET_WM_STATE_HIDDEN,
                self.atoms.NET_WM_STATE_MAXIMIZED_VERT,
                0,
                1,
            ],
        )?;
        self.send_ewmh_message(
            wid,
            self.atoms.NET_WM_STATE,
            [
                0,
                self.atoms.NET_WM_STATE_MAXIMIZED_HORZ,
                0,
                0,
                1,
            ],
        )
    }

    pub fn close_window(&self, window_id: u64) -> Result<(), GuiError> {
        let wid = window_id as u32;

        // Try _NET_CLOSE_WINDOW first
        self.send_ewmh_message(
            wid,
            self.atoms.NET_CLOSE_WINDOW,
            [1, 0, 0, 0, 0], // source=1 (application)
        )
        .or_else(|_| {
            // Fallback: send WM_DELETE_WINDOW protocol
            let event = xproto::ClientMessageEvent::new(
                32,
                wid,
                self.atoms.WM_PROTOCOLS,
                xproto::ClientMessageData::from([
                    self.atoms.WM_DELETE_WINDOW,
                    1, // timestamp
                    0,
                    0,
                    0,
                ]),
            );

            self.conn
                .send_event(
                    false,
                    wid,
                    xproto::EventMask::SUBSTRUCTURE_REDIRECT
                        | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                    event,
                )
                .map_err(|e| {
                    GuiError::PlatformError(format!("X11 close window failed: {e}"))
                })?
                .check()
                .map_err(|e| {
                    GuiError::PlatformError(format!("X11 close window check failed: {e}"))
                })?;

            self.conn.flush().map_err(|e| {
                GuiError::PlatformError(format!("X11 flush failed: {e}"))
            })
        })
    }

    pub fn get_window_bounds(&self, window_id: u64) -> Result<Region, GuiError> {
        let wid = window_id as u32;

        let geometry = self
            .conn
            .get_geometry(wid)
            .map_err(|e| GuiError::PlatformError(format!("X11 get_geometry: {e}")))?
            .reply()
            .map_err(|e| GuiError::PlatformError(format!("X11 get_geometry: {e}")))?;

        let (mut x, mut y) = (geometry.x as i32, geometry.y as i32);

        // Translate to root window coordinates
        if let Ok(cookie) = self.conn.translate_coordinates(wid, self.root(), 0, 0) {
            if let Ok(reply) = cookie.reply() {
                x = reply.dst_x as i32;
                y = reply.dst_y as i32;
            }
        }

        // Subtract frame extents if available
        if let Ok(extents) = self.read_card32_property(wid, self.atoms.NET_FRAME_EXTENTS) {
            if extents.len() >= 4 {
                x -= extents[0] as i32; // left
                y -= extents[1] as i32; // top
            }
        }

        Ok(Region {
            x: x as u32,
            y: y as u32,
            width: geometry.width as u32,
            height: geometry.height as u32,
        })
    }

    pub fn find_windows_by_title(&self, query: &str) -> Result<Vec<WindowInfo>, GuiError> {
        let all = self.list_windows()?;
        let query_lower = query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|w| w.title.to_lowercase().contains(&query_lower))
            .collect())
    }
}
