# Run Highlighted SQL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When text is selected in the SQL editor, clicking Run (or pressing `Cmd+Enter`) executes only the selected text; when nothing is selected, the full buffer runs as before.

**Architecture:** Add a `selected_sql() -> Option<&str>` accessor to `SqlEditor` that returns the selected text when the selection is non-empty. Update `EditorView::run_query()` to call this accessor and fall back to the full buffer. Register a `RunQuery` action on `EditorView` bound to `Cmd+Enter`.

**Tech Stack:** Rust, GPUI (Zed's UI framework), `#[gpui::test]` for unit tests.

---

## Files

| File | Change |
|---|---|
| `src/ui/sql_editor.rs` | Add `selected_sql()` public method; add two unit tests |
| `src/ui/editor_view.rs` | Declare `RunQuery` action; add `register_editor_view_actions()`; update `run_query()`; wire `on_action` + `key_context` in `render()` |
| `src/main.rs` | Call `register_editor_view_actions(cx)` alongside the other action registrations |

---

## Task 1: Add `selected_sql()` to `SqlEditor`

**Files:**
- Modify: `src/ui/sql_editor.rs`

- [ ] **Step 1: Write the failing tests**

Add these two tests to the `mod tests` block at the bottom of `src/ui/sql_editor.rs` (inside the existing `#[cfg(test)] mod tests { ... }`). The test module is a child of the `sql_editor` module, so it can access private fields directly.

```rust
#[gpui::test]
fn test_selected_sql_no_selection(cx: &mut gpui::TestAppContext) {
    let editor = cx.new(SqlEditor::new);
    editor.update(cx, |editor, cx| {
        editor.set_content("SELECT 1", cx);
        // selected_range is 0..0 after set_content — no selection
    });
    editor.read_with(cx, |editor, _| {
        assert_eq!(editor.selected_sql(), None);
    });
}

#[gpui::test]
fn test_selected_sql_with_selection(cx: &mut gpui::TestAppContext) {
    let editor = cx.new(SqlEditor::new);
    editor.update(cx, |editor, cx| {
        editor.set_content("SELECT 1", cx);
        editor.selected_range = 0..6; // covers "SELECT"
    });
    editor.read_with(cx, |editor, _| {
        assert_eq!(editor.selected_sql(), Some("SELECT"));
    });
}
```

Also add `use super::SqlEditor;` inside `mod tests` if it isn't already imported (it currently isn't — the existing tests only import `build_text_runs`).

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_selected_sql 2>&1 | tail -20
```

Expected: compile error — `selected_sql` method does not exist yet. (If tests fail to compile for an unrelated GPUI reason, note the error and proceed to the implementation step; the GPUI test harness may need a window context — see note below.)

> **Note:** If `#[gpui::test]` requires a display/window context not available in CI, you can convert these to plain `#[test]` functions that call a free helper function `fn selected_sql_from(content: &str, range: std::ops::Range<usize>) -> Option<String>` that takes content and range as arguments. Only do this if the GPUI test approach fails to compile or run.

- [ ] **Step 3: Implement `selected_sql()`**

In `src/ui/sql_editor.rs`, add the following method to the `impl SqlEditor` block, just after `set_content()` (around line 70):

```rust
pub fn selected_sql(&self) -> Option<&str> {
    if self.selected_range.is_empty() {
        None
    } else {
        Some(&self.content[self.selected_range.clone()])
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test test_selected_sql 2>&1 | tail -20
```

Expected: both tests pass.

- [ ] **Step 5: Verify full test suite still passes**

```bash
cargo test 2>&1 | tail -10
```

Expected: all existing tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/ui/sql_editor.rs
git commit -m "feat: add selected_sql() accessor to SqlEditor"
```

---

## Task 2: Update `run_query()` to execute selection when present

**Files:**
- Modify: `src/ui/editor_view.rs`

- [ ] **Step 1: Replace the SQL extraction in `run_query()`**

In `src/ui/editor_view.rs`, replace the current `run_query()` method (lines 75–116) with the following. The key changes are:
1. Extract `sql` using `selected_sql()` with fallback to full content, in a block so the `Ref<SqlEditor>` is dropped before mutating `self`
2. Derive `sql_clone` from `sql` before it moves into the async closure, eliminating the redundant second read of `editor.content` on the old line 101

```rust
fn run_query(&mut self, cx: &mut Context<Self>) {
    let sql = {
        let editor = self.editor.read(cx);
        editor
            .selected_sql()
            .unwrap_or(editor.content.as_ref())
            .to_string()
    };
    if sql.trim().is_empty() {
        return;
    }
    let Some(driver) = self.driver.clone() else {
        return;
    };
    self.running = true;
    self.error = None;
    self.result = None;
    cx.notify();

    let sql_clone = sql.clone();
    let database = self.database.clone();
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<QueryResult, String>>();
    crate::db_runtime().spawn(async move {
        match driver.query_in(database.as_deref(), &sql, &[]).await {
            Ok(result) => {
                tx.send(Ok(result)).ok();
            }
            Err(e) => {
                tx.send(Err(e.to_string())).ok();
            }
        }
    });

    cx.spawn(
        async move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| match rx.await {
            Ok(Ok(result)) => {
                this.update(cx, |ev, cx| ev.set_result(sql_clone, result, cx))
                    .ok();
            }
            Ok(Err(e)) => {
                this.update(cx, |ev, cx| ev.set_error(sql_clone, e, cx))
                    .ok();
            }
            Err(_) => {}
        },
    )
    .detach();
}
```

- [ ] **Step 2: Verify it builds**

```bash
cargo build 2>&1 | grep -E "^error"
```

Expected: no output (no errors).

- [ ] **Step 3: Commit**

```bash
git add src/ui/editor_view.rs
git commit -m "feat: run only selected SQL when a selection exists"
```

---

## Task 3: Add `RunQuery` action and `Cmd+Enter` keybinding

**Files:**
- Modify: `src/ui/editor_view.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Declare the `RunQuery` action in `editor_view.rs`**

At the top of `src/ui/editor_view.rs`, after the existing `use` statements and before the `pub struct EditorView` declaration, add:

```rust
actions!(editor_view, [RunQuery]);

pub fn register_editor_view_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("cmd-enter", RunQuery, Some("EditorView"))]);
}
```

- [ ] **Step 2: Wire `key_context` and `on_action` in `EditorView::render()`**

Replace the `render()` method body in `src/ui/editor_view.rs` (currently the `impl Render for EditorView` block starting at line 119):

```rust
impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("EditorView")
            .on_action(cx.listener(|this, _: &RunQuery, _, cx| {
                this.run_query(cx);
            }))
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_editor_pane(cx))
            .child(self.render_results_pane())
    }
}
```

- [ ] **Step 3: Register the new actions in `main.rs`**

In `src/main.rs`, add a call to `register_editor_view_actions` alongside the existing registrations (currently lines 36–38):

```rust
ui::text_field::register_text_field_actions(cx);
ui::sql_editor::register_sql_editor_actions(cx);
ui::table_view::register_table_view_actions(cx);
ui::editor_view::register_editor_view_actions(cx);  // add this line
```

Also add the import at the top of the `application().run(...)` block. The function is accessible as `ui::editor_view::register_editor_view_actions` since `editor_view` is already a public module under `ui`.

- [ ] **Step 4: Verify it builds**

```bash
cargo build 2>&1 | grep -E "^error"
```

Expected: no output. If you see `unused import` warnings about `RunQuery`, ensure the `on_action` handler references it.

- [ ] **Step 5: Smoke test**

Start the app and connect to the dev database (`cd dev && docker compose up -d` if not running):

```bash
cargo run
```

1. Open a New Query tab.
2. Type `SELECT 1; SELECT 2;` in the editor.
3. Select only `SELECT 1` with the mouse.
4. Press `Cmd+Enter` — the results panel should show 1 row with value `1`.
5. Click somewhere to clear the selection, then press `Cmd+Enter` — both statements run and you see the result of the last one (or an error if the driver doesn't support multi-statement — either way, the full buffer was used, not just `SELECT 1`).
6. With `SELECT 2` selected, click the **Run** button — results show `2`.

- [ ] **Step 6: Commit**

```bash
git add src/ui/editor_view.rs src/main.rs
git commit -m "feat: add Cmd+Enter keybinding to run query in editor view"
```

---

## Task 4: Mark TODO complete

**Files:**
- Modify: `TODO.md`

- [ ] **Step 1: Mark the item done**

In `TODO.md`, change:

```markdown
- [ ] Run only highlighted SQL in "New Query" view
```

to:

```markdown
- [x] Run only highlighted SQL in "New Query" view
```

- [ ] **Step 2: Commit**

```bash
git add TODO.md
git commit -m "chore: mark run-highlighted-sql as complete in TODO"
```
