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
//! # Feature Flags
//!
//! The `tauri` and `cli` features are mutually exclusive:
//! - `tauri`: Enables TauriRuntime (requires Tauri framework)
//! - `cli`: Enables CliRuntime (no external dependencies)
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

// Compile-time guard: ensure tauri and cli features are mutually exclusive
#[cfg(all(feature = "tauri", feature = "cli"))]
compile_error!("Features 'tauri' and 'cli' are mutually exclusive. Use --features tauri OR --features cli, not both.");

// Re-export core runtime types for convenience
pub use qbit_core::runtime::{ApprovalResult, QbitRuntime, RuntimeError, RuntimeEvent};

// Feature-gated runtime implementations
#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "tauri")]
pub mod tauri;

// Re-exports for convenience (feature-gated)
#[cfg(feature = "cli")]
pub use cli::CliRuntime;
#[cfg(feature = "tauri")]
pub use tauri::TauriRuntime;
