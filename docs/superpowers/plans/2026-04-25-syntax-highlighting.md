# Syntax Highlighting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Color SQL tokens (keywords, strings, numbers, operators, comments) in the `SqlEditor` component using the Catppuccin Mocha palette.

**Architecture:** A new `highlight()` function tokenizes SQL using sqlparser's `Tokenizer`, converts token locations to byte ranges, and maps token types to RGBA colors. `SqlEditorElement::prepaint` calls `highlight()` once per render and splits each line into multiple `TextRun`s instead of one.

**Tech Stack:** Rust, GPUI, sqlparser 0.54 (already in `Cargo.toml`)

---

## File Map

| File | Change |
|---|---|
| `src/query/highlight.rs` | **Create** — `highlight()` function + inline `#[cfg(test)]` tests |
| `src/query/mod.rs` | **Modify** — add `pub mod highlight` |
| `src/ui/sql_editor.rs` | **Modify** — add `build_text_runs` helper, update `SqlEditorElement::prepaint` |

---

## Task 1: Create `src/query/highlight.rs`

**Files:**
- Create: `src/query/highlight.rs`

- [ ] **Step 1: Write the failing tests**

Add this file in its entirety — the tests will fail because the module doesn't exist yet.

```rust
// src/query/highlight.rs

use std::ops::Range;

use gpui::{rgba, Rgba};
use sqlparser::dialect::GenericDialect;
use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};

pub type HighlightSpan = (Range<usize>, Rgba);

pub fn highlight(_sql: &str) -> Vec<HighlightSpan> {
    vec![] // placeholder — tests will fail
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_no_spans() {
        assert!(highlight("").is_empty());
    }

    #[test]
    fn test_keyword_is_blue() {
        let spans = highlight("SELECT");
        assert_eq!(spans.len(), 1);
        assert_eq!(&"SELECT"[spans[0].0.clone()], "SELECT");
        assert_eq!(spans[0].1, rgba(0x89b4faff));
    }

    #[test]
    fn test_string_is_green() {
        let spans = highlight("'hello'");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0xa6e3a1ff));
    }

    #[test]
    fn test_number_is_peach() {
        let spans = highlight("42");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0xfab387ff));
    }

    #[test]
    fn test_identifier_has_no_span() {
        let spans = highlight("users");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_single_line_comment_is_muted() {
        let spans = highlight("-- a comment");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x6c7086ff));
    }

    #[test]
    fn test_block_comment_is_muted() {
        let spans = highlight("/* block */");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x6c7086ff));
    }

    #[test]
    fn test_eq_operator_is_sky() {
        let spans = highlight("=");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x89dcebff));
    }

    #[test]
    fn test_multiline_keywords() {
        let sql = "SELECT id\nFROM users";
        let spans = highlight(sql);
        let has_select = spans
            .iter()
            .any(|(r, c)| &sql[r.clone()] == "SELECT" && *c == rgba(0x89b4faff));
        let has_from = spans
            .iter()
            .any(|(r, c)| &sql[r.clone()] == "FROM" && *c == rgba(0x89b4faff));
        assert!(has_select, "SELECT not found in spans");
        assert!(has_from, "FROM not found in spans");
    }

    #[test]
    fn test_spans_within_bounds() {
        let sql = "SELECT id FROM users WHERE active = 1";
        let spans = highlight(sql);
        for (range, _) in &spans {
            assert!(
                range.end <= sql.len(),
                "span {range:?} out of bounds (sql len {})",
                sql.len()
            );
            assert!(range.start <= range.end, "inverted span {range:?}");
        }
    }

    #[test]
    fn test_malformed_sql_does_not_panic() {
        // Should return empty or partial spans — not panic
        let _ = highlight("SELECT !!! @@@ ###");
    }
}
```

- [ ] **Step 2: Register module so the tests compile**

Add `pub mod highlight;` to `src/query/mod.rs` (it currently has three lines):

```rust
pub mod filter;
pub mod format;
pub mod highlight;
pub mod history;
```

- [ ] **Step 3: Run tests to confirm they fail**

```
cargo test highlight -- --nocapture
```

Expected: compilation succeeds, but `test_keyword_is_blue`, `test_string_is_green`, etc. fail because `highlight()` returns `vec![]`.

- [ ] **Step 4: Implement `highlight()`**

Replace the placeholder `highlight()` (and add the helpers) — keep the tests unchanged:

```rust
// src/query/highlight.rs

use std::ops::Range;

use gpui::{rgba, Rgba};
use sqlparser::dialect::GenericDialect;
use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};

pub type HighlightSpan = (Range<usize>, Rgba);

pub fn highlight(sql: &str) -> Vec<HighlightSpan> {
    if sql.is_empty() {
        return vec![];
    }
    let dialect = GenericDialect {};
    let tokens = match Tokenizer::new(&dialect, sql).tokenize_with_location() {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    let line_starts = build_line_starts(sql);

    // Compute the byte-start of every token from its (line, col) location.
    let starts: Vec<usize> = tokens
        .iter()
        .map(|twl| {
            location_to_byte(
                sql,
                twl.location.line as usize,
                twl.location.column as usize,
                &line_starts,
            )
        })
        .collect();

    let mut spans = Vec::new();
    for (i, twl) in tokens.iter().enumerate() {
        let start = starts[i];
        // End of this token = start of the next token (stream is contiguous).
        // EOF token terminates at sql.len().
        let end = starts.get(i + 1).copied().unwrap_or(sql.len());
        if start >= end {
            continue;
        }
        if let Some(color) = token_color(&twl.token) {
            spans.push((start..end, color));
        }
    }
    spans
}

fn token_color(token: &Token) -> Option<Rgba> {
    match token {
        Token::Word(word) if word.keyword != Keyword::NoKeyword => {
            Some(rgba(0x89b4faff)) // blue — SQL keywords
        }
        Token::SingleQuotedString(_)
        | Token::DoubleQuotedString(_)
        | Token::EscapedStringLiteral(_)
        | Token::NationalStringLiteral(_)
        | Token::SingleQuotedByteStringLiteral(_)
        | Token::DoubleQuotedByteStringLiteral(_)
        | Token::HexStringLiteral(_) => Some(rgba(0xa6e3a1ff)), // green — strings
        Token::Number(_, _) => Some(rgba(0xfab387ff)),          // peach — numbers
        Token::Whitespace(Whitespace::SingleLineComment { .. })
        | Token::Whitespace(Whitespace::MultiLineComment(_)) => Some(rgba(0x6c7086ff)), // muted — comments
        Token::Eq
        | Token::Neq
        | Token::Lt
        | Token::Gt
        | Token::LtEq
        | Token::GtEq
        | Token::Plus
        | Token::Minus
        | Token::Multiply
        | Token::Divide
        | Token::Modulo => Some(rgba(0x89dcebff)), // sky — operators
        _ => None, // identifiers, punctuation, whitespace → default (no span)
    }
}

/// Byte offset of the first character on each line (0-indexed).
fn build_line_starts(sql: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(sql.char_indices().filter_map(|(i, c)| {
            if c == '\n' {
                Some(i + 1)
            } else {
                None
            }
        }))
        .collect()
}

/// Convert a sqlparser Location (1-indexed line, 1-indexed character column)
/// to a byte offset in `sql`.
fn location_to_byte(sql: &str, line: usize, col: usize, line_starts: &[usize]) -> usize {
    let line_start_byte = line_starts
        .get(line.saturating_sub(1))
        .copied()
        .unwrap_or(sql.len());
    let line_text = &sql[line_start_byte..];
    line_text
        .char_indices()
        .nth(col.saturating_sub(1))
        .map(|(b, _)| line_start_byte + b)
        .unwrap_or(line_start_byte + line_text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_no_spans() {
        assert!(highlight("").is_empty());
    }

    #[test]
    fn test_keyword_is_blue() {
        let spans = highlight("SELECT");
        assert_eq!(spans.len(), 1);
        assert_eq!(&"SELECT"[spans[0].0.clone()], "SELECT");
        assert_eq!(spans[0].1, rgba(0x89b4faff));
    }

    #[test]
    fn test_string_is_green() {
        let spans = highlight("'hello'");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0xa6e3a1ff));
    }

    #[test]
    fn test_number_is_peach() {
        let spans = highlight("42");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0xfab387ff));
    }

    #[test]
    fn test_identifier_has_no_span() {
        let spans = highlight("users");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_single_line_comment_is_muted() {
        let spans = highlight("-- a comment");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x6c7086ff));
    }

    #[test]
    fn test_block_comment_is_muted() {
        let spans = highlight("/* block */");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x6c7086ff));
    }

    #[test]
    fn test_eq_operator_is_sky() {
        let spans = highlight("=");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].1, rgba(0x89dcebff));
    }

    #[test]
    fn test_multiline_keywords() {
        let sql = "SELECT id\nFROM users";
        let spans = highlight(sql);
        let has_select = spans
            .iter()
            .any(|(r, c)| &sql[r.clone()] == "SELECT" && *c == rgba(0x89b4faff));
        let has_from = spans
            .iter()
            .any(|(r, c)| &sql[r.clone()] == "FROM" && *c == rgba(0x89b4faff));
        assert!(has_select, "SELECT not found in spans");
        assert!(has_from, "FROM not found in spans");
    }

    #[test]
    fn test_spans_within_bounds() {
        let sql = "SELECT id FROM users WHERE active = 1";
        let spans = highlight(sql);
        for (range, _) in &spans {
            assert!(
                range.end <= sql.len(),
                "span {range:?} out of bounds (sql len {})",
                sql.len()
            );
            assert!(range.start <= range.end, "inverted span {range:?}");
        }
    }

    #[test]
    fn test_malformed_sql_does_not_panic() {
        let _ = highlight("SELECT !!! @@@ ###");
    }
}
```

- [ ] **Step 5: Run tests to confirm they pass**

```
cargo test highlight -- --nocapture
```

Expected: all 10 tests pass.

If `test_keyword_is_blue` fails with a span count other than 1, sqlparser may be splitting `SELECT` into multiple tokens (unlikely but possible). Debug by printing the full span list.

If `test_eq_operator_is_sky` fails, the `=` token may have a different variant name in sqlparser 0.54 — check with `cargo doc --open` for `sqlparser::tokenizer::Token`.

- [ ] **Step 6: Run cargo clippy**

```
cargo clippy -- -D warnings
```

Expected: no warnings. Fix any that appear before continuing.

- [ ] **Step 7: Commit**

```bash
git add src/query/highlight.rs src/query/mod.rs
git commit -m "feat: add SQL syntax highlight tokenizer"
```

---

## Task 2: Wire highlight spans into `SqlEditorElement::prepaint`

**Files:**
- Modify: `src/ui/sql_editor.rs`

- [ ] **Step 1: Add `build_text_runs` helper at the bottom of `sql_editor.rs`**

Add this function after the closing `}` of `impl Element for SqlEditorElement` (around line 728, after the closing `}` of the `paint` method's outer impl block):

```rust
fn build_text_runs(
    line_text: &str,
    line_start: usize,
    spans: &[(std::ops::Range<usize>, gpui::Rgba)],
    style: &gpui::TextStyle,
    default_color: gpui::Rgba,
) -> Vec<gpui::TextRun> {
    let line_end = line_start + line_text.len();
    let mut runs: Vec<gpui::TextRun> = Vec::new();
    let mut pos = 0usize; // byte position within line_text

    for (range, color) in spans {
        let span_start = range.start.max(line_start);
        let span_end = range.end.min(line_end);
        if span_start >= span_end {
            continue;
        }
        let local_start = span_start - line_start;
        let local_end = span_end - line_start;

        if local_start > pos {
            runs.push(gpui::TextRun {
                len: local_start - pos,
                font: style.font(),
                color: default_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            });
        }
        runs.push(gpui::TextRun {
            len: local_end - local_start,
            font: style.font(),
            color: *color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
        pos = local_end;
    }

    if pos < line_text.len() {
        runs.push(gpui::TextRun {
            len: line_text.len() - pos,
            font: style.font(),
            color: default_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }

    // Empty line: one zero-length run so shape_line gets a valid (empty) slice.
    if runs.is_empty() {
        runs.push(gpui::TextRun {
            len: 0,
            font: style.font(),
            color: default_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }

    runs
}
```

- [ ] **Step 2: Replace the per-line loop in `prepaint`**

In `SqlEditorElement::prepaint` (around line 595), find this block:

```rust
        for raw_line in content.split('\n') {
            let display: SharedString = raw_line.to_string().into();
            let run = TextRun {
                len: display.len(),
                font: style.font(),
                color: style.color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = window
                .text_system()
                .shape_line(display.clone(), font_size, &[run], None);
            line_layouts.push((line_start, shaped));
            line_start += raw_line.len() + 1; // +1 for '\n'
        }
```

Replace it with:

```rust
        let highlight_spans = crate::query::highlight::highlight(&content);
        let default_color = style.color;

        for raw_line in content.split('\n') {
            let display: SharedString = raw_line.to_string().into();
            let runs = build_text_runs(raw_line, line_start, &highlight_spans, &style, default_color);
            let shaped = window
                .text_system()
                .shape_line(display.clone(), font_size, &runs, None);
            line_layouts.push((line_start, shaped));
            line_start += raw_line.len() + 1; // +1 for '\n'
        }
```

- [ ] **Step 3: Build to confirm it compiles**

```
cargo build 2>&1
```

Expected: compiles with no errors. Common issues and fixes:

- **"cannot find function `build_text_runs`"** — make sure the function is at the module level (not inside an `impl` block).
- **"mismatched types: expected `Rgba`, found `Rgb`"** — the `default_color` from `style.color` is already `Rgba`; no conversion needed.
- **"use of undeclared crate `highlight`"** — confirm `pub mod highlight;` was added to `src/query/mod.rs` in Task 1.

- [ ] **Step 4: Run all tests**

```
cargo test 2>&1
```

Expected: all tests pass (including the highlight tests from Task 1 and the format tests in `src/query/format.rs`).

- [ ] **Step 5: Run cargo clippy**

```
cargo clippy -- -D warnings
```

Expected: no warnings. Fix any that appear.

- [ ] **Step 6: Verify visually**

Start the dev database:
```
cd dev && docker compose up -d && cd ..
```

Run the app:
```
cargo run
```

Open the SQL editor tab and type or paste this query:

```sql
-- Get top customers
SELECT u.name, u.email, COUNT(o.id) AS order_count
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE u.active = 1
  AND u.created_at > '2024-01-01'
GROUP BY u.id, u.name, u.email
ORDER BY order_count DESC
LIMIT 25;
```

Confirm:
- `SELECT`, `FROM`, `LEFT JOIN`, `WHERE`, `AND`, `GROUP BY`, `ORDER BY`, `LIMIT`, `AS`, `ON`, `COUNT` → blue
- `'2024-01-01'` → green
- `1`, `25` → peach/orange
- `=`, `>` → sky/cyan
- `-- Get top customers` → muted gray
- `u.name`, `users`, `orders` (identifiers) → default white

Also confirm the editor still types, deletes, selects, and copies text normally — the highlighting must not break input handling.

- [ ] **Step 7: Commit**

```bash
git add src/ui/sql_editor.rs
git commit -m "feat: wire syntax highlighting into SQL editor prepaint"
```
