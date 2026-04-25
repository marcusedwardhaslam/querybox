# Format SQL Button — Design Spec

**Date:** 2026-04-25  
**Status:** Approved

## Summary

Wire up the existing inert "Format" button in the New Query view to reformat the SQL in the editor using a full AST-based formatter backed by `sqlparser-rs`. Parse errors are surfaced in the existing error pane below the editor.

---

## Architecture

### New module: `src/query/format.rs`

Owns all formatting logic. Single public entry point:

```rust
pub fn format_sql(sql: &str) -> Result<String, String>
```

- Parses `sql` using `sqlparser` with `GenericDialect`
- On parse failure: returns `Err(human-readable message)`
- On success: passes each `Statement` through a custom `Formatter` struct, joins multiple statements with `\n\n`, returns `Ok(formatted)`

The `Formatter` struct walks the AST recursively and builds a formatted string. It is not a GPUI component — pure data transformation with no UI dependencies.

### Changes to `src/ui/editor_view.rs`

- Add `format_query(&mut self, cx: &mut Context<Self>)` method:
  1. Read `self.editor.read(cx).content`
  2. Return early if empty
  3. Call `format_sql(&content)`
  4. On `Ok(formatted)`: update the editor content by replacing the full range via `editor.update`
  5. On `Err(msg)`: call `self.set_error(original_sql, msg, cx)` to show error in the existing pane
- Wire the existing inert `"format-btn"` div with `on_click(cx.listener(|this, _, _, cx| this.format_query(cx)))`

### Changes to `Cargo.toml`

Add: `sqlparser = "0.54"`

---

## Formatter Output Style

```sql
SELECT
    u.id,
    u.name,
    o.total
FROM users AS u
JOIN orders AS o
    ON o.user_id = u.id
WHERE
    u.active = 1
    AND o.total > 100
ORDER BY
    o.total DESC
LIMIT 10
```

**Rules:**
- All keywords uppercase
- `SELECT`, `WHERE`, `ORDER BY`, `GROUP BY`, `HAVING` — keyword on its own line; each item indented 4 spaces, one per line
- `FROM`, `JOIN`, `LIMIT`, `OFFSET` — keyword and value on the same line
- `ON` for joins indented 4 spaces under the `JOIN` line
- `WHERE` / `HAVING` conditions split one per line; `AND` / `OR` at the start of each continuation line
- Subqueries indented an additional 4 spaces
- CTEs (`WITH`) each on their own line before the main query
- Multiple statements (semicolon-separated) each formatted independently and joined with `\n\n`

---

## Error Handling & Edge Cases

| Scenario | Behaviour |
|---|---|
| Parse failure | `set_error` called; error shown in red pane below editor; editor content unchanged |
| Empty editor | `format_query` returns early; no-op |
| Multiple statements | Each formatted independently, joined with `\n\n` |
| Unsupported syntax | Treated as parse failure |
| Dialect | `GenericDialect` used to accept MySQL, PostgreSQL, and SQLite input |

---

## Out of Scope

- Keyboard shortcut (e.g. Cmd+Shift+F) — not in this iteration
- Dialect-specific formatting differences
- Format-on-save
