# Insert Error Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a new-row INSERT fails, keep the form open with data intact, highlight the row red, show the error in the toolbar, and let the user fix and resubmit.

**Architecture:** Add two fields (`new_row_insert_error`, `new_row_dirty`) to `TableView`. Stop pre-clearing new-row state in `save_new_row` — let success (`set_data`) or cancel clean up instead. Route insert errors to a new `set_insert_error` method (not the full-screen `set_error`). Render the error in the toolbar strip and the row background in red until the user edits a cell.

**Tech Stack:** Rust, GPUI

---

## Files Changed

- Modify: `src/ui/table_view.rs` — state fields, methods, render
- Modify: `src/ui/app_view.rs` — error routing in `execute_insert`

---

### Task 1: Add `new_row_insert_error` and `new_row_dirty` fields to `TableView`

**Files:**
- Modify: `src/ui/table_view.rs`

- [ ] **Step 1: Add the two fields to the struct definition**

In the `// New row insert` section of the `TableView` struct (around line 86), add the two new fields after `editing_new_row_col`:

```rust
    // New row insert
    new_row_active: bool,
    new_row_edits: HashMap<usize, String>,
    editing_new_row_col: Option<usize>,
    new_row_insert_error: Option<String>,
    new_row_dirty: bool,
```

- [ ] **Step 2: Initialise both fields in `TableView::new`**

In the `Self { ... }` initialiser (around line 121), after `editing_new_row_col: None,`:

```rust
            new_row_active: false,
            new_row_edits: HashMap::new(),
            editing_new_row_col: None,
            new_row_insert_error: None,
            new_row_dirty: false,
```

- [ ] **Step 3: Build**

```bash
cd /Users/marcus/Projects/querybox && cargo build 2>&1 | head -40
```

Expected: compiles clean (unused field warnings are fine at this stage).

- [ ] **Step 4: Commit**

```bash
git add src/ui/table_view.rs
git commit -m "feat(insert-error): add new_row_insert_error and new_row_dirty fields to TableView"
```

---

### Task 2: Add `set_insert_error` method and update `cancel_new_row` / `set_data`

**Files:**
- Modify: `src/ui/table_view.rs`

- [ ] **Step 1: Add `set_insert_error` after the existing `set_error` method (around line 155)**

```rust
    pub fn set_insert_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.new_row_insert_error = Some(error);
        self.new_row_dirty = false;
        cx.notify();
    }
```

- [ ] **Step 2: Clear the new fields in `set_data`**

The clear block in `set_data` (around line 144) currently ends:

```rust
        self.new_row_active = false;
        self.new_row_edits.clear();
        self.editing_new_row_col = None;
        cx.notify();
```

Replace with:

```rust
        self.new_row_active = false;
        self.new_row_edits.clear();
        self.editing_new_row_col = None;
        self.new_row_insert_error = None;
        self.new_row_dirty = false;
        cx.notify();
```

- [ ] **Step 3: Clear the new fields in `cancel_new_row` (around line 216)**

Replace:

```rust
    fn cancel_new_row(&mut self, cx: &mut Context<Self>) {
        self.new_row_active = false;
        self.new_row_edits.clear();
        self.editing_new_row_col = None;
        cx.notify();
    }
```

With:

```rust
    fn cancel_new_row(&mut self, cx: &mut Context<Self>) {
        self.new_row_active = false;
        self.new_row_edits.clear();
        self.editing_new_row_col = None;
        self.new_row_insert_error = None;
        self.new_row_dirty = false;
        cx.notify();
    }
```

- [ ] **Step 4: Build**

```bash
cd /Users/marcus/Projects/querybox && cargo build 2>&1 | head -40
```

Expected: compiles clean.

- [ ] **Step 5: Commit**

```bash
git add src/ui/table_view.rs
git commit -m "feat(insert-error): add set_insert_error, update set_data and cancel_new_row"
```

---

### Task 3: Remove premature state-clearing from `save_new_row`

**Files:**
- Modify: `src/ui/table_view.rs`

Currently `save_new_row` clears the row state before the async result is known. Remove those lines — cleanup now happens via `set_data` (success) or `cancel_new_row` (cancel).

- [ ] **Step 1: Remove the three clearing lines from `save_new_row` (around line 320)**

The current end of `save_new_row` is:

```rust
        cx.emit(TableViewEvent::InsertRow(NewRowInsert {
            database: self.database.clone(),
            table: self.table_name.clone(),
            column_values,
        }));
        self.new_row_active = false;
        self.new_row_edits.clear();
        self.editing_new_row_col = None;
        cx.notify();
    }
```

Replace with:

```rust
        cx.emit(TableViewEvent::InsertRow(NewRowInsert {
            database: self.database.clone(),
            table: self.table_name.clone(),
            column_values,
        }));
        cx.notify();
    }
```

- [ ] **Step 2: Build**

```bash
cd /Users/marcus/Projects/querybox && cargo build 2>&1 | head -40
```

Expected: compiles clean.

- [ ] **Step 3: Commit**

```bash
git add src/ui/table_view.rs
git commit -m "feat(insert-error): keep new-row state alive until success or cancel"
```

---

### Task 4: Wire `set_insert_error` into `AppView`

**Files:**
- Modify: `src/ui/app_view.rs`

- [ ] **Step 1: Replace `set_error` with `set_insert_error` in the `execute_insert` error arm (around line 477)**

Find:

```rust
                Ok(Err(e)) => {
                    view.update(cx, |v, cx| v.set_error(e, cx));
                }
```

Replace with:

```rust
                Ok(Err(e)) => {
                    view.update(cx, |v, cx| v.set_insert_error(e, cx));
                }
```

- [ ] **Step 2: Build**

```bash
cd /Users/marcus/Projects/querybox && cargo build 2>&1 | head -40
```

Expected: compiles clean.

- [ ] **Step 3: Commit**

```bash
git add src/ui/app_view.rs
git commit -m "feat(insert-error): route execute_insert errors to set_insert_error"
```

---

### Task 5: Render — toolbar error strip

**Files:**
- Modify: `src/ui/table_view.rs` — `render_toolbar`

The toolbar already shows `save_error` as a red text strip above the button row (around line 496). Add an identical strip for `new_row_insert_error` immediately after it.

- [ ] **Step 1: Add the insert error strip after the `save_error` strip**

Locate the existing save_error block:

```rust
        // Save error notice
        if let Some(ref err) = self.save_error {
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

Add immediately after it:

```rust
        // Insert error notice
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

- [ ] **Step 2: Build**

```bash
cd /Users/marcus/Projects/querybox && cargo build 2>&1 | head -40
```

Expected: compiles clean.

- [ ] **Step 3: Commit**

```bash
git add src/ui/table_view.rs
git commit -m "feat(insert-error): show insert error in toolbar strip"
```

---

### Task 6: Render — red row background, dirty flag on cell click, disabled Insert button

**Files:**
- Modify: `src/ui/table_view.rs` — `render_grid`

- [ ] **Step 1: Switch new-row row background and border based on error state**

The new-row div starts (around line 1075):

```rust
            let mut new_row_el = div()
                .flex()
                .flex_row()
                .bg(rgba(0xa6e3a115u32))
                .border_b_1()
                .border_color(rgb(0xa6e3a1));
```

Replace with:

```rust
            let has_insert_error = self.new_row_insert_error.is_some();
            let mut new_row_el = div()
                .flex()
                .flex_row()
                .bg(if has_insert_error {
                    rgba(0xf38ba815u32)
                } else {
                    rgba(0xa6e3a115u32)
                })
                .border_b_1()
                .border_color(if has_insert_error {
                    rgb(0xf38ba8)
                } else {
                    rgb(0xa6e3a1)
                });
```

- [ ] **Step 2: Set `new_row_dirty` and clear error when the user clicks a new-row cell**

The cell `on_click` closure (around line 1123) currently is:

```rust
                        .on_click(cx.listener(move |this, _event: &ClickEvent, window, cx| {
                            this.commit_edit(cx);
                            this.commit_new_row_edit(cx);
                            this.editing_new_row_col = Some(col_idx);
                            let val = this.new_row_edits.get(&col_idx).cloned().unwrap_or_default();
                            this.edit_field.update(cx, |f, cx| f.set_content(&val, cx));
                            let fh = this.edit_field.read(cx).focus_handle.clone();
                            window.focus(&fh, cx);
                            cx.notify();
                        }))
```

Replace with:

```rust
                        .on_click(cx.listener(move |this, _event: &ClickEvent, window, cx| {
                            this.commit_edit(cx);
                            this.commit_new_row_edit(cx);
                            this.editing_new_row_col = Some(col_idx);
                            this.new_row_dirty = true;
                            this.new_row_insert_error = None;
                            let val = this.new_row_edits.get(&col_idx).cloned().unwrap_or_default();
                            this.edit_field.update(cx, |f, cx| f.set_content(&val, cx));
                            let fh = this.edit_field.read(cx).focus_handle.clone();
                            window.focus(&fh, cx);
                            cx.notify();
                        }))
```

- [ ] **Step 3: Make the Insert button conditional on error + dirty state**

The Insert/Cancel button block (around line 1140) currently is:

```rust
            new_row_el = new_row_el.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .px(px(8.))
                    .child(
                        div()
                            .id("new-row-insert-btn")
                            .bg(rgb(0xa6e3a1))
                            .text_color(rgb(0x1e1e2e))
                            .font_weight(FontWeight::SEMIBOLD)
                            .rounded(px(4.))
                            .px(px(10.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.save_new_row(cx);
                            }))
                            .child("Insert"),
                    )
                    .child(
                        div()
                            .id("new-row-cancel-btn")
                            .bg(rgb(0x313244))
                            .text_color(rgb(0xa6adc8))
                            .rounded(px(4.))
                            .px(px(10.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_new_row(cx);
                            }))
                            .child("Cancel"),
                    ),
            );
```

Replace with:

```rust
            let insert_enabled = self.new_row_insert_error.is_none() || self.new_row_dirty;

            let insert_btn = if insert_enabled {
                div()
                    .id("new-row-insert-btn")
                    .bg(rgb(0xa6e3a1))
                    .text_color(rgb(0x1e1e2e))
                    .font_weight(FontWeight::SEMIBOLD)
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.save_new_row(cx);
                    }))
                    .child("Insert")
            } else {
                div()
                    .id("new-row-insert-btn")
                    .bg(rgb(0x45475a))
                    .text_color(rgb(0x6c7086))
                    .font_weight(FontWeight::SEMIBOLD)
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .child("Insert")
            };

            new_row_el = new_row_el.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .px(px(8.))
                    .child(insert_btn)
                    .child(
                        div()
                            .id("new-row-cancel-btn")
                            .bg(rgb(0x313244))
                            .text_color(rgb(0xa6adc8))
                            .rounded(px(4.))
                            .px(px(10.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_new_row(cx);
                            }))
                            .child("Cancel"),
                    ),
            );
```

- [ ] **Step 4: Build**

```bash
cd /Users/marcus/Projects/querybox && cargo build 2>&1 | head -40
```

Expected: compiles clean.

- [ ] **Step 5: Run clippy**

```bash
cd /Users/marcus/Projects/querybox && cargo clippy 2>&1 | head -60
```

Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/ui/table_view.rs
git commit -m "feat(insert-error): red row on error, clear on cell click, disable Insert button"
```

---

### Task 7: Manual verification

- [ ] **Step 1: Start the dev database and run the app**

```bash
cd /Users/marcus/Projects/querybox/dev && docker compose up -d
cd /Users/marcus/Projects/querybox && cargo run
```

- [ ] **Step 2: Verify error path**
  - Open the `users` table. Click **+ New Row**.
  - Enter a duplicate value for a UNIQUE column (e.g. an existing email or id).
  - Click **Insert**.
  - Expected: the new row row turns red. An error message appears in the toolbar strip above the button row. The Insert button turns grey/non-clickable. The table grid is still fully visible and usable.

- [ ] **Step 3: Verify editing clears the error**
  - Click any cell in the red new-row row.
  - Expected: the toolbar error strip disappears, the row background reverts to green, Insert re-enables (turns green and clickable).

- [ ] **Step 4: Verify re-submission of bad data**
  - Without fixing the bad value, click Insert again.
  - Expected: error re-appears, row turns red, Insert dims again.

- [ ] **Step 5: Verify successful insert**
  - Fix the bad value (enter a valid unique value), click Insert.
  - Expected: new row row disappears, table reloads and shows the inserted row.

- [ ] **Step 6: Verify cancel clears everything**
  - Trigger an error again. Click **Cancel**.
  - Expected: new row row disappears, no error strip in toolbar.
