use gpui::*;
use std::sync::Arc;

use crate::connection::ConnectionManager;
use crate::db::types::Value;
use crate::db::{types::Dialect, DatabaseDriver};
use crate::query::filter::filters_to_sql;

use super::connection_dialog::{ConnectionDialog, ConnectionDialogEvent};
use super::editor_view::EditorView;
use super::sidebar::{Sidebar, SidebarEvent};
use super::tab_bar::TabBar;
use super::table_view::{RowUpdate, TableView, TableViewEvent};

pub struct AppView {
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
        let connection_dialog = cx.new(|cx| ConnectionDialog::new(cx));

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
                        AppView::open_table(this, driver, database, table, cx);
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
        cx.notify();

        // Subscribe to events on this view
        let view2 = view.clone();
        cx.subscribe(
            &view,
            move |this, _entity, event: &TableViewEvent, cx| match event {
                TableViewEvent::FiltersChanged => {
                    if let Some(driver) = this.connection_manager.driver_arc() {
                        let tv = view2.read(cx);
                        let database = tv.database.clone();
                        let table = tv.table_name.clone();
                        let filters = tv.active_filters.clone();
                        let dialect = this
                            .connection_manager
                            .driver()
                            .map(|d| d.dialect())
                            .unwrap_or(Dialect::MySql);
                        drop(tv);
                        AppView::query_table(
                            driver,
                            database,
                            table,
                            filters,
                            dialect,
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
            },
        )
        .detach();

        // Initial load
        let dialect = this
            .connection_manager
            .driver()
            .map(|d| d.dialect())
            .unwrap_or(Dialect::MySql);
        AppView::query_table(driver, database, table, vec![], dialect, view, cx);
    }

    fn query_table(
        driver: Arc<dyn DatabaseDriver>,
        database: String,
        table: String,
        filters: Vec<crate::query::filter::Filter>,
        dialect: Dialect,
        view: Entity<TableView>,
        cx: &mut Context<Self>,
    ) {
        view.update(cx, |v, cx| v.set_loading(cx));
        let (where_clause, params) = filters_to_sql(&filters, dialect);
        let sql = format!(
            "SELECT * FROM `{}`.`{}` {} LIMIT 500",
            database, table, where_clause
        );
        let (tx, rx) =
            tokio::sync::oneshot::channel::<Result<crate::db::types::QueryResult, String>>();
        crate::db_runtime().spawn(async move {
            match driver.query(&sql, &params).await {
                Ok(result) => {
                    tx.send(Ok(result)).ok();
                }
                Err(e) => {
                    tx.send(Err(e.to_string())).ok();
                }
            }
        });
        cx.spawn(
            async move |_this: WeakEntity<AppView>, cx: &mut AsyncApp| match rx.await {
                Ok(Ok(result)) => {
                    view.update(cx, |v, cx| v.set_data(result, cx));
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
                let set_clauses: Vec<String> = update
                    .edits
                    .iter()
                    .map(|e| format!("`{}` = ?", e.column))
                    .collect();
                let where_clauses: Vec<String> = update
                    .pk_columns
                    .iter()
                    .map(|pk| format!("`{}` = ?", pk))
                    .collect();
                let sql = format!(
                    "UPDATE `{}`.`{}` SET {} WHERE {}",
                    update.database,
                    update.table,
                    set_clauses.join(", "),
                    where_clauses.join(" AND "),
                );
                let mut params: Vec<Value> = update
                    .edits
                    .iter()
                    .map(|e| Value::String(e.new_value.clone()))
                    .collect();
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
                        let (database, table, filters, dialect) = {
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

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab_bar = self.tab_bar.read(cx);
        let active_tab = tab_bar.active_tab;
        let sidebar = self.sidebar.read(cx);

        let title = {
            let conn_name = sidebar.connection_name.clone();
            match conn_name {
                None => "Querybox - Configuring a connection".to_string(),
                Some(name) => {
                    let suffix = active_tab
                        .and_then(|id| tab_bar.tabs.iter().find(|t| t.id == id))
                        .map(|tab| match &tab.kind {
                            super::tab_bar::TabKind::Table { table, .. } => format!(" - {}", table),
                            super::tab_bar::TabKind::Query { .. } => " - New Query".to_string(),
                        })
                        .unwrap_or_default();
                    format!("Querybox - {}{}", name, suffix)
                }
            }
        };
        window.set_window_title(&title);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x181825))
            .text_color(rgb(0xcdd6f4))
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(self.sidebar.clone())
                    .child(
                        div()
                            .flex_1()
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
