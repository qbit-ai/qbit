use crate::{HistoryEntry, Result};
use chrono::Utc;
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub struct Storage {
    dir: PathBuf,
    current_path: PathBuf,
    file: File,
    pub current_size: u64,
    pub last_offset: u64,
    pub last_entry: Option<HistoryEntry>,
}

impl Storage {
    pub fn open(dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&dir)?;
        let current_path = dir.join("current.jsonl");

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&current_path)?;

        let size = file.metadata()?.len();

        let (last_offset, last_entry) = Self::read_last_entry(&current_path)?;

        // Ensure file cursor is at end for appends.
        file.seek(SeekFrom::End(0))?;

        Ok(Self {
            dir,
            current_path,
            file,
            current_size: size,
            last_offset,
            last_entry,
        })
    }

    fn read_last_entry(path: &Path) -> Result<(u64, Option<HistoryEntry>)> {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((0, None)),
            Err(e) => return Err(e.into()),
        };

        if bytes.is_empty() {
            return Ok((0, None));
        }

        // Find end of last non-empty line.
        let mut end = bytes.len();
        while end > 0 && (bytes[end - 1] == b'\n' || bytes[end - 1] == b'\r') {
            end -= 1;
        }
        if end == 0 {
            return Ok((0, None));
        }

        // Find start of last line.
        let mut start = end;
        while start > 0 && bytes[start - 1] != b'\n' {
            start -= 1;
        }

        let line = &bytes[start..end];
        let parsed = serde_json::from_slice::<HistoryEntry>(line).ok();

        Ok((start as u64, parsed))
    }

    pub fn rotate_if_needed(&mut self, max_size_bytes: u64, incoming_len: u64) -> Result<()> {
        if self.current_size.saturating_add(incoming_len) <= max_size_bytes {
            return Ok(());
        }

        // Lock while rotating to avoid multi-process writers stepping on us.
        self.file.lock_exclusive()?;
        let _unlock = UnlockOnDrop(self.file.try_clone()?);

        // Re-check after lock.
        self.current_size = self.file.metadata()?.len();
        if self.current_size.saturating_add(incoming_len) <= max_size_bytes {
            return Ok(());
        }

        let ts = Utc::now().timestamp_millis();
        let rotated = self.dir.join(format!("{}.jsonl", ts));

        // Close current file handle by creating a fresh one after rename.
        drop(self.file.try_clone()?);

        // Rename current file.
        fs::rename(&self.current_path, &rotated)?;

        // Open a fresh current file.
        self.file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.current_path)?;

        self.current_size = self.file.metadata()?.len();
        self.last_offset = 0;
        self.last_entry = None;

        Ok(())
    }

    pub fn append_or_replace_last(&mut self, entry: HistoryEntry) -> Result<()> {
        let mut line = serde_json::to_vec(&entry)?;
        line.push(b'\n');

        // Lock for the duration of the write.
        self.file.lock_exclusive()?;
        let _unlock = UnlockOnDrop(self.file.try_clone()?);

        if let Some(mut last) = self.last_entry.clone() {
            if last.is_same_dedup_key(&entry) {
                // Replace last line by truncating from last_offset.
                last.bump_count_and_ts(now_ms());
                let mut new_line = serde_json::to_vec(&last)?;
                new_line.push(b'\n');

                self.file.set_len(self.last_offset)?;
                self.file.seek(SeekFrom::Start(self.last_offset))?;
                self.file.write_all(&new_line)?;
                self.file.flush()?;

                self.current_size = self.file.metadata()?.len();
                self.last_entry = Some(last);
                return Ok(());
            }
        }

        // Append new line.
        let start_offset = self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&line)?;
        self.file.flush()?;

        self.current_size = self.file.metadata()?.len();
        self.last_offset = start_offset;
        self.last_entry = Some(entry);

        Ok(())
    }

    pub fn load_recent(
        dir: &Path,
        limit: usize,
        entry_type: Option<&str>,
    ) -> Result<Vec<HistoryEntry>> {
        let path = dir.join("current.jsonl");
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };

        let reader = BufReader::new(file);
        let mut entries: Vec<HistoryEntry> = Vec::new();

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.trim().is_empty() {
                continue;
            }

            let entry: HistoryEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            if let Some(t) = entry_type {
                if !matches_entry_type(&entry, t) {
                    continue;
                }
            }

            entries.push(entry);
        }

        if entries.len() > limit {
            entries = entries[entries.len() - limit..].to_vec();
        }

        Ok(entries)
    }

    pub fn search(
        dir: &Path,
        query: &str,
        include_archives: bool,
        limit: usize,
        entry_type: Option<&str>,
    ) -> Result<Vec<HistoryEntry>> {
        let q = query.to_lowercase();
        let mut results: Vec<HistoryEntry> = Vec::new();

        let mut files: Vec<PathBuf> = vec![dir.join("current.jsonl")];
        if include_archives {
            if let Ok(read_dir) = fs::read_dir(dir) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.file_name().and_then(|n| n.to_str()) == Some("current.jsonl") {
                        continue;
                    }
                    if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        files.push(path);
                    }
                }
            }
        }

        for path in files {
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let reader = BufReader::new(file);

            for line in reader.lines().map_while(|r| r.ok()) {
                if line.trim().is_empty() {
                    continue;
                }
                let entry: HistoryEntry = match serde_json::from_str(&line) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                if let Some(t) = entry_type {
                    if !matches_entry_type(&entry, t) {
                        continue;
                    }
                }

                let content = match &entry {
                    HistoryEntry::Command { c, .. } => c,
                    HistoryEntry::Prompt { c, .. } => c,
                };

                if content.to_lowercase().contains(&q) {
                    results.push(entry);
                    if results.len() >= limit {
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }

    pub fn clear_all(dir: &Path) -> Result<()> {
        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let _ = fs::remove_file(path);
                }
            }
        }
        Ok(())
    }
}

fn matches_entry_type(entry: &HistoryEntry, t: &str) -> bool {
    matches!(
        (entry, t),
        (HistoryEntry::Command { .. }, "cmd") | (HistoryEntry::Prompt { .. }, "prompt")
    )
}

fn now_ms() -> u64 {
    Utc::now().timestamp_millis() as u64
}

struct UnlockOnDrop(File);

impl Drop for UnlockOnDrop {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
