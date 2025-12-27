//! Tools module - re-exports from qbit-tools crate.
//!
//! This module provides a thin wrapper around the qbit-tools infrastructure crate.
//!
//! # Architecture
//!
//! - **qbit-tools**: Infrastructure crate with tool execution system
//! - **qbit/tools/mod.rs**: Re-exports for compatibility

// Re-export everything from qbit-tools
pub use qbit_tools::*;
