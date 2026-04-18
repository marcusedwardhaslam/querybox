use gpui::*;

use crate::db::types::{Column, Index};

pub struct SchemaView {
    pub database: String,
    pub table_name: String,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub loading: bool,
    scroll_handle: ScrollHandle,
}

impl SchemaView {
    pub fn new(database: String, table_name: String) -> Self {
        Self {
            database,
            table_name,
            columns: vec![],
            indexes: vec![],
            loading: true,
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn set_schema(
        &mut self,
        columns: Vec<Column>,
        indexes: Vec<Index>,
        cx: &mut Context<Self>,
    ) {
        self.columns = columns;
        self.indexes = indexes;
        self.loading = false;
        cx.notify();
    }
}

impl Render for SchemaView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.loading {
            return div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .child(div().text_color(rgb(0x6c7086)).child("Loading schema..."))
                .into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .id("schema-view")
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .p(px(16.))
            .gap_4()
            .child(self.render_columns_section())
            .child(self.render_indexes_section())
            .into_any_element()
    }
}

impl SchemaView {
    fn render_columns_section(&self) -> impl IntoElement {
        let mut section = div().flex().flex_col().child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0xcdd6f4))
                .mb(px(8.))
                .child(format!("Columns ({})", self.columns.len())),
        );

        section = section.child(
            div()
                .flex()
                .flex_row()
                .bg(rgb(0x1e1e2e))
                .border_b_1()
                .border_color(rgb(0x333333))
                .py(px(6.))
                .child(
                    div()
                        .w(px(150.))
                        .px(px(12.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x89b4fa))
                        .child("Name"),
                )
                .child(
                    div()
                        .w(px(120.))
                        .px(px(12.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x89b4fa))
                        .child("Type"),
                )
                .child(
                    div()
                        .w(px(80.))
                        .px(px(12.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x89b4fa))
                        .child("Nullable"),
                )
                .child(
                    div()
                        .w(px(120.))
                        .px(px(12.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x89b4fa))
                        .child("Default"),
                )
                .child(
                    div()
                        .w(px(60.))
                        .px(px(12.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x89b4fa))
                        .child("Key"),
                )
                .child(
                    div()
                        .flex_1()
                        .px(px(12.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x89b4fa))
                        .child("Extra"),
                ),
        );

        for (i, col) in self.columns.iter().enumerate() {
            let bg = if i % 2 == 0 {
                rgb(0x181825)
            } else {
                rgb(0x1e1e2e)
            };
            let key_label = if col.is_primary_key { "PRI" } else { "" };
            let nullable_label = if col.nullable { "YES" } else { "NO" };
            let default_label = col
                .default_value
                .clone()
                .unwrap_or_else(|| "NULL".to_string());

            section = section.child(
                div()
                    .flex()
                    .flex_row()
                    .bg(bg)
                    .border_b_1()
                    .border_color(rgb(0x222222))
                    .py(px(5.))
                    .child(
                        div()
                            .w(px(150.))
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcdd6f4))
                            .child(col.name.clone()),
                    )
                    .child(
                        div()
                            .w(px(120.))
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0xfab387))
                            .child(col.data_type.clone()),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0x6c7086))
                            .child(nullable_label.to_string()),
                    )
                    .child(
                        div()
                            .w(px(120.))
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0x6c7086))
                            .child(default_label),
                    )
                    .child(
                        div()
                            .w(px(60.))
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0xf9e2af))
                            .child(key_label.to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0x6c7086))
                            .child(col.extra.clone()),
                    ),
            );
        }

        section
    }

    fn render_indexes_section(&self) -> impl IntoElement {
        let mut section = div().flex().flex_col().child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0xcdd6f4))
                .mb(px(8.))
                .child(format!("Indexes ({})", self.indexes.len())),
        );

        for (i, idx) in self.indexes.iter().enumerate() {
            let bg = if i % 2 == 0 {
                rgb(0x181825)
            } else {
                rgb(0x1e1e2e)
            };
            let unique_label = if idx.unique { "UNIQUE" } else { "" };

            section = section.child(
                div()
                    .flex()
                    .flex_row()
                    .bg(bg)
                    .border_b_1()
                    .border_color(rgb(0x222222))
                    .py(px(5.))
                    .child(
                        div()
                            .w(px(200.))
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0xcdd6f4))
                            .child(idx.name.clone()),
                    )
                    .child(
                        div()
                            .w(px(250.))
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0xa6adc8))
                            .child(idx.columns.join(", ")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .px(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0xf9e2af))
                            .child(unique_label.to_string()),
                    ),
            );
        }

        section
    }
}
