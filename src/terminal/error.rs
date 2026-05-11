use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("Terminal not found: {0}")]
    NotFound(String),

    #[error("Terminal already exists: {0}")]
    AlreadyExists(String),

    #[error("PTY error: {0}")]
    PtyError(#[from] std::io::Error),

    #[error("Send error: {0}")]
    SendError(String),

    #[error("Maximum terminals reached")]
    MaxTerminalsReached,

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}