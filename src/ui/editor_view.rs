use gpui::prelude::FluentBuilder;
use gpui::*;
use std::sync::Arc;

use super::sql_editor::SqlEditor;
use crate::db::{types::QueryResult, types::Value, DatabaseDriver};
use crate::query::format::format_sql;
use crate::query::history::QueryHistory;

pub struct EditorView {
    pub result: Option<QueryResult>,
    pub error: Option<String>,
    pub running: bool,
    pub history: QueryHistory,
    editor: Entity<SqlEditor>,
    driver: Option<Arc<dyn DatabaseDriver>>,
    database: Option<String>,
    scroll_handle: ScrollHandle,
}

impl EditorView {
    pub fn new(
        driver: Option<Arc<dyn DatabaseDriver>>,
        database: Option<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            result: None,
            error: None,
            running: false,
            history: QueryHistory::new(),
            editor: cx.new(SqlEditor::new),
            driver,
            database,
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn set_result(&mut self, sql: String, result: QueryResult, cx: &mut Context<Self>) {
        self.history.add(sql, result.execution_time_ms, true);
        self.result = Some(result);
        self.error = None;
        self.running = false;
        cx.notify();
    }

    pub fn set_error(&mut self, sql: String, error: String, cx: &mut Context<Self>) {
        self.history.add(sql, 0, false);
        self.error = Some(error);
        self.result = None;
        self.running = false;
        cx.notify();
    }

    fn format_query(&mut self, cx: &mut Context<Self>) {
        let sql = self.editor.read(cx).content.to_string();
        if sql.trim().is_empty() {
            return;
        }
        match format_sql(&sql) {
            Ok(formatted) => {
                self.editor.update(cx, |editor, cx| {
                    editor.set_content(formatted, cx);
                });
            }
            Err(msg) => {
                self.error = Some(msg);
                self.result = None;
                self.running = false;
                cx.notify();
            }
        }
    }

    fn run_query(&mut self, cx: &mut Context<Self>) {
        let sql = self.editor.read(cx).content.to_string();
        if sql.trim().is_empty() {
            return;
        }
        let Some(driver) = self.driver.clone() else {
            return;
        };
        self.running = true;
        self.error = None;
        self.result = None;
        cx.notify();

        let database = self.database.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<QueryResult, String>>();
        crate::db_runtime().spawn(async move {
            match driver.query_in(database.as_deref(), &sql, &[]).await {
                Ok(result) => {
                    tx.send(Ok(result)).ok();
                }
                Err(e) => {
                    tx.send(Err(e.to_string())).ok();
                }
            }
        });

        let sql_clone = self.editor.read(cx).content.to_string();
        cx.spawn(
            async move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| match rx.await {
                Ok(Ok(result)) => {
                    this.update(cx, |ev, cx| ev.set_result(sql_clone, result, cx))
                        .ok();
                }
                Ok(Err(e)) => {
                    this.update(cx, |ev, cx| ev.set_error(sql_clone, e, cx))
                        .ok();
                }
                Err(_) => {}
            },
        )
        .detach();
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_editor_pane(cx))
            .child(self.render_results_pane())
    }
}

impl EditorView {
    fn render_editor_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_running = self.running;
        div()
            .flex_1()
            .flex()
            .flex_col()
            .border_b_2()
            .border_color(rgb(0x45475a))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(12.))
                    .py(px(6.))
                    .gap_2()
                    .bg(rgb(0x1e1e2e))
                    .border_b_1()
                    .border_color(rgb(0x333333))
                    .child(
                        div()
                            .id("run-btn")
                            .bg(if is_running {
                                rgb(0x45475a)
                            } else {
                                rgb(0xa6e3a1)
                            })
                            .text_color(rgb(0x1e1e2e))
                            .rounded(px(4.))
                            .px(px(12.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .when(!is_running, |d| {
                                d.on_click(cx.listener(|this, _, _, cx| {
                                    this.run_query(cx);
                                }))
                            })
                            .child(if is_running { "Running…" } else { "Run" }),
                    )
                    .child(
                        div()
                            .id("format-btn")
                            .bg(rgb(0x313244))
                            .text_color(rgb(0xa6adc8))
                            .rounded(px(4.))
                            .px(px(10.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.format_query(cx);
                            }))
                            .child("Format"),
                    )
                    .child(div().flex_1())
                    .when(self.driver.is_none(), |d| {
                        d.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(0xf38ba8))
                                .child("No connection"),
                        )
                    })
                    .when_some(self.database.as_ref(), |d, db| {
                        d.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(0x6c7086))
                                .child(db.clone()),
                        )
                    }),
            )
            .child(div().flex_1().bg(rgb(0x181825)).child(self.editor.clone()))
    }

    fn render_results_pane(&self) -> impl IntoElement {
        let pane = div().flex_1().flex().flex_col();

        if self.running {
            return pane
                .justify_center()
                .items_center()
                .child(div().text_color(rgb(0x6c7086)).child("Running query…"))
                .into_any_element();
        }

        if let Some(err) = &self.error {
            return pane
                .child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(rgb(0x1e1e2e))
                        .border_b_1()
                        .border_color(rgb(0x333333))
                        .text_size(px(11.))
                        .text_color(rgb(0xf38ba8))
                        .child(format!("Error: {}", err)),
                )
                .into_any_element();
        }

        if let Some(result) = &self.result {
            let info = format!(
                "{} rows — {}ms",
                result.rows.len(),
                result.execution_time_ms
            );

            let mut grid = div()
                .flex_1()
                .flex()
                .flex_col()
                .id("results-grid")
                .overflow_y_scroll()
                .track_scroll(&self.scroll_handle);

            let mut header = div()
                .flex()
                .flex_row()
                .bg(rgb(0x1e1e2e))
                .border_b_1()
                .border_color(rgb(0x333333))
                .flex_shrink_0();

            for col in &result.columns {
                header = header.child(
                    div()
                        .w(px(150.))
                        .flex_shrink_0()
                        .px(px(12.))
                        .py(px(8.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x89b4fa))
                        .child(col.name.clone()),
                );
            }
            grid = grid.child(header);

            for (row_idx, row) in result.rows.iter().enumerate() {
                let bg = if row_idx % 2 == 0 {
                    rgb(0x181825)
                } else {
                    rgb(0x1e1e2e)
                };
                let mut row_el = div()
                    .flex()
                    .flex_row()
                    .bg(bg)
                    .border_b_1()
                    .border_color(rgb(0x222222));

                for val in row {
                    let color = match val {
                        Value::Null => rgb(0x6c7086),
                        Value::Int(_) | Value::Float(_) => rgb(0xfab387),
                        Value::Bool(_) => rgb(0xcba6f7),
                        Value::DateTime(_) => rgb(0xa6e3a1),
                        _ => rgb(0xcdd6f4),
                    };
                    row_el = row_el.child(
                        div()
                            .w(px(150.))
                            .flex_shrink_0()
                            .px(px(12.))
                            .py(px(6.))
                            .text_size(px(12.))
                            .text_color(color)
                            .overflow_hidden()
                            .child(val.to_string()),
                    );
                }
                grid = grid.child(row_el);
            }

            return pane
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(rgb(0x1e1e2e))
                        .border_b_1()
                        .border_color(rgb(0x333333))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(0xa6e3a1))
                                .child(info),
                        )
                        .child(div().flex_1()),
                )
                .child(grid)
                .into_any_element();
        }

        pane.justify_center()
            .items_center()
            .child(
                div()
                    .text_color(rgb(0x6c7086))
                    .child("Run a query to see results"),
            )
            .into_any_element()
    }
}
