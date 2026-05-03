---
title: Text Editor Keybindings
date: 2026-05-03
status: approved
---

# Text Editor Keybindings

## Overview

Add standard macOS text editor keybindings to all text input components in QueryBox: `SqlEditor` (multi-line) and `TextField` (single-line). Required bindings include `cmd-left/right/up/down`, shift-selection extension, option/alt word navigation, word-level deletion, line deletion, and undo/redo.

## Scope

Applies to:
- `src/ui/sql_editor.rs` — multi-line SQL editor
- `src/ui/text_field.rs` — single-line input fields (connection config, filters, etc.)

## Architecture

### New module: `src/ui/text_motion.rs`

Pure functions with no GPUI dependencies. Take `&str` + `usize` byte offset, return `usize` byte offset.

```
line_start(text, offset) -> usize
line_end(text, offset) -> usize
prev_word_start(text, offset) -> usize
next_word_end(text, offset) -> usize
```

Word boundary convention (Mac-style): skip whitespace in the direction of travel, then skip a contiguous run of the same character class (alphanumeric+underscore vs. punctuation).

Both components import these functions. Existing grapheme-level `previous_boundary`/`next_boundary` helpers remain unchanged for plain arrow keys.

### Changes to `SqlEditor` and `TextField`

**Reuse existing actions** — add new keystroke bindings pointing at them:

| Keystroke | Existing action |
|-----------|----------------|
| `cmd-left` | `Home` |
| `cmd-right` | `End` |

**New actions for both components:**

| Keystroke(s) | New action |
|---|---|
| `alt-left` | `MovePrevWord` |
| `alt-right` | `MoveNextWord` |
| `shift-left` | `SelectLeft` |
| `shift-right` | `SelectRight` |
| `shift-home`, `shift-cmd-left` | `SelectLineStart` |
| `shift-end`, `shift-cmd-right` | `SelectLineEnd` |
| `shift-alt-left` | `SelectPrevWord` |
| `shift-alt-right` | `SelectNextWord` |
| `alt-backspace` | `DeleteWordBack` |
| `alt-delete` | `DeleteWordForward` |
| `cmd-backspace` | `DeleteToLineStart` |
| `cmd-z` | `Undo` |
| `cmd-shift-z` | `Redo` |

**New actions for `SqlEditor` only** (multi-line):

| Keystroke(s) | New action |
|---|---|
| `cmd-up` | `MoveDocStart` |
| `cmd-down` | `MoveDocEnd` |
| `shift-up` | `SelectUp` |
| `shift-down` | `SelectDown` |
| `shift-cmd-up` | `SelectDocStart` |
| `shift-cmd-down` | `SelectDocEnd` |

## Handler Patterns

All handlers follow the existing conventions in the codebase:

- **Move actions**: call `self.move_to(text_motion::fn(...), cx)` — collapses any selection
- **Select actions**: call `self.select_to(target_offset, cx)` — extends from current anchor
- **Delete actions**: if selection is empty, select to the target boundary first; then `replace_text_in_range(None, "", window, cx)` — same pattern as existing `on_backspace`/`on_delete`. Applies to `DeleteWordBack`, `DeleteWordForward`, and `DeleteToLineStart`.

`SelectUp`/`SelectDown` in `SqlEditor` use the existing `cursor_line_col()` + `offset_at_line_col()` helpers, calling `select_to` instead of `move_to`.

New bindings are registered in each component's existing `register_*_actions` function. Handlers are wired in each component's `render()` via `.on_action(cx.listener(...))`.

## Undo / Redo

Both `SqlEditor` and `TextField` gain two new fields:

```rust
undo_stack: Vec<(SharedString, Range<usize>)>,
redo_stack: Vec<(SharedString, Range<usize>)>,
```

Each entry is a `(content, selected_range)` snapshot taken **before** a mutation.

**Recording**: `replace_text_in_range` pushes the current state onto `undo_stack` before mutating, and clears `redo_stack` (any edit that is not an undo/redo breaks the redo chain).

**`Undo` handler** (`cmd-z`): pop from `undo_stack`, push current state onto `redo_stack`, restore the popped snapshot.

**`Redo` handler** (`cmd-shift-z`): pop from `redo_stack`, push current state onto `undo_stack`, restore the popped snapshot.

Granularity is one undo step per `replace_text_in_range` call (i.e., per keystroke). Consecutive-character grouping is out of scope for this task.

## Files Changed

| File | Change |
|------|--------|
| `src/ui/text_motion.rs` | New file — pure movement calculation functions |
| `src/ui/sql_editor.rs` | New actions, keybindings, and handlers |
| `src/ui/text_field.rs` | New actions, keybindings, and handlers |
| `src/ui/mod.rs` | Expose `text_motion` module |

## Out of Scope

- Consecutive-character undo grouping (each keystroke is its own undo step for now)
- Vertical selection extension in `TextField` — single-line only, not applicable
