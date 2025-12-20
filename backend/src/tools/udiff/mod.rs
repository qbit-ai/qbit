//! Unified diff editing module for surgical multi-hunk code edits.
//!
//! This module provides functionality to parse unified diffs from LLM output
//! and apply them to files with flexible matching strategies.

mod applier;
mod error;
mod parser;

pub use applier::{ApplyResult, UdiffApplier};
pub use error::{PatchError, PatchErrorType};
pub use parser::{ParsedDiff, ParsedHunk, UdiffParser};
