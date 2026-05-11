use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

use super::error::TerminalError;

#[derive(Clone)]
pub struct TerminalHandle {
    pub input_tx: mpsc::Sender<Vec<u8>>,
}

struct TerminalState {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
}

pub struct PtyManager {
    terminals: Arc<RwLock<HashMap<String, Arc<Mutex<TerminalState>>>>>,
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            terminals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn spawn<F>(
        &self,
        terminal_id: String,
        cols: u16,
        rows: u16,
        cwd: Option<String>,
        env: Vec<(String, String)>,
        output_callback: F,
    ) -> Result<TerminalHandle, TerminalError>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        {
            let terminals = self.terminals.read().await;
            if terminals.contains_key(&terminal_id) {
                return Err(TerminalError::AlreadyExists(terminal_id));
            }
        }

        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(get_default_shell());

        #[cfg(unix)]
        cmd.arg("--login");

        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        for (key, value) in env {
            cmd.env(key, value);
        }

        cmd.env("TERM", "xterm-256color");

        let child = pair.slave.spawn_command(cmd)?;

        let master = pair.master;

        let (input_tx, mut input_rx) = mpsc::channel::<Vec<u8>>(256);

        let mut reader = master.try_clone_reader()?;
        let mut writer = master.take_writer()?;

        let tid_out = terminal_id.clone();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        tracing::debug!("Terminal {} EOF", tid_out);
                        break;
                    }
                    Ok(n) => {
                        output_callback(buffer[..n].to_vec());
                    }
                    Err(e) => {
                        tracing::debug!("Terminal {} read error: {}", tid_out, e);
                        break;
                    }
                }
            }
        });

        let tid_in = terminal_id.clone();
        tokio::spawn(async move {
            while let Some(data) = input_rx.recv().await {
                if let Err(e) = writer.write_all(&data) {
                    tracing::debug!("Terminal {} write error: {}", tid_in, e);
                    break;
                }
                let _ = writer.flush();
            }
        });

        let terminal_state = TerminalState { master, child };

        self.terminals
            .write()
            .await
            .insert(terminal_id.clone(), Arc::new(Mutex::new(terminal_state)));

        Ok(TerminalHandle { input_tx })
    }

    pub async fn resize(
        &self,
        terminal_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), TerminalError> {
        let terminals = self.terminals.read().await;

        if let Some(terminal) = terminals.get(terminal_id) {
            let state = terminal.lock().await;
            state.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
            Ok(())
        } else {
            Err(TerminalError::NotFound(terminal_id.to_string()))
        }
    }

    pub async fn close(&self, terminal_id: &str) -> Result<(), TerminalError> {
        let mut terminals = self.terminals.write().await;

        if let Some(terminal) = terminals.remove(terminal_id) {
            let mut state = terminal.lock().await;
            if let Err(e) = state.child.kill() {
                tracing::warn!("Error killing terminal process {}: {}", terminal_id, e);
            }
            let _ = state.child.wait();
            tracing::info!("Terminal {} closed", terminal_id);
            Ok(())
        } else {
            Err(TerminalError::NotFound(terminal_id.to_string()))
        }
    }

    pub async fn list(&self) -> Vec<String> {
        let terminals = self.terminals.read().await;
        terminals.keys().cloned().collect()
    }
}

fn get_default_shell() -> &'static str {
    #[cfg(windows)]
    {
        "powershell.exe"
    }
    #[cfg(unix)]
    {
        std::env::var("SHELL")
            .ok()
            .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
            .unwrap_or("/bin/bash")
    }
}