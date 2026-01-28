use crate::{HistoryEntry, HistoryError, Result, Storage};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct HistoryConfig {
    pub enabled: bool,
    pub max_file_size_mb: u32,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_file_size_mb: 10,
        }
    }
}

pub struct HistoryManager {
    dir: PathBuf,
    config: HistoryConfig,
    storage: Mutex<Storage>,
}

impl HistoryManager {
    pub fn new(config: HistoryConfig) -> Result<Self> {
        let home = dirs::home_dir().ok_or(HistoryError::HomeDirNotFound)?;
        let dir = home.join(".qbit").join("history");
        let storage = Storage::open(dir.clone())?;
        Ok(Self {
            dir,
            config,
            storage: Mutex::new(storage),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn add_command(&self, session_id: String, command: String, exit_code: i32) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        if command.trim().is_empty() {
            return Ok(());
        }

        let entry = HistoryEntry::Command {
            v: 1,
            ts: chrono::Utc::now().timestamp_millis() as u64,
            sid: session_id,
            c: command,
            exit: exit_code,
            count: 1,
        };

        self.add_entry(entry)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_prompt(
        &self,
        session_id: String,
        prompt: String,
        model: String,
        provider: String,
        tokens_in: u32,
        tokens_out: u32,
        success: bool,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        if prompt.trim().is_empty() {
            return Ok(());
        }

        let entry = HistoryEntry::Prompt {
            v: 1,
            ts: chrono::Utc::now().timestamp_millis() as u64,
            sid: session_id,
            c: prompt,
            model,
            provider,
            tok_in: tokens_in,
            tok_out: tokens_out,
            ok: success,
            count: 1,
        };

        self.add_entry(entry)
    }

    fn add_entry(&self, entry: HistoryEntry) -> Result<()> {
        let max_bytes = self.config.max_file_size_mb as u64 * 1024 * 1024;

        let mut storage = self.storage.lock();

        // Rough estimate for rotation check.
        let incoming_len = serde_json::to_vec(&entry)?.len() as u64 + 1;
        storage.rotate_if_needed(max_bytes, incoming_len)?;

        storage.append_or_replace_last(entry)?;

        Ok(())
    }

    pub fn load_recent(&self, limit: usize, entry_type: Option<&str>) -> Result<Vec<HistoryEntry>> {
        Storage::load_recent(&self.dir, limit, entry_type)
    }

    pub fn search(
        &self,
        query: String,
        include_archives: bool,
        limit: usize,
        entry_type: Option<&str>,
    ) -> Result<Vec<HistoryEntry>> {
        Storage::search(&self.dir, &query, include_archives, limit, entry_type)
    }

    pub fn clear_all(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Remove files.
        Storage::clear_all(&self.dir)?;

        // Re-open storage.
        let mut storage = self.storage.lock();
        *storage = Storage::open(self.dir.clone())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn dedups_consecutive_commands_by_replacing_last_line() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path().to_path_buf()).unwrap();
        let mgr = HistoryManager {
            dir: dir.path().to_path_buf(),
            config: HistoryConfig {
                enabled: true,
                max_file_size_mb: 10,
            },
            storage: Mutex::new(storage),
        };

        mgr.add_command("s1".to_string(), "git status".to_string(), 0)
            .unwrap();
        mgr.add_command("s2".to_string(), "git status".to_string(), 0)
            .unwrap();

        let entries = mgr.load_recent(100, Some("cmd")).unwrap();
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            HistoryEntry::Command { c, count, exit, .. } => {
                assert_eq!(c, "git status");
                assert_eq!(*exit, 0);
                assert_eq!(*count, 2);
            }
            _ => panic!("expected cmd"),
        }
    }
}
