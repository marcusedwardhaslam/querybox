use gpui::*;

use crate::query::filter::{Filter, FilterOp};

pub struct FilterPanel {
    pub filters: Vec<Filter>,
    pub available_columns: Vec<String>,
    pub visible: bool,
}

impl FilterPanel {
    pub fn new() -> Self {
        Self {
            filters: vec![],
            available_columns: vec![],
            visible: false,
        }
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        cx.notify();
    }

    pub fn add_filter(
        &mut self,
        column: String,
        op: FilterOp,
        value: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.filters.push(Filter { column, op, value });
        cx.notify();
    }

    pub fn remove_filter(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.filters.len() {
            self.filters.remove(index);
            cx.notify();
        }
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.filters.clear();
        cx.notify();
    }
}

impl Render for FilterPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible || self.filters.is_empty() {
            return div().into_any_element();
        }

        let mut panel = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_2()
            .px(px(12.))
            .py(px(6.))
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x333333));

        for filter in self.filters.iter() {
            let op_str = filter.op.label();

            let label = if let Some(val) = &filter.value {
                format!("{} {} {}", filter.column, op_str, val)
            } else {
                format!("{} {}", filter.column, op_str)
            };

            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .bg(rgb(0x313244))
                    .rounded(px(4.))
                    .px(px(8.))
                    .py(px(3.))
                    .text_size(px(11.))
                    .text_color(rgb(0xa6adc8))
                    .child(label)
                    .child(div().text_color(rgb(0x6c7086)).cursor_pointer().child("x")),
            );
        }

        panel = panel.child(
            div()
                .text_size(px(11.))
                .text_color(rgb(0xf38ba8))
                .cursor_pointer()
                .child("Clear all"),
        );

        panel.into_any_element()
    }
}
