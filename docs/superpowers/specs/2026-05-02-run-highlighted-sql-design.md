# Design: Run Highlighted SQL in New Query View

**Date:** 2026-05-02  
**Status:** Approved

## Overview

When the user has text selected in the SQL editor, pressing Run (or `Cmd+Enter`) executes only the selected text instead of the full buffer. When nothing is selected, the full buffer runs as before.

## Scope

Two files change:

- `src/ui/sql_editor.rs` — add `selected_sql()` helper method
- `src/ui/editor_view.rs` — update `run_query()` to use selection; add `RunQuery` action and `Cmd+Enter` keybinding

No changes to the database driver, result rendering, or any other module.

## Data Flow

### `SqlEditor` — new method

```rust
pub fn selected_sql(&self) -> Option<&str> {
    if self.selected_range.is_empty() {
        None
    } else {
        Some(&self.content[self.selected_range.clone()])
    }
}
```

`selected_range` is already maintained by the editor for copy/cut/selection-rendering. This method is a thin read-only accessor — no new state.

### `EditorView::run_query()` — updated SQL extraction

Current (line 76):
```rust
let sql = self.editor.read(cx).content.to_string();
```

Becomes:
```rust
let editor = self.editor.read(cx);
let sql = editor.selected_sql()
    .unwrap_or(&editor.content)
    .to_string();
```

Everything downstream (async task spawn, `query_in`, result/error rendering) is unchanged.

## Keybinding

A `RunQuery` action is registered on `EditorView` and bound to `Cmd+Enter` scoped to the editor view's key context. The handler calls the existing `run_query()` method, identical to the click handler on the Run button.

```rust
// In EditorView action registration:
actions!(editor_view, [RunQuery]);
KeyBinding::new("cmd-enter", RunQuery, Some("EditorView"));

// Handler:
cx.on_action(|this: &mut EditorView, _: &RunQuery, _, cx| {
    this.run_query(cx);
});
```

`SqlEditor` is not involved in the keybinding — `EditorView` owns the action, keeping SQL execution logic centralized there.

> **Implementation note:** The string passed to `KeyBinding::new` as the context (`"EditorView"`) must match the key context set via `cx.set_key_context()` in `EditorView`'s `render()` method. Verify or add this context string during implementation.

## Behaviour Summary

| Scenario | Result |
|---|---|
| Nothing selected, click Run | Full buffer executes (unchanged) |
| Nothing selected, `Cmd+Enter` | Full buffer executes |
| Text selected, click Run | Selected text executes |
| Text selected, `Cmd+Enter` | Selected text executes |
| Selected text is whitespace-only | Empty check in `run_query()` rejects it (unchanged guard) |

## What Does Not Change

- Run button label stays "Run" in all cases
- Result panel, error display, loading state — all unchanged
- Database driver interface — unchanged
- All other keybindings — unchanged
