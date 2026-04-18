use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub sql: String,
    pub timestamp: i64,
    pub execution_time_ms: u64,
    pub success: bool,
}

pub struct QueryHistory {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
}

impl QueryHistory {
    pub fn new() -> Self {
        Self {
            entries: vec![],
            max_entries: 500,
        }
    }

    pub fn add(&mut self, sql: String, execution_time_ms: u64, success: bool) {
        let entry = HistoryEntry {
            sql,
            timestamp: chrono::Utc::now().timestamp(),
            execution_time_ms,
            success,
        };
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    #[allow(dead_code)]
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    #[allow(dead_code)]
    pub fn load(connection_id: &str) -> Self {
        let path = Self::history_path(connection_id);
        let entries = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|data| serde_json::from_str(&data).ok())
                .unwrap_or_default()
        } else {
            vec![]
        };
        Self {
            entries,
            max_entries: 500,
        }
    }

    #[allow(dead_code)]
    pub fn save(&self, connection_id: &str) -> Result<(), std::io::Error> {
        let path = Self::history_path(connection_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string(&self.entries)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        fs::write(&path, data)
    }

    #[allow(dead_code)]
    fn history_path(connection_id: &str) -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("querybox")
            .join("history")
            .join(format!("{}.json", connection_id))
    }
}
