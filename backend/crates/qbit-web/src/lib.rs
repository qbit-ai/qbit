//! Web search and content fetching for Qbit AI.
//!
//! This crate provides:
//! - Tavily web search integration
//! - Web content fetching and extraction

pub mod tavily;
pub mod web_fetch;

pub use tavily::TavilyState;
pub use web_fetch::{FetchResult, WebFetcher};
