//! Runtime implementations for Qbit.
//!
//! This crate provides platform-specific runtime implementations for the Qbit application:
//! - **TauriRuntime**: For GUI application (Tauri framework)
//! - **CliRuntime**: For headless CLI usage
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate:
//! - Depends on: qbit-core (for QbitRuntime trait and types)
//! - Used by: qbit (main application)
//!
//! # Usage
//!
//! ```rust,ignore
//! // Tauri runtime (GUI)
//! use crate::runtime::TauriRuntime;
//! use qbit_core::runtime::QbitRuntime;
//!
//! let runtime = TauriRuntime::new(app_handle);
//! runtime.emit(RuntimeEvent::Ai { ... })?;
//!
//! // CLI runtime (headless)
//! use crate::runtime::CliRuntime;
//!
//! let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
//! let runtime = CliRuntime::new(tx, auto_approve, json_mode);
//! ```

// Re-export core runtime types for convenience
pub use qbit_core::runtime::{ApprovalResult, QbitRuntime, RuntimeError, RuntimeEvent};

// Both runtime implementations are always available
pub mod cli;
#[path = "tauri_runtime.rs"]
pub mod tauri_runtime;

// Re-exports for convenience
pub use cli::CliRuntime;
pub use tauri_runtime::TauriRuntime;
