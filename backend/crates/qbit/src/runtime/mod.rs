//! Runtime abstraction - re-exports from qbit-runtime crate.
//!
//! This module provides a thin wrapper around the qbit-runtime infrastructure crate,
//! which contains the platform-specific implementations (TauriRuntime and CliRuntime).
//!
//! # Architecture
//!
//! - **qbit-core**: Runtime trait definitions (QbitRuntime, RuntimeEvent, etc.)
//! - **qbit-runtime**: Runtime implementations (TauriRuntime, CliRuntime)
//! - **qbit/runtime/mod.rs**: Re-exports for backward compatibility

// Compile-time guard: ensure tauri and cli features are mutually exclusive
#[cfg(all(feature = "tauri", feature = "cli"))]
compile_error!("Features 'tauri' and 'cli' are mutually exclusive. Use --features tauri OR --features cli, not both.");

// Re-export everything from qbit-runtime
// Note: qbit-runtime itself re-exports from qbit-core, so this gives us all runtime types
pub use qbit_runtime::*;
