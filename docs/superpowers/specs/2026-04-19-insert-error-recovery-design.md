---
name: Insert Error Recovery
description: Row insert errors are reflected inline in the new row UI without breaking the tab; recoverable by editing a field
type: feature
status: approved
---

# Insert Error Recovery

## Problem

When a row insert fails (e.g. constraint violation, duplicate key), the current code calls `view.set_error(e, cx)`, which:

1. Replaces the entire table grid with a full-screen red error message.
2. Destroys all new-row state — the user's typed values are gone.

The tab is effectively broken until the user navigates away and back. There is no in-place recovery.

---

## Goal

Insert errors are shown inline in the new row UI. The Insert button is disabled until the user edits at least one field, preventing blind re-submission of the same broken data. Cancelling the row remains available at all times.

---

## Scope

Two files change: `src/ui/table_view.rs` and `src/ui/app_view.rs`.

---

## State Changes — `TableView`

Two new fields:

```rust
new_row_insert_error: Option<String>,  // error message from the last failed INSERT
new_row_dirty: bool,                   // true once the user edits any cell after an error
```

Initialised to `None` and `false` respectively in `TableView::new`.

### `set_data` (success / reload)

Add to existing clear block:

```rust
self.new_row_insert_error = None;
self.new_row_dirty = false;
```

### `save_new_row`

Remove the early state-clearing lines that currently run before emitting the event:

```rust
// REMOVE these three lines:
self.new_row_active = false;
self.new_row_edits.clear();
self.editing_new_row_col = None;
```

The row survives until either a successful reload clears it (via `set_data`) or the user cancels it. Do not clear `new_row_insert_error` or `new_row_dirty` here either — that is done by `set_insert_error` on failure.

### `set_insert_error` (new method)

```rust
pub fn set_insert_error(&mut self, error: String, cx: &mut Context<Self>) {
    self.new_row_insert_error = Some(error);
    self.new_row_dirty = false;
    cx.notify();
}
```

### `cancel_new_row`

Already clears `new_row_active`, `new_row_edits`, and `editing_new_row_col`. Add:

```rust
self.new_row_insert_error = None;
self.new_row_dirty = false;
```

### New row cell click handler

When the user clicks a new-row cell to start editing it, set `new_row_dirty = true` and clear `new_row_insert_error` so the error banner disappears as soon as they engage with the row:

```rust
this.new_row_dirty = true;
this.new_row_insert_error = None;
```

---

## Rendering — toolbar error strip

When `new_row_insert_error.is_some()`, render an error strip in the toolbar directly above the
button row — same position and style as the existing `save_error` strip:

```rust
if let Some(ref err) = self.new_row_insert_error {
    toolbar = toolbar.child(
        div()
            .px(px(12.))
            .py(px(4.))
            .text_size(px(11.))
            .text_color(rgb(0xf38ba8))
            .child(err.clone()),
    );
}
```

Both `save_error` and `new_row_insert_error` may appear simultaneously; each renders its own strip.

## Rendering — new row row background

The new-row row background switches based on error state:

| Condition | Background |
|---|---|
| No error | `rgba(0xa6e3a115)` — green tint (unchanged) |
| Error present | `rgba(0xf38ba815)` — red tint |

Background reverts to green as soon as the user clicks any new-row cell (which clears
`new_row_insert_error`).

## Rendering — Insert button

The button area retains Insert and Cancel. Insert has two states:

| Condition | Appearance | Clickable |
|---|---|---|
| No error, or error+dirty | Green (`0xa6e3a1`), dark text | Yes |
| Error + not dirty | Grey (`0x45475a`), muted text | No (`on_click` omitted, no `cursor_pointer`) |

The "enabled" predicate: `self.new_row_insert_error.is_none() || self.new_row_dirty`.

---

## AppView change

In `AppView::execute_insert`, the error arm:

**Before:**
```rust
Ok(Err(e)) => {
    view.update(cx, |v, cx| v.set_error(e, cx));
}
```

**After:**
```rust
Ok(Err(e)) => {
    view.update(cx, |v, cx| v.set_insert_error(e, cx));
}
```

---

## Error handling

- Only insert errors are affected. `set_error` continues to be used for query/load errors (those still replace the grid — correct behaviour for a broken table state).
- The `set_insert_error` path does not reload or modify the grid data in any way.

---

## Success path (unchanged)

On a successful INSERT, `execute_insert` calls `query_table` as before. `query_table` calls `set_data` on completion, which clears `new_row_active` and the two new fields. The user sees the refreshed table with their inserted row.

---

## Testing notes

Manual test scenarios:

1. Insert a row that violates a NOT NULL constraint → error appears inline, Insert dims, Cancel still works.
2. Edit any field after the error → error disappears, Insert re-enables.
3. Submit again with the same bad value → error re-appears, Insert dims again.
4. Submit with a valid value → new row disappears, table reloads with the inserted row.
5. Cancel after error → new row disappears cleanly, no error lingers.
