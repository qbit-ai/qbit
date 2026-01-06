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
pub fn write_frontend_log(
    level: FrontendLogLevel,
    message: String,
    context: Option<String>,
) -> Result<()> {
    // Trace-level logging to avoid noise in backend.log
    tracing::trace!(
        level = %level,
        message_len = message.len(),
        "[frontend-log] Received log request"
    );

    let log_path = match frontend_log_path() {
        Some(p) => p,
        None => {
            tracing::error!("[frontend-log] Could not determine home directory");
            return Err(crate::error::QbitError::Internal(
                "Could not determine home directory".to_string(),
            ));
        }
    };

    // Ensure ~/.qbit directory exists
    if let Some(parent) = log_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            tracing::error!("[frontend-log] Failed to create directory: {}", e);
            return Err(e.into());
        }
    }

    // Format the log line
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = if let Some(ctx) = context {
        format!("[{}] {} [{}] {}\n", timestamp, level, ctx, message)
    } else {
        format!("[{}] {} {}\n", timestamp, level, message)
    };

    // Append to the log file
    let mut file = match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(
                "[frontend-log] Failed to open log file {:?}: {}",
                log_path,
                e
            );
            return Err(e.into());
        }
    };

    if let Err(e) = file.write_all(log_line.as_bytes()) {
        tracing::error!("[frontend-log] Failed to write to log file: {}", e);
        return Err(e.into());
    }

    tracing::trace!("[frontend-log] Successfully wrote log entry");
    Ok(())
}
