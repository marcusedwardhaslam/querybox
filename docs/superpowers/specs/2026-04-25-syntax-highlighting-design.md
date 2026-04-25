# Syntax Highlighting for the SQL Editor

**Date:** 2026-04-25

## Goal

Color SQL tokens in the `SqlEditor` component so keywords, strings, numbers, operators, and comments are visually distinct. Uses the Catppuccin Mocha palette already present throughout the app.

## Scope

- Full semantic token coloring: keywords, strings, numbers, operators, comments, identifiers (default)
- Runs live on every render — no manual trigger, no debounce
- No new dependencies — reuses the `sqlparser` crate already in `Cargo.toml`

Out of scope: bracket matching, error underlining, autocomplete, hover tooltips.

## Architecture

### New module: `src/query/highlight.rs`

```rust
pub type HighlightSpan = (Range<usize>, Rgba);
pub fn highlight(sql: &str) -> Vec<HighlightSpan>;
```

`highlight()` tokenizes the SQL using `sqlparser::tokenizer::Tokenizer` with `GenericDialect`, then converts each token's `Location { line, column }` (1-indexed, character-based) to a byte range in the source string.

**Position conversion:** Before tokenizing, build `line_starts: Vec<usize>` — the byte offset of the first character of each line. For token _i_ at `(line, col)`, convert to byte offset via `line_starts[line-1] + char_nth_to_byte(&sql[line_starts[line-1]..], col-1)`, where `char_nth_to_byte` is implemented with `str::char_indices().nth(n)`. `Location` in sqlparser is 1-indexed for both line and column (verify against `Location::default()` in the source before shipping). The token's end byte is the start byte of the next token — `tokenize_with_location()` includes `Token::Whitespace` variants so the token stream is contiguous; `Token::EOF` terminates the list at `sql.len()`, so `end_last = sql.len()`.

**Error handling:** If the tokenizer returns an error (malformed SQL), `highlight()` returns an empty `Vec` — the editor falls back to flat default-color rendering with no visible glitch.

### Token → color mapping

| Token variant | Color | Hex |
|---|---|---|
| `Word` with `keyword != NoKeyword` | blue | `#89b4fa` |
| `SingleQuotedString`, `DoubleQuotedString`, `EscapedStringLiteral`, and other string variants | green | `#a6e3a1` |
| `Number` | peach | `#fab387` |
| `Whitespace::Comment` | muted gray | `#6c7086` |
| `Eq`, `Neq`, `Lt`, `Gt`, `LtEq`, `GtEq`, `Plus`, `Minus`, `Multiply`, `Divide`, `Modulo` | sky | `#89dceb` |
| Everything else (identifiers, punctuation, whitespace) | default text | `#cdd6f4` |

No span is emitted for whitespace or default-colored tokens — gaps in the span list are filled with default color during run construction.

### Changes to `src/ui/sql_editor.rs`

`SqlEditorElement::prepaint` currently shapes each line with a single `TextRun`. New behavior:

1. Call `highlight(&content)` once at the top of `prepaint` to get a sorted `Vec<HighlightSpan>`.
2. For each line, compute its `[line_start, line_end)` byte range.
3. Filter spans to those overlapping this range; clip each to the line boundary.
4. Walk forward from `pos = 0` (relative to line start), emitting:
   - A default-colored `TextRun` for any gap before the next span
   - A colored `TextRun` for the span itself
5. Emit a final default-colored run for any trailing bytes after the last span.
6. Pass the resulting `Vec<TextRun>` (summing to `line.len()` bytes) to `shape_line`.

Empty lines (`line.len() == 0`) use a single zero-length `TextRun` as before — GPUI handles this correctly.

### Module registration

Add `pub mod highlight;` to `src/query/mod.rs`.

## Files Changed

| File | Change |
|---|---|
| `src/query/highlight.rs` | New — `highlight()` function |
| `src/query/mod.rs` | Add `pub mod highlight` |
| `src/ui/sql_editor.rs` | Modify `prepaint` to use multi-run line shaping |
