# SQL Expressions in Cell Editing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to type SQL expressions (e.g. `NOW()`, `UUID()`, `NULL`, `CURRENT_TIMESTAMP`) into table cells during both insert and update, and have them injected as raw SQL rather than bound as string parameters.

**Architecture:** Add `Value::RawSql(String)` to the type enum as an input-only variant. Detect SQL expressions at the point where user text becomes a `Value`, using a heuristic function. Update the SQL builders for INSERT and UPDATE to inline `RawSql` values directly instead of using `?` placeholders.

**Tech Stack:** Rust, GPUI, `mysql_async` / `tokio_postgres` / `rusqlite`

---

## File Map

| File | Change |
|---|---|
| `src/db/types.rs` | Add `RawSql` variant, `is_sql_expression`, `text_to_value`, update `Display` |
| `src/ui/table_view.rs` | Change `CellEdit.new_value: String` → `Value`; call `text_to_value` in `save_new_row` and `save_changes` |
| `src/ui/app_view.rs` | Update `execute_insert` and `save_and_reload` to handle `RawSql` inline |
| `src/db/mysql.rs` | Add `RawSql` arm to `value_to_mysql` |
| `src/db/postgres.rs` | Add `RawSql` arm to `to_pg_params` |
| `src/db/sqlite.rs` | Add `RawSql` arm to `to_rusqlite_value` |
| `src/export/json_export.rs` | Add `RawSql` arm to `value_to_json` |
| `src/export/sql_export.rs` | Add `RawSql` arm to `sql_literal` |

---

### Task 1: Add `Value::RawSql` and detection helpers to `src/db/types.rs`

**Files:**
- Modify: `src/db/types.rs`

- [ ] **Step 1: Add the `RawSql` variant and update `Display`**

Open `src/db/types.rs`. Add the new variant to the enum and its `Display` arm:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    DateTime(NaiveDateTime),
    RawSql(String), // user-supplied SQL expression; input-only, never returned from DB
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
            Value::RawSql(expr) => write!(f, "{}", expr),
        }
    }
}
```

- [ ] **Step 2: Add `is_sql_expression` and `text_to_value`**

Append these two functions at the bottom of `src/db/types.rs` (before the closing of the module, after the existing `impl Dialect` block):

```rust
/// Returns true if `s` looks like a SQL expression rather than a literal value.
/// Matches SQL keyword constants and function call patterns.
pub fn is_sql_expression(s: &str) -> bool {
    let t = s.trim();
    // Keyword constants (case-insensitive)
    const KEYWORDS: &[&str] = &[
        "NULL",
        "DEFAULT",
        "TRUE",
        "FALSE",
        "CURRENT_TIMESTAMP",
        "CURRENT_DATE",
        "CURRENT_TIME",
    ];
    if KEYWORDS.iter().any(|k| t.eq_ignore_ascii_case(k)) {
        return true;
    }
    // Function call heuristic: starts with letter/underscore, contains '(', ends with ')'
    let first = t.chars().next();
    matches!(first, Some(c) if c.is_ascii_alphabetic() || c == '_')
        && t.contains('(')
        && t.ends_with(')')
}

/// Convert user-typed text into the appropriate `Value`.
/// Detects SQL expressions and returns `Value::RawSql`; otherwise `Value::String`.
pub fn text_to_value(s: &str) -> Value {
    if is_sql_expression(s) {
        Value::RawSql(s.trim().to_string())
    } else {
        Value::String(s.to_string())
    }
}
```

- [ ] **Step 3: Write tests for `is_sql_expression`**

Add a test module at the bottom of `src/db/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sql_expression_keywords() {
        assert!(is_sql_expression("NULL"));
        assert!(is_sql_expression("null"));
        assert!(is_sql_expression("  NULL  "));
        assert!(is_sql_expression("DEFAULT"));
        assert!(is_sql_expression("CURRENT_TIMESTAMP"));
        assert!(is_sql_expression("current_date"));
        assert!(is_sql_expression("TRUE"));
        assert!(is_sql_expression("FALSE"));
    }

    #[test]
    fn test_is_sql_expression_functions() {
        assert!(is_sql_expression("NOW()"));
        assert!(is_sql_expression("now()"));
        assert!(is_sql_expression("UUID()"));
        assert!(is_sql_expression("DATE_ADD(NOW(), INTERVAL 1 DAY)"));
        assert!(is_sql_expression("COALESCE(NULL, 0)"));
    }

    #[test]
    fn test_is_sql_expression_plain_strings() {
        assert!(!is_sql_expression("hello"));
        assert!(!is_sql_expression("123"));
        assert!(!is_sql_expression("some text with spaces"));
        assert!(!is_sql_expression(""));
        assert!(!is_sql_expression("O'Brien"));
    }

    #[test]
    fn test_text_to_value_sql() {
        assert_eq!(text_to_value("NOW()"), Value::RawSql("NOW()".to_string()));
        assert_eq!(text_to_value("NULL"), Value::RawSql("NULL".to_string()));
        assert_eq!(text_to_value("  NOW()  "), Value::RawSql("NOW()".to_string()));
    }

    #[test]
    fn test_text_to_value_string() {
        assert_eq!(text_to_value("hello"), Value::String("hello".to_string()));
        assert_eq!(text_to_value(""), Value::String("".to_string()));
    }
}
```

- [ ] **Step 4: Run the tests**

```bash
cargo test -p querybox db::types::tests 2>&1
```

Expected: all tests pass. If any fail, fix `is_sql_expression` or `text_to_value` before continuing.

- [ ] **Step 5: Commit**

```bash
git add src/db/types.rs
git commit -m "feat(sql-expr): add Value::RawSql variant, is_sql_expression, text_to_value"
```

---

### Task 2: Fix exhaustive `match` sites in db drivers and export modules

The drivers never receive `RawSql` (it's consumed before SQL is sent to the DB), so these arms are unreachable. The export modules also won't encounter `RawSql` in practice, but the compiler requires exhaustiveness.

**Files:**
- Modify: `src/db/mysql.rs` (function `value_to_mysql`, line ~257)
- Modify: `src/db/postgres.rs` (closure in `to_pg_params`, line ~207)
- Modify: `src/db/sqlite.rs` (function `to_rusqlite_value`, line ~11)
- Modify: `src/export/json_export.rs` (function `value_to_json`, line ~28)
- Modify: `src/export/sql_export.rs` (function `sql_literal`, line ~22)

- [ ] **Step 1: Update `value_to_mysql` in `src/db/mysql.rs`**

Find `fn value_to_mysql` and add the arm:

```rust
fn value_to_mysql(v: &Value) -> mysql_async::Value {
    match v {
        Value::Null => mysql_async::Value::NULL,
        Value::Bool(b) => mysql_async::Value::from(*b),
        Value::Int(i) => mysql_async::Value::from(*i),
        Value::Float(f) => mysql_async::Value::from(*f),
        Value::String(s) => mysql_async::Value::from(s.as_str()),
        Value::Bytes(b) => mysql_async::Value::from(b.as_slice()),
        Value::DateTime(dt) => mysql_async::Value::from(dt.format("%Y-%m-%d %H:%M:%S").to_string()),
        Value::RawSql(_) => unreachable!("RawSql is consumed before reaching the driver"),
    }
}
```

- [ ] **Step 2: Update `to_pg_params` in `src/db/postgres.rs`**

Find the closure that matches `Value` variants (around line 207) and add the arm:

```rust
Value::RawSql(_) => unreachable!("RawSql is consumed before reaching the driver"),
```

The full closure becomes:

```rust
.map(|v| -> Box<dyn tokio_postgres::types::ToSql + Sync + Send> {
    match v {
        Value::Null => Box::new(Option::<String>::None),
        Value::Bool(b) => Box::new(*b),
        Value::Int(i) => Box::new(*i),
        Value::Float(f) => Box::new(*f),
        Value::String(s) => Box::new(s.clone()),
        Value::Bytes(b) => Box::new(b.clone()),
        Value::DateTime(dt) => Box::new(dt.to_string()),
        Value::RawSql(_) => unreachable!("RawSql is consumed before reaching the driver"),
    }
})
```

- [ ] **Step 3: Update `to_rusqlite_value` in `src/db/sqlite.rs`**

```rust
fn to_rusqlite_value(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        Value::Int(i) => rusqlite::types::Value::Integer(*i),
        Value::Float(f) => rusqlite::types::Value::Real(*f),
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        Value::Bytes(b) => rusqlite::types::Value::Blob(b.clone()),
        Value::DateTime(dt) => rusqlite::types::Value::Text(dt.to_string()),
        Value::RawSql(_) => unreachable!("RawSql is consumed before reaching the driver"),
    }
}
```

- [ ] **Step 4: Update `value_to_json` in `src/export/json_export.rs`**

Find `fn value_to_json` and add the arm (treat as string for export context):

```rust
fn value_to_json(v: &Value) -> serde_json::Value {
    use crate::db::types::Value;
    match v {
        Value::Null => json!(null),
        Value::Bool(b) => json!(b),
        Value::Int(i) => json!(i),
        Value::Float(f) => json!(f),
        Value::String(s) => json!(s),
        Value::Bytes(b) => json!(format!("<{} bytes>", b.len())),
        Value::DateTime(dt) => json!(dt.to_string()),
        Value::RawSql(expr) => json!(expr),
    }
}
```

- [ ] **Step 5: Update `sql_literal` in `src/export/sql_export.rs`**

Find `fn sql_literal` and add the arm:

```rust
fn sql_literal(v: &Value) -> String {
    match v {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Bytes(b) => format!("X'{}'", hex::encode(b)),
        Value::DateTime(dt) => format!("'{}'", dt.format("%Y-%m-%d %H:%M:%S")),
        Value::RawSql(expr) => expr.clone(),
    }
}
```

- [ ] **Step 6: Verify it compiles**

```bash
cargo check 2>&1
```

Expected: no errors (warnings about `unreachable!` are fine, clippy may flag them — that's addressed in Task 5).

- [ ] **Step 7: Commit**

```bash
git add src/db/mysql.rs src/db/postgres.rs src/db/sqlite.rs src/export/json_export.rs src/export/sql_export.rs
git commit -m "feat(sql-expr): add RawSql arms to exhaustive match sites in drivers and export"
```

---

### Task 3: Update `CellEdit` and `save_changes` in `src/ui/table_view.rs`

`CellEdit.new_value` is currently a `String`. Change it to `Value` so the `RawSql`/`String` distinction survives the trip to `app_view.rs`. Also call `text_to_value` in `save_new_row`.

**Files:**
- Modify: `src/ui/table_view.rs`

- [ ] **Step 1: Import `text_to_value` at the top of `table_view.rs`**

Find the existing import line:

```rust
use crate::db::types::{Column, QueryResult, Row, Value};
```

Change it to:

```rust
use crate::db::types::{text_to_value, Column, QueryResult, Row, Value};
```

- [ ] **Step 2: Change `CellEdit.new_value` from `String` to `Value`**

Find the `CellEdit` struct (around line 23):

```rust
#[derive(Clone, Debug)]
pub struct CellEdit {
    pub column: String,
    pub new_value: String,
}
```

Change it to:

```rust
#[derive(Clone, Debug)]
pub struct CellEdit {
    pub column: String,
    pub new_value: Value,
}
```

- [ ] **Step 3: Update the `CellEdit` construction site in `save_changes`**

Find the `filter_map` inside `save_changes` that builds `CellEdit` values (around line 279):

```rust
let edits: Vec<CellEdit> = col_edits
    .into_iter()
    .filter_map(|(col_idx, new_value)| {
        self.columns.get(col_idx).map(|c| CellEdit {
            column: c.name.clone(),
            new_value,
        })
    })
    .collect();
```

Change it to call `text_to_value`:

```rust
let edits: Vec<CellEdit> = col_edits
    .into_iter()
    .filter_map(|(col_idx, new_value)| {
        self.columns.get(col_idx).map(|c| CellEdit {
            column: c.name.clone(),
            new_value: text_to_value(&new_value),
        })
    })
    .collect();
```

- [ ] **Step 4: Update `save_new_row` to call `text_to_value`**

Find this line in `save_new_row` (around line 331):

```rust
.map(|col| (col.name.clone(), crate::db::types::Value::String(value.clone())))
```

Change it to:

```rust
.map(|col| (col.name.clone(), text_to_value(value)))
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check 2>&1
```

Expected: errors in `app_view.rs` about `CellEdit.new_value` type mismatch — these will be fixed in Task 4. No other errors.

- [ ] **Step 6: Commit**

```bash
git add src/ui/table_view.rs
git commit -m "feat(sql-expr): change CellEdit.new_value to Value, call text_to_value at input boundary"
```

---

### Task 4: Update SQL builders in `src/ui/app_view.rs`

Both `execute_insert` and `save_and_reload` need to handle `Value::RawSql` by inlining the expression rather than using a `?` placeholder.

**Files:**
- Modify: `src/ui/app_view.rs`

- [ ] **Step 1: Update `execute_insert`**

Find the `execute_insert` function (around line 411). Currently it builds `placeholders` as all `"?"`:

```rust
let col_names: Vec<String> = insert
    .column_values
    .iter()
    .map(|(col, _)| format!("`{}`", col.replace('`', "``")))
    .collect();
let placeholders: Vec<&str> = insert.column_values.iter().map(|_| "?").collect();
let sql = format!(
    "INSERT INTO `{}`.`{}` ({}) VALUES ({})",
    insert.database.replace('`', "``"),
    insert.table.replace('`', "``"),
    col_names.join(", "),
    placeholders.join(", "),
);
let params: Vec<Value> = insert.column_values.into_iter().map(|(_, v)| v).collect();
```

Replace that entire block with:

```rust
let mut placeholders: Vec<String> = Vec::new();
let mut params: Vec<Value> = Vec::new();
let col_names: Vec<String> = insert
    .column_values
    .iter()
    .map(|(col, _)| format!("`{}`", col.replace('`', "``")))
    .collect();
for (_, v) in insert.column_values {
    match v {
        Value::RawSql(expr) => placeholders.push(expr),
        other => {
            placeholders.push("?".to_string());
            params.push(other);
        }
    }
}
let sql = format!(
    "INSERT INTO `{}`.`{}` ({}) VALUES ({})",
    insert.database.replace('`', "``"),
    insert.table.replace('`', "``"),
    col_names.join(", "),
    placeholders.join(", "),
);
```

- [ ] **Step 2: Update `save_and_reload`**

Find `save_and_reload` (around line 328). Currently it builds SET clauses as all `col = ?`:

```rust
let set_clauses: Vec<String> = update
    .edits
    .iter()
    .map(|e| format!("`{}` = ?", e.column.replace('`', "``")))
    .collect();
// ...
let mut params: Vec<Value> = update
    .edits
    .iter()
    .map(|e| Value::String(e.new_value.clone()))
    .collect();
params.extend(update.pk_values.clone());
```

Replace those two blocks with:

```rust
let mut set_clauses: Vec<String> = Vec::new();
let mut params: Vec<Value> = Vec::new();
for e in &update.edits {
    match &e.new_value {
        Value::RawSql(expr) => {
            set_clauses.push(format!("`{}` = {}", e.column.replace('`', "``"), expr));
        }
        other => {
            set_clauses.push(format!("`{}` = ?", e.column.replace('`', "``")));
            params.push(other.clone());
        }
    }
}
params.extend(update.pk_values.clone());
```

- [ ] **Step 3: Verify it compiles and all tests pass**

```bash
cargo test 2>&1
```

Expected: all tests pass, no errors.

- [ ] **Step 4: Commit**

```bash
git add src/ui/app_view.rs
git commit -m "feat(sql-expr): inline RawSql values in INSERT and UPDATE SQL builders"
```

---

### Task 5: Clippy and formatting pass

**Files:**
- All modified files

- [ ] **Step 1: Run clippy**

```bash
cargo clippy 2>&1
```

Fix any warnings. Common ones to expect:
- If `unreachable!` in driver match arms triggers a lint, that's fine — leave as-is (it's intentional documentation).
- Fix any other clippy suggestions in the modified code.

- [ ] **Step 2: Run formatter**

```bash
cargo fmt 2>&1
```

- [ ] **Step 3: Commit if any changes**

```bash
git add -p
git commit -m "chore: clippy and fmt fixes for sql-expr feature"
```

(Skip this commit if there are no changes.)

---

### Task 6: Manual smoke test

- [ ] **Step 1: Start the dev database**

```bash
cd dev && docker compose up -d && cd ..
```

- [ ] **Step 2: Run the app**

```bash
cargo run 2>&1
```

- [ ] **Step 3: Test INSERT with SQL functions**

1. Open a table (e.g. `users`)
2. Click "New Row" to activate the insert row
3. In a datetime or timestamp column, type `NOW()` and click another cell
4. Click Insert
5. Verify: row saves without error, reloads showing the evaluated timestamp (not the string "NOW()")

- [ ] **Step 4: Test INSERT with SQL keyword**

1. Open "New Row" again
2. In a nullable column, type `NULL`
3. Click Insert
4. Verify: row saves, the cell shows as null/empty in the reloaded table

- [ ] **Step 5: Test UPDATE with SQL function**

1. Click on a cell in an existing row (e.g. a datetime column)
2. Type `NOW()` into the cell
3. Press Escape to commit, then Save (or use the Save keyboard shortcut)
4. Verify: update saves without error, cell reloads with the evaluated timestamp

- [ ] **Step 6: Test plain string still works**

1. Click on a cell in an existing row
2. Type `hello world`
3. Save
4. Verify: cell saves and reloads as the string "hello world"

- [ ] **Step 7: Test invalid SQL expression returns DB error**

1. Open a new row
2. In a column, type `BADFUNC(`  (invalid SQL)
3. Click Insert
4. Verify: the red error strip appears with a DB error message — the app does not crash
