use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "t")]
pub enum HistoryEntry {
    #[serde(rename = "cmd")]
    Command {
        v: u8,
        ts: u64,
        sid: String,
        c: String,
        exit: i32,
        count: u32,
    },

    #[serde(rename = "prompt")]
    Prompt {
        v: u8,
        ts: u64,
        sid: String,
        c: String,
        model: String,
        provider: String,
        tok_in: u32,
        tok_out: u32,
        ok: bool,
        count: u32,
    },
}

impl HistoryEntry {
    pub fn is_same_dedup_key(&self, other: &HistoryEntry) -> bool {
        match (self, other) {
            (
                HistoryEntry::Command { c, exit, .. },
                HistoryEntry::Command {
                    c: c2, exit: exit2, ..
                },
            ) => c == c2 && exit == exit2,
            (HistoryEntry::Prompt { c, ok, .. }, HistoryEntry::Prompt { c: c2, ok: ok2, .. }) => {
                c == c2 && ok == ok2
            }
            _ => false,
        }
    }

    pub fn bump_count_and_ts(&mut self, new_ts: u64) {
        match self {
            HistoryEntry::Command { ts, count, .. } => {
                *ts = new_ts;
                *count = count.saturating_add(1);
            }
            HistoryEntry::Prompt { ts, count, .. } => {
                *ts = new_ts;
                *count = count.saturating_add(1);
            }
        }
    }
}
