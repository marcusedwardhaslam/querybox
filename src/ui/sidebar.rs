use gpui::prelude::FluentBuilder;
use gpui::*;

pub enum SidebarEvent {
    OpenConnectionDialog,
    NewQuery,
    SelectDatabase(String),
    OpenTable { database: String, table: String },
}

pub struct Sidebar {
    pub connection_name: Option<String>,
    pub engine_info: Option<String>,
    pub databases: Vec<String>,
    pub selected_database: Option<String>,
    pub tables: Vec<String>,
    pub selected_table: Option<String>,
    db_expanded: bool,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            connection_name: None,
            engine_info: None,
            databases: vec![],
            selected_database: None,
            tables: vec![],
            selected_table: None,
            db_expanded: false,
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(220.))
            .flex_shrink_0()
            .bg(rgb(0x1e1e2e))
            .border_r_1()
            .border_color(rgb(0x333333))
            .flex()
            .flex_col()
            .child(self.render_connection_header(cx))
            .child(self.render_database_selector(cx))
            .child(self.render_table_list(cx))
            .child(self.render_new_query_button(cx))
    }
}

impl Sidebar {
    fn render_connection_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (name, info) = match (&self.connection_name, &self.engine_info) {
            (Some(name), Some(info)) => (name.clone(), info.clone()),
            _ => ("No connection".to_string(), "Click to connect".to_string()),
        };

        div()
            .id("connection-header")
            .p(px(12.))
            .border_b_1()
            .border_color(rgb(0x333333))
            .cursor_pointer()
            .on_click(cx.listener(|_, _, _, cx| {
                cx.emit(SidebarEvent::OpenConnectionDialog);
            }))
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0xcdd6f4))
                    .text_size(px(13.))
                    .child(name),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x6c7086))
                    .mt(px(2.))
                    .child(info),
            )
    }

    fn render_database_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let db_name = self
            .selected_database
            .clone()
            .unwrap_or_else(|| "Select database".to_string());

        let expanded = self.db_expanded;
        let databases = self.databases.clone();

        let mut container = div()
            .px(px(12.))
            .py(px(8.))
            .border_b_1()
            .border_color(rgb(0x333333));

        // The selector button
        let selector = div()
            .id("db-selector")
            .bg(rgb(0x313244))
            .rounded(px(4.))
            .px(px(10.))
            .py(px(6.))
            .text_size(px(12.))
            .text_color(rgb(0xa6adc8))
            .flex()
            .justify_between()
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                if !this.databases.is_empty() {
                    this.db_expanded = !this.db_expanded;
                    cx.notify();
                }
            }))
            .child(db_name)
            .child(
                div()
                    .text_size(px(10.))
                    .child(if expanded { "▲" } else { "▼" }),
            );

        container = container.child(selector);

        if expanded && !databases.is_empty() {
            let mut dropdown = div()
                .mt(px(2.))
                .bg(rgb(0x313244))
                .rounded(px(4.))
                .overflow_hidden()
                .flex()
                .flex_col();

            for db in databases {
                let db_clone = db.clone();
                let is_selected = self.selected_database.as_deref() == Some(&db);
                dropdown = dropdown.child(
                    div()
                        .id(ElementId::Name(db.clone().into()))
                        .px(px(10.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .when(is_selected, |d| {
                            d.bg(rgb(0x45475a)).text_color(rgb(0xcdd6f4))
                        })
                        .when(!is_selected, |d| d.text_color(rgb(0xa6adc8)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected_database = Some(db_clone.clone());
                            this.db_expanded = false;
                            cx.emit(SidebarEvent::SelectDatabase(db_clone.clone()));
                            cx.notify();
                        }))
                        .child(db),
                );
            }

            container = container.child(dropdown);
        }

        container
    }

    fn render_table_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .id("sidebar-table-list")
            .flex_1()
            .overflow_y_scroll()
            .py(px(4.))
            .child(
                div()
                    .px(px(12.))
                    .py(px(4.))
                    .text_size(px(10.))
                    .text_color(rgb(0x6c7086))
                    .child("TABLES"),
            );

        if self.tables.is_empty() {
            list = list.child(
                div()
                    .px(px(12.))
                    .py(px(4.))
                    .text_size(px(12.))
                    .text_color(rgb(0x6c7086))
                    .child(if self.selected_database.is_some() {
                        "No tables"
                    } else {
                        "Select a database"
                    }),
            );
        } else {
            for table in &self.tables {
                let is_selected = self.selected_table.as_deref() == Some(table.as_str());
                let table_name = table.clone();
                let db_name = self.selected_database.clone().unwrap_or_default();

                let mut row = div()
                    .id(ElementId::Name(format!("table-{table_name}").into()))
                    .px(px(12.))
                    .pl(px(20.))
                    .py(px(6.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_table = Some(table_name.clone());
                        cx.emit(SidebarEvent::OpenTable {
                            database: db_name.clone(),
                            table: table_name.clone(),
                        });
                        cx.notify();
                    }));

                if is_selected {
                    row = row
                        .bg(rgb(0x45475a))
                        .text_color(rgb(0xcdd6f4))
                        .border_l_2()
                        .border_color(rgb(0x89b4fa));
                } else {
                    row = row.text_color(rgb(0xa6adc8));
                }

                list = list.child(row.child(table.clone()));
            }
        }

        list
    }

    fn render_new_query_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(10.))
            .border_t_1()
            .border_color(rgb(0x333333))
            .child(
                div()
                    .id("new-query-btn")
                    .bg(rgb(0x89b4fa))
                    .text_color(rgb(0x1e1e2e))
                    .text_size(px(12.))
                    .rounded(px(4.))
                    .py(px(6.))
                    .flex()
                    .justify_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.emit(SidebarEvent::NewQuery);
                    }))
                    .child("+ New Query"),
            )
    }
}
