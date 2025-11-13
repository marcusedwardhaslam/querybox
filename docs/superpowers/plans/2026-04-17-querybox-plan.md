# QueryBox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a native SQL GUI in Rust/GPUI that supports browsing, editing, querying, filtering, and exporting data across MySQL, PostgreSQL, and SQLite.

**Architecture:** Trait-based database abstraction (`DatabaseDriver`) with per-engine implementations. GPUI retained-mode UI with async database I/O dispatched via `cx.spawn()`. Connection profiles persisted to disk, passwords in OS keychain.

**Tech Stack:** Rust, GPUI (git dep from Zed), mysql_async, tokio-postgres, rusqlite, tokio, serde, thiserror, keyring, dirs

**Spec:** `docs/superpowers/specs/2026-04-17-querybox-design.md`

---

## File Map

```
Cargo.toml
src/
├── main.rs                   # Entry point
├── app.rs                    # Root app state, window setup
│
├── db/
│   ├── mod.rs                # DatabaseDriver trait, Dialect enum, re-exports
│   ├── types.rs              # Column, Index, Value, Row, QueryResult, Schema
│   ├── mysql.rs              # MySqlDriver
│   ├── postgres.rs           # PostgresDriver
│   └── sqlite.rs             # SqliteDriver
│
├── connection/
│   ├── mod.rs                # ConnectionManager
│   ├── profile.rs            # ConnectionProfile, DatabaseEngine enum
│   └── storage.rs            # Load/save profiles to JSON, keyring for passwords
│
├── query/
│   ├── mod.rs                # QueryExecutor
│   ├── filter.rs             # Filter struct, FilterOp, filter_to_sql()
│   └── history.rs            # QueryHistory
│
├── export/
│   ├── mod.rs                # Export trait, ExportFormat enum
│   ├── csv.rs                # CsvExporter
│   ├── sql.rs                # SqlExporter
│   └── json.rs               # JsonExporter
│
└── ui/
    ├── mod.rs                # Re-exports
    ├── app_view.rs           # Root view: sidebar + tabs + status bar
    ├── sidebar.rs            # Sidebar: connection info, db selector, table list
    ├── tab_bar.rs            # TabBar, Tab enum (Table/Query)
    ├── table_view.rs         # TableView: data grid, pagination, pending changes
    ├── editor_view.rs        # EditorView: SQL editor, results pane
    ├── filter_panel.rs       # FilterPanel: column filter UI
    ├── schema_view.rs        # SchemaView: table schema display
    └── connection_dialog.rs  # ConnectionDialog: new/edit connection form
```

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "querybox"
version = "0.1.0"
edition = "2021"

[dependencies]
gpui = { git = "https://github.com/zed-industries/zed", rev = "main" }
gpui_platform = { git = "https://github.com/zed-industries/zed", rev = "main" }
mysql_async = "0.34"
tokio-postgres = "0.7"
rusqlite = { version = "0.31", features = ["bundled"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"
csv = "1.3"
keyring = { version = "3", features = ["apple-native", "linux-native"] }
dirs = "5"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 2: Create src/main.rs with a basic GPUI window**

```rust
use gpui::*;
use gpui_platform::application;

struct QueryBoxApp;

impl Render for QueryBoxApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .bg(rgb(0x181825))
            .text_color(rgb(0xcdd6f4))
            .justify_center()
            .items_center()
            .text_xl()
            .child("QueryBox")
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| QueryBoxApp),
        )
        .unwrap();
        cx.activate(true);
    });
}
```

- [ ] **Step 3: Build and verify the window opens**

Run: `cargo build 2>&1 | tail -5`
Expected: Successful build (may take a while first time for GPUI dependencies)

Run: `cargo run`
Expected: A window opens showing "QueryBox" centered on a dark background. Close it manually.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "feat: project scaffolding with basic GPUI window"
```

---

### Task 2: Database Types

**Files:**
- Create: `src/db/mod.rs`
- Create: `src/db/types.rs`

- [ ] **Step 1: Create src/db/types.rs with core data types**

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A single value from a database cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    DateTime(NaiveDateTime),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Bytes(b) => write!(f, "<{} bytes>", b.len()),
            Value::DateTime(dt) => write!(f, "{}", dt),
        }
    }
}

/// Metadata about a single column in a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub extra: String, // e.g. "auto_increment"
}

/// An index on a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

/// A single row of query results.
pub type Row = Vec<Value>;

/// The result of a query execution.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub affected_rows: u64,
    pub execution_time_ms: u64,
}

impl QueryResult {
    pub fn empty() -> Self {
        Self {
            columns: vec![],
            rows: vec![],
            affected_rows: 0,
            execution_time_ms: 0,
        }
    }
}

/// SQL dialect for quoting identifiers and generating SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    MySql,
    PostgreSql,
    Sqlite,
}

impl Dialect {
    /// Quote a table or column identifier for this dialect.
    pub fn quote_identifier(&self, name: &str) -> String {
        match self {
            Dialect::MySql => format!("`{}`", name.replace('`', "``")),
            Dialect::PostgreSql => format!("\"{}\"", name.replace('"', "\"\"")),
            Dialect::Sqlite => format!("\"{}\"", name.replace('"', "\"\"")),
        }
    }
}
```

- [ ] **Step 2: Create src/db/mod.rs with the DatabaseDriver trait**

```rust
pub mod types;

use async_trait::async_trait;
use thiserror::Error;
use types::*;

use crate::connection::profile::ConnectionProfile;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Query failed: {0}")]
    Query(String),
    #[error("Not connected")]
    NotConnected,
    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    async fn connect(profile: &ConnectionProfile) -> Result<Self, DbError>
    where
        Self: Sized;

    async fn disconnect(&self) -> Result<(), DbError>;

    async fn databases(&self) -> Result<Vec<String>, DbError>;

    async fn tables(&self, database: &str) -> Result<Vec<String>, DbError>;

    async fn columns(&self, database: &str, table: &str) -> Result<Vec<Column>, DbError>;

    async fn indexes(&self, database: &str, table: &str) -> Result<Vec<Index>, DbError>;

    async fn query(&self, sql: &str, params: &[Value]) -> Result<QueryResult, DbError>;

    async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, DbError>;

    fn dialect(&self) -> Dialect;
}
```

- [ ] **Step 3: Wire up the module in main.rs**

Add to top of `src/main.rs`:
```rust
mod db;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check 2>&1 | tail -5`
Expected: Compilation error about missing `connection::profile` module — that's expected and will be fixed in Task 3.

- [ ] **Step 5: Commit**

```bash
git add src/db/
git commit -m "feat: database types and DatabaseDriver trait"
```

---

### Task 3: Connection Profiles

**Files:**
- Create: `src/connection/mod.rs`
- Create: `src/connection/profile.rs`
- Create: `src/connection/storage.rs`

- [ ] **Step 1: Create src/connection/profile.rs**

```rust
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
```

- [ ] **Step 2: Create src/connection/storage.rs**

```rust
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use super::profile::ConnectionProfile;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Keyring error: {0}")]
    Keyring(String),
}

fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("querybox")
}

fn profiles_path() -> PathBuf {
    config_dir().join("connections.json")
}

pub fn load_profiles() -> Result<Vec<ConnectionProfile>, StorageError> {
    let path = profiles_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = fs::read_to_string(&path)?;
    let profiles: Vec<ConnectionProfile> = serde_json::from_str(&data)?;
    Ok(profiles)
}

pub fn save_profiles(profiles: &[ConnectionProfile]) -> Result<(), StorageError> {
    let path = profiles_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(profiles)?;
    fs::write(&path, data)?;
    Ok(())
}

pub fn store_password(profile: &ConnectionProfile, password: &str) -> Result<(), StorageError> {
    let entry = keyring::Entry::new("querybox", &profile.keyring_key())
        .map_err(|e| StorageError::Keyring(e.to_string()))?;
    entry
        .set_password(password)
        .map_err(|e| StorageError::Keyring(e.to_string()))?;
    Ok(())
}

pub fn get_password(profile: &ConnectionProfile) -> Result<Option<String>, StorageError> {
    let entry = keyring::Entry::new("querybox", &profile.keyring_key())
        .map_err(|e| StorageError::Keyring(e.to_string()))?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(StorageError::Keyring(e.to_string())),
    }
}
```

- [ ] **Step 3: Create src/connection/mod.rs**

```rust
pub mod profile;
pub mod storage;

use crate::db::{DatabaseDriver, DbError};
use profile::ConnectionProfile;

/// Manages saved connection profiles and the active connection.
pub struct ConnectionManager {
    pub profiles: Vec<ConnectionProfile>,
    active_driver: Option<Box<dyn DatabaseDriver>>,
    pub active_profile: Option<ConnectionProfile>,
    pub active_database: Option<String>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let profiles = storage::load_profiles().unwrap_or_default();
        Self {
            profiles,
            active_driver: None,
            active_profile: None,
            active_database: None,
        }
    }

    pub fn driver(&self) -> Option<&dyn DatabaseDriver> {
        self.active_driver.as_deref()
    }

    pub fn set_active_driver(
        &mut self,
        driver: Box<dyn DatabaseDriver>,
        profile: ConnectionProfile,
    ) {
        self.active_driver = Some(driver);
        self.active_database = profile.default_database.clone();
        self.active_profile = Some(profile);
    }

    pub async fn disconnect(&mut self) -> Result<(), DbError> {
        if let Some(driver) = self.active_driver.take() {
            driver.disconnect().await?;
        }
        self.active_profile = None;
        self.active_database = None;
        Ok(())
    }

    pub fn save(&self) -> Result<(), storage::StorageError> {
        storage::save_profiles(&self.profiles)
    }
}
```

- [ ] **Step 4: Wire up in main.rs**

Add to `src/main.rs`:
```rust
mod connection;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation (or warnings only).

- [ ] **Step 6: Commit**

```bash
git add src/connection/
git commit -m "feat: connection profiles and storage"
```

---

### Task 4: MySQL Driver

**Files:**
- Create: `src/db/mysql.rs`
- Modify: `src/db/mod.rs`

- [ ] **Step 1: Create src/db/mysql.rs**

```rust
use async_trait::async_trait;
use mysql_async::prelude::*;
use mysql_async::{Conn, Opts, OptsBuilder, Pool};
use std::time::Instant;

use super::types::*;
use super::{DatabaseDriver, DbError};
use crate::connection::profile::ConnectionProfile;

pub struct MySqlDriver {
    pool: Pool,
}

impl MySqlDriver {
    fn opts_from_profile(profile: &ConnectionProfile, password: &str) -> Opts {
        OptsBuilder::default()
            .ip_or_hostname(&profile.host)
            .tcp_port(profile.port)
            .user(Some(&profile.user))
            .pass(Some(password))
            .db_name(profile.default_database.as_deref())
            .into()
    }

    async fn get_conn(&self) -> Result<Conn, DbError> {
        self.pool
            .get_conn()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))
    }
}

#[async_trait]
impl DatabaseDriver for MySqlDriver {
    async fn connect(profile: &ConnectionProfile) -> Result<Self, DbError> {
        let password = crate::connection::storage::get_password(profile)
            .map_err(|e| DbError::Connection(e.to_string()))?
            .unwrap_or_default();
        let opts = Self::opts_from_profile(profile, &password);
        let pool = Pool::new(opts);
        // Test the connection
        let _conn = pool
            .get_conn()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;
        Ok(Self { pool })
    }

    async fn disconnect(&self) -> Result<(), DbError> {
        self.pool.clone().disconnect().await.map_err(|e| DbError::Other(e.to_string()))
    }

    async fn databases(&self) -> Result<Vec<String>, DbError> {
        let mut conn = self.get_conn().await?;
        let rows: Vec<String> = conn
            .query("SHOW DATABASES")
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows)
    }

    async fn tables(&self, database: &str) -> Result<Vec<String>, DbError> {
        let mut conn = self.get_conn().await?;
        let query = format!(
            "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = ? ORDER BY TABLE_NAME",
        );
        let rows: Vec<String> = conn
            .exec(&query, (database,))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows)
    }

    async fn columns(&self, database: &str, table: &str) -> Result<Vec<Column>, DbError> {
        let mut conn = self.get_conn().await?;
        let query = "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_DEFAULT, COLUMN_KEY, EXTRA \
                     FROM INFORMATION_SCHEMA.COLUMNS \
                     WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                     ORDER BY ORDINAL_POSITION";
        let rows: Vec<(String, String, String, Option<String>, String, String)> = conn
            .exec(query, (database, table))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|(name, data_type, nullable, default_value, key, extra)| Column {
                name,
                data_type,
                nullable: nullable == "YES",
                default_value,
                is_primary_key: key == "PRI",
                extra,
            })
            .collect())
    }

    async fn indexes(&self, database: &str, table: &str) -> Result<Vec<Index>, DbError> {
        let mut conn = self.get_conn().await?;
        let query = "SELECT INDEX_NAME, COLUMN_NAME, NON_UNIQUE \
                     FROM INFORMATION_SCHEMA.STATISTICS \
                     WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                     ORDER BY INDEX_NAME, SEQ_IN_INDEX";
        let rows: Vec<(String, String, i32)> = conn
            .exec(query, (database, table))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let mut indexes: Vec<Index> = vec![];
        for (name, column, non_unique) in rows {
            if let Some(idx) = indexes.iter_mut().find(|i| i.name == name) {
                idx.columns.push(column);
            } else {
                indexes.push(Index {
                    name,
                    columns: vec![column],
                    unique: non_unique == 0,
                });
            }
        }
        Ok(indexes)
    }

    async fn query(&self, sql: &str, params: &[Value]) -> Result<QueryResult, DbError> {
        let mut conn = self.get_conn().await?;
        let start = Instant::now();

        let mysql_params = values_to_mysql_params(params);
        let result: Vec<mysql_async::Row> = conn
            .exec(sql, mysql_params)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        if result.is_empty() {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: 0,
                execution_time_ms,
            });
        }

        let columns: Vec<Column> = result[0]
            .columns()
            .iter()
            .map(|c| Column {
                name: c.name_str().to_string(),
                data_type: format!("{:?}", c.column_type()),
                nullable: true,
                default_value: None,
                is_primary_key: false,
                extra: String::new(),
            })
            .collect();

        let rows: Vec<Row> = result.iter().map(mysql_row_to_values).collect();

        Ok(QueryResult {
            columns,
            rows,
            affected_rows: 0,
            execution_time_ms,
        })
    }

    async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, DbError> {
        let mut conn = self.get_conn().await?;
        let mysql_params = values_to_mysql_params(params);
        conn.exec_drop(sql, mysql_params)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(conn.affected_rows())
    }

    fn dialect(&self) -> Dialect {
        Dialect::MySql
    }
}

fn values_to_mysql_params(params: &[Value]) -> mysql_async::Params {
    if params.is_empty() {
        return mysql_async::Params::Empty;
    }
    let values: Vec<mysql_async::Value> = params.iter().map(value_to_mysql).collect();
    mysql_async::Params::Positional(values)
}

fn value_to_mysql(v: &Value) -> mysql_async::Value {
    match v {
        Value::Null => mysql_async::Value::NULL,
        Value::Bool(b) => mysql_async::Value::from(*b),
        Value::Int(i) => mysql_async::Value::from(*i),
        Value::Float(f) => mysql_async::Value::from(*f),
        Value::String(s) => mysql_async::Value::from(s.as_str()),
        Value::Bytes(b) => mysql_async::Value::from(b.as_slice()),
        Value::DateTime(dt) => mysql_async::Value::from(dt.format("%Y-%m-%d %H:%M:%S").to_string()),
    }
}

fn mysql_row_to_values(row: &mysql_async::Row) -> Row {
    (0..row.len())
        .map(|i| {
            if let Some(val) = row.as_ref(i) {
                match val {
                    mysql_async::Value::NULL => Value::Null,
                    mysql_async::Value::Int(n) => Value::Int(*n),
                    mysql_async::Value::UInt(n) => Value::Int(*n as i64),
                    mysql_async::Value::Float(f) => Value::Float(*f as f64),
                    mysql_async::Value::Double(f) => Value::Float(*f),
                    mysql_async::Value::Bytes(b) => {
                        match String::from_utf8(b.clone()) {
                            Ok(s) => Value::String(s),
                            Err(_) => Value::Bytes(b.clone()),
                        }
                    }
                    _ => Value::String(format!("{:?}", val)),
                }
            } else {
                Value::Null
            }
        })
        .collect()
}
```

- [ ] **Step 2: Add mysql module to src/db/mod.rs**

Add after the trait definition:
```rust
pub mod mysql;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check 2>&1 | tail -10`
Expected: Clean compilation. If there are mysql_async API mismatches, fix them — the API may have changed between versions. Check the specific version's docs.

- [ ] **Step 4: Test against dev database**

Start the dev database if not running:
```bash
cd dev && docker compose up -d && cd ..
```

Create a quick integration test at the bottom of `src/db/mysql.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::profile::{ConnectionProfile, DatabaseEngine};

    fn test_profile() -> ConnectionProfile {
        ConnectionProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            engine: DatabaseEngine::MySql,
            host: "127.0.0.1".to_string(),
            port: 3306,
            user: "root".to_string(),
            default_database: Some("querybox".to_string()),
            file_path: None,
        }
    }

    #[tokio::test]
    async fn test_connect_and_list_tables() {
        // Note: requires dev MySQL running. Store password for test.
        let profile = test_profile();
        // For testing, we bypass keyring and connect directly.
        let opts = MySqlDriver::opts_from_profile(&profile, "password");
        let pool = Pool::new(opts);
        let driver = MySqlDriver { pool };

        let tables = driver.tables("querybox").await.unwrap();
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));

        driver.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_query() {
        let profile = test_profile();
        let opts = MySqlDriver::opts_from_profile(&profile, "password");
        let pool = Pool::new(opts);
        let driver = MySqlDriver { pool };

        let result = driver.query("SELECT * FROM querybox.users", &[]).await.unwrap();
        assert!(!result.columns.is_empty());
        assert!(result.rows.len() >= 3); // seed data has 3 users

        driver.disconnect().await.unwrap();
    }
}
```

Run: `cargo test db::mysql::tests -- --nocapture 2>&1 | tail -10`
Expected: Both tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/db/
git commit -m "feat: MySQL driver with connect, query, schema introspection"
```

---

### Task 5: Query Execution & Filters

**Files:**
- Create: `src/query/mod.rs`
- Create: `src/query/filter.rs`
- Create: `src/query/history.rs`

- [ ] **Step 1: Create src/query/filter.rs**

```rust
use crate::db::types::{Dialect, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOp {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    IsNull,
    IsNotNull,
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub column: String,
    pub op: FilterOp,
    pub value: Option<String>,
}

/// Convert a list of filters into a WHERE clause and parameter values.
/// Returns (where_clause, params). The where_clause includes "WHERE" if non-empty.
pub fn filters_to_sql(filters: &[Filter], dialect: Dialect) -> (String, Vec<Value>) {
    if filters.is_empty() {
        return (String::new(), vec![]);
    }

    let mut conditions = vec![];
    let mut params = vec![];

    for filter in filters {
        let col = dialect.quote_identifier(&filter.column);
        match filter.op {
            FilterOp::IsNull => {
                conditions.push(format!("{} IS NULL", col));
            }
            FilterOp::IsNotNull => {
                conditions.push(format!("{} IS NOT NULL", col));
            }
            FilterOp::Contains => {
                conditions.push(format!("{} LIKE ?", col));
                let val = filter.value.clone().unwrap_or_default();
                params.push(Value::String(format!("%{}%", val)));
            }
            FilterOp::NotContains => {
                conditions.push(format!("{} NOT LIKE ?", col));
                let val = filter.value.clone().unwrap_or_default();
                params.push(Value::String(format!("%{}%", val)));
            }
            FilterOp::Equals => {
                conditions.push(format!("{} = ?", col));
                params.push(Value::String(filter.value.clone().unwrap_or_default()));
            }
            FilterOp::NotEquals => {
                conditions.push(format!("{} != ?", col));
                params.push(Value::String(filter.value.clone().unwrap_or_default()));
            }
            FilterOp::GreaterThan => {
                conditions.push(format!("{} > ?", col));
                params.push(Value::String(filter.value.clone().unwrap_or_default()));
            }
            FilterOp::LessThan => {
                conditions.push(format!("{} < ?", col));
                params.push(Value::String(filter.value.clone().unwrap_or_default()));
            }
            FilterOp::GreaterOrEqual => {
                conditions.push(format!("{} >= ?", col));
                params.push(Value::String(filter.value.clone().unwrap_or_default()));
            }
            FilterOp::LessOrEqual => {
                conditions.push(format!("{} <= ?", col));
                params.push(Value::String(filter.value.clone().unwrap_or_default()));
            }
        }
    }

    let clause = format!("WHERE {}", conditions.join(" AND "));
    (clause, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filters() {
        let (clause, params) = filters_to_sql(&[], Dialect::MySql);
        assert_eq!(clause, "");
        assert!(params.is_empty());
    }

    #[test]
    fn test_equals_filter_mysql() {
        let filters = vec![Filter {
            column: "username".to_string(),
            op: FilterOp::Equals,
            value: Some("alice".to_string()),
        }];
        let (clause, params) = filters_to_sql(&filters, Dialect::MySql);
        assert_eq!(clause, "WHERE `username` = ?");
        assert_eq!(params, vec![Value::String("alice".to_string())]);
    }

    #[test]
    fn test_contains_filter_postgres() {
        let filters = vec![Filter {
            column: "email".to_string(),
            op: FilterOp::Contains,
            value: Some("example".to_string()),
        }];
        let (clause, params) = filters_to_sql(&filters, Dialect::PostgreSql);
        assert_eq!(clause, "WHERE \"email\" LIKE ?");
        assert_eq!(params, vec![Value::String("%example%".to_string())]);
    }

    #[test]
    fn test_multiple_filters() {
        let filters = vec![
            Filter {
                column: "age".to_string(),
                op: FilterOp::GreaterThan,
                value: Some("18".to_string()),
            },
            Filter {
                column: "status".to_string(),
                op: FilterOp::IsNotNull,
                value: None,
            },
        ];
        let (clause, params) = filters_to_sql(&filters, Dialect::MySql);
        assert_eq!(clause, "WHERE `age` > ? AND `status` IS NOT NULL");
        assert_eq!(params.len(), 1);
    }
}
```

- [ ] **Step 2: Create src/query/history.rs**

```rust
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

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

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

    pub fn save(&self, connection_id: &str) -> Result<(), std::io::Error> {
        let path = Self::history_path(connection_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string(&self.entries).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;
        fs::write(&path, data)
    }

    fn history_path(connection_id: &str) -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("querybox")
            .join("history")
            .join(format!("{}.json", connection_id))
    }
}
```

- [ ] **Step 3: Create src/query/mod.rs**

```rust
pub mod filter;
pub mod history;
```

- [ ] **Step 4: Wire up in main.rs**

Add to `src/main.rs`:
```rust
mod query;
```

- [ ] **Step 5: Run tests**

Run: `cargo test query:: -- --nocapture 2>&1 | tail -10`
Expected: All filter tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/query/
git commit -m "feat: query filters and history"
```

---

### Task 6: Export Module

**Files:**
- Create: `src/export/mod.rs`
- Create: `src/export/csv.rs`
- Create: `src/export/sql.rs`
- Create: `src/export/json.rs`

- [ ] **Step 1: Create src/export/mod.rs**

```rust
pub mod csv_export;
pub mod json_export;
pub mod sql_export;

use crate::db::types::QueryResult;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Sql,
    Json,
}

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    Csv(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn export_to_string(
    result: &QueryResult,
    format: ExportFormat,
    table_name: Option<&str>,
) -> Result<String, ExportError> {
    match format {
        ExportFormat::Csv => csv_export::export(result),
        ExportFormat::Sql => Ok(sql_export::export(result, table_name.unwrap_or("table"))),
        ExportFormat::Json => json_export::export(result),
    }
}
```

- [ ] **Step 2: Create src/export/csv_export.rs**

```rust
use crate::db::types::QueryResult;
use super::ExportError;

pub fn export(result: &QueryResult) -> Result<String, ExportError> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    // Header row
    let headers: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    wtr.write_record(&headers).map_err(|e| ExportError::Csv(e.to_string()))?;

    // Data rows
    for row in &result.rows {
        let fields: Vec<String> = row.iter().map(|v| v.to_string()).collect();
        wtr.write_record(&fields).map_err(|e| ExportError::Csv(e.to_string()))?;
    }

    let bytes = wtr.into_inner().map_err(|e| ExportError::Csv(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| ExportError::Csv(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::{Column, Row, Value};

    fn sample_result() -> QueryResult {
        QueryResult {
            columns: vec![
                Column { name: "id".into(), data_type: "INT".into(), nullable: false, default_value: None, is_primary_key: true, extra: String::new() },
                Column { name: "name".into(), data_type: "VARCHAR".into(), nullable: false, default_value: None, is_primary_key: false, extra: String::new() },
            ],
            rows: vec![
                vec![Value::Int(1), Value::String("alice".into())],
                vec![Value::Int(2), Value::String("bob".into())],
            ],
            affected_rows: 0,
            execution_time_ms: 5,
        }
    }

    #[test]
    fn test_csv_export() {
        let result = export(&sample_result()).unwrap();
        assert!(result.contains("id,name"));
        assert!(result.contains("1,alice"));
        assert!(result.contains("2,bob"));
    }
}
```

- [ ] **Step 3: Create src/export/sql_export.rs**

```rust
use crate::db::types::{QueryResult, Value};

pub fn export(result: &QueryResult, table_name: &str) -> String {
    let mut lines = vec![];
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    let cols_joined = col_names.join(", ");

    for row in &result.rows {
        let values: Vec<String> = row.iter().map(|v| sql_literal(v)).collect();
        lines.push(format!(
            "INSERT INTO {} ({}) VALUES ({});",
            table_name,
            cols_joined,
            values.join(", ")
        ));
    }

    lines.join("\n")
}

fn sql_literal(v: &Value) -> String {
    match v {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Bytes(b) => format!("X'{}'", hex::encode(b)),
        Value::DateTime(dt) => format!("'{}'", dt.format("%Y-%m-%d %H:%M:%S")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::{Column, Value};

    #[test]
    fn test_sql_export() {
        let result = QueryResult {
            columns: vec![
                Column { name: "id".into(), data_type: "INT".into(), nullable: false, default_value: None, is_primary_key: true, extra: String::new() },
                Column { name: "name".into(), data_type: "VARCHAR".into(), nullable: false, default_value: None, is_primary_key: false, extra: String::new() },
            ],
            rows: vec![
                vec![Value::Int(1), Value::String("alice".into())],
            ],
            affected_rows: 0,
            execution_time_ms: 5,
        };
        let output = export(&result, "users");
        assert_eq!(output, "INSERT INTO users (id, name) VALUES (1, 'alice');");
    }

    #[test]
    fn test_sql_escaping() {
        let val = Value::String("O'Brien".into());
        assert_eq!(sql_literal(&val), "'O''Brien'");
    }
}
```

Note: Add `hex = "0.4"` to `Cargo.toml` dependencies for the hex encoding in bytes export. If you prefer to avoid the dependency, replace the Bytes arm with a simple debug format.

- [ ] **Step 4: Create src/export/json_export.rs**

```rust
use crate::db::types::QueryResult;
use super::ExportError;
use serde_json::{json, Map};

pub fn export(result: &QueryResult) -> Result<String, ExportError> {
    let rows: Vec<serde_json::Value> = result
        .rows
        .iter()
        .map(|row| {
            let mut obj = Map::new();
            for (i, val) in row.iter().enumerate() {
                let col_name = result
                    .columns
                    .get(i)
                    .map(|c| c.name.as_str())
                    .unwrap_or("unknown");
                obj.insert(col_name.to_string(), value_to_json(val));
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    serde_json::to_string_pretty(&rows).map_err(ExportError::Json)
}

fn value_to_json(v: &crate::db::types::Value) -> serde_json::Value {
    use crate::db::types::Value;
    match v {
        Value::Null => json!(null),
        Value::Bool(b) => json!(b),
        Value::Int(i) => json!(i),
        Value::Float(f) => json!(f),
        Value::String(s) => json!(s),
        Value::Bytes(b) => json!(format!("<{} bytes>", b.len())),
        Value::DateTime(dt) => json!(dt.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::{Column, Row, Value};

    #[test]
    fn test_json_export() {
        let result = QueryResult {
            columns: vec![
                Column { name: "id".into(), data_type: "INT".into(), nullable: false, default_value: None, is_primary_key: true, extra: String::new() },
                Column { name: "name".into(), data_type: "VARCHAR".into(), nullable: false, default_value: None, is_primary_key: false, extra: String::new() },
            ],
            rows: vec![
                vec![Value::Int(1), Value::String("alice".into())],
            ],
            affected_rows: 0,
            execution_time_ms: 5,
        };
        let output = export(&result).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["id"], 1);
        assert_eq!(parsed[0]["name"], "alice");
    }
}
```

- [ ] **Step 5: Wire up in main.rs and add hex dep**

Add to `src/main.rs`:
```rust
mod export;
```

Add to `Cargo.toml` under `[dependencies]`:
```toml
hex = "0.4"
```

- [ ] **Step 6: Run tests**

Run: `cargo test export:: -- --nocapture 2>&1 | tail -15`
Expected: All export tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/export/ Cargo.toml
git commit -m "feat: CSV, SQL, and JSON export"
```

---

### Task 7: App Layout Shell

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/app_view.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/ui/mod.rs**

```rust
pub mod app_view;
```

- [ ] **Step 2: Create src/ui/app_view.rs**

This is the root GPUI view — sidebar on the left, main content on the right, status bar at the bottom.

```rust
use gpui::*;

use crate::connection::ConnectionManager;

pub struct AppView {
    connection_manager: ConnectionManager,
    status_message: String,
}

impl AppView {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            connection_manager: ConnectionManager::new(),
            status_message: "Disconnected".to_string(),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x181825))
            .text_color(rgb(0xcdd6f4))
            .text_size(px(13.))
            .child(
                // Main area: sidebar + content
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(self.render_sidebar())
                    .child(self.render_main_content()),
            )
            .child(self.render_status_bar())
    }
}

impl AppView {
    fn render_sidebar(&self) -> impl IntoElement {
        div()
            .w(px(220.))
            .flex_shrink_0()
            .bg(rgb(0x1e1e2e))
            .border_r_1()
            .border_color(rgb(0x333333))
            .flex()
            .flex_col()
            .child(
                // Connection header
                div()
                    .p(px(12.))
                    .border_b_1()
                    .border_color(rgb(0x333333))
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .text_size(px(12.))
                            .child("No connection"),
                    ),
            )
            .child(
                // Table list placeholder
                div()
                    .flex_1()
                    .p(px(12.))
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .text_size(px(11.))
                            .child("Connect to see tables"),
                    ),
            )
            .child(
                // New Query button
                div()
                    .p(px(10.))
                    .border_t_1()
                    .border_color(rgb(0x333333))
                    .child(
                        div()
                            .bg(rgb(0x89b4fa))
                            .text_color(rgb(0x1e1e2e))
                            .text_size(px(12.))
                            .rounded(px(4.))
                            .py(px(6.))
                            .flex()
                            .justify_center()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("+ New Query"),
                    ),
            )
    }

    fn render_main_content(&self) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .bg(rgb(0x181825))
            .child(
                // Empty state
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .text_xl()
                            .child("QueryBox"),
                    ),
            )
    }

    fn render_status_bar(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .px(px(12.))
            .py(px(4.))
            .bg(rgb(0x1e1e2e))
            .border_t_1()
            .border_color(rgb(0x333333))
            .text_size(px(11.))
            .text_color(rgb(0x6c7086))
            .child(self.status_message.clone())
    }
}
```

- [ ] **Step 3: Update src/main.rs to use AppView**

Replace the `QueryBoxApp` struct and its Render impl with:

```rust
mod connection;
mod db;
mod export;
mod query;
mod ui;

use gpui::*;
use gpui_platform::application;

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ui::app_view::AppView::new(cx)),
        )
        .unwrap();
        cx.activate(true);
    });
}
```

- [ ] **Step 4: Build and verify**

Run: `cargo run`
Expected: Window opens showing a dark sidebar on the left with "No connection" and "Connect to see tables", a "New Query" button at the bottom, main area showing "QueryBox" centered, and a status bar.

- [ ] **Step 5: Commit**

```bash
git add src/ui/ src/main.rs
git commit -m "feat: app layout shell with sidebar and status bar"
```

---

### Task 8: Connection Dialog

**Files:**
- Create: `src/ui/connection_dialog.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/app_view.rs`

- [ ] **Step 1: Create src/ui/connection_dialog.rs**

```rust
use gpui::*;

use crate::connection::profile::{ConnectionProfile, DatabaseEngine};

pub struct ConnectionDialog {
    pub name: String,
    pub engine: DatabaseEngine,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub file_path: String,
    pub visible: bool,
    focus_handle: FocusHandle,
}

impl ConnectionDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            name: "New Connection".to_string(),
            engine: DatabaseEngine::MySql,
            host: "127.0.0.1".to_string(),
            port: "3306".to_string(),
            user: "root".to_string(),
            password: String::new(),
            database: String::new(),
            file_path: String::new(),
            visible: false,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        cx.notify();
    }

    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.visible = false;
        cx.notify();
    }

    pub fn to_profile(&self) -> ConnectionProfile {
        let id = uuid_v4();
        ConnectionProfile {
            id,
            name: self.name.clone(),
            engine: self.engine,
            host: self.host.clone(),
            port: self.port.parse().unwrap_or(3306),
            user: self.user.clone(),
            default_database: if self.database.is_empty() {
                None
            } else {
                Some(self.database.clone())
            },
            file_path: if self.file_path.is_empty() {
                None
            } else {
                Some(self.file_path.clone())
            },
        }
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

impl Focusable for ConnectionDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ConnectionDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        div()
            .absolute()
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .bg(rgba(0x00000088))
            .child(
                div()
                    .w(px(400.))
                    .bg(rgb(0x1e1e2e))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgb(0x45475a))
                    .p(px(24.))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xcdd6f4))
                            .child("New Connection"),
                    )
                    .child(self.render_field("Name", &self.name.clone()))
                    .child(self.render_field("Host", &self.host.clone()))
                    .child(self.render_field("Port", &self.port.clone()))
                    .child(self.render_field("User", &self.user.clone()))
                    .child(self.render_field("Password", "••••••"))
                    .child(self.render_field("Database", &self.database.clone()))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                div()
                                    .px(px(16.))
                                    .py(px(6.))
                                    .bg(rgb(0x313244))
                                    .rounded(px(4.))
                                    .text_size(px(12.))
                                    .text_color(rgb(0xa6adc8))
                                    .cursor_pointer()
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .px(px(16.))
                                    .py(px(6.))
                                    .bg(rgb(0x89b4fa))
                                    .rounded(px(4.))
                                    .text_size(px(12.))
                                    .text_color(rgb(0x1e1e2e))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .child("Connect"),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl ConnectionDialog {
    fn render_field(&self, label: &str, value: &str) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x6c7086))
                    .child(label.to_string()),
            )
            .child(
                div()
                    .bg(rgb(0x313244))
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(6.))
                    .text_size(px(13.))
                    .text_color(rgb(0xcdd6f4))
                    .child(value.to_string()),
            )
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", t)
}
```

- [ ] **Step 2: Update src/ui/mod.rs**

```rust
pub mod app_view;
pub mod connection_dialog;
```

- [ ] **Step 3: Add connection dialog to AppView**

In `src/ui/app_view.rs`, add the dialog entity as a field and wire up the "New Connection" flow. Update AppView:

Add field:
```rust
use super::connection_dialog::ConnectionDialog;

pub struct AppView {
    connection_manager: ConnectionManager,
    connection_dialog: Entity<ConnectionDialog>,
    status_message: String,
}
```

Update `AppView::new()`:
```rust
pub fn new(cx: &mut Context<Self>) -> Self {
    let connection_dialog = cx.new(|cx| ConnectionDialog::new(cx));
    Self {
        connection_manager: ConnectionManager::new(),
        connection_dialog,
        status_message: "Disconnected".to_string(),
    }
}
```

In the `Render` impl, add the dialog as an overlay child after the main area:
```rust
fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(rgb(0x181825))
        .text_color(rgb(0xcdd6f4))
        .text_size(px(13.))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_1()
                .child(self.render_sidebar())
                .child(self.render_main_content()),
        )
        .child(self.render_status_bar())
        .child(self.connection_dialog.clone())
}
```

- [ ] **Step 4: Build and verify**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation.

- [ ] **Step 5: Commit**

```bash
git add src/ui/
git commit -m "feat: connection dialog UI"
```

---

### Task 9: Sidebar with Table List

**Files:**
- Create: `src/ui/sidebar.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/app_view.rs`

- [ ] **Step 1: Create src/ui/sidebar.rs**

```rust
use gpui::*;

pub struct Sidebar {
    pub connection_name: Option<String>,
    pub engine_info: Option<String>,
    pub databases: Vec<String>,
    pub selected_database: Option<String>,
    pub tables: Vec<String>,
    pub selected_table: Option<String>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            connection_name: None,
            engine_info: None,
            databases: vec![],
            selected_database: None,
            tables: vec![],
            selected_table: None,
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(220.))
            .flex_shrink_0()
            .bg(rgb(0x1e1e2e))
            .border_r_1()
            .border_color(rgb(0x333333))
            .flex()
            .flex_col()
            .child(self.render_connection_header())
            .child(self.render_database_selector())
            .child(self.render_table_list())
            .child(self.render_new_query_button())
    }
}

impl Sidebar {
    fn render_connection_header(&self) -> impl IntoElement {
        let (name, info) = match (&self.connection_name, &self.engine_info) {
            (Some(name), Some(info)) => (name.clone(), info.clone()),
            _ => ("No connection".to_string(), "Click to connect".to_string()),
        };

        div()
            .p(px(12.))
            .border_b_1()
            .border_color(rgb(0x333333))
            .cursor_pointer()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0xcdd6f4))
                    .text_size(px(13.))
                    .child(name),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x6c7086))
                    .mt(px(2.))
                    .child(info),
            )
    }

    fn render_database_selector(&self) -> impl IntoElement {
        let db_name = self
            .selected_database
            .clone()
            .unwrap_or_else(|| "Select database".to_string());

        div()
            .px(px(12.))
            .py(px(8.))
            .child(
                div()
                    .bg(rgb(0x313244))
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(0xa6adc8))
                    .flex()
                    .justify_between()
                    .items_center()
                    .cursor_pointer()
                    .child(db_name)
                    .child(
                        div()
                            .text_size(px(10.))
                            .child("▼"),
                    ),
            )
    }

    fn render_table_list(&self) -> impl IntoElement {
        let mut list = div()
            .flex_1()
            .overflow_y_scroll()
            .py(px(4.))
            .child(
                div()
                    .px(px(12.))
                    .py(px(4.))
                    .text_size(px(10.))
                    .text_color(rgb(0x6c7086))
                    .child("TABLES"),
            );

        if self.tables.is_empty() {
            list = list.child(
                div()
                    .px(px(12.))
                    .py(px(4.))
                    .text_size(px(12.))
                    .text_color(rgb(0x6c7086))
                    .child("No tables"),
            );
        } else {
            for table in &self.tables {
                let is_selected = self.selected_table.as_deref() == Some(table.as_str());
                let mut row = div()
                    .px(px(12.))
                    .pl(px(20.))
                    .py(px(6.))
                    .text_size(px(12.))
                    .cursor_pointer();

                if is_selected {
                    row = row
                        .bg(rgb(0x45475a))
                        .text_color(rgb(0xcdd6f4))
                        .border_l_2()
                        .border_color(rgb(0x89b4fa));
                } else {
                    row = row.text_color(rgb(0xa6adc8));
                }

                list = list.child(row.child(table.clone()));
            }
        }

        list
    }

    fn render_new_query_button(&self) -> impl IntoElement {
        div()
            .p(px(10.))
            .border_t_1()
            .border_color(rgb(0x333333))
            .child(
                div()
                    .bg(rgb(0x89b4fa))
                    .text_color(rgb(0x1e1e2e))
                    .text_size(px(12.))
                    .rounded(px(4.))
                    .py(px(6.))
                    .flex()
                    .justify_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .child("+ New Query"),
            )
    }
}
```

- [ ] **Step 2: Update src/ui/mod.rs**

Add: `pub mod sidebar;`

- [ ] **Step 3: Integrate Sidebar into AppView**

In `src/ui/app_view.rs`, replace the inline `render_sidebar()` method with the Sidebar entity:

Add field to AppView:
```rust
sidebar: Entity<Sidebar>,
```

Initialize in `new()`:
```rust
let sidebar = cx.new(|_| Sidebar::new());
```

Replace `self.render_sidebar()` in render with `self.sidebar.clone()`.

Remove the old `render_sidebar` method from AppView.

- [ ] **Step 4: Build and verify**

Run: `cargo run`
Expected: Window shows the sidebar with "No connection", database selector dropdown, empty table list, and the New Query button.

- [ ] **Step 5: Commit**

```bash
git add src/ui/
git commit -m "feat: sidebar with connection info, database selector, table list"
```

---

### Task 10: Tab System

**Files:**
- Create: `src/ui/tab_bar.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/app_view.rs`

- [ ] **Step 1: Create src/ui/tab_bar.rs**

```rust
use gpui::*;

#[derive(Debug, Clone, PartialEq)]
pub enum TabKind {
    Table { database: String, table: String },
    Query { name: String },
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: usize,
    pub kind: TabKind,
}

impl Tab {
    pub fn label(&self) -> String {
        match &self.kind {
            TabKind::Table { table, .. } => table.clone(),
            TabKind::Query { name } => name.clone(),
        }
    }

    pub fn icon(&self) -> &'static str {
        match &self.kind {
            TabKind::Table { .. } => "📋",
            TabKind::Query { .. } => "⌨️",
        }
    }
}

pub struct TabBar {
    pub tabs: Vec<Tab>,
    pub active_tab: Option<usize>,
    next_id: usize,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            tabs: vec![],
            active_tab: None,
            next_id: 1,
        }
    }

    pub fn open_table(&mut self, database: String, table: String, cx: &mut Context<Self>) -> usize {
        // Check if already open
        for tab in &self.tabs {
            if let TabKind::Table {
                database: d,
                table: t,
            } = &tab.kind
            {
                if d == &database && t == &table {
                    self.active_tab = Some(tab.id);
                    cx.notify();
                    return tab.id;
                }
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            kind: TabKind::Table { database, table },
        });
        self.active_tab = Some(id);
        cx.notify();
        id
    }

    pub fn open_query(&mut self, cx: &mut Context<Self>) -> usize {
        let query_count = self
            .tabs
            .iter()
            .filter(|t| matches!(t.kind, TabKind::Query { .. }))
            .count();
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            kind: TabKind::Query {
                name: format!("Query {}", query_count + 1),
            },
        });
        self.active_tab = Some(id);
        cx.notify();
        id
    }

    pub fn close_tab(&mut self, tab_id: usize, cx: &mut Context<Self>) {
        self.tabs.retain(|t| t.id != tab_id);
        if self.active_tab == Some(tab_id) {
            self.active_tab = self.tabs.last().map(|t| t.id);
        }
        cx.notify();
    }

    pub fn set_active(&mut self, tab_id: usize, cx: &mut Context<Self>) {
        self.active_tab = Some(tab_id);
        cx.notify();
    }
}

impl Render for TabBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut bar = div()
            .flex()
            .flex_row()
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x333333))
            .h(px(36.));

        for tab in &self.tabs {
            let is_active = self.active_tab == Some(tab.id);
            let label = format!("{} {}", tab.icon(), tab.label());

            let mut tab_el = div()
                .px(px(16.))
                .flex()
                .items_center()
                .text_size(px(12.))
                .border_r_1()
                .border_color(rgb(0x333333))
                .cursor_pointer();

            if is_active {
                tab_el = tab_el
                    .bg(rgb(0x181825))
                    .text_color(rgb(0xcdd6f4))
                    .border_t_2()
                    .border_color(rgb(0x89b4fa));
            } else {
                tab_el = tab_el.text_color(rgb(0x6c7086));
            }

            bar = bar.child(tab_el.child(label));
        }

        bar
    }
}
```

- [ ] **Step 2: Update src/ui/mod.rs**

Add: `pub mod tab_bar;`

- [ ] **Step 3: Integrate TabBar into AppView**

Add `tab_bar: Entity<TabBar>` field to AppView, initialize in `new()`, and render it above the main content area.

- [ ] **Step 4: Build and verify**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation.

- [ ] **Step 5: Commit**

```bash
git add src/ui/
git commit -m "feat: tab bar with table and query tabs"
```

---

### Task 11: Table View (Data Grid)

**Files:**
- Create: `src/ui/table_view.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create src/ui/table_view.rs**

```rust
use gpui::*;
use std::collections::HashMap;

use crate::db::types::{Column, QueryResult, Row, Value};

/// A pending cell edit, keyed by (row_index, column_name).
#[derive(Debug, Clone)]
pub struct PendingEdit {
    pub original: Value,
    pub new_value: String,
}

pub struct TableView {
    pub database: String,
    pub table_name: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub total_rows: Option<u64>,
    pub page: usize,
    pub page_size: usize,
    pub pending_changes: HashMap<(usize, String), PendingEdit>,
    pub editing_cell: Option<(usize, usize)>, // (row_idx, col_idx)
    pub loading: bool,
    pub error: Option<String>,
}

impl TableView {
    pub fn new(database: String, table_name: String) -> Self {
        Self {
            database,
            table_name,
            columns: vec![],
            rows: vec![],
            total_rows: None,
            page: 0,
            page_size: 50,
            pending_changes: HashMap::new(),
            editing_cell: None,
            loading: true,
            error: None,
        }
    }

    pub fn set_data(&mut self, result: QueryResult, cx: &mut Context<Self>) {
        self.columns = result.columns;
        self.rows = result.rows;
        self.loading = false;
        self.error = None;
        cx.notify();
    }

    pub fn set_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        self.loading = false;
        cx.notify();
    }

    pub fn has_pending_changes(&self) -> bool {
        !self.pending_changes.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_changes.len()
    }

    pub fn offset(&self) -> usize {
        self.page * self.page_size
    }
}

impl Render for TableView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_toolbar())
            .child(self.render_grid())
    }
}

impl TableView {
    fn render_toolbar(&self) -> impl IntoElement {
        let page_info = if let Some(total) = self.total_rows {
            let start = self.offset() + 1;
            let end = (self.offset() + self.rows.len()).min(total as usize);
            format!("{}-{} of {}", start, end, total)
        } else {
            format!("{} rows", self.rows.len())
        };

        let pending_text = if self.has_pending_changes() {
            format!("{} pending", self.pending_count())
        } else {
            String::new()
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .px(px(12.))
            .py(px(8.))
            .gap_2()
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x333333))
            .child(
                div()
                    .bg(rgb(0x313244))
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(rgb(0xa6adc8))
                    .cursor_pointer()
                    .child("+ Filter"),
            )
            .child(
                div()
                    .bg(rgb(0x313244))
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(rgb(0xa6adc8))
                    .cursor_pointer()
                    .child("Sort"),
            )
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x6c7086))
                    .child(page_info),
            )
            .when(!pending_text.is_empty(), |el| {
                el.child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(0xf9e2af))
                        .child(pending_text),
                )
            })
            .child(
                div()
                    .bg(rgb(0x313244))
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(rgb(0xa6adc8))
                    .cursor_pointer()
                    .child("Export ▼"),
            )
    }

    fn render_grid(&self) -> impl IntoElement {
        if self.loading {
            return div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .child(
                    div()
                        .text_color(rgb(0x6c7086))
                        .child("Loading..."),
                )
                .into_any_element();
        }

        if let Some(err) = &self.error {
            return div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .child(
                    div()
                        .text_color(rgb(0xf38ba8))
                        .child(err.clone()),
                )
                .into_any_element();
        }

        let mut table = div().flex().flex_col().flex_1().overflow_y_scroll();

        // Header row
        let mut header = div()
            .flex()
            .flex_row()
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x333333));

        for col in &self.columns {
            header = header.child(
                div()
                    .w(px(150.))
                    .flex_shrink_0()
                    .px(px(12.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x89b4fa))
                    .child(col.name.clone()),
            );
        }
        table = table.child(header);

        // Data rows
        for (row_idx, row) in self.rows.iter().enumerate() {
            let bg = if row_idx % 2 == 0 {
                rgb(0x181825)
            } else {
                rgb(0x1e1e2e)
            };

            let mut row_el = div()
                .flex()
                .flex_row()
                .bg(bg)
                .border_b_1()
                .border_color(rgb(0x222222));

            for (col_idx, val) in row.iter().enumerate() {
                let col_name = self
                    .columns
                    .get(col_idx)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                let is_dirty = self
                    .pending_changes
                    .contains_key(&(row_idx, col_name.to_string()));

                let color = match val {
                    Value::Null => rgb(0x6c7086),
                    Value::Int(_) | Value::Float(_) => rgb(0xfab387),
                    Value::Bool(_) => rgb(0xcba6f7),
                    Value::DateTime(_) => rgb(0x6c7086),
                    _ => rgb(0xcdd6f4),
                };

                let display = val.to_string();

                let mut cell = div()
                    .w(px(150.))
                    .flex_shrink_0()
                    .px(px(12.))
                    .py(px(6.))
                    .text_size(px(12.))
                    .text_color(color)
                    .cursor_pointer();

                if is_dirty {
                    cell = cell
                        .bg(rgba(0xf9e2af18))
                        .border_1()
                        .border_color(rgb(0xf9e2af));
                }

                row_el = row_el.child(cell.child(display));
            }

            table = table.child(row_el);
        }

        table.into_any_element()
    }
}
```

- [ ] **Step 2: Update src/ui/mod.rs**

Add: `pub mod table_view;`

- [ ] **Step 3: Build and verify**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add src/ui/
git commit -m "feat: table view with data grid, toolbar, and pending changes"
```

---

### Task 12: SQL Editor View

**Files:**
- Create: `src/ui/editor_view.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create src/ui/editor_view.rs**

```rust
use gpui::*;

use crate::db::types::QueryResult;
use crate::query::history::{HistoryEntry, QueryHistory};

pub struct EditorView {
    pub sql: String,
    pub result: Option<QueryResult>,
    pub error: Option<String>,
    pub running: bool,
    pub history: QueryHistory,
    focus_handle: FocusHandle,
}

impl EditorView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            sql: String::new(),
            result: None,
            error: None,
            running: false,
            history: QueryHistory::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_result(&mut self, result: QueryResult, cx: &mut Context<Self>) {
        self.history.add(
            self.sql.clone(),
            result.execution_time_ms,
            true,
        );
        self.result = Some(result);
        self.error = None;
        self.running = false;
        cx.notify();
    }

    pub fn set_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.history.add(self.sql.clone(), 0, false);
        self.error = Some(error);
        self.result = None;
        self.running = false;
        cx.notify();
    }
}

impl Focusable for EditorView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_editor_pane())
            .child(self.render_results_pane())
    }
}

impl EditorView {
    fn render_editor_pane(&self) -> impl IntoElement {
        div()
            .h(px(250.))
            .flex()
            .flex_col()
            .border_b_2()
            .border_color(rgb(0x45475a))
            // Editor toolbar
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(12.))
                    .py(px(6.))
                    .gap_2()
                    .bg(rgb(0x1e1e2e))
                    .border_b_1()
                    .border_color(rgb(0x333333))
                    .child(
                        div()
                            .bg(rgb(0xa6e3a1))
                            .text_color(rgb(0x1e1e2e))
                            .rounded(px(4.))
                            .px(px(12.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .child("▶ Run"),
                    )
                    .child(
                        div()
                            .bg(rgb(0x313244))
                            .rounded(px(4.))
                            .px(px(10.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .text_color(rgb(0xa6adc8))
                            .cursor_pointer()
                            .child("▶ Run Selected"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .bg(rgb(0x313244))
                            .rounded(px(4.))
                            .px(px(10.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .text_color(rgb(0xa6adc8))
                            .cursor_pointer()
                            .child("History ▼"),
                    ),
            )
            // SQL text area
            .child(
                div()
                    .flex_1()
                    .p(px(12.))
                    .bg(rgb(0x181825))
                    .overflow_y_scroll()
                    .text_size(px(13.))
                    .child(if self.sql.is_empty() {
                        div()
                            .text_color(rgb(0x6c7086))
                            .child("Write your SQL here...")
                            .into_any_element()
                    } else {
                        div()
                            .text_color(rgb(0xcdd6f4))
                            .child(self.sql.clone())
                            .into_any_element()
                    }),
            )
    }

    fn render_results_pane(&self) -> impl IntoElement {
        let mut pane = div().flex_1().flex().flex_col();

        if self.running {
            return pane
                .justify_center()
                .items_center()
                .child(
                    div()
                        .text_color(rgb(0x6c7086))
                        .child("Running query..."),
                )
                .into_any_element();
        }

        if let Some(err) = &self.error {
            return pane
                .child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(rgb(0x1e1e2e))
                        .border_b_1()
                        .border_color(rgb(0x333333))
                        .text_size(px(11.))
                        .text_color(rgb(0xf38ba8))
                        .child(format!("✗ Error: {}", err)),
                )
                .into_any_element();
        }

        if let Some(result) = &self.result {
            let info = format!(
                "✓ {} rows returned — {}ms",
                result.rows.len(),
                result.execution_time_ms
            );

            // Results toolbar
            pane = pane.child(
                div()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .py(px(6.))
                    .bg(rgb(0x1e1e2e))
                    .border_b_1()
                    .border_color(rgb(0x333333))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0xa6e3a1))
                            .child(info),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .bg(rgb(0x313244))
                            .rounded(px(4.))
                            .px(px(10.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .text_color(rgb(0xa6adc8))
                            .cursor_pointer()
                            .child("Export ▼"),
                    ),
            );

            // Results table (reuse similar grid rendering as TableView)
            let mut grid = div().flex_1().flex().flex_col().overflow_y_scroll();

            // Header
            let mut header = div()
                .flex()
                .flex_row()
                .bg(rgb(0x1e1e2e))
                .border_b_1()
                .border_color(rgb(0x333333));

            for col in &result.columns {
                header = header.child(
                    div()
                        .w(px(150.))
                        .flex_shrink_0()
                        .px(px(12.))
                        .py(px(8.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x89b4fa))
                        .child(col.name.clone()),
                );
            }
            grid = grid.child(header);

            // Rows
            for (row_idx, row) in result.rows.iter().enumerate() {
                let bg = if row_idx % 2 == 0 {
                    rgb(0x181825)
                } else {
                    rgb(0x1e1e2e)
                };

                let mut row_el = div()
                    .flex()
                    .flex_row()
                    .bg(bg)
                    .border_b_1()
                    .border_color(rgb(0x222222));

                for val in row {
                    use crate::db::types::Value;
                    let color = match val {
                        Value::Null => rgb(0x6c7086),
                        Value::Int(_) | Value::Float(_) => rgb(0xfab387),
                        _ => rgb(0xcdd6f4),
                    };
                    row_el = row_el.child(
                        div()
                            .w(px(150.))
                            .flex_shrink_0()
                            .px(px(12.))
                            .py(px(6.))
                            .text_size(px(12.))
                            .text_color(color)
                            .child(val.to_string()),
                    );
                }

                grid = grid.child(row_el);
            }

            pane = pane.child(grid);
        } else {
            pane = pane
                .justify_center()
                .items_center()
                .child(
                    div()
                        .text_color(rgb(0x6c7086))
                        .child("Run a query to see results"),
                );
        }

        pane.into_any_element()
    }
}
```

- [ ] **Step 2: Update src/ui/mod.rs**

Add: `pub mod editor_view;`

- [ ] **Step 3: Build and verify**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add src/ui/
git commit -m "feat: SQL editor view with editor pane and results pane"
```

---

### Task 13: Filter Panel

**Files:**
- Create: `src/ui/filter_panel.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create src/ui/filter_panel.rs**

```rust
use gpui::*;

use crate::query::filter::{Filter, FilterOp};

pub struct FilterPanel {
    pub filters: Vec<Filter>,
    pub available_columns: Vec<String>,
    pub visible: bool,
}

impl FilterPanel {
    pub fn new() -> Self {
        Self {
            filters: vec![],
            available_columns: vec![],
            visible: false,
        }
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        cx.notify();
    }

    pub fn add_filter(&mut self, column: String, op: FilterOp, value: Option<String>, cx: &mut Context<Self>) {
        self.filters.push(Filter { column, op, value });
        cx.notify();
    }

    pub fn remove_filter(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.filters.len() {
            self.filters.remove(index);
            cx.notify();
        }
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.filters.clear();
        cx.notify();
    }
}

impl Render for FilterPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible || self.filters.is_empty() {
            return div().into_any_element();
        }

        let mut panel = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_2()
            .px(px(12.))
            .py(px(6.))
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x333333));

        for (i, filter) in self.filters.iter().enumerate() {
            let op_str = match filter.op {
                FilterOp::Equals => "=",
                FilterOp::NotEquals => "≠",
                FilterOp::Contains => "contains",
                FilterOp::NotContains => "!contains",
                FilterOp::GreaterThan => ">",
                FilterOp::LessThan => "<",
                FilterOp::GreaterOrEqual => "≥",
                FilterOp::LessOrEqual => "≤",
                FilterOp::IsNull => "IS NULL",
                FilterOp::IsNotNull => "IS NOT NULL",
            };

            let label = if let Some(val) = &filter.value {
                format!("{} {} {}", filter.column, op_str, val)
            } else {
                format!("{} {}", filter.column, op_str)
            };

            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .bg(rgb(0x313244))
                    .rounded(px(4.))
                    .px(px(8.))
                    .py(px(3.))
                    .text_size(px(11.))
                    .text_color(rgb(0xa6adc8))
                    .child(label)
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .cursor_pointer()
                            .child("✕"),
                    ),
            );
        }

        panel = panel.child(
            div()
                .text_size(px(11.))
                .text_color(rgb(0xf38ba8))
                .cursor_pointer()
                .child("Clear all"),
        );

        panel.into_any_element()
    }
}
```

- [ ] **Step 2: Update src/ui/mod.rs**

Add: `pub mod filter_panel;`

- [ ] **Step 3: Build and verify**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add src/ui/
git commit -m "feat: filter panel with active filter display"
```

---

### Task 14: Schema View

**Files:**
- Create: `src/ui/schema_view.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create src/ui/schema_view.rs**

```rust
use gpui::*;

use crate::db::types::{Column, Index};

pub struct SchemaView {
    pub database: String,
    pub table_name: String,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub loading: bool,
}

impl SchemaView {
    pub fn new(database: String, table_name: String) -> Self {
        Self {
            database,
            table_name,
            columns: vec![],
            indexes: vec![],
            loading: true,
        }
    }

    pub fn set_schema(
        &mut self,
        columns: Vec<Column>,
        indexes: Vec<Index>,
        cx: &mut Context<Self>,
    ) {
        self.columns = columns;
        self.indexes = indexes;
        self.loading = false;
        cx.notify();
    }
}

impl Render for SchemaView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.loading {
            return div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .child(div().text_color(rgb(0x6c7086)).child("Loading schema..."))
                .into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_y_scroll()
            .p(px(16.))
            .gap_4()
            .child(self.render_columns_section())
            .child(self.render_indexes_section())
            .into_any_element()
    }
}

impl SchemaView {
    fn render_columns_section(&self) -> impl IntoElement {
        let mut section = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0xcdd6f4))
                    .mb(px(8.))
                    .child(format!("Columns ({})", self.columns.len())),
            );

        // Column header
        section = section.child(
            div()
                .flex()
                .flex_row()
                .bg(rgb(0x1e1e2e))
                .border_b_1()
                .border_color(rgb(0x333333))
                .py(px(6.))
                .child(div().w(px(150.)).px(px(12.)).text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x89b4fa)).child("Name"))
                .child(div().w(px(120.)).px(px(12.)).text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x89b4fa)).child("Type"))
                .child(div().w(px(80.)).px(px(12.)).text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x89b4fa)).child("Nullable"))
                .child(div().w(px(120.)).px(px(12.)).text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x89b4fa)).child("Default"))
                .child(div().w(px(60.)).px(px(12.)).text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x89b4fa)).child("Key"))
                .child(div().flex_1().px(px(12.)).text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x89b4fa)).child("Extra")),
        );

        for (i, col) in self.columns.iter().enumerate() {
            let bg = if i % 2 == 0 { rgb(0x181825) } else { rgb(0x1e1e2e) };
            let key_label = if col.is_primary_key { "PRI" } else { "" };
            let nullable_label = if col.nullable { "YES" } else { "NO" };
            let default_label = col.default_value.clone().unwrap_or_else(|| "NULL".to_string());

            section = section.child(
                div()
                    .flex()
                    .flex_row()
                    .bg(bg)
                    .border_b_1()
                    .border_color(rgb(0x222222))
                    .py(px(5.))
                    .child(div().w(px(150.)).px(px(12.)).text_size(px(12.)).text_color(rgb(0xcdd6f4)).child(col.name.clone()))
                    .child(div().w(px(120.)).px(px(12.)).text_size(px(12.)).text_color(rgb(0xfab387)).child(col.data_type.clone()))
                    .child(div().w(px(80.)).px(px(12.)).text_size(px(12.)).text_color(rgb(0x6c7086)).child(nullable_label.to_string()))
                    .child(div().w(px(120.)).px(px(12.)).text_size(px(12.)).text_color(rgb(0x6c7086)).child(default_label))
                    .child(div().w(px(60.)).px(px(12.)).text_size(px(12.)).text_color(rgb(0xf9e2af)).child(key_label.to_string()))
                    .child(div().flex_1().px(px(12.)).text_size(px(12.)).text_color(rgb(0x6c7086)).child(col.extra.clone())),
            );
        }

        section
    }

    fn render_indexes_section(&self) -> impl IntoElement {
        let mut section = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0xcdd6f4))
                    .mb(px(8.))
                    .child(format!("Indexes ({})", self.indexes.len())),
            );

        for (i, idx) in self.indexes.iter().enumerate() {
            let bg = if i % 2 == 0 { rgb(0x181825) } else { rgb(0x1e1e2e) };
            let unique_label = if idx.unique { "UNIQUE" } else { "" };

            section = section.child(
                div()
                    .flex()
                    .flex_row()
                    .bg(bg)
                    .border_b_1()
                    .border_color(rgb(0x222222))
                    .py(px(5.))
                    .child(div().w(px(200.)).px(px(12.)).text_size(px(12.)).text_color(rgb(0xcdd6f4)).child(idx.name.clone()))
                    .child(div().w(px(250.)).px(px(12.)).text_size(px(12.)).text_color(rgb(0xa6adc8)).child(idx.columns.join(", ")))
                    .child(div().flex_1().px(px(12.)).text_size(px(12.)).text_color(rgb(0xf9e2af)).child(unique_label.to_string())),
            );
        }

        section
    }
}
```

- [ ] **Step 2: Update src/ui/mod.rs**

Add: `pub mod schema_view;`

- [ ] **Step 3: Build and verify**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add src/ui/
git commit -m "feat: schema view for table columns and indexes"
```

---

### Task 15: Wire Up App — Connect, Browse, Query

This task connects all the pieces: sidebar clicks load tables, opening a table loads data, the SQL editor runs queries.

**Files:**
- Modify: `src/ui/app_view.rs`
- Modify: `src/ui/sidebar.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add event/callback infrastructure to AppView**

Update `src/ui/app_view.rs` to hold all entities and coordinate between them:

```rust
use gpui::*;

use crate::connection::profile::DatabaseEngine;
use crate::connection::ConnectionManager;
use crate::db::mysql::MySqlDriver;
use crate::db::types::Dialect;
use crate::db::DatabaseDriver;

use super::connection_dialog::ConnectionDialog;
use super::editor_view::EditorView;
use super::sidebar::Sidebar;
use super::tab_bar::{TabBar, TabKind};
use super::table_view::TableView;

pub struct AppView {
    connection_manager: ConnectionManager,
    sidebar: Entity<Sidebar>,
    tab_bar: Entity<TabBar>,
    connection_dialog: Entity<ConnectionDialog>,
    table_views: Vec<(usize, Entity<TableView>)>,
    editor_views: Vec<(usize, Entity<EditorView>)>,
    status_message: String,
}

impl AppView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|_| Sidebar::new());
        let tab_bar = cx.new(|_| TabBar::new());
        let connection_dialog = cx.new(|cx| ConnectionDialog::new(cx));

        Self {
            connection_manager: ConnectionManager::new(),
            sidebar,
            tab_bar,
            connection_dialog,
            table_views: vec![],
            editor_views: vec![],
            status_message: "Disconnected".to_string(),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = self.tab_bar.read(_cx).active_tab;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x181825))
            .text_color(rgb(0xcdd6f4))
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(self.sidebar.clone())
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(self.tab_bar.clone())
                            .child(self.render_active_content(active_tab)),
                    ),
            )
            .child(self.render_status_bar())
            .child(self.connection_dialog.clone())
    }
}

impl AppView {
    fn render_active_content(&self, active_tab: Option<usize>) -> impl IntoElement {
        if let Some(tab_id) = active_tab {
            // Check table views
            for (id, view) in &self.table_views {
                if *id == tab_id {
                    return view.clone().into_any_element();
                }
            }
            // Check editor views
            for (id, view) in &self.editor_views {
                if *id == tab_id {
                    return view.clone().into_any_element();
                }
            }
        }

        // Empty state
        div()
            .flex_1()
            .flex()
            .justify_center()
            .items_center()
            .child(
                div()
                    .text_color(rgb(0x6c7086))
                    .text_xl()
                    .child("QueryBox"),
            )
            .into_any_element()
    }

    fn render_status_bar(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .px(px(12.))
            .py(px(4.))
            .bg(rgb(0x1e1e2e))
            .border_t_1()
            .border_color(rgb(0x333333))
            .text_size(px(11.))
            .text_color(rgb(0x6c7086))
            .child(self.status_message.clone())
    }
}
```

- [ ] **Step 2: Build and verify the wiring compiles**

Run: `cargo check 2>&1 | tail -10`
Expected: Clean compilation. The app won't be fully interactive yet (click handlers need GPUI event wiring which depends on the specific GPUI version's API), but the structure is in place.

- [ ] **Step 3: Commit**

```bash
git add src/
git commit -m "feat: wire up app view with all components"
```

---

### Task 16: PostgreSQL Driver

**Files:**
- Create: `src/db/postgres.rs`
- Modify: `src/db/mod.rs`

- [ ] **Step 1: Create src/db/postgres.rs**

```rust
use async_trait::async_trait;
use std::time::Instant;
use tokio_postgres::{Client, NoTls};

use super::types::*;
use super::{DatabaseDriver, DbError};
use crate::connection::profile::ConnectionProfile;

pub struct PostgresDriver {
    client: Client,
    // tokio task handle for the connection — must be kept alive
    _connection_handle: tokio::task::JoinHandle<()>,
}

#[async_trait]
impl DatabaseDriver for PostgresDriver {
    async fn connect(profile: &ConnectionProfile) -> Result<Self, DbError> {
        let password = crate::connection::storage::get_password(profile)
            .map_err(|e| DbError::Connection(e.to_string()))?
            .unwrap_or_default();

        let conn_str = format!(
            "host={} port={} user={} password={} dbname={}",
            profile.host,
            profile.port,
            profile.user,
            password,
            profile.default_database.as_deref().unwrap_or("postgres")
        );

        let (client, connection) = tokio_postgres::connect(&conn_str, NoTls)
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;

        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        Ok(Self {
            client,
            _connection_handle: handle,
        })
    }

    async fn disconnect(&self) -> Result<(), DbError> {
        // tokio-postgres doesn't have an explicit disconnect; dropping the client closes it
        Ok(())
    }

    async fn databases(&self) -> Result<Vec<String>, DbError> {
        let rows = self
            .client
            .query("SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname", &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    async fn tables(&self, _database: &str) -> Result<Vec<String>, DbError> {
        let rows = self
            .client
            .query(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name",
                &[],
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    async fn columns(&self, _database: &str, table: &str) -> Result<Vec<Column>, DbError> {
        let rows = self
            .client
            .query(
                "SELECT c.column_name, c.data_type, c.is_nullable, c.column_default, \
                 CASE WHEN tc.constraint_type = 'PRIMARY KEY' THEN 'PRI' ELSE '' END as key_type \
                 FROM information_schema.columns c \
                 LEFT JOIN information_schema.key_column_usage kcu ON c.column_name = kcu.column_name AND c.table_name = kcu.table_name \
                 LEFT JOIN information_schema.table_constraints tc ON kcu.constraint_name = tc.constraint_name AND tc.constraint_type = 'PRIMARY KEY' \
                 WHERE c.table_name = $1 AND c.table_schema = 'public' \
                 ORDER BY c.ordinal_position",
                &[&table],
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|r| {
                let key: String = r.get(4);
                Column {
                    name: r.get(0),
                    data_type: r.get(1),
                    nullable: r.get::<_, String>(2) == "YES",
                    default_value: r.get(3),
                    is_primary_key: key == "PRI",
                    extra: String::new(),
                }
            })
            .collect())
    }

    async fn indexes(&self, _database: &str, table: &str) -> Result<Vec<Index>, DbError> {
        let rows = self
            .client
            .query(
                "SELECT i.relname as index_name, a.attname as column_name, ix.indisunique \
                 FROM pg_class t, pg_class i, pg_index ix, pg_attribute a \
                 WHERE t.oid = ix.indrelid AND i.oid = ix.indexrelid AND a.attrelid = t.oid \
                 AND a.attnum = ANY(ix.indkey) AND t.relkind = 'r' AND t.relname = $1 \
                 ORDER BY i.relname, a.attnum",
                &[&table],
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let mut indexes: Vec<Index> = vec![];
        for row in &rows {
            let name: String = row.get(0);
            let column: String = row.get(1);
            let unique: bool = row.get(2);

            if let Some(idx) = indexes.iter_mut().find(|i| i.name == name) {
                idx.columns.push(column);
            } else {
                indexes.push(Index {
                    name,
                    columns: vec![column],
                    unique,
                });
            }
        }
        Ok(indexes)
    }

    async fn query(&self, sql: &str, _params: &[Value]) -> Result<QueryResult, DbError> {
        let start = Instant::now();
        let rows = self
            .client
            .query(sql, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        if rows.is_empty() {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: 0,
                execution_time_ms,
            });
        }

        let columns: Vec<Column> = rows[0]
            .columns()
            .iter()
            .map(|c| Column {
                name: c.name().to_string(),
                data_type: format!("{:?}", c.type_()),
                nullable: true,
                default_value: None,
                is_primary_key: false,
                extra: String::new(),
            })
            .collect();

        let result_rows: Vec<Row> = rows.iter().map(pg_row_to_values).collect();

        Ok(QueryResult {
            columns,
            rows: result_rows,
            affected_rows: 0,
            execution_time_ms,
        })
    }

    async fn execute(&self, sql: &str, _params: &[Value]) -> Result<u64, DbError> {
        let result = self
            .client
            .execute(sql, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(result)
    }

    fn dialect(&self) -> Dialect {
        Dialect::PostgreSql
    }
}

fn pg_row_to_values(row: &tokio_postgres::Row) -> Row {
    let mut values = vec![];
    for i in 0..row.len() {
        let col_type = row.columns()[i].type_();
        let val = match col_type.name() {
            "int2" | "int4" => row
                .try_get::<_, i32>(i)
                .ok()
                .map(|v| Value::Int(v as i64))
                .unwrap_or(Value::Null),
            "int8" => row
                .try_get::<_, i64>(i)
                .ok()
                .map(Value::Int)
                .unwrap_or(Value::Null),
            "float4" => row
                .try_get::<_, f32>(i)
                .ok()
                .map(|v| Value::Float(v as f64))
                .unwrap_or(Value::Null),
            "float8" => row
                .try_get::<_, f64>(i)
                .ok()
                .map(Value::Float)
                .unwrap_or(Value::Null),
            "bool" => row
                .try_get::<_, bool>(i)
                .ok()
                .map(Value::Bool)
                .unwrap_or(Value::Null),
            "text" | "varchar" | "name" | "bpchar" => row
                .try_get::<_, String>(i)
                .ok()
                .map(Value::String)
                .unwrap_or(Value::Null),
            _ => row
                .try_get::<_, String>(i)
                .ok()
                .map(Value::String)
                .unwrap_or(Value::Null),
        };
        values.push(val);
    }
    values
}
```

- [ ] **Step 2: Add postgres module to src/db/mod.rs**

Add: `pub mod postgres;`

- [ ] **Step 3: Build and verify**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add src/db/
git commit -m "feat: PostgreSQL driver"
```

---

### Task 17: SQLite Driver

**Files:**
- Create: `src/db/sqlite.rs`
- Modify: `src/db/mod.rs`

- [ ] **Step 1: Create src/db/sqlite.rs**

```rust
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::types::*;
use super::{DatabaseDriver, DbError};
use crate::connection::profile::ConnectionProfile;

pub struct SqliteDriver {
    conn: Arc<Mutex<Connection>>,
}

#[async_trait]
impl DatabaseDriver for SqliteDriver {
    async fn connect(profile: &ConnectionProfile) -> Result<Self, DbError> {
        let path = profile
            .file_path
            .as_deref()
            .ok_or_else(|| DbError::Connection("No file path specified for SQLite".into()))?;

        let conn = tokio::task::spawn_blocking({
            let path = path.to_string();
            move || Connection::open(&path).map_err(|e| DbError::Connection(e.to_string()))
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))??;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    async fn disconnect(&self) -> Result<(), DbError> {
        Ok(())
    }

    async fn databases(&self) -> Result<Vec<String>, DbError> {
        // SQLite is single-database
        Ok(vec!["main".to_string()])
    }

    async fn tables(&self, _database: &str) -> Result<Vec<String>, DbError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .map_err(|e| DbError::Query(e.to_string()))?;
            let tables: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| DbError::Query(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(tables)
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    async fn columns(&self, _database: &str, table: &str) -> Result<Vec<Column>, DbError> {
        let conn = self.conn.clone();
        let table = table.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let mut stmt = conn
                .prepare(&format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\"")))
                .map_err(|e| DbError::Query(e.to_string()))?;
            let columns: Vec<Column> = stmt
                .query_map([], |row| {
                    Ok(Column {
                        name: row.get(1)?,
                        data_type: row.get(2)?,
                        nullable: {
                            let notnull: i32 = row.get(3)?;
                            notnull == 0
                        },
                        default_value: row.get(4)?,
                        is_primary_key: {
                            let pk: i32 = row.get(5)?;
                            pk > 0
                        },
                        extra: String::new(),
                    })
                })
                .map_err(|e| DbError::Query(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(columns)
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    async fn indexes(&self, _database: &str, table: &str) -> Result<Vec<Index>, DbError> {
        let conn = self.conn.clone();
        let table = table.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let mut stmt = conn
                .prepare(&format!(
                    "PRAGMA index_list(\"{}\")",
                    table.replace('"', "\"\"")
                ))
                .map_err(|e| DbError::Query(e.to_string()))?;

            let index_info: Vec<(String, bool)> = stmt
                .query_map([], |row| {
                    let name: String = row.get(1)?;
                    let unique: bool = row.get(2)?;
                    Ok((name, unique))
                })
                .map_err(|e| DbError::Query(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            let mut indexes = vec![];
            for (name, unique) in index_info {
                let mut idx_stmt = conn
                    .prepare(&format!(
                        "PRAGMA index_info(\"{}\")",
                        name.replace('"', "\"\"")
                    ))
                    .map_err(|e| DbError::Query(e.to_string()))?;

                let columns: Vec<String> = idx_stmt
                    .query_map([], |row| row.get(2))
                    .map_err(|e| DbError::Query(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect();

                indexes.push(Index {
                    name,
                    columns,
                    unique,
                });
            }
            Ok(indexes)
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    async fn query(&self, sql: &str, _params: &[Value]) -> Result<QueryResult, DbError> {
        let conn = self.conn.clone();
        let sql = sql.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let start = Instant::now();
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| DbError::Query(e.to_string()))?;

            let col_count = stmt.column_count();
            let columns: Vec<Column> = (0..col_count)
                .map(|i| Column {
                    name: stmt.column_name(i).unwrap_or("?").to_string(),
                    data_type: String::new(),
                    nullable: true,
                    default_value: None,
                    is_primary_key: false,
                    extra: String::new(),
                })
                .collect();

            let rows: Vec<Row> = stmt
                .query_map([], |row| {
                    let mut values = vec![];
                    for i in 0..col_count {
                        let val = match row.get_ref(i) {
                            Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                            Ok(rusqlite::types::ValueRef::Integer(n)) => Value::Int(n),
                            Ok(rusqlite::types::ValueRef::Real(f)) => Value::Float(f),
                            Ok(rusqlite::types::ValueRef::Text(t)) => {
                                Value::String(String::from_utf8_lossy(t).to_string())
                            }
                            Ok(rusqlite::types::ValueRef::Blob(b)) => Value::Bytes(b.to_vec()),
                            Err(_) => Value::Null,
                        };
                        values.push(val);
                    }
                    Ok(values)
                })
                .map_err(|e| DbError::Query(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            let execution_time_ms = start.elapsed().as_millis() as u64;

            Ok(QueryResult {
                columns,
                rows,
                affected_rows: 0,
                execution_time_ms,
            })
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    async fn execute(&self, sql: &str, _params: &[Value]) -> Result<u64, DbError> {
        let conn = self.conn.clone();
        let sql = sql.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| DbError::Other(e.to_string()))?;
            let affected = conn
                .execute(&sql, [])
                .map_err(|e| DbError::Query(e.to_string()))?;
            Ok(affected as u64)
        })
        .await
        .map_err(|e| DbError::Other(e.to_string()))?
    }

    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
}
```

- [ ] **Step 2: Add sqlite module to src/db/mod.rs**

Add: `pub mod sqlite;`

- [ ] **Step 3: Build and verify**

Run: `cargo check 2>&1 | tail -5`
Expected: Clean compilation.

- [ ] **Step 4: Add a quick SQLite integration test**

At the bottom of `src/db/sqlite.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::profile::{ConnectionProfile, DatabaseEngine};

    fn test_profile(path: &str) -> ConnectionProfile {
        ConnectionProfile {
            id: "sqlite-test".to_string(),
            name: "SQLite Test".to_string(),
            engine: DatabaseEngine::Sqlite,
            host: String::new(),
            port: 0,
            user: String::new(),
            default_database: None,
            file_path: Some(path.to_string()),
        }
    }

    #[tokio::test]
    async fn test_sqlite_create_and_query() {
        let tmp = std::env::temp_dir().join("querybox_test.db");
        let path = tmp.to_str().unwrap();

        // Clean up from previous run
        let _ = std::fs::remove_file(path);

        let profile = test_profile(path);
        let conn = Connection::open(path).unwrap();
        let driver = SqliteDriver {
            conn: Arc::new(Mutex::new(conn)),
        };

        driver
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)", &[])
            .await
            .unwrap();
        driver
            .execute("INSERT INTO test (name) VALUES ('alice')", &[])
            .await
            .unwrap();

        let result = driver.query("SELECT * FROM test", &[]).await.unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.columns[0].name, "id");
        assert_eq!(result.columns[1].name, "name");

        let tables = driver.tables("main").await.unwrap();
        assert!(tables.contains(&"test".to_string()));

        // Cleanup
        let _ = std::fs::remove_file(path);
    }
}
```

Run: `cargo test db::sqlite::tests -- --nocapture 2>&1 | tail -10`
Expected: Test passes.

- [ ] **Step 5: Commit**

```bash
git add src/db/
git commit -m "feat: SQLite driver"
```

---

### Task 18: Integration — Full App Run

This final task ensures all modules compile together and the app launches with the full UI.

**Files:**
- Modify: `src/main.rs` (ensure all modules are declared)
- Modify: `src/db/mod.rs` (ensure all drivers are exported)

- [ ] **Step 1: Verify all module declarations in main.rs**

`src/main.rs` should have:
```rust
mod connection;
mod db;
mod export;
mod query;
mod ui;
```

- [ ] **Step 2: Verify all driver exports in db/mod.rs**

`src/db/mod.rs` should have:
```rust
pub mod types;
pub mod mysql;
pub mod postgres;
pub mod sqlite;
```

- [ ] **Step 3: Run all tests**

Run: `cargo test 2>&1 | tail -20`
Expected: All unit tests pass (filter tests, export tests, SQLite test). MySQL test passes if dev DB is running.

- [ ] **Step 4: Build and launch**

Run: `cargo run`
Expected: Full QueryBox window opens with sidebar, tab bar (empty), main content area, and status bar. The app renders without panics.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete QueryBox MVP — all modules integrated"
```

---

## Summary

| Task | Component | What it produces |
|------|-----------|-----------------|
| 1 | Scaffolding | Cargo.toml + basic GPUI window |
| 2 | DB Types | Value, Column, Index, QueryResult, Dialect, DatabaseDriver trait |
| 3 | Connection | ConnectionProfile, storage (JSON + keyring), ConnectionManager |
| 4 | MySQL | Full MySqlDriver implementation + integration tests |
| 5 | Filters & History | Filter→SQL generation (dialect-aware) + query history |
| 6 | Export | CSV, SQL, JSON export from QueryResult |
| 7 | App Layout | Root AppView with sidebar + main + status bar |
| 8 | Connection Dialog | Modal dialog for new connections |
| 9 | Sidebar | Connection info, database selector, table list |
| 10 | Tab Bar | Tab management for tables and queries |
| 11 | Table View | Data grid with pagination, pending changes, toolbar |
| 12 | SQL Editor | Editor pane + results pane + history |
| 13 | Filter Panel | Visual filter chips with add/remove |
| 14 | Schema View | Column and index display |
| 15 | Wiring | Connect AppView to all components |
| 16 | PostgreSQL | Full PostgresDriver implementation |
| 17 | SQLite | Full SqliteDriver implementation + tests |
| 18 | Integration | Full build, all tests, launch verification |
