//! Unified diff editing module for surgical multi-hunk code edits.
//!
//! This crate provides functionality to parse unified diffs from LLM output
//! and apply them to files with flexible matching strategies.
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate:
//! - Depends on: nothing (pure Rust implementation)
//! - Used by: qbit-tools (tool system)
//!
//! # Usage
//!
//! ```rust,ignore
//! use qbit_udiff::{UdiffParser, UdiffApplier, ApplyResult};
//!
//! // Parse diff blocks from LLM output
//! let diffs = UdiffParser::parse(llm_output);
//!
//! // Apply hunks to file content
//! let result = UdiffApplier::apply_hunks(&file_content, &diffs[0].hunks);
//!
//! match result {
//!     ApplyResult::Success { new_content } => {
//!         // Write new_content to file
//!     }
//!     ApplyResult::NoMatch { hunk_idx, suggestion } => {
//!         // Report error to LLM
//!     }
//!     _ => {}
//! }
//! ```

mod applier;
mod error;
mod parser;

pub use applier::{ApplyResult, UdiffApplier};
pub use error::{PatchError, PatchErrorType};
pub use parser::{ParsedDiff, ParsedHunk, UdiffParser};
