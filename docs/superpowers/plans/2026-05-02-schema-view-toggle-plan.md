# Schema View Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Data/Schema pill toggle to `TableView`'s toolbar so users can switch between the row grid and the table's column/index schema without opening a new tab.

**Architecture:** `ViewMode` enum lives in `table_view.rs`. `TableView` owns an `Entity<SchemaView>` created at construction. `AppView::load_schema` fires immediately alongside the existing data and foreign-key loads in `open_table`, fetching columns and indexes via the driver and pushing results into the view. The toggle pill in the toolbar flips `view_mode`; the main render branches on it.

**Tech Stack:** Rust, GPUI, `DatabaseDriver::columns` + `DatabaseDriver::indexes` (both already on the trait)

**Spec:** `docs/superpowers/specs/2026-05-02-schema-view-toggle-design.md`

---

## File Map

```
src/ui/table_view.rs    — add ViewMode, schema_view field, set_schema, toolbar toggle, render branch
src/ui/schema_view.rs   — remove #![allow(dead_code)]
src/app_view.rs         — add load_schema(), call it from open_table()
```

---

### Task 1: Add `ViewMode`, `schema_view`, and `set_schema` to `TableView`

**Files:**
- Modify: `src/ui/table_view.rs`

No visual change yet — this task just adds the data model and method. Build verifies correctness.

- [ ] **Step 1: Extend the imports at the top of `src/ui/table_view.rs`**

The file currently imports `Column` but not `Index`, and doesn't import `SchemaView`. Find this line:

```rust
use crate::db::types::{text_to_value, Column, QueryResult, Row, Value};
```

Replace it with:

```rust
use crate::db::types::{text_to_value, Column, Index, QueryResult, Row, Value};
```

Then add this import directly below the existing `use super::text_field::TextField;` line:

```rust
use super::schema_view::SchemaView;
```

- [ ] **Step 2: Add `ViewMode` enum after the `actions!` macro block**

Insert after the closing `;` of the `actions!(...)` call and before `pub fn register_table_view_actions`:

```rust
#[derive(Clone, Debug, PartialEq, Default)]
enum ViewMode {
    #[default]
    Data,
    Schema,
}
```

- [ ] **Step 3: Add two new fields to the `TableView` struct**

At the bottom of the `TableView` struct, after `pub foreign_keys: Vec<crate::db::types::ForeignKey>,`, add:

```rust
    view_mode: ViewMode,
    schema_view: Entity<SchemaView>,
```

- [ ] **Step 4: Initialize the new fields in `TableView::new`**

`TableView::new` currently starts with:

```rust
pub fn new(database: String, table_name: String, cx: &mut Context<Self>) -> Self {
    Self {
        database,
        table_name,
```

`database` and `table_name` move into `Self`, so clone them first. Replace the entire `new` function signature + opening with:

```rust
pub fn new(database: String, table_name: String, cx: &mut Context<Self>) -> Self {
    let schema_view = cx.new(|_| SchemaView::new(database.clone(), table_name.clone()));
    Self {
        database,
        table_name,
```

Then at the end of the `Self { ... }` block, after `foreign_keys: vec![],`, add:

```rust
        view_mode: ViewMode::Data,
        schema_view,
```

- [ ] **Step 5: Add `set_schema` method to `TableView`**

Add this public method inside the `impl TableView` block that contains `set_foreign_keys` (around line 172), after `set_foreign_keys`:

```rust
pub fn set_schema(
    &mut self,
    columns: Vec<Column>,
    indexes: Vec<Index>,
    cx: &mut Context<Self>,
) {
    self.schema_view
        .update(cx, |sv, cx| sv.set_schema(columns, indexes, cx));
}
```

- [ ] **Step 6: Verify it compiles**

```bash
cargo build 2>&1 | head -30
```

Expected: no errors. Warnings about unused `view_mode` / `schema_view` are fine — they go away in Task 2.

---

### Task 2: Add the Data/Schema toggle to the toolbar and branch the render

**Files:**
- Modify: `src/ui/table_view.rs`

After this task, opening any table tab shows a Data/Schema pill in the toolbar. Clicking Schema shows the loading spinner (schema data arrives in Task 3).

- [ ] **Step 1: Add the toggle pills to `render_toolbar`**

In `render_toolbar`, find where `button_row` is initialised (the large `let mut button_row = div() ...` block). The block currently starts with `.child(div().id("filter-btn") ...)` as its first child.

Insert two pill children **before** the `filter-btn` child and add a thin separator after them. Replace the opening of the `button_row` initialiser up to (but not including) the `filter-btn` child:

```rust
let view_mode = self.view_mode.clone();

let mut button_row = div()
    .flex()
    .flex_row()
    .items_center()
    .px(px(12.))
    .py(px(8.))
    .gap_2()
    .child(
        div()
            .id("view-mode-data")
            .bg(if view_mode == ViewMode::Data {
                rgb(0x89b4fa)
            } else {
                rgb(0x313244)
            })
            .text_color(if view_mode == ViewMode::Data {
                rgb(0x1e1e2e)
            } else {
                rgb(0x6c7086)
            })
            .font_weight(if view_mode == ViewMode::Data {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            })
            .rounded(px(4.))
            .px(px(10.))
            .py(px(4.))
            .text_size(px(11.))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                this.view_mode = ViewMode::Data;
                cx.notify();
            }))
            .child("Data"),
    )
    .child(
        div()
            .id("view-mode-schema")
            .bg(if view_mode == ViewMode::Schema {
                rgb(0x89b4fa)
            } else {
                rgb(0x313244)
            })
            .text_color(if view_mode == ViewMode::Schema {
                rgb(0x1e1e2e)
            } else {
                rgb(0x6c7086)
            })
            .font_weight(if view_mode == ViewMode::Schema {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            })
            .rounded(px(4.))
            .px(px(10.))
            .py(px(4.))
            .text_size(px(11.))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                this.view_mode = ViewMode::Schema;
                cx.notify();
            }))
            .child("Schema"),
    )
    .child(
        div()
            .w(px(1.))
            .h(px(16.))
            .bg(rgb(0x45475a))
            .flex_shrink_0(),
    )
    .child(
        div()
            .id("filter-btn")
            // ... rest of filter-btn unchanged
```

The remainder of the `button_row` block (filter-btn, new-row-btn, save buttons, spacer, row info, pagination) stays exactly as it is.

- [ ] **Step 2: Update `Render::render` to branch on `view_mode`**

The current `Render` impl ends with:

```rust
.child(self.render_toolbar(cx))
.when(self.filter_form_visible, |d| {
    d.child(self.render_filter_form(cx))
})
.child(self.render_grid(cx))
```

Replace those three lines with:

```rust
.child(self.render_toolbar(cx))
.when(
    self.filter_form_visible && self.view_mode == ViewMode::Data,
    |d| d.child(self.render_filter_form(cx)),
)
.child(match self.view_mode {
    ViewMode::Data => self.render_grid(cx).into_any_element(),
    ViewMode::Schema => self.schema_view.clone().into_any_element(),
})
```

- [ ] **Step 3: Build and verify**

```bash
cargo build 2>&1 | head -30
```

Expected: clean build. Run the app and open a table — you should see Data/Schema pills in the toolbar. Clicking Schema shows "Loading schema..." (no data yet).

```bash
cargo run
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/table_view.rs
git commit -m "feat: add Data/Schema toggle to table view toolbar"
```

---

### Task 3: Wire schema loading in `AppView` and clean up dead-code attributes

**Files:**
- Modify: `src/app_view.rs`
- Modify: `src/ui/schema_view.rs`

After this task the Schema tab populates immediately when a table opens.

- [ ] **Step 1: Add `Column` and `Index` to the imports in `src/app_view.rs`**

Find:

```rust
use crate::db::types::Value;
```

Replace with:

```rust
use crate::db::types::{Column, Index, Value};
```

- [ ] **Step 2: Add `load_schema` to `AppView`**

Add this static method to the `impl AppView` block that contains `load_foreign_keys` (add it directly after `load_foreign_keys`):

```rust
fn load_schema(
    driver: Arc<dyn DatabaseDriver>,
    database: String,
    table: String,
    view: Entity<TableView>,
    cx: &mut Context<Self>,
) {
    let (tx, rx) =
        tokio::sync::oneshot::channel::<Result<(Vec<Column>, Vec<Index>), String>>();
    let db = database.clone();
    let tbl = table.clone();
    crate::db_runtime().spawn(async move {
        let columns = match driver.columns(&db, &tbl).await {
            Ok(c) => c,
            Err(e) => {
                tx.send(Err(e.to_string())).ok();
                return;
            }
        };
        let indexes = match driver.indexes(&db, &tbl).await {
            Ok(i) => i,
            Err(e) => {
                tx.send(Err(e.to_string())).ok();
                return;
            }
        };
        tx.send(Ok((columns, indexes))).ok();
    });
    cx.spawn(async move |_this: WeakEntity<AppView>, cx: &mut AsyncApp| {
        if let Ok(Ok((columns, indexes))) = rx.await {
            view.update(cx, |v, cx| v.set_schema(columns, indexes, cx))
                .ok();
        }
    })
    .detach();
}
```

- [ ] **Step 3: Call `load_schema` from `open_table`**

In `open_table`, find the last two lines (the final two static-method calls):

```rust
AppView::query_table(
    driver.clone(),
    database.clone(),
    table.clone(),
    initial_filters,
    dialect,
    0,
    view.clone(),
    cx,
);
AppView::load_foreign_keys(driver, database, table, view, cx);
```

Replace with:

```rust
AppView::query_table(
    driver.clone(),
    database.clone(),
    table.clone(),
    initial_filters,
    dialect,
    0,
    view.clone(),
    cx,
);
AppView::load_schema(driver.clone(), database.clone(), table.clone(), view.clone(), cx);
AppView::load_foreign_keys(driver, database, table, view, cx);
```

- [ ] **Step 4: Remove `#![allow(dead_code)]` from `src/ui/schema_view.rs`**

Delete the first line of the file:

```rust
#![allow(dead_code)]
```

- [ ] **Step 5: Build and verify**

```bash
cargo build 2>&1 | head -30
```

Expected: clean build, no new warnings.

Run the app, connect to the dev database (`cd dev && docker compose up -d` first if not running), open any table, click Schema — columns and indexes should appear.

```bash
cargo run
```

Verify:
- Data pill is active by default
- Clicking Schema shows column names, types, nullable, key info, and indexes
- Clicking back to Data returns to the row grid with all filters/pagination intact
- The filter form does not appear while in Schema mode

- [ ] **Step 6: Commit**

```bash
git add src/app_view.rs src/ui/schema_view.rs
git commit -m "feat: load table schema in background and display in Schema tab"
```
