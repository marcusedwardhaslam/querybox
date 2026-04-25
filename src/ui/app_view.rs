use gpui::*;
use std::sync::Arc;

use crate::connection::ConnectionManager;
use crate::db::types::Value;
use crate::db::{types::Dialect, DatabaseDriver};
use crate::query::filter::{filters_to_sql, Filter, FilterOp};

use super::connection_dialog::{ConnectionDialog, ConnectionDialogEvent};
use super::editor_view::EditorView;
use super::sidebar::{Sidebar, SidebarEvent};
use super::tab_bar::TabBar;
use super::table_view::{NewRowInsert, RowUpdate, TableView, TableViewEvent};

pub struct AppView {
    focus_handle: FocusHandle,
    connection_manager: ConnectionManager,
    sidebar: Entity<Sidebar>,
    tab_bar: Entity<TabBar>,
    connection_dialog: Entity<ConnectionDialog>,
    table_views: Vec<(usize, Entity<TableView>)>,
    editor_views: Vec<(usize, Entity<EditorView>)>,
    status_message: String,
}

impl AppView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|_| Sidebar::new());
        let tab_bar = cx.new(|_| TabBar::new());
        let connection_dialog = cx.new(ConnectionDialog::new);

        cx.subscribe(&sidebar, {
            let connection_dialog = connection_dialog.clone();
            move |this, _sidebar, event, cx| match event {
                SidebarEvent::OpenConnectionDialog => {
                    connection_dialog.update(cx, |dialog, cx| dialog.show(cx));
                }
                SidebarEvent::NewQuery => {
                    let driver = this.connection_manager.driver_arc();
                    let database = this.sidebar.read(cx).selected_database.clone();
                    let tab_id = this.tab_bar.update(cx, |bar, cx| bar.open_query(cx));
                    let view =
                        cx.new(|cx| super::editor_view::EditorView::new(driver, database, cx));
                    this.editor_views.push((tab_id, view));
                    cx.notify();
                }
                SidebarEvent::SelectDatabase(db) => {
                    let db = db.clone();
                    if let Some(driver) = this.connection_manager.driver_arc() {
                        this.sidebar.update(cx, |s, cx| {
                            s.selected_database = Some(db.clone());
                            s.tables = vec![];
                            cx.notify();
                        });
                        AppView::load_tables(driver, db, this.sidebar.clone(), cx);
                    }
                }
                SidebarEvent::OpenTable { database, table } => {
                    let database = database.clone();
                    let table = table.clone();
                    if let Some(driver) = this.connection_manager.driver_arc() {
                        AppView::open_table(this, driver, database, table, vec![], cx);
                    }
                }
            }
        })
        .detach();

        cx.subscribe(&connection_dialog, |this, _dialog, event, cx| match event {
            ConnectionDialogEvent::Connect { profile, password } => {
                let profile = profile.clone();
                let password = password.clone();
                this.status_message = "Connecting…".to_string();
                cx.notify();

                let (tx, rx) = tokio::sync::oneshot::channel::<Result<ConnectionManager, String>>();
                crate::db_runtime().spawn(async move {
                    let mut manager = ConnectionManager::new();
                    match manager.connect_new(profile, &password).await {
                        Ok(()) => {
                            tx.send(Ok(manager)).ok();
                        }
                        Err(e) => {
                            tx.send(Err(e.to_string())).ok();
                        }
                    }
                });

                let sidebar = this.sidebar.clone();
                cx.spawn(async move |this: WeakEntity<AppView>, cx: &mut AsyncApp| {
                    let result = rx.await;
                    this.update(cx, |app_view, cx| {
                        match result {
                            Ok(Ok(manager)) => {
                                if let Some((name, detail)) = manager.active_info() {
                                    app_view.sidebar.update(cx, |s, cx| {
                                        s.connection_name = Some(name);
                                        s.engine_info = Some(detail);
                                        s.databases = vec![];
                                        s.tables = vec![];
                                        cx.notify();
                                    });
                                    app_view.connection_manager = manager;
                                    app_view.status_message = "Connected".to_string();
                                    app_view.connection_dialog.update(cx, |d, cx| d.hide(cx));
                                    // Load databases immediately after connect
                                    if let Some(driver) = app_view.connection_manager.driver_arc() {
                                        AppView::load_databases(driver, sidebar, cx);
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                app_view.status_message = format!("Connection failed: {e}");
                            }
                            Err(_) => {
                                app_view.status_message = "Connection cancelled".to_string();
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            connection_manager: ConnectionManager::new(),
            sidebar,
            tab_bar,
            connection_dialog,
            table_views: vec![],
            editor_views: vec![],
            status_message: "Disconnected".to_string(),
        }
    }

    fn open_table(
        this: &mut AppView,
        driver: Arc<dyn DatabaseDriver>,
        database: String,
        table: String,
        initial_filters: Vec<Filter>,
        cx: &mut Context<Self>,
    ) {
        // Reuse existing tab/view if already open
        let tab_id = this.tab_bar.update(cx, |bar, cx| {
            bar.open_table(database.clone(), table.clone(), cx)
        });

        if this.table_views.iter().any(|(id, _)| *id == tab_id) {
            return; // already loaded
        }

        let view = cx.new(|cx| TableView::new(database.clone(), table.clone(), cx));
        this.table_views.push((tab_id, view.clone()));

        if !initial_filters.is_empty() {
            view.update(cx, |v, _cx| {
                v.active_filters = initial_filters.clone();
                v.page = 0;
            });
        }

        cx.notify();

        // Subscribe to events on this view
        let view2 = view.clone();
        cx.subscribe(
            &view,
            move |this, _entity, event: &TableViewEvent, cx| match event {
                TableViewEvent::FiltersChanged | TableViewEvent::PageChanged => {
                    if let Some(driver) = this.connection_manager.driver_arc() {
                        let tv = view2.read(cx);
                        let database = tv.database.clone();
                        let table = tv.table_name.clone();
                        let filters = tv.active_filters.clone();
                        let page = tv.page;
                        let dialect = this
                            .connection_manager
                            .driver()
                            .map(|d| d.dialect())
                            .unwrap_or(Dialect::MySql);
                        let _ = tv;
                        AppView::query_table(
                            driver,
                            database,
                            table,
                            filters,
                            dialect,
                            page,
                            view2.clone(),
                            cx,
                        );
                    }
                }
                TableViewEvent::SaveChanges(updates) => {
                    let updates = updates.clone();
                    if let Some(driver) = this.connection_manager.driver_arc() {
                        AppView::save_and_reload(driver, updates, view2.clone(), cx);
                    }
                }
                TableViewEvent::InsertRow(insert) => {
                    let insert = insert.clone();
                    if let Some(driver) = this.connection_manager.driver_arc() {
                        AppView::execute_insert(driver, insert, view2.clone(), cx);
                    }
                }
                TableViewEvent::NavigateToFk {
                    database,
                    table,
                    column,
                    value,
                } => {
                    let filter_value = match value {
                        Value::Int(n) => n.to_string(),
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    let filter = Filter {
                        column: column.clone(),
                        op: FilterOp::Equals,
                        value: Some(filter_value),
                    };
                    if let Some(driver) = this.connection_manager.driver_arc() {
                        AppView::open_table(
                            this,
                            driver,
                            database.clone(),
                            table.clone(),
                            vec![filter],
                            cx,
                        );
                    }
                }
            },
        )
        .detach();

        // Initial load
        let dialect = this
            .connection_manager
            .driver()
            .map(|d| d.dialect())
            .unwrap_or(Dialect::MySql);
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
    }

    #[allow(clippy::too_many_arguments)]
    fn query_table(
        driver: Arc<dyn DatabaseDriver>,
        database: String,
        table: String,
        filters: Vec<crate::query::filter::Filter>,
        dialect: Dialect,
        page: usize,
        view: Entity<TableView>,
        cx: &mut Context<Self>,
    ) {
        const PAGE_SIZE: usize = 100;
        view.update(cx, |v, cx| v.set_loading(cx));
        let (where_clause, params) = filters_to_sql(&filters, dialect);
        let data_sql = format!(
            "SELECT * FROM `{}`.`{}` {} LIMIT {} OFFSET {}",
            database,
            table,
            where_clause,
            PAGE_SIZE,
            page * PAGE_SIZE
        );
        let count_sql = format!(
            "SELECT COUNT(*) FROM `{}`.`{}` {}",
            database, table, where_clause
        );
        let (tx, rx) = tokio::sync::oneshot::channel::<
            Result<(crate::db::types::QueryResult, Option<u64>), String>,
        >();
        let driver2 = driver.clone();
        let params2 = params.clone();
        crate::db_runtime().spawn(async move {
            let data_result = match driver.query(&data_sql, &params).await {
                Ok(r) => r,
                Err(e) => {
                    tx.send(Err(e.to_string())).ok();
                    return;
                }
            };
            let total = match driver2.query(&count_sql, &params2).await {
                Ok(r) => r
                    .rows
                    .first()
                    .and_then(|row| row.first())
                    .and_then(|v| match v {
                        crate::db::types::Value::Int(n) => Some(*n as u64),
                        crate::db::types::Value::String(s) => s.parse().ok(),
                        _ => None,
                    }),
                Err(_) => None,
            };
            tx.send(Ok((data_result, total))).ok();
        });
        cx.spawn(
            async move |_this: WeakEntity<AppView>, cx: &mut AsyncApp| match rx.await {
                Ok(Ok((result, total))) => {
                    view.update(cx, |v, cx| v.set_data(result, total, cx));
                }
                Ok(Err(e)) => {
                    view.update(cx, |v, cx| v.set_error(e, cx));
                }
                Err(_) => {}
            },
        )
        .detach();
    }

    fn save_and_reload(
        driver: Arc<dyn DatabaseDriver>,
        updates: Vec<RowUpdate>,
        view: Entity<TableView>,
        cx: &mut Context<Self>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
        crate::db_runtime().spawn(async move {
            for update in &updates {
                let mut set_clauses: Vec<String> = Vec::new();
                let mut params: Vec<Value> = Vec::new();
                for e in &update.edits {
                    match &e.new_value {
                        // Intentional: RawSql expressions are inlined directly — the user is operating their own DB.
                        Value::RawSql(expr) => {
                            set_clauses.push(format!(
                                "`{}` = {}",
                                e.column.replace('`', "``"),
                                expr
                            ));
                        }
                        other => {
                            set_clauses.push(format!("`{}` = ?", e.column.replace('`', "``")));
                            params.push(other.clone());
                        }
                    }
                }
                let where_clauses: Vec<String> = update
                    .pk_columns
                    .iter()
                    .map(|pk| format!("`{}` = ?", pk.replace('`', "``")))
                    .collect();
                let sql = format!(
                    "UPDATE `{}`.`{}` SET {} WHERE {}",
                    update.database.replace('`', "``"),
                    update.table.replace('`', "``"),
                    set_clauses.join(", "),
                    where_clauses.join(" AND "),
                );
                params.extend(update.pk_values.clone());
                if let Err(e) = driver.execute(&sql, &params).await {
                    tx.send(Err(e.to_string())).ok();
                    return;
                }
            }
            tx.send(Ok(())).ok();
        });

        cx.spawn(
            async move |this: WeakEntity<AppView>, cx: &mut AsyncApp| match rx.await {
                Ok(Ok(())) => {
                    this.update(cx, |app_view, cx| {
                        let (database, table, filters, page, dialect) = {
                            let tv = view.read(cx);
                            let dialect = app_view
                                .connection_manager
                                .driver()
                                .map(|d| d.dialect())
                                .unwrap_or(Dialect::MySql);
                            (
                                tv.database.clone(),
                                tv.table_name.clone(),
                                tv.active_filters.clone(),
                                tv.page,
                                dialect,
                            )
                        };
                        if let Some(driver) = app_view.connection_manager.driver_arc() {
                            AppView::query_table(
                                driver,
                                database,
                                table,
                                filters,
                                dialect,
                                page,
                                view.clone(),
                                cx,
                            );
                        }
                    })
                    .ok();
                }
                Ok(Err(e)) => {
                    view.update(cx, |v, cx| v.set_error(e, cx));
                }
                Err(_) => {}
            },
        )
        .detach();
    }

    fn execute_insert(
        driver: Arc<dyn DatabaseDriver>,
        insert: NewRowInsert,
        view: Entity<TableView>,
        cx: &mut Context<Self>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
        crate::db_runtime().spawn(async move {
            let mut placeholders: Vec<String> = Vec::new();
            let mut params: Vec<Value> = Vec::new();
            let col_names: Vec<String> = insert
                .column_values
                .iter()
                .map(|(col, _)| format!("`{}`", col.replace('`', "``")))
                .collect();
            for (_, v) in insert.column_values {
                match v {
                    // Intentional: RawSql expressions are inlined directly — the user is operating their own DB.
                    Value::RawSql(expr) => placeholders.push(expr),
                    other => {
                        placeholders.push("?".to_string());
                        params.push(other);
                    }
                }
            }
            let sql = format!(
                "INSERT INTO `{}`.`{}` ({}) VALUES ({})",
                insert.database.replace('`', "``"),
                insert.table.replace('`', "``"),
                col_names.join(", "),
                placeholders.join(", "),
            );
            match driver.execute(&sql, &params).await {
                Ok(_) => {
                    tx.send(Ok(())).ok();
                }
                Err(e) => {
                    tx.send(Err(e.to_string())).ok();
                }
            }
        });

        cx.spawn(
            async move |this: WeakEntity<AppView>, cx: &mut AsyncApp| match rx.await {
                Ok(Ok(())) => {
                    this.update(cx, |app_view, cx| {
                        let (database, table, filters, page, dialect) = {
                            let tv = view.read(cx);
                            let dialect = app_view
                                .connection_manager
                                .driver()
                                .map(|d| d.dialect())
                                .unwrap_or(Dialect::MySql);
                            (
                                tv.database.clone(),
                                tv.table_name.clone(),
                                tv.active_filters.clone(),
                                tv.page,
                                dialect,
                            )
                        };
                        if let Some(driver) = app_view.connection_manager.driver_arc() {
                            AppView::query_table(
                                driver,
                                database,
                                table,
                                filters,
                                dialect,
                                page,
                                view.clone(),
                                cx,
                            );
                        }
                    })
                    .ok();
                }
                Ok(Err(e)) => {
                    view.update(cx, |v, cx| v.set_insert_error(e, cx));
                }
                Err(_) => {
                    view.update(cx, |v, cx| {
                        v.set_insert_error("Insert failed: connection lost".to_string(), cx);
                    });
                }
            },
        )
        .detach();
    }

    fn load_databases(
        driver: Arc<dyn DatabaseDriver>,
        sidebar: Entity<Sidebar>,
        cx: &mut Context<Self>,
    ) {
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
        cx.spawn(async move |_this: WeakEntity<AppView>, cx: &mut AsyncApp| {
            if let Ok(Ok(databases)) = rx.await {
                sidebar.update(cx, |s, cx| {
                    s.databases = databases;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn load_foreign_keys(
        driver: Arc<dyn DatabaseDriver>,
        database: String,
        table: String,
        view: Entity<TableView>,
        cx: &mut Context<Self>,
    ) {
        let (tx, rx) =
            tokio::sync::oneshot::channel::<Result<Vec<crate::db::types::ForeignKey>, String>>();
        crate::db_runtime().spawn(async move {
            match driver.foreign_keys(&database, &table).await {
                Ok(fks) => {
                    tx.send(Ok(fks)).ok();
                }
                Err(e) => {
                    tx.send(Err(e.to_string())).ok();
                }
            }
        });
        cx.spawn(async move |_this: WeakEntity<AppView>, cx: &mut AsyncApp| {
            if let Ok(Ok(fks)) = rx.await {
                view.update(cx, |v, cx| v.set_foreign_keys(fks, cx));
            }
        })
        .detach();
    }

    fn load_tables(
        driver: Arc<dyn DatabaseDriver>,
        database: String,
        sidebar: Entity<Sidebar>,
        cx: &mut Context<Self>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<Vec<String>, String>>();
        crate::db_runtime().spawn(async move {
            match driver.tables(&database).await {
                Ok(tables) => {
                    tx.send(Ok(tables)).ok();
                }
                Err(e) => {
                    tx.send(Err(e.to_string())).ok();
                }
            }
        });
        cx.spawn(async move |_this: WeakEntity<AppView>, cx: &mut AsyncApp| {
            if let Ok(Ok(tables)) = rx.await {
                sidebar.update(cx, |s, cx| {
                    s.tables = tables;
                    cx.notify();
                });
            }
        })
        .detach();
    }
}

impl Focusable for AppView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (active_tab, title) = {
            let tab_bar = self.tab_bar.read(cx);
            let sidebar = self.sidebar.read(cx);
            let active_tab = tab_bar.active_tab;
            let title = match &sidebar.connection_name {
                None => "Querybox - Configuring a connection".to_string(),
                Some(name) => {
                    let suffix = active_tab
                        .and_then(|id| tab_bar.tabs.iter().find(|t| t.id == id))
                        .map(|tab| match &tab.kind {
                            super::tab_bar::TabKind::Table { table, .. } => {
                                format!(" - {}", table)
                            }
                            super::tab_bar::TabKind::Query { .. } => " - New Query".to_string(),
                        })
                        .unwrap_or_default();
                    format!("Querybox - {}{}", name, suffix)
                }
            };
            (active_tab, title)
        };
        window.set_window_title(&title);

        div()
            .flex()
            .flex_col()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::close_active_tab))
            .bg(rgb(0x181825))
            .text_color(rgb(0xcdd6f4))
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h(px(0.))
                    .child(self.sidebar.clone())
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.))
                            .flex()
                            .flex_col()
                            .child(self.tab_bar.clone())
                            .child(self.render_active_content(active_tab)),
                    ),
            )
            .child(self.render_status_bar())
            .child(self.connection_dialog.clone())
    }
}

impl AppView {
    fn close_active_tab(
        &mut self,
        _: &crate::CloseTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let active_tab = self.tab_bar.read(cx).active_tab;
        if let Some(tab_id) = active_tab {
            self.tab_bar.update(cx, |bar, cx| bar.close_tab(tab_id, cx));
            self.table_views.retain(|(id, _)| *id != tab_id);
            self.editor_views.retain(|(id, _)| *id != tab_id);
            cx.notify();
        }
    }

    fn render_active_content(&self, active_tab: Option<usize>) -> impl IntoElement {
        if let Some(tab_id) = active_tab {
            for (id, view) in &self.table_views {
                if *id == tab_id {
                    return view.clone().into_any_element();
                }
            }
            for (id, view) in &self.editor_views {
                if *id == tab_id {
                    return view.clone().into_any_element();
                }
            }
        }

        div()
            .flex_1()
            .flex()
            .justify_center()
            .items_center()
            .child(div().text_color(rgb(0x6c7086)).text_xl().child("QueryBox"))
            .into_any_element()
    }

    fn render_status_bar(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .px(px(12.))
            .py(px(4.))
            .bg(rgb(0x1e1e2e))
            .border_t_1()
            .border_color(rgb(0x333333))
            .text_size(px(11.))
            .text_color(rgb(0x6c7086))
            .child(self.status_message.clone())
    }
}
