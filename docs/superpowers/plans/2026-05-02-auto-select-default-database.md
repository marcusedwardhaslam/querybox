# Auto-select Default Database on Connect — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After connecting with a `default_database` set in the connection profile, automatically select that database in the sidebar and load its tables.

**Architecture:** Add an `auto_select: Option<String>` parameter to `load_databases` in `app_view.rs`. After the database list loads, if the value matches an entry, set `sidebar.selected_database` and call `load_tables` via the existing `WeakEntity<AppView>` in the async closure. Update the connect event handler to pass `profile.default_database` as `auto_select`.

**Tech Stack:** Rust, GPUI

---

## File Map

| File | Change |
|------|--------|
| `src/ui/app_view.rs` | Modify `load_databases` signature + body; update its call site in the connect handler |

---

### Task 1: Modify `load_databases` to accept and act on `auto_select`

**Files:**
- Modify: `src/ui/app_view.rs:509-534`

- [ ] **Step 1: Replace the `load_databases` function**

  Open `src/ui/app_view.rs`. Find the `load_databases` function (currently at ~line 509). Replace it entirely with:

  ```rust
  fn load_databases(
      driver: Arc<dyn DatabaseDriver>,
      sidebar: Entity<Sidebar>,
      auto_select: Option<String>,
      cx: &mut Context<Self>,
  ) {
      let driver2 = driver.clone();
      let (tx, rx) = tokio::sync::oneshot::channel::<Result<Vec<String>, String>>();
      crate::db_runtime().spawn(async move {
          match driver.databases().await {
              Ok(dbs) => {
                  tx.send(Ok(dbs)).ok();
              }
              Err(e) => {
                  tx.send(Err(e.to_string())).ok();
              }
          }
      });
      cx.spawn(async move |this: WeakEntity<AppView>, cx: &mut AsyncApp| {
          if let Ok(Ok(databases)) = rx.await {
              let auto_select_match = auto_select
                  .as_ref()
                  .filter(|db| databases.contains(db))
                  .cloned();
              sidebar.update(cx, |s, cx| {
                  s.databases = databases;
                  cx.notify();
              });
              if let Some(db) = auto_select_match {
                  sidebar.update(cx, |s, cx| {
                      s.selected_database = Some(db.clone());
                      s.tables = vec![];
                      cx.notify();
                  });
                  this.update(cx, |app_view, cx| {
                      AppView::load_tables(driver2, db, app_view.sidebar.clone(), cx);
                  })
                  .ok();
              }
          }
      })
      .detach();
  }
  ```

  Key differences from the original:
  - New `auto_select: Option<String>` parameter
  - `driver2 = driver.clone()` — needed for `load_tables` call inside the async closure
  - `_this` renamed to `this` and used for `this.update(cx, ...)` to call `load_tables`
  - `auto_select_match` computed before `databases` is moved into the sidebar update closure

- [ ] **Step 2: Verify it compiles**

  ```bash
  cargo build 2>&1 | head -40
  ```

  Expected: compile error about mismatched argument count at the `load_databases` call site (we haven't updated it yet). Something like:
  ```
  error[E0061]: this function takes 4 arguments but 3 arguments were supplied
  ```

  This is expected — the call site still passes 3 args.

---

### Task 2: Update the call site in the connect event handler

**Files:**
- Modify: `src/ui/app_view.rs:106-109`

- [ ] **Step 1: Update the call site**

  Find the block starting around line 106 that reads:

  ```rust
  // Load databases immediately after connect
  if let Some(driver) = app_view.connection_manager.driver_arc() {
      AppView::load_databases(driver, sidebar, cx);
  }
  ```

  Replace it with:

  ```rust
  // Load databases immediately after connect; auto-select default_database if set
  let auto_select = app_view
      .connection_manager
      .active_profile
      .as_ref()
      .and_then(|p| p.default_database.clone());
  if let Some(driver) = app_view.connection_manager.driver_arc() {
      AppView::load_databases(driver, sidebar, auto_select, cx);
  }
  ```

- [ ] **Step 2: Build cleanly**

  ```bash
  cargo build 2>&1
  ```

  Expected: no errors, no warnings related to these changes.

- [ ] **Step 3: Run clippy**

  ```bash
  cargo clippy 2>&1
  ```

  Expected: clean — no new warnings.

- [ ] **Step 4: Commit**

  ```bash
  git add src/ui/app_view.rs
  git commit -m "feat: auto-select default database in sidebar after connecting"
  ```

---

### Task 3: Manual verification

- [ ] **Step 1: Start the dev database**

  ```bash
  cd dev && docker compose up -d
  ```

- [ ] **Step 2: Run the app**

  ```bash
  cargo run
  ```

- [ ] **Step 3: Test auto-select path**

  1. Open the connection dialog
  2. Fill in host `localhost`, port `3306`, user `queryuser`, password `querypass`, database `querybox`
  3. Click Connect
  4. Verify: the sidebar dropdown shows `querybox` as selected and the tables (`users`, `orders`) load immediately — no manual database click required

- [ ] **Step 4: Test blank database field path**

  1. Edit the connection (or create a new one) with the database field left blank
  2. Click Connect
  3. Verify: databases load in the sidebar dropdown but none is selected and no tables are shown — same behaviour as before this change

- [ ] **Step 5: Mark "Make database field connect directly" as done in TODO.md**

  In `TODO.md`, change:

  ```markdown
  - [ ] Make "database" field in connection configuration window connect directly to that database
  ```

  to:

  ```markdown
  - [x] Make "database" field in connection configuration window connect directly to that database
  ```

  ```bash
  git add TODO.md
  git commit -m "chore: mark default-database auto-select as complete in TODO"
  ```
