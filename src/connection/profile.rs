use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseEngine {
    MySql,
    PostgreSql,
    Sqlite,
}

impl std::fmt::Display for DatabaseEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseEngine::MySql => write!(f, "MySQL"),
            DatabaseEngine::PostgreSql => write!(f, "PostgreSQL"),
            DatabaseEngine::Sqlite => write!(f, "SQLite"),
        }
    }
}

/// A saved connection profile. Passwords are NOT stored here — they go in the OS keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub engine: DatabaseEngine,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub default_database: Option<String>,
    /// For SQLite, the file path.
    pub file_path: Option<String>,
}

impl ConnectionProfile {
    pub fn keyring_key(&self) -> String {
        format!("querybox:{}", self.id)
    }
}
