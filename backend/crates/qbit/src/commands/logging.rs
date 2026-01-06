//! Frontend logging commands
//!
//! Provides Tauri commands for the frontend to write logs to ~/.qbit/frontend.log

use crate::error::Result;
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Get the path to the frontend log file
fn frontend_log_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".qbit").join("frontend.log"))
}

/// Log level for frontend logs
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrontendLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for FrontendLogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrontendLogLevel::Debug => write!(f, "DEBUG"),
            FrontendLogLevel::Info => write!(f, "INFO"),
            FrontendLogLevel::Warn => write!(f, "WARN"),
            FrontendLogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Write a log message from the frontend to ~/.qbit/frontend.log
#[tauri::command]
pub async fn write_frontend_log(
    level: FrontendLogLevel,
    message: String,
    context: Option<String>,
) -> Result<()> {
    let log_path = frontend_log_path().ok_or_else(|| {
        crate::error::QbitError::Internal("Could not determine home directory".to_string())
    })?;

    // Ensure ~/.qbit directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Format the log line
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = if let Some(ctx) = context {
        format!("[{}] {} [{}] {}\n", timestamp, level, ctx, message)
    } else {
        format!("[{}] {} {}\n", timestamp, level, message)
    };

    // Append to the log file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    file.write_all(log_line.as_bytes())?;

    Ok(())
}
