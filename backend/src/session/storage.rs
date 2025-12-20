//! Session file I/O operations.
//!
//! This module handles reading and writing session files to disk.
//! Sessions are stored as JSON files in `~/.qbit/sessions/` (or `$VT_SESSION_DIR`).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::listing::{SessionListing, SessionSnapshot};

/// Get the sessions directory path.
///
/// Respects the `VT_SESSION_DIR` environment variable for compatibility
/// with vtcode-core's session_archive module.
///
/// Default: `~/.qbit/sessions/`
pub fn get_sessions_dir() -> Result<PathBuf> {
    let dir = if let Ok(custom) = std::env::var("VT_SESSION_DIR") {
        PathBuf::from(custom)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
            .join(".qbit")
            .join("sessions")
    };

    // Ensure directory exists
    fs::create_dir_all(&dir).context("Failed to create sessions directory")?;

    Ok(dir)
}

/// Generate a filename for a session based on its metadata.
///
/// Format: `session-{workspace_label}-{timestamp}_{session_id_prefix}.json`
///
/// Example: `session-my-project-20251214T084335Z_012542-99688.json`
pub fn generate_filename(
    workspace_label: &str,
    started_at: &chrono::DateTime<chrono::Utc>,
    session_id: &str,
) -> String {
    // Format timestamp as ISO 8601 compact
    let timestamp = started_at.format("%Y%m%dT%H%M%SZ_%f");

    // Use first 5 chars of session ID
    let id_prefix = &session_id[..session_id.len().min(5)];

    format!(
        "session-{}-{}-{}.json",
        workspace_label, timestamp, id_prefix
    )
}

/// Save a session snapshot to disk.
///
/// Returns the path to the saved file.
pub fn save_session(dir: &std::path::Path, snapshot: &SessionSnapshot) -> Result<PathBuf> {
    let filename = generate_filename(
        &snapshot.metadata.workspace_label,
        &snapshot.started_at,
        &snapshot.metadata.session_id,
    );
    let path = dir.join(&filename);

    let json =
        serde_json::to_string_pretty(snapshot).context("Failed to serialize session snapshot")?;

    fs::write(&path, json).context("Failed to write session file")?;

    Ok(path)
}

/// Find a session by its identifier.
///
/// The identifier can be:
/// - A session ID (or prefix thereof)
/// - Part of the filename
///
/// Returns the first matching session.
pub fn find_session(identifier: &str) -> Result<Option<SessionListing>> {
    let dir = get_sessions_dir()?;

    // Collect all session files
    let entries: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();

    for entry in entries {
        let path = entry.path();

        // Try to match by filename first (quick check)
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        let matches_filename = filename.contains(identifier);

        if matches_filename {
            // Load and verify the session
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(snapshot) = serde_json::from_str::<SessionSnapshot>(&content) {
                    // Double-check session_id match
                    if snapshot.metadata.session_id.starts_with(identifier)
                        || filename.contains(identifier)
                    {
                        return Ok(Some(SessionListing::from_snapshot(snapshot, path)));
                    }
                }
            }
        } else {
            // Check by loading the file (slower but more thorough)
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(snapshot) = serde_json::from_str::<SessionSnapshot>(&content) {
                    if snapshot.metadata.session_id.starts_with(identifier) {
                        return Ok(Some(SessionListing::from_snapshot(snapshot, path)));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// List all sessions, sorted by start time (most recent first).
///
/// # Arguments
/// * `limit` - Maximum number of sessions to return. Pass 0 for unlimited.
pub fn list_sessions(limit: usize) -> Result<Vec<SessionListing>> {
    let dir = get_sessions_dir()?;
    let mut sessions = Vec::new();

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process JSON files
        if !path.extension().map(|ext| ext == "json").unwrap_or(false) {
            continue;
        }

        // Try to load the session
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(snapshot) = serde_json::from_str::<SessionSnapshot>(&content) {
                sessions.push(SessionListing::from_snapshot(snapshot, path));
            }
        }
    }

    // Sort by started_at descending (most recent first)
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    // Apply limit
    if limit > 0 && sessions.len() > limit {
        sessions.truncate(limit);
    }

    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::archive::SessionArchiveMetadata;
    use crate::session::message::SessionMessage;
    use chrono::Utc;
    use serial_test::serial;
    use tempfile::TempDir;

    fn create_test_snapshot(workspace: &str, session_id: &str) -> SessionSnapshot {
        SessionSnapshot {
            metadata: SessionArchiveMetadata {
                session_id: session_id.to_string(),
                workspace_label: workspace.to_string(),
                workspace_path: format!("/tmp/{}", workspace),
                model: "test-model".to_string(),
                provider: "test-provider".to_string(),
                theme: "default".to_string(),
                reasoning_effort: "standard".to_string(),
            },
            started_at: Utc::now(),
            ended_at: Utc::now(),
            total_messages: 2,
            distinct_tools: vec![],
            transcript: vec!["User: Hello".to_string(), "Assistant: Hi".to_string()],
            messages: vec![
                SessionMessage::user("Hello"),
                SessionMessage::assistant("Hi"),
            ],
        }
    }

    // ==========================================================================
    // get_sessions_dir Tests
    // ==========================================================================

    mod sessions_dir {
        use super::*;

        #[test]
        #[serial]
        fn returns_custom_dir_from_env() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            let dir = get_sessions_dir().unwrap();
            assert_eq!(dir, temp.path());

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn creates_directory_if_not_exists() {
            let temp = TempDir::new().unwrap();
            let nested = temp.path().join("nested").join("sessions");
            std::env::set_var("VT_SESSION_DIR", &nested);

            let dir = get_sessions_dir().unwrap();
            assert!(dir.exists());

            std::env::remove_var("VT_SESSION_DIR");
        }
    }

    // ==========================================================================
    // generate_filename Tests
    // ==========================================================================

    mod filename {
        use super::*;
        use chrono::TimeZone;

        #[test]
        fn generates_correct_format() {
            let timestamp = chrono::Utc
                .with_ymd_and_hms(2025, 12, 14, 8, 43, 35)
                .unwrap();
            let filename = generate_filename("my-project", &timestamp, "abc123def456");

            assert!(filename.starts_with("session-my-project-20251214T"));
            assert!(filename.ends_with(".json"));
            assert!(filename.contains("abc12")); // First 5 chars of session_id
        }

        #[test]
        fn handles_short_session_id() {
            let timestamp = Utc::now();
            let filename = generate_filename("test", &timestamp, "ab");

            assert!(filename.contains("ab")); // Should use full ID if short
        }
    }

    // ==========================================================================
    // save_session Tests
    // ==========================================================================

    mod save {
        use super::*;

        #[test]
        #[serial]
        fn saves_session_to_disk() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            let snapshot = create_test_snapshot("test-workspace", "session123456");
            let path = save_session(&temp.path().to_path_buf(), &snapshot).unwrap();

            assert!(path.exists());

            // Verify file content
            let content = fs::read_to_string(&path).unwrap();
            assert!(content.contains("test-workspace"));
            assert!(content.contains("test-model"));

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn creates_valid_json() {
            let temp = TempDir::new().unwrap();
            let snapshot = create_test_snapshot("json-test", "jsonid12345");
            let path = save_session(&temp.path().to_path_buf(), &snapshot).unwrap();

            let content = fs::read_to_string(&path).unwrap();
            let restored: SessionSnapshot = serde_json::from_str(&content).unwrap();

            assert_eq!(restored.metadata.workspace_label, "json-test");
            assert_eq!(restored.metadata.session_id, "jsonid12345");
            assert_eq!(restored.messages.len(), 2);
        }
    }

    // ==========================================================================
    // find_session Tests
    // ==========================================================================

    mod find {
        use super::*;

        #[test]
        #[serial]
        fn finds_by_session_id_prefix() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            let snapshot = create_test_snapshot("find-test", "unique123456789");
            save_session(&temp.path().to_path_buf(), &snapshot).unwrap();

            let found = find_session("unique123").unwrap();
            assert!(found.is_some());
            assert_eq!(
                found.unwrap().snapshot.metadata.session_id,
                "unique123456789"
            );

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn returns_none_for_nonexistent() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            let found = find_session("nonexistent").unwrap();
            assert!(found.is_none());

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn finds_by_filename_content() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            let snapshot = create_test_snapshot("myproject", "projid12345");
            save_session(&temp.path().to_path_buf(), &snapshot).unwrap();

            // Should find by workspace label in filename
            let found = find_session("myproject").unwrap();
            assert!(found.is_some());

            std::env::remove_var("VT_SESSION_DIR");
        }
    }

    // ==========================================================================
    // list_sessions Tests
    // ==========================================================================

    mod list {
        use super::*;
        use std::thread;
        use std::time::Duration;

        #[test]
        #[serial]
        fn returns_empty_for_empty_dir() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            let sessions = list_sessions(10).unwrap();
            assert!(sessions.is_empty());

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn returns_all_sessions() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create multiple sessions
            for i in 0..3 {
                let snapshot =
                    create_test_snapshot(&format!("workspace-{}", i), &format!("id{}", i));
                save_session(&temp.path().to_path_buf(), &snapshot).unwrap();
                thread::sleep(Duration::from_millis(10)); // Ensure different timestamps
            }

            let sessions = list_sessions(0).unwrap();
            assert_eq!(sessions.len(), 3);

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn respects_limit() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create 5 sessions
            for i in 0..5 {
                let snapshot = create_test_snapshot(&format!("limit-{}", i), &format!("lid{}", i));
                save_session(&temp.path().to_path_buf(), &snapshot).unwrap();
            }

            let sessions = list_sessions(2).unwrap();
            assert_eq!(sessions.len(), 2);

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn sorts_by_date_descending() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create sessions with different timestamps
            for i in 0..3 {
                let mut snapshot =
                    create_test_snapshot(&format!("sort-{}", i), &format!("sid{}", i));
                // Add delay to ensure different timestamps
                thread::sleep(Duration::from_millis(50));
                snapshot.started_at = Utc::now();
                save_session(&temp.path().to_path_buf(), &snapshot).unwrap();
            }

            let sessions = list_sessions(0).unwrap();

            // Verify descending order
            for i in 0..sessions.len() - 1 {
                assert!(sessions[i].started_at >= sessions[i + 1].started_at);
            }

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn ignores_non_json_files() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create a valid session
            let snapshot = create_test_snapshot("valid", "validid");
            save_session(&temp.path().to_path_buf(), &snapshot).unwrap();

            // Create non-JSON files
            fs::write(temp.path().join("readme.txt"), "not a session").unwrap();
            fs::write(temp.path().join("data.csv"), "a,b,c").unwrap();

            let sessions = list_sessions(0).unwrap();
            assert_eq!(sessions.len(), 1);

            std::env::remove_var("VT_SESSION_DIR");
        }

        #[test]
        #[serial]
        fn ignores_invalid_json() {
            let temp = TempDir::new().unwrap();
            std::env::set_var("VT_SESSION_DIR", temp.path());

            // Create a valid session
            let snapshot = create_test_snapshot("valid", "validid2");
            save_session(&temp.path().to_path_buf(), &snapshot).unwrap();

            // Create invalid JSON file
            fs::write(temp.path().join("invalid.json"), "{ not valid json }").unwrap();

            let sessions = list_sessions(0).unwrap();
            assert_eq!(sessions.len(), 1);

            std::env::remove_var("VT_SESSION_DIR");
        }
    }
}
