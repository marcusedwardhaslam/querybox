use gpui::*;

#[derive(Debug, Clone, PartialEq)]
pub enum TabKind {
    Table { database: String, table: String },
    Query { name: String },
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: usize,
    pub kind: TabKind,
}

impl Tab {
    pub fn label(&self) -> String {
        match &self.kind {
            TabKind::Table { table, .. } => table.clone(),
            TabKind::Query { name } => name.clone(),
        }
    }

    pub fn icon(&self) -> &'static str {
        match &self.kind {
            TabKind::Table { .. } => "T",
            TabKind::Query { .. } => "Q",
        }
    }
}

pub struct TabBar {
    pub tabs: Vec<Tab>,
    pub active_tab: Option<usize>,
    next_id: usize,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            tabs: vec![],
            active_tab: None,
            next_id: 1,
        }
    }

    pub fn open_table(&mut self, database: String, table: String, cx: &mut Context<Self>) -> usize {
        for tab in &self.tabs {
            if let TabKind::Table {
                database: d,
                table: t,
            } = &tab.kind
            {
                if d == &database && t == &table {
                    self.active_tab = Some(tab.id);
                    cx.notify();
                    return tab.id;
                }
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            kind: TabKind::Table { database, table },
        });
        self.active_tab = Some(id);
        cx.notify();
        id
    }

    pub fn open_query(&mut self, cx: &mut Context<Self>) -> usize {
        let query_count = self
            .tabs
            .iter()
            .filter(|t| matches!(t.kind, TabKind::Query { .. }))
            .count();
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            kind: TabKind::Query {
                name: format!("Query {}", query_count + 1),
            },
        });
        self.active_tab = Some(id);
        cx.notify();
        id
    }

    pub fn close_tab(&mut self, tab_id: usize, cx: &mut Context<Self>) {
        self.tabs.retain(|t| t.id != tab_id);
        if self.active_tab == Some(tab_id) {
            self.active_tab = self.tabs.last().map(|t| t.id);
        }
        cx.notify();
    }

    pub fn set_active(&mut self, tab_id: usize, cx: &mut Context<Self>) {
        self.active_tab = Some(tab_id);
        cx.notify();
    }
}

impl Render for TabBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut bar = div()
            .flex()
            .flex_row()
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x333333))
            .h(px(36.));

        for tab in &self.tabs {
            let is_active = self.active_tab == Some(tab.id);
            let label = tab.label();
            let icon = tab.icon();
            let tab_id = tab.id;

            let mut tab_el = div()
                .id(ElementId::Integer(tab_id as u64))
                .flex()
                .items_center()
                .gap_2()
                .px(px(14.))
                .text_size(px(12.))
                .border_r_1()
                .border_color(rgb(0x333333))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_active(tab_id, cx);
                }));

            if is_active {
                tab_el = tab_el
                    .bg(rgb(0x181825))
                    .text_color(rgb(0xcdd6f4))
                    .border_t_2()
                    .border_color(rgb(0x89b4fa));
            } else {
                tab_el = tab_el.text_color(rgb(0x6c7086));
            }

            tab_el = tab_el
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(0x89b4fa))
                        .child(icon),
                )
                .child(label)
                .child(
                    div()
                        .id(ElementId::Integer((tab_id as u64) << 32 | 1))
                        .ml(px(6.))
                        .text_size(px(11.))
                        .text_color(rgb(0x585b70))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.close_tab(tab_id, cx);
                        }))
                        .child("×"),
                );

            bar = bar.child(tab_el);
        }

        bar
    }
}
