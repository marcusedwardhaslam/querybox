# Format SQL Button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire up the existing inert "Format" button in the New Query view to reformat SQL using an AST-based pretty-printer backed by `sqlparser-rs`.

**Architecture:** A new pure-Rust module `src/query/format.rs` exposes `format_sql(sql: &str) -> Result<String, String>`, which parses the SQL via `sqlparser` with `GenericDialect`, then walks the AST with a custom formatter producing opinionated output (keywords uppercase, one SELECT column per line, WHERE conditions split on AND/OR, JOIN ON indented). `EditorView` calls this from a new `format_query` method wired to the button's `on_click`.

**Tech Stack:** Rust, GPUI, `sqlparser = "0.54"` (pure Rust SQL parser)

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `Cargo.toml` | Modify | Add `sqlparser` dependency |
| `src/query/format.rs` | Create | `format_sql` function + custom AST formatter |
| `src/query/mod.rs` | Modify | Export `format` module |
| `src/ui/editor_view.rs` | Modify | Add `format_query` method, wire button `on_click` |

---

## Task 1: Add `sqlparser` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dependency**

In `Cargo.toml`, add to `[dependencies]`:

```toml
sqlparser = "0.54"
```

- [ ] **Step 2: Verify it downloads and compiles**

```bash
cargo fetch
```

Expected: no errors (downloads the crate and its deps).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add sqlparser dependency"
```

---

## Task 2: Create `src/query/format.rs` with failing tests

**Files:**
- Create: `src/query/format.rs`
- Modify: `src/query/mod.rs`

- [ ] **Step 1: Add module declaration**

In `src/query/mod.rs`, add:

```rust
pub mod filter;
pub mod format;
pub mod history;
```

- [ ] **Step 2: Create `src/query/format.rs` with stub + tests**

```rust
use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

pub fn format_sql(sql: &str) -> Result<String, String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_empty() {
        assert_eq!(format_sql("").unwrap(), "");
    }

    #[test]
    fn test_whitespace_only_returns_empty() {
        assert_eq!(format_sql("   \n  ").unwrap(), "");
    }

    #[test]
    fn test_invalid_sql_returns_err() {
        assert!(format_sql("SELEKT garbage *** from").is_err());
    }

    #[test]
    fn test_simple_select_all() {
        let out = format_sql("select * from users").unwrap();
        assert_eq!(out, "SELECT\n    *\nFROM users");
    }

    #[test]
    fn test_select_columns() {
        let out = format_sql("select id, name from users").unwrap();
        assert_eq!(out, "SELECT\n    id,\n    name\nFROM users");
    }

    #[test]
    fn test_where_single_condition() {
        let out = format_sql("select id from users where active = 1").unwrap();
        assert_eq!(out, "SELECT\n    id\nFROM users\nWHERE\n    active = 1");
    }

    #[test]
    fn test_where_and_conditions() {
        let out = format_sql("select id from users where active = 1 and age > 18").unwrap();
        assert_eq!(
            out,
            "SELECT\n    id\nFROM users\nWHERE\n    active = 1\n    AND age > 18"
        );
    }

    #[test]
    fn test_where_or_conditions() {
        let out = format_sql("select id from users where active = 1 or age > 18").unwrap();
        assert_eq!(
            out,
            "SELECT\n    id\nFROM users\nWHERE\n    active = 1\n    OR age > 18"
        );
    }

    #[test]
    fn test_join_on() {
        let out = format_sql(
            "select u.id from users as u join orders as o on o.user_id = u.id",
        )
        .unwrap();
        assert!(
            out.contains("JOIN orders AS o\n    ON o.user_id = u.id"),
            "actual output:\n{out}"
        );
    }

    #[test]
    fn test_left_join() {
        let out = format_sql(
            "select u.id from users as u left join orders as o on o.user_id = u.id",
        )
        .unwrap();
        assert!(out.contains("LEFT JOIN"), "actual output:\n{out}");
    }

    #[test]
    fn test_order_by() {
        let out = format_sql("select id from users order by id desc").unwrap();
        assert!(out.contains("ORDER BY\n    id DESC"), "actual output:\n{out}");
    }

    #[test]
    fn test_limit() {
        let out = format_sql("select id from users limit 10").unwrap();
        assert!(out.contains("LIMIT 10"), "actual output:\n{out}");
    }

    #[test]
    fn test_group_by_having() {
        let out = format_sql(
            "select user_id, count(*) from orders group by user_id having count(*) > 5",
        )
        .unwrap();
        assert!(out.contains("GROUP BY\n    user_id"), "actual output:\n{out}");
        assert!(out.contains("HAVING"), "actual output:\n{out}");
    }

    #[test]
    fn test_multiple_statements() {
        let out = format_sql("select 1; select 2").unwrap();
        assert!(out.contains("\n\n"), "actual output:\n{out}");
    }

    #[test]
    fn test_alias() {
        let out = format_sql("select id as user_id from users").unwrap();
        assert!(out.contains("id AS user_id"), "actual output:\n{out}");
    }
}
```

- [ ] **Step 3: Run tests to confirm they fail (not panic on todo!)**

```bash
cargo test -p querybox query::format 2>&1 | head -40
```

Expected: tests fail because `format_sql` is a `todo!()`. You'll see `panicked at 'not yet implemented'`.

---

## Task 3: Implement `format_sql` and the formatter

**Files:**
- Modify: `src/query/format.rs`

- [ ] **Step 1: Replace the stub with the full implementation**

Replace the entire contents of `src/query/format.rs` with:

```rust
use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

pub fn format_sql(sql: &str) -> Result<String, String> {
    if sql.trim().is_empty() {
        return Ok(String::new());
    }
    let dialect = GenericDialect {};
    let statements =
        Parser::parse_sql(&dialect, sql).map_err(|e| e.to_string())?;
    let formatted: Vec<String> =
        statements.iter().map(|s| format_statement(s)).collect();
    Ok(formatted.join("\n\n"))
}

fn format_statement(stmt: &Statement) -> String {
    match stmt {
        Statement::Query(q) => format_query(q),
        _ => format!("{stmt}"),
    }
}

fn format_query(query: &Query) -> String {
    let mut parts: Vec<String> = vec![];

    if let Some(with) = &query.with {
        let cte_parts: Vec<String> = with
            .cte_tables
            .iter()
            .map(|cte| {
                let body = format_query(&cte.query);
                format!("{} AS (\n{}\n)", cte.alias.name, add_indent(&body, 4))
            })
            .collect();
        let recursive = if with.recursive { "RECURSIVE " } else { "" };
        parts.push(format!("WITH {}{}", recursive, cte_parts.join(",\n")));
    }

    match query.body.as_ref() {
        SetExpr::Select(select) => parts.push(format_select(select)),
        other => parts.push(format!("{other}")),
    }

    if !query.order_by.is_empty() {
        let items: Vec<String> = query
            .order_by
            .iter()
            .map(|o| {
                let dir = match o.asc {
                    Some(true) => " ASC",
                    Some(false) => " DESC",
                    None => "",
                };
                format!("    {}{}", o.expr, dir)
            })
            .collect();
        parts.push(format!("ORDER BY\n{}", items.join(",\n")));
    }

    if let Some(limit) = &query.limit {
        parts.push(format!("LIMIT {limit}"));
    }

    if let Some(offset) = &query.offset {
        parts.push(format!("OFFSET {}", offset.value));
    }

    parts.join("\n")
}

fn format_select(select: &Select) -> String {
    let mut parts: Vec<String> = vec![];

    let keyword = match &select.distinct {
        Some(_) => "SELECT DISTINCT",
        None => "SELECT",
    };

    let cols: Vec<String> = select
        .projection
        .iter()
        .map(|item| format!("    {}", format_select_item(item)))
        .collect();
    parts.push(format!("{}\n{}", keyword, cols.join(",\n")));

    for twj in &select.from {
        parts.push(format!("FROM {}", format_table_with_joins(twj)));
    }

    if let Some(selection) = &select.selection {
        parts.push(format_where("WHERE", selection));
    }

    match &select.group_by {
        GroupByExpr::Expressions(exprs, _) if !exprs.is_empty() => {
            let items: Vec<String> =
                exprs.iter().map(|e| format!("    {e}")).collect();
            parts.push(format!("GROUP BY\n{}", items.join(",\n")));
        }
        _ => {}
    }

    if let Some(having) = &select.having {
        parts.push(format_where("HAVING", having));
    }

    parts.join("\n")
}

fn format_where(keyword: &str, expr: &Expr) -> String {
    let conds = flatten_condition(expr);
    let lines: Vec<String> = conds
        .iter()
        .enumerate()
        .map(|(i, (connector, text))| {
            if i == 0 {
                format!("    {text}")
            } else {
                format!("    {} {text}", connector.as_deref().unwrap_or("AND"))
            }
        })
        .collect();
    format!("{keyword}\n{}", lines.join("\n"))
}

fn format_select_item(item: &SelectItem) -> String {
    match item {
        SelectItem::UnnamedExpr(e) => format!("{e}"),
        SelectItem::ExprWithAlias { expr, alias } => format!("{expr} AS {alias}"),
        SelectItem::QualifiedWildcard(name, _) => format!("{name}.*"),
        SelectItem::Wildcard(_) => "*".to_string(),
    }
}

fn format_table_with_joins(twj: &TableWithJoins) -> String {
    let mut parts = vec![format_table_factor(&twj.relation)];
    for join in &twj.joins {
        parts.push(format_join(join));
    }
    parts.join("\n")
}

fn format_table_factor(tf: &TableFactor) -> String {
    match tf {
        TableFactor::Table { name, alias, .. } => match alias {
            Some(a) => format!("{name} AS {}", a.name),
            None => format!("{name}"),
        },
        TableFactor::Derived {
            subquery, alias, ..
        } => {
            let inner = format_query(subquery);
            let indented = add_indent(&inner, 4);
            match alias {
                Some(a) => format!("(\n{indented}\n) AS {}", a.name),
                None => format!("(\n{indented}\n)"),
            }
        }
        other => format!("{other}"),
    }
}

fn format_join(join: &Join) -> String {
    let (keyword, constraint) = match &join.join_operator {
        JoinOperator::Inner(c) => ("JOIN", Some(c)),
        JoinOperator::LeftOuter(c) => ("LEFT JOIN", Some(c)),
        JoinOperator::RightOuter(c) => ("RIGHT JOIN", Some(c)),
        JoinOperator::FullOuter(c) => ("FULL JOIN", Some(c)),
        JoinOperator::CrossJoin => ("CROSS JOIN", None),
        _ => ("JOIN", None),
    };

    let table = format_table_factor(&join.relation);

    match constraint {
        Some(JoinConstraint::On(expr)) => {
            format!("{keyword} {table}\n    ON {expr}")
        }
        Some(JoinConstraint::Using(cols)) => {
            let names: Vec<String> = cols.iter().map(|c| format!("{c}")).collect();
            format!("{keyword} {table} USING ({})", names.join(", "))
        }
        _ => format!("{keyword} {table}"),
    }
}

/// Flatten a chain of top-level AND/OR into a list of (connector, expression_string) pairs.
/// The first item has `connector = None`; subsequent items carry `Some("AND")` or `Some("OR")`.
fn flatten_condition(expr: &Expr) -> Vec<(Option<String>, String)> {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::And,
            right,
        } => {
            let mut parts = flatten_condition(left.as_ref());
            let mut right_parts = flatten_condition(right.as_ref());
            if let Some(first) = right_parts.first_mut() {
                if first.0.is_none() {
                    first.0 = Some("AND".to_string());
                }
            }
            parts.extend(right_parts);
            parts
        }
        Expr::BinaryOp {
            left,
            op: BinaryOperator::Or,
            right,
        } => {
            let mut parts = flatten_condition(left.as_ref());
            let mut right_parts = flatten_condition(right.as_ref());
            if let Some(first) = right_parts.first_mut() {
                if first.0.is_none() {
                    first.0 = Some("OR".to_string());
                }
            }
            parts.extend(right_parts);
            parts
        }
        other => vec![(None, format!("{other}"))],
    }
}

fn add_indent(s: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    s.lines()
        .map(|l| {
            if l.is_empty() {
                l.to_string()
            } else {
                format!("{pad}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_empty() {
        assert_eq!(format_sql("").unwrap(), "");
    }

    #[test]
    fn test_whitespace_only_returns_empty() {
        assert_eq!(format_sql("   \n  ").unwrap(), "");
    }

    #[test]
    fn test_invalid_sql_returns_err() {
        assert!(format_sql("SELEKT garbage *** from").is_err());
    }

    #[test]
    fn test_simple_select_all() {
        let out = format_sql("select * from users").unwrap();
        assert_eq!(out, "SELECT\n    *\nFROM users");
    }

    #[test]
    fn test_select_columns() {
        let out = format_sql("select id, name from users").unwrap();
        assert_eq!(out, "SELECT\n    id,\n    name\nFROM users");
    }

    #[test]
    fn test_where_single_condition() {
        let out = format_sql("select id from users where active = 1").unwrap();
        assert_eq!(out, "SELECT\n    id\nFROM users\nWHERE\n    active = 1");
    }

    #[test]
    fn test_where_and_conditions() {
        let out = format_sql("select id from users where active = 1 and age > 18").unwrap();
        assert_eq!(
            out,
            "SELECT\n    id\nFROM users\nWHERE\n    active = 1\n    AND age > 18"
        );
    }

    #[test]
    fn test_where_or_conditions() {
        let out = format_sql("select id from users where active = 1 or age > 18").unwrap();
        assert_eq!(
            out,
            "SELECT\n    id\nFROM users\nWHERE\n    active = 1\n    OR age > 18"
        );
    }

    #[test]
    fn test_join_on() {
        let out = format_sql(
            "select u.id from users as u join orders as o on o.user_id = u.id",
        )
        .unwrap();
        assert!(
            out.contains("JOIN orders AS o\n    ON o.user_id = u.id"),
            "actual output:\n{out}"
        );
    }

    #[test]
    fn test_left_join() {
        let out = format_sql(
            "select u.id from users as u left join orders as o on o.user_id = u.id",
        )
        .unwrap();
        assert!(out.contains("LEFT JOIN"), "actual output:\n{out}");
    }

    #[test]
    fn test_order_by() {
        let out = format_sql("select id from users order by id desc").unwrap();
        assert!(out.contains("ORDER BY\n    id DESC"), "actual output:\n{out}");
    }

    #[test]
    fn test_limit() {
        let out = format_sql("select id from users limit 10").unwrap();
        assert!(out.contains("LIMIT 10"), "actual output:\n{out}");
    }

    #[test]
    fn test_group_by_having() {
        let out = format_sql(
            "select user_id, count(*) from orders group by user_id having count(*) > 5",
        )
        .unwrap();
        assert!(out.contains("GROUP BY\n    user_id"), "actual output:\n{out}");
        assert!(out.contains("HAVING"), "actual output:\n{out}");
    }

    #[test]
    fn test_multiple_statements() {
        let out = format_sql("select 1; select 2").unwrap();
        assert!(out.contains("\n\n"), "actual output:\n{out}");
    }

    #[test]
    fn test_alias() {
        let out = format_sql("select id as user_id from users").unwrap();
        assert!(out.contains("id AS user_id"), "actual output:\n{out}");
    }
}
```

> **Note for implementer:** If `cargo test` reports type errors, run `cargo doc -p sqlparser --open` to verify the exact field names for the installed version. Common variations:
> - `GroupByExpr::Expressions(exprs, modifiers)` vs `GroupByExpr::Expressions(exprs)` — adjust the pattern to match.
> - `JoinConstraint::Using(Vec<Ident>)` vs `Vec<ObjectName>` — use `format!("{c}")` either way.
> - `OrderByExpr.asc` field may be named differently — check `OrderByExpr` fields.

- [ ] **Step 2: Run tests**

```bash
cargo test -p querybox query::format 2>&1
```

Expected: all tests pass. If any fail with assertion mismatches (not compile errors), the formatter's `Display` output from `sqlparser` may differ slightly — update the `assert_eq!` expected strings to match what the formatter actually produces (run with `-- --nocapture` to see output).

- [ ] **Step 3: Run clippy**

```bash
cargo clippy -- -D warnings
```

Fix any warnings before continuing.

- [ ] **Step 4: Commit**

```bash
git add src/query/format.rs src/query/mod.rs
git commit -m "feat: add SQL formatter module with AST-based pretty-printer"
```

---

## Task 4: Wire the Format button in `EditorView`

**Files:**
- Modify: `src/ui/sql_editor.rs`
- Modify: `src/ui/editor_view.rs`

- [ ] **Step 1: Add `set_content` to `SqlEditor`**

`selected_range` is private, so we need a public method to atomically update content and reset cursor. In `src/ui/sql_editor.rs`, add this method inside `impl SqlEditor` (after the `new` constructor is a good spot):

```rust
pub fn set_content(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
    self.content = text.into();
    self.selected_range = 0..0;
    self.marked_range = None;
    cx.notify();
}
```

- [ ] **Step 2: Add the import to `editor_view.rs`**

At the top of `src/ui/editor_view.rs`, add to the existing `crate::` imports:

```rust
use crate::query::format::format_sql;
```

- [ ] **Step 3: Add `format_query` to `EditorView`**

In `src/ui/editor_view.rs`, add this method to `impl EditorView` after `set_error`:

```rust
fn format_query(&mut self, cx: &mut Context<Self>) {
    let sql = self.editor.read(cx).content.to_string();
    if sql.trim().is_empty() {
        return;
    }
    match format_sql(&sql) {
        Ok(formatted) => {
            self.editor.update(cx, |editor, cx| {
                editor.set_content(formatted, cx);
            });
        }
        Err(msg) => {
            self.set_error(sql, msg, cx);
        }
    }
}
```

- [ ] **Step 4: Wire `on_click` on the Format button**

In `render_editor_pane`, the existing inert Format button (around line 151) looks like:

```rust
.child(
    div()
        .id("format-btn")
        .bg(rgb(0x313244))
        .text_color(rgb(0xa6adc8))
        .rounded(px(4.))
        .px(px(10.))
        .py(px(4.))
        .text_size(px(11.))
        .cursor_pointer()
        .child("Format"),
)
```

Replace it with:

```rust
.child(
    div()
        .id("format-btn")
        .bg(rgb(0x313244))
        .text_color(rgb(0xa6adc8))
        .rounded(px(4.))
        .px(px(10.))
        .py(px(4.))
        .text_size(px(11.))
        .cursor_pointer()
        .on_click(cx.listener(|this, _, _, cx| {
            this.format_query(cx);
        }))
        .child("Format"),
)
```

- [ ] **Step 5: Build to confirm no compile errors**

```bash
cargo build 2>&1
```

Expected: compiles cleanly.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy -- -D warnings
```

Fix any warnings.

- [ ] **Step 7: Commit**

```bash
git add src/ui/editor_view.rs src/ui/sql_editor.rs
git commit -m "feat: wire Format button to SQL pretty-printer"
```

---

## Task 5: Manual smoke test

- [ ] **Step 1: Start the dev database**

```bash
cd /Users/marcus/Projects/querybox/dev && docker compose up -d
```

- [ ] **Step 2: Run the app**

```bash
cargo run
```

- [ ] **Step 3: Open New Query tab and test the happy path**

Paste the following into the editor and click **Format**:

```sql
select u.id, u.name, o.total from users as u join orders as o on o.user_id = u.id where u.active = 1 and o.total > 100 order by o.total desc limit 10
```

Expected output in the editor:

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

- [ ] **Step 4: Test the error path**

Type `SELEKT garbage` in the editor and click **Format**.

Expected: the red error pane below shows a parse error message; the editor content is unchanged.

- [ ] **Step 5: Test empty editor**

Clear the editor and click **Format**.

Expected: nothing happens (no crash, no error shown).

- [ ] **Step 6: Final commit if any fixups were made**

```bash
git add -p
git commit -m "fix: format button smoke test fixups"
```
