# SQL Expressions in Cell Editing

**Date:** 2026-04-25
**Status:** Approved

## Overview

Allow users to type SQL expressions (functions like `NOW()`, `UUID()`, and keywords like `NULL`, `CURRENT_TIMESTAMP`) directly into table cells when inserting or editing rows. Values that match a SQL expression pattern are injected as raw SQL rather than bound as string parameters.

## Scope

Applies to both:
- New row insertion (the new-row editing UI)
- Existing row cell editing (inline UPDATE)

## Detection

A free function `is_sql_expression(s: &str) -> bool` in `src/db/types.rs` performs case-insensitive matching against two categories:

**Keyword set** (exact match, case-insensitive):
- `NULL`, `DEFAULT`, `TRUE`, `FALSE`
- `CURRENT_TIMESTAMP`, `CURRENT_DATE`, `CURRENT_TIME`

**Function call pattern** (heuristic, no regex):
- Trimmed string starts with an ASCII letter or underscore
- Contains a `(`
- Ends with `)`
- Examples: `NOW()`, `UUID()`, `DATE_ADD(NOW(), INTERVAL 1 DAY)`, `COALESCE(NULL, 0)`

A companion function `text_to_value(s: &str) -> Value` returns `Value::RawSql(s.trim().to_string())` if `is_sql_expression` returns true, otherwise `Value::String(s.to_string())`. This is the single conversion point for user text → `Value`.

**Known trade-off:** A user who genuinely wants to store the literal string `"NULL"` or `"now()"` in a text column cannot do so via the cell editor. This is accepted — the behaviour is consistent with tools like TablePlus, and the workaround is a raw SQL editor.

## Data Model

Add `RawSql(String)` to the `Value` enum in `src/db/types.rs`:

```rust
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    DateTime(NaiveDateTime),
    RawSql(String),   // user-supplied SQL expression; input-only, never returned from DB
}
```

Update the `Display` impl for `RawSql` to output the expression as-is. Update all exhaustive `match` sites (db drivers, export modules) — `RawSql` is consumed by the SQL builders before ever reaching the drivers, so these arms are unreachable; use `unreachable!()` or a sensible fallback string.

`CellEdit.new_value` changes from `String` to `Value` so the `RawSql`/`String` distinction is preserved from `table_view.rs` through to `app_view.rs`.

## SQL Building

### INSERT (`execute_insert` in `src/ui/app_view.rs`)

Iterate `column_values` to build placeholders and params separately:

- `Value::RawSql(expr)` → push `expr` directly into the placeholder list; skip from `params`
- All other variants → push `"?"` into placeholders; add value to `params`

Result: `INSERT INTO t (a, b, c) VALUES (NOW(), ?, ?)` with two bound params.

### UPDATE (`save_and_reload` in `src/ui/app_view.rs`)

Iterate edits to build SET clauses and params separately:

- `Value::RawSql(expr)` → emit `` `col` = <expr> `` inline; skip from params
- All other variants → emit `` `col` = ? ``; add value to params

The `WHERE` clause uses PK values that come from existing DB rows, never from user text — no change needed there.

### Conversion call-sites

| Location | Current | After |
|---|---|---|
| `save_new_row` in `table_view.rs` | `Value::String(value.clone())` | `text_to_value(value)` |
| `save_and_reload` in `app_view.rs` | `Value::String(e.new_value.clone())` | `e.new_value.clone()` (already a `Value`) |

## Files Changed

| File | Change |
|---|---|
| `src/db/types.rs` | Add `Value::RawSql`, `is_sql_expression`, `text_to_value`; update `Display` |
| `src/ui/table_view.rs` | Change `CellEdit.new_value: String` → `Value`; call `text_to_value` in `save_new_row` |
| `src/ui/app_view.rs` | Update `execute_insert` and `save_and_reload` to handle `RawSql` inline |
| `src/db/mysql.rs` / `postgres.rs` / `sqlite.rs` | Add `RawSql` arm to exhaustive matches (no-op / unreachable) |
| `src/export/*.rs` | Add `RawSql` arm to exhaustive matches (treat as string or skip) |

## Error Handling

No special error handling needed. If a user types an invalid SQL expression (e.g. `BADFUNC(`), the database will return an error, which is already surfaced via the existing insert/update error display.

## Testing

Manual test cases:
- Insert row with `NOW()` in a datetime column → value saves and reloads as the evaluated timestamp
- Insert row with `UUID()` in a varchar column → value saves and reloads as a UUID string
- Insert row with `NULL` → cell saves as SQL NULL, reloads as empty/null display
- Edit existing row cell with `CURRENT_TIMESTAMP` → UPDATE runs with inline SQL, not bound param
- Edit existing row cell with a plain string like `hello` → still treated as a string literal
- Edit existing row cell with a string like `foo(bar)` → treated as a SQL function call (accepted trade-off)
