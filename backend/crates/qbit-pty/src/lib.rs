//! PTY and terminal management for Qbit.
//!
//! This crate provides pseudo-terminal (PTY) session management, including:
//! - Session lifecycle management (create, read, write, resize, terminate)
//! - Terminal escape sequence parsing (OSC, CSI)
//! - Shell integration via OSC 133 sequences
//! - Alternative screen buffer detection for TUI applications
//! - Working directory tracking
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate:
//! - Depends on: qbit-core (for runtime types), qbit-settings (for terminal config)
//! - Used by: qbit (main application via Tauri commands)
//!
//! # Features
//!
//! - `tauri`: Enables full PTY support (default disabled for CLI-only builds)
//!
//! # Usage
//!
//! ```rust,ignore
//! use qbit_pty::PtyManager;
//! use qbit_runtime::TauriRuntime;
//!
//! // Create PTY manager with runtime for event emission
//! let manager = PtyManager::new(runtime, settings);
//!
//! // Create a new PTY session
//! let session_id = manager.create_session("/path/to/workspace").await?;
//!
//! // Write input to session
//! manager.write_to_session(&session_id, "ls -la\n").await?;
//!
//! // Resize terminal
//! manager.resize_session(&session_id, 80, 24).await?;
//! ```

#[cfg(any(feature = "tauri", feature = "cli"))]
mod manager;
#[cfg(any(feature = "tauri", feature = "cli"))]
mod parser;
mod shell;

// Error types (always available)
mod error;
pub use error::{PtyError, Result};

// Public exports (feature-gated for tauri or cli)
#[cfg(any(feature = "tauri", feature = "cli"))]
pub use manager::{PtyManager, PtySession};

#[cfg(any(feature = "tauri", feature = "cli"))]
pub use parser::{OscEvent, TerminalParser};
pub use shell::{detect_shell, ShellInfo, ShellType};
