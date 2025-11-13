# QueryBox — Design Spec

**Date:** 2026-04-17
**Status:** Approved

## Overview

QueryBox is a free, open-source SQL GUI built in Rust with GPUI. It targets macOS and Linux, supporting MySQL, PostgreSQL, and SQLite. The goal is a polished alternative to TablePlus with zero cost.

## Core Features

1. **Browse tables** — paginated data grid showing table contents
2. **Edit tables** — inline cell editing with explicit save (batch pending changes, commit on Cmd+S)
3. **Column-level filters** — click a column, pick an operator and value, generates WHERE clauses per dialect
4. **Raw SQL editor** — syntax-highlighted editor with table/column autocomplete, query history, multiple query tabs, run full or selected text
5. **Export** — export any table view or query result as CSV, SQL (INSERT statements), or JSON
6. **Multiple databases** — switch between databases on the same connection via sidebar dropdown
7. **Schema inspector** — view all tables, columns, types, keys, and indexes
8. **Saved connections** — persist connection profiles to disk, one active connection at a time

## Target Platforms

- macOS (primary)
- Linux

## Supported Databases

- MySQL (via `mysql_async`)
- PostgreSQL (via `tokio-postgres`)
- SQLite (via `rusqlite`, wrapped in `spawn_blocking`)

## Application Layout

The main window has four zones:

### Left Sidebar (fixed width, ~220px)
- **Connection header** — active connection name, engine version, user
- **Database selector** — dropdown to switch databases on the current connection
- **Table list** — scrollable list of tables in the selected database; clicking opens a table tab
- **New Query button** — opens a SQL editor tab

### Tab Bar
- Each open table and SQL query gets its own tab
- Active tab is visually distinguished (top border accent)
- Tabs can be closed individually

### Main Content Area
- **For table tabs:** toolbar (filter, sort, pagination, export) + data grid
- **For query tabs:** split pane — editor on top, results on bottom, with a resizable divider

### Status Bar
- Connection status and engine info
- Pending changes count (when editing)
- Last query execution time

## SQL Editor

Each query tab contains:

- **Editor pane (top):**
  - Syntax-highlighted SQL editing area
  - Autocomplete popup triggered by `.` or after 2+ characters — shows table/column names with types from cached schema
  - Toolbar: "Run" (full query), "Run Selected" (highlighted text), "History" dropdown
- **Results pane (bottom):**
  - Data grid showing query results (same component as table browsing)
  - Row count, execution time, export button
  - If multiple statements, results from the last SELECT are shown

## Architecture

### Module Structure

```
src/
├── main.rs                  # Entry point, GPUI + Tokio initialization
├── app.rs                   # Root application state & window setup
│
├── db/                      # Database abstraction layer
│   ├── mod.rs               # DatabaseDriver trait
│   ├── types.rs             # Row, Column, Value, Schema, QueryResult
│   ├── mysql.rs             # MySQL implementation
│   ├── postgres.rs          # PostgreSQL implementation
│   └── sqlite.rs            # SQLite implementation
│
├── connection/              # Connection management
│   ├── mod.rs               # ConnectionManager
│   ├── profile.rs           # ConnectionProfile struct
│   └── storage.rs           # Persist profiles to JSON
│
├── query/                   # Query execution & filtering
│   ├── mod.rs               # QueryExecutor
│   ├── filter.rs            # Filter → WHERE clause (dialect-aware)
│   └── history.rs           # Query history storage & recall
│
├── export/                  # Export functionality
│   ├── mod.rs               # Exporter trait
│   ├── csv.rs               # CSV export
│   ├── sql.rs               # SQL INSERT export
│   └── json.rs              # JSON export
│
└── ui/                      # GPUI views
    ├── mod.rs
    ├── app_view.rs           # Root view — sidebar + tab area
    ├── sidebar.rs            # Connection info, DB selector, table list
    ├── tab_bar.rs            # Tab management
    ├── table_view.rs         # Data grid with inline editing
    ├── editor_view.rs        # SQL editor with syntax highlighting
    ├── filter_panel.rs       # Column filter UI
    ├── schema_view.rs        # Table schema inspector
    └── connection_dialog.rs  # New/edit connection form
```

### DatabaseDriver Trait

The core abstraction. Each engine implements this independently.

```rust
#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    async fn connect(profile: &ConnectionProfile) -> Result<Self, DbError> where Self: Sized;
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

### Async Model

- GPUI runs on the main thread. All database I/O dispatched via `cx.spawn()` onto a Tokio runtime.
- UI never blocks on DB operations. Results arrive asynchronously and trigger view updates.
- `rusqlite` is sync — wrapped in `tokio::task::spawn_blocking`.

### Connection Management

- `ConnectionManager` holds a `Vec<ConnectionProfile>` (saved) and an `Option<Box<dyn DatabaseDriver>>` (active).
- Profiles persisted as JSON in platform config directory.
- Passwords stored in OS keychain via `keyring` crate — not in the JSON file.
- Switching connections: disconnect current driver, connect new one, refresh sidebar.

## Data Flow

### Opening a Table
1. User clicks table name in sidebar → new tab opens
2. `driver.columns(db, table)` fetches schema; `driver.query()` fetches first page (LIMIT 50 OFFSET 0)
3. Results render in data grid; pagination controls adjust OFFSET

### Inline Editing
1. Click cell → becomes editable input
2. Changes stored in `PendingChanges` buffer (keyed by primary key + column name)
3. Modified cells highlighted yellow
4. "Save" (Cmd+S) generates UPDATE/INSERT/DELETE statements, executes in a transaction
5. Success → clear dirty state, refresh. Failure → show error, keep changes pending.

### Column Filtering
1. Click "+ Filter" → column dropdown → operator picker → value input
2. `filter.rs` generates WHERE clause in correct dialect (MySQL backticks vs PostgreSQL double-quotes)
3. Table re-queries with filter. Multiple filters combine with AND.

### Schema Inspector
1. User right-clicks a table in sidebar (or clicks an "info" icon) → schema view opens in a tab
2. `driver.columns(db, table)` and `driver.indexes(db, table)` fetch full schema
3. Displayed as a read-only table: column name, type, nullable, default, key info, and a separate section for indexes

### SQL Execution
1. User types SQL in editor pane
2. Autocomplete queries cached schema on `.` or 2+ chars
3. "Run" sends full text to `driver.query()` or `driver.execute()` (detected by statement prefix — SELECT/SHOW/DESCRIBE → query, otherwise → execute)
4. Results render in bottom pane
5. Query saved to history

### Export
1. "Export ▼" → pick CSV, SQL, or JSON
2. Export module serializes current `QueryResult`
3. File save dialog → write to disk

## Dependencies

| Crate | Purpose |
|-------|---------|
| `gpui` | UI framework (git dep from Zed) |
| `mysql_async` | Async MySQL driver |
| `tokio-postgres` | Async PostgreSQL driver |
| `rusqlite` | SQLite driver |
| `tokio` | Async runtime |
| `serde` / `serde_json` | Serialization |
| `thiserror` | Typed error enums |
| `anyhow` | Application boundary errors |
| `csv` | CSV export |
| `keyring` | OS keychain for passwords |
| `dirs` | Platform config directories |

## Error Handling

- Each module defines its own error enum via `thiserror` (`DbError`, `ConnectionError`, `ExportError`)
- All fallible operations return `Result<T, E>` — no panics
- UI displays transient errors (query failures) in a notification bar at the bottom
- Connection failures shown as a dialog
- Failed edits keep `PendingChanges` intact for retry

## Data Storage

- **Connection profiles:** `~/.config/querybox/connections.json` (Linux) / `~/Library/Application Support/querybox/connections.json` (macOS)
- **Query history:** per-connection files in the same config directory
- **Passwords:** OS keychain via `keyring` (never plaintext on disk)

## Security

- All user-provided values in queries use parameterized queries — no string interpolation
- Filter-generated SQL uses parameterized queries for values; identifiers (table/column names) are quoted per dialect
- Connection passwords stored in OS keychain, not on disk
