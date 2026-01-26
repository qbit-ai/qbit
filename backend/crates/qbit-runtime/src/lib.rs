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
//! use qbit_runtime::TauriRuntime;
//! use qbit_core::runtime::QbitRuntime;
//!
//! let runtime = TauriRuntime::new(app_handle);
//! runtime.emit(RuntimeEvent::Ai { ... })?;
//!
//! // CLI runtime (headless)
//! use qbit_runtime::CliRuntime;
//!
//! let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
//! let runtime = CliRuntime::new(tx, auto_approve, json_mode);
//! ```

// Re-export core runtime types for convenience
pub use qbit_core::runtime::{ApprovalResult, QbitRuntime, RuntimeError, RuntimeEvent};

// Both runtime implementations are always available
pub mod cli;
pub mod tauri;

// Re-exports for convenience
pub use cli::CliRuntime;
pub use tauri::TauriRuntime;
