# Schema View / Results View Toggle

**Date:** 2026-05-02  
**Status:** Approved

## Goal

Add a Data/Schema toggle to the table tab so users can inspect a table's column definitions and indexes without opening a separate view. Schema data loads in the background immediately when the table tab opens.

## Design

### Placement

A **Data / Schema pill toggle** sits in the `TableView` toolbar, to the right of the existing action buttons. Clicking it switches the content area in-place — same tab, no new tabs opened. This follows the TablePlus convention.

### Components affected

**`src/ui/table_view.rs`**

- Add `ViewMode` enum (`Data`, `Schema`; default `Data`) defined at the top of the file.
- Add two fields to `TableView`:
  - `view_mode: ViewMode`
  - `schema_view: Entity<SchemaView>`
- In `TableView::new`, create the `SchemaView` entity (`SchemaView::new(database, table_name)`) — it initialises with `loading: true`.
- Add `pub fn set_schema(&mut self, columns: Vec<Column>, indexes: Vec<Index>, cx: &mut Context<Self>)` — delegates to `self.schema_view.update(...)`.
- In the toolbar render, add the Data/Schema pill toggle after the existing action buttons. Active pill: solid blue background (`#89b4fa`/`#1e1e2e`). Inactive pill: muted background (`#313244`/`#6c7086`). Clicking each pill sets `self.view_mode` and calls `cx.notify()`.
- In the main content render, branch on `view_mode`: `Data` renders the existing data grid; `Schema` renders `self.schema_view.clone()`.

**`src/app_view.rs`**

- Add `fn load_schema(driver, database, table, view: Entity<TableView>, cx)` static method. It spawns an async task that calls `driver.columns(&database, &table).await` and `driver.indexes(&database, &table).await` sequentially, then sends both results back and calls `view.set_schema(columns, indexes, cx)`.
- In `open_table()`, call `AppView::load_schema(driver.clone(), database.clone(), table.clone(), view.clone(), cx)` directly after the existing `AppView::load_foreign_keys(...)` call.

**`src/ui/schema_view.rs`**

- Remove `#![allow(dead_code)]` — the component is now in active use.

### Data flow

```
open_table()
  ├── query_table()          → TableView::set_data()
  ├── load_foreign_keys()    → TableView::set_foreign_keys()
  └── load_schema()          → TableView::set_schema()
                                 └── SchemaView::set_schema()
```

All three fire concurrently from `open_table`. Schema arrives whenever the driver responds — `SchemaView` shows "Loading schema..." until then.

### No new files, no new events, no tab bar changes.

## Out of scope

- Filtering/searching columns in the schema view
- Editing column definitions
- Showing foreign key relationships in the schema view
