# Design: Auto-select default database on connect

**Date:** 2026-05-02
**Status:** Approved

## Problem

When a user fills in the "Database (optional)" field in the connection configuration dialog and connects, the `default_database` value is stored in `ConnectionProfile` and used by the MySQL/Postgres drivers to establish the connection to that database. However, the sidebar dropdown does not automatically select it — the user must manually click the database in the sidebar before tables load.

## Goal

After connecting, if `default_database` is set in the profile, automatically select that database in the sidebar dropdown and load its tables — identical to the user clicking the database manually.

## Approach

Extend `load_databases` in `src/ui/app_view.rs` with an `auto_select: Option<String>` parameter.

After the database list is fetched and written to the sidebar, if `auto_select` is `Some(db)` and `db` is present in the fetched list, the function:
1. Sets `sidebar.selected_database = Some(db.clone())`
2. Clears `sidebar.tables`
3. Calls `AppView::load_tables(driver, db, sidebar, cx)` via the existing `WeakEntity<AppView>` in the async closure (currently unused as `_this`)

### Call site change

In the connect event handler (`app_view.rs` ~line 107), after `app_view.connection_manager = manager`:

```rust
let auto_select = app_view.connection_manager.active_profile
    .as_ref()
    .and_then(|p| p.default_database.clone());
AppView::load_databases(driver, sidebar, auto_select, cx);
```

### `load_databases` signature change

```rust
fn load_databases(
    driver: Arc<dyn DatabaseDriver>,
    sidebar: Entity<Sidebar>,
    auto_select: Option<String>,
    cx: &mut Context<Self>,
)
```

Inside the async spawn, the driver must be cloned for use in the `load_tables` call:

```rust
let driver2 = driver.clone();
// ...after sidebar.databases is set:
if let Some(db) = auto_select {
    if databases.contains(&db) {
        sidebar.update(cx, |s, cx| {
            s.selected_database = Some(db.clone());
            s.tables = vec![];
            cx.notify();
        });
        this.update(cx, |app_view, cx| {
            AppView::load_tables(driver2, db, app_view.sidebar.clone(), cx);
        }).ok();
    }
}
```

## Edge cases

| Scenario | Behaviour |
|----------|-----------|
| `default_database` is `None` | No auto-select; sidebar behaves as before |
| Specified database does not exist on server | `databases.contains(&db)` is false; no selection, no `load_tables` call |
| Database exists and is valid | Selected in dropdown, tables loaded immediately |

## Files changed

- `src/ui/app_view.rs` — modify `load_databases` signature and body; update call site in connect handler

## Out of scope

- No changes to `ConnectionProfile`, `ConnectionManager`, or the connection dialog
- No changes to database switching behaviour after initial connection
