//! Error types for PTY operations.

use thiserror::Error;

/// Errors that can occur during PTY operations.
#[derive(Debug, Error)]
pub enum PtyError {
    /// PTY system error (e.g., failed to spawn, read, write)
    #[error("PTY error: {0}")]
    Pty(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Session has already exited
    #[error("Session {0} has already exited")]
    SessionExited(String),

    /// Invalid session state
    #[error("Invalid session state: {0}")]
    InvalidState(String),
}

/// Result type for PTY operations.
pub type Result<T> = std::result::Result<T, PtyError>;
