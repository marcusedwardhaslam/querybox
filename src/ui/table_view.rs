use std::collections::HashMap;

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::schema_view::SchemaView;
use super::text_field::TextField;
use crate::db::types::{text_to_value, Column, Index, QueryResult, Row, Value};
use crate::query::filter::{Filter, FilterOp};

actions!(
    table_view,
    [CommitEdit, CancelEdit, SaveEdits, GoToPage, InsertNewRow]
);

#[derive(Clone, Copy, Debug, PartialEq, Default)]
enum ViewMode {
    #[default]
    Data,
    Schema,
}

pub fn register_table_view_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", CommitEdit, Some("TableView")),
        KeyBinding::new("escape", CancelEdit, Some("TableView")),
        KeyBinding::new("cmd-s", SaveEdits, Some("TableView")),
        KeyBinding::new("enter", GoToPage, Some("PageJumpField")),
        KeyBinding::new("cmd-return", InsertNewRow, Some("TableView")),
    ]);
}

#[derive(Clone, Debug)]
pub struct CellEdit {
    pub column: String,
    pub new_value: Value,
}

#[derive(Clone, Debug)]
pub struct RowUpdate {
    pub database: String,
    pub table: String,
    pub pk_columns: Vec<String>,
    pub pk_values: Vec<Value>,
    pub edits: Vec<CellEdit>,
}

#[derive(Clone, Debug)]
pub struct NewRowInsert {
    pub database: String,
    pub table: String,
    pub column_values: Vec<(String, crate::db::types::Value)>,
}

pub enum TableViewEvent {
    FiltersChanged,
    PageChanged,
    SaveChanges(Vec<RowUpdate>),
    InsertRow(NewRowInsert),
    NavigateToFk {
        database: String,
        table: String,
        column: String,
        value: crate::db::types::Value,
    },
}

impl EventEmitter<TableViewEvent> for TableView {}

pub struct TableView {
    pub database: String,
    pub table_name: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub total_rows: Option<u64>,
    pub page: usize,
    pub page_size: usize,
    pub loading: bool,
    pub error: Option<String>,

    pub active_filters: Vec<Filter>,

    // Filter form state
    filter_form_visible: bool,
    filter_form_column_idx: usize,
    filter_form_op_idx: usize,
    filter_form_value: Entity<TextField>,
    column_dropdown_open: bool,
    op_dropdown_open: bool,

    // Cell editing
    pending_edits: HashMap<(usize, usize), String>,
    editing_cell: Option<(usize, usize)>,
    edit_field: Entity<TextField>,
    save_error: Option<String>,

    // New row insert
    new_row_active: bool,
    new_row_edits: HashMap<usize, String>,
    editing_new_row_col: Option<usize>,
    new_row_insert_error: Option<String>,
    new_row_dirty: bool,

    // Page jump
    page_jump_field: Entity<TextField>,

    scroll_handle: ScrollHandle,
    pub foreign_keys: Vec<crate::db::types::ForeignKey>,
    view_mode: ViewMode,
    schema_view: Entity<SchemaView>,
}

impl TableView {
    pub fn new(database: String, table_name: String, cx: &mut Context<Self>) -> Self {
        let schema_view = cx.new(|_| SchemaView::new(database.clone(), table_name.clone()));
        Self {
            database,
            table_name,
            columns: vec![],
            rows: vec![],
            total_rows: None,
            page: 0,
            page_size: 100,
            loading: true,
            error: None,
            active_filters: vec![],
            filter_form_visible: false,
            filter_form_column_idx: 0,
            filter_form_op_idx: 0,
            filter_form_value: cx.new(|cx| TextField::new(cx, "value")),
            column_dropdown_open: false,
            op_dropdown_open: false,
            pending_edits: HashMap::new(),
            editing_cell: None,
            edit_field: cx.new(|cx| TextField::new(cx, "")),
            save_error: None,
            new_row_active: false,
            new_row_edits: HashMap::new(),
            editing_new_row_col: None,
            new_row_insert_error: None,
            new_row_dirty: false,
            page_jump_field: cx.new(|cx| TextField::new(cx, "#")), // page number input
            scroll_handle: ScrollHandle::new(),
            foreign_keys: vec![],
            view_mode: ViewMode::Data,
            schema_view,
        }
    }

    pub fn set_data(
        &mut self,
        result: QueryResult,
        total_rows: Option<u64>,
        cx: &mut Context<Self>,
    ) {
        self.columns = result.columns;
        self.rows = result.rows;
        self.total_rows = total_rows;
        self.loading = false;
        self.error = None;
        // Clear any pending edits — the table has refreshed
        self.pending_edits.clear();
        self.editing_cell = None;
        self.save_error = None;
        self.new_row_active = false;
        self.new_row_edits.clear();
        self.editing_new_row_col = None;
        self.new_row_insert_error = None;
        self.new_row_dirty = false;
        cx.notify();
    }

    pub fn set_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        self.loading = false;
        cx.notify();
    }

    pub fn set_insert_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.new_row_insert_error = Some(error);
        self.new_row_dirty = false;
        cx.notify();
    }

    pub fn set_foreign_keys(
        &mut self,
        fks: Vec<crate::db::types::ForeignKey>,
        cx: &mut Context<Self>,
    ) {
        self.foreign_keys = fks;
        cx.notify();
    }

    // Called by AppView once schema data arrives (Task 3).
    pub fn set_schema(
        &mut self,
        columns: Vec<Column>,
        indexes: Vec<Index>,
        cx: &mut Context<Self>,
    ) {
        self.schema_view
            .update(cx, |sv, cx| sv.set_schema(columns, indexes, cx));
    }

    pub fn set_loading(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.error = None;
        cx.notify();
    }

    // ── Editing ──────────────────────────────────────────────────────────────

    fn start_editing(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        self.commit_edit(cx);
        self.commit_new_row_edit(cx);
        let current = self
            .rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(|v| match v {
                Value::Null => String::new(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        self.editing_cell = Some((row, col));
        self.save_error = None;
        self.edit_field
            .update(cx, |f, cx| f.set_content(&current, cx));
        cx.notify();
    }

    fn commit_edit(&mut self, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.editing_cell.take() {
            let value = self.edit_field.read(cx).content.to_string();
            self.pending_edits.insert((row, col), value);
            cx.notify();
        }
    }

    fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        self.editing_cell = None;
        cx.notify();
    }

    fn commit_new_row_edit(&mut self, cx: &mut Context<Self>) {
        if let Some(col_idx) = self.editing_new_row_col.take() {
            let value = self.edit_field.read(cx).content.to_string();
            if !value.is_empty() {
                self.new_row_edits.insert(col_idx, value);
            }
            cx.notify();
        }
    }

    fn cancel_new_row(&mut self, cx: &mut Context<Self>) {
        self.new_row_active = false;
        self.new_row_edits.clear();
        self.editing_new_row_col = None;
        self.new_row_insert_error = None;
        self.new_row_dirty = false;
        cx.notify();
    }

    fn save_changes(&mut self, cx: &mut Context<Self>) {
        self.commit_edit(cx);
        if self.pending_edits.is_empty() {
            return;
        }

        let pk_indices: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_primary_key)
            .map(|(i, _)| i)
            .collect();

        if pk_indices.is_empty() {
            self.save_error = Some("Cannot save: table has no primary key".into());
            cx.notify();
            return;
        }

        let pk_col_names: Vec<String> = pk_indices
            .iter()
            .map(|&i| self.columns[i].name.clone())
            .collect();

        // Group edits by row
        let mut by_row: HashMap<usize, Vec<(usize, String)>> = HashMap::new();
        for (&(row, col), value) in &self.pending_edits {
            by_row.entry(row).or_default().push((col, value.clone()));
        }

        let mut updates = vec![];
        for (row_idx, col_edits) in by_row {
            let Some(row) = self.rows.get(row_idx) else {
                continue;
            };
            let pk_values: Vec<Value> = pk_indices
                .iter()
                .map(|&i| row.get(i).cloned().unwrap_or(Value::Null))
                .collect();
            let edits: Vec<CellEdit> = col_edits
                .into_iter()
                .filter_map(|(col_idx, new_value)| {
                    self.columns.get(col_idx).map(|c| CellEdit {
                        column: c.name.clone(),
                        new_value: text_to_value(&new_value),
                    })
                })
                .collect();
            updates.push(RowUpdate {
                database: self.database.clone(),
                table: self.table_name.clone(),
                pk_columns: pk_col_names.clone(),
                pk_values,
                edits,
            });
        }

        cx.emit(TableViewEvent::SaveChanges(updates));
    }

    // ── Action handlers ───────────────────────────────────────────────────────

    fn on_commit_edit(&mut self, _: &CommitEdit, _: &mut Window, cx: &mut Context<Self>) {
        self.commit_edit(cx);
    }

    fn on_cancel_edit(&mut self, _: &CancelEdit, _: &mut Window, cx: &mut Context<Self>) {
        self.cancel_edit(cx);
    }

    fn on_save_edits(&mut self, _: &SaveEdits, _: &mut Window, cx: &mut Context<Self>) {
        self.save_changes(cx);
    }

    fn on_go_to_page(&mut self, _: &GoToPage, _: &mut Window, cx: &mut Context<Self>) {
        self.go_to_page(cx);
    }

    fn on_insert_new_row(&mut self, _: &InsertNewRow, _: &mut Window, cx: &mut Context<Self>) {
        self.save_new_row(cx);
    }

    fn save_new_row(&mut self, cx: &mut Context<Self>) {
        self.commit_new_row_edit(cx);
        if self.new_row_edits.is_empty() {
            return;
        }
        let column_values: Vec<(String, crate::db::types::Value)> = self
            .new_row_edits
            .iter()
            .filter_map(|(&col_idx, value)| {
                self.columns
                    .get(col_idx)
                    .map(|col| (col.name.clone(), text_to_value(value)))
            })
            .collect();
        cx.emit(TableViewEvent::InsertRow(NewRowInsert {
            database: self.database.clone(),
            table: self.table_name.clone(),
            column_values,
        }));
        cx.notify();
    }

    // ── Filters ───────────────────────────────────────────────────────────────

    pub fn go_to_page(&mut self, cx: &mut Context<Self>) {
        let input = self.page_jump_field.read(cx).content.to_string();
        let Ok(n) = input.trim().parse::<usize>() else {
            return;
        };
        let n = n.saturating_sub(1); // convert 1-based to 0-based
        let max_page = self
            .total_rows
            .map(|t| (t as usize).saturating_sub(1) / self.page_size)
            .unwrap_or(0);
        let target = n.min(max_page);
        if target != self.page {
            self.page = target;
            cx.emit(TableViewEvent::PageChanged);
        }
        self.page_jump_field
            .update(cx, |f, cx| f.set_content("", cx));
        cx.notify();
    }

    pub fn go_next_page(&mut self, cx: &mut Context<Self>) {
        let max_page = self
            .total_rows
            .map(|t| (t as usize).saturating_sub(1) / self.page_size)
            .unwrap_or(0);
        if self.page < max_page {
            self.page += 1;
            cx.emit(TableViewEvent::PageChanged);
            cx.notify();
        }
    }

    pub fn go_prev_page(&mut self, cx: &mut Context<Self>) {
        if self.page > 0 {
            self.page -= 1;
            cx.emit(TableViewEvent::PageChanged);
            cx.notify();
        }
    }

    fn add_filter(&mut self, cx: &mut Context<Self>) {
        let Some(col) = self.columns.get(self.filter_form_column_idx) else {
            return;
        };
        let op = FilterOp::all()[self.filter_form_op_idx].clone();
        let value = if op.needs_value() {
            let v = self.filter_form_value.read(cx).content.to_string();
            if v.is_empty() {
                return;
            }
            Some(v)
        } else {
            None
        };
        self.active_filters.push(Filter {
            column: col.name.clone(),
            op,
            value,
        });
        self.filter_form_value
            .update(cx, |f, cx| f.set_content("", cx));
        self.filter_form_visible = false;
        self.page = 0;
        cx.emit(TableViewEvent::FiltersChanged);
        cx.notify();
    }

    fn remove_filter(&mut self, idx: usize, cx: &mut Context<Self>) {
        self.active_filters.remove(idx);
        self.page = 0;
        cx.emit(TableViewEvent::FiltersChanged);
        cx.notify();
    }
}

impl Render for TableView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("TableView")
            .on_action(cx.listener(Self::on_commit_edit))
            .on_action(cx.listener(Self::on_cancel_edit))
            .on_action(cx.listener(Self::on_save_edits))
            .on_action(cx.listener(Self::on_go_to_page))
            .on_action(cx.listener(Self::on_insert_new_row))
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_toolbar(cx))
            .when(
                self.filter_form_visible && self.view_mode == ViewMode::Data,
                |d| d.child(self.render_filter_form(cx)),
            )
            .child(match self.view_mode {
                ViewMode::Data => self.render_grid(cx).into_any_element(),
                ViewMode::Schema => self.schema_view.clone().into_any_element(),
            })
    }
}

impl TableView {
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let row_info = if let Some(total) = self.total_rows {
            let start = self.page * self.page_size + 1;
            let end = self.page * self.page_size + self.rows.len();
            format!("{}\u{2013}{} of {}", start, end, total)
        } else {
            format!("{} rows", self.rows.len())
        };

        let on_first_page = self.page == 0;
        let on_last_page = self
            .total_rows
            .map(|t| (self.page + 1) * self.page_size >= t as usize)
            .unwrap_or(true);

        let pending_count =
            self.pending_edits.len() + if self.editing_cell.is_some() { 1 } else { 0 };
        let has_pending = pending_count > 0;

        let mut toolbar = div()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x333333));

        // Active filter chips
        if !self.active_filters.is_empty() {
            let mut chips = div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap_1()
                .px(px(12.))
                .pt(px(6.));
            for (i, filter) in self.active_filters.iter().enumerate() {
                let summary = filter.summary();
                chips = chips.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .bg(rgba(0x89b4fa22))
                        .border_1()
                        .border_color(rgb(0x89b4fa))
                        .rounded(px(4.))
                        .px(px(8.))
                        .py(px(3.))
                        .text_size(px(11.))
                        .text_color(rgb(0x89b4fa))
                        .child(summary)
                        .child(
                            div()
                                .id(ElementId::Integer(i as u64 + 8000))
                                .text_color(rgb(0x585b70))
                                .cursor_pointer()
                                .ml(px(4.))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.remove_filter(i, cx);
                                }))
                                .child("×"),
                        ),
                );
            }
            toolbar = toolbar.child(chips);
        }

        // Save error notice
        if let Some(ref err) = self.save_error {
            toolbar = toolbar.child(
                div()
                    .px(px(12.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(rgb(0xf38ba8))
                    .child(err.clone()),
            );
        }

        // Insert error notice
        if let Some(ref err) = self.new_row_insert_error {
            toolbar = toolbar.child(
                div()
                    .px(px(12.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(rgb(0xf38ba8))
                    .child(err.clone()),
            );
        }

        // Buttons row
        let mut button_row = div()
            .flex()
            .flex_row()
            .items_center()
            .px(px(12.))
            .py(px(8.))
            .gap_2()
            .child(
                div()
                    .id("view-mode-data")
                    .bg(if self.view_mode == ViewMode::Data {
                        rgb(0x89b4fa)
                    } else {
                        rgb(0x313244)
                    })
                    .text_color(if self.view_mode == ViewMode::Data {
                        rgb(0x1e1e2e)
                    } else {
                        rgb(0x6c7086)
                    })
                    .font_weight(if self.view_mode == ViewMode::Data {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.view_mode = ViewMode::Data;
                        cx.notify();
                    }))
                    .child("Data"),
            )
            .child(
                div()
                    .id("view-mode-schema")
                    .bg(if self.view_mode == ViewMode::Schema {
                        rgb(0x89b4fa)
                    } else {
                        rgb(0x313244)
                    })
                    .text_color(if self.view_mode == ViewMode::Schema {
                        rgb(0x1e1e2e)
                    } else {
                        rgb(0x6c7086)
                    })
                    .font_weight(if self.view_mode == ViewMode::Schema {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.view_mode = ViewMode::Schema;
                        this.filter_form_visible = false;
                        this.new_row_active = false;
                        cx.notify();
                    }))
                    .child("Schema"),
            )
            .child(
                div()
                    .w(px(1.))
                    .h(px(16.))
                    .bg(rgb(0x45475a))
                    .flex_shrink_0(),
            )
            .child(
                div()
                    .id("filter-btn")
                    .bg(rgb(0x313244))
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(rgb(0xa6adc8))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.filter_form_visible = !this.filter_form_visible;
                        this.column_dropdown_open = false;
                        this.op_dropdown_open = false;
                        cx.notify();
                    }))
                    .child("+ Filter"),
            )
            .child(
                div()
                    .id("new-row-btn")
                    .bg(if self.new_row_active {
                        rgb(0xa6e3a1)
                    } else {
                        rgb(0x313244)
                    })
                    .text_color(if self.new_row_active {
                        rgb(0x1e1e2e)
                    } else {
                        rgb(0xa6adc8)
                    })
                    .font_weight(if self.new_row_active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.new_row_active {
                            this.cancel_new_row(cx);
                        } else {
                            this.new_row_active = true;
                            cx.notify();
                        }
                    }))
                    .child("+ New Row"),
            );

        if has_pending {
            let label = if pending_count == 1 {
                "Save 1 change".to_string()
            } else {
                format!("Save {} changes", pending_count)
            };
            button_row = button_row
                .child(
                    div()
                        .id("save-edits-btn")
                        .bg(rgb(0xa6e3a1))
                        .text_color(rgb(0x1e1e2e))
                        .font_weight(FontWeight::SEMIBOLD)
                        .rounded(px(4.))
                        .px(px(10.))
                        .py(px(4.))
                        .text_size(px(11.))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.save_changes(cx);
                        }))
                        .child(label),
                )
                .child(
                    div()
                        .id("discard-edits-btn")
                        .bg(rgb(0x313244))
                        .text_color(rgb(0xf38ba8))
                        .rounded(px(4.))
                        .px(px(10.))
                        .py(px(4.))
                        .text_size(px(11.))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.pending_edits.clear();
                            this.editing_cell = None;
                            this.save_error = None;
                            cx.notify();
                        }))
                        .child("Discard"),
                );
        }

        let has_multiple_pages = self
            .total_rows
            .map(|t| t as usize > self.page_size)
            .unwrap_or(false);

        button_row = button_row.child(div().flex_1());

        // Row info label (always shown when data is loaded)
        button_row = button_row.child(
            div()
                .text_size(px(11.))
                .text_color(rgb(0x6c7086))
                .child(row_info),
        );

        // Pagination controls — only when there is more than one page
        if has_multiple_pages {
            button_row = button_row
                .child(
                    div()
                        .id("page-prev-btn")
                        .bg(if on_first_page {
                            rgb(0x1e1e2e)
                        } else {
                            rgb(0x313244)
                        })
                        .rounded(px(4.))
                        .px(px(8.))
                        .py(px(3.))
                        .ml(px(8.))
                        .text_size(px(12.))
                        .text_color(if on_first_page {
                            rgb(0x45475a)
                        } else {
                            rgb(0xa6adc8)
                        })
                        .when(!on_first_page, |d| d.cursor_pointer())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.go_prev_page(cx);
                        }))
                        .child("‹"),
                )
                .child(
                    div()
                        .key_context("PageJumpField")
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .w(px(64.))
                                .overflow_hidden()
                                .child(self.page_jump_field.clone()),
                        )
                        .child(
                            div()
                                .id("page-go-btn")
                                .bg(rgb(0x313244))
                                .rounded(px(4.))
                                .px(px(8.))
                                .py(px(3.))
                                .text_size(px(11.))
                                .text_color(rgb(0xa6adc8))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.go_to_page(cx);
                                }))
                                .child("Go"),
                        ),
                )
                .child(
                    div()
                        .id("page-next-btn")
                        .bg(if on_last_page {
                            rgb(0x1e1e2e)
                        } else {
                            rgb(0x313244)
                        })
                        .rounded(px(4.))
                        .px(px(8.))
                        .py(px(3.))
                        .text_size(px(12.))
                        .text_color(if on_last_page {
                            rgb(0x45475a)
                        } else {
                            rgb(0xa6adc8)
                        })
                        .when(!on_last_page, |d| d.cursor_pointer())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.go_next_page(cx);
                        }))
                        .child("›"),
                );
        }

        toolbar.child(button_row)
    }

    fn render_filter_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ops = FilterOp::all();
        let selected_op = &ops[self.filter_form_op_idx];
        let col_name = self
            .columns
            .get(self.filter_form_column_idx)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "—".to_string());

        let col_open = self.column_dropdown_open;
        let op_open = self.op_dropdown_open;

        let form = div()
            .flex()
            .flex_col()
            .px(px(12.))
            .py(px(8.))
            .gap_2()
            .bg(rgb(0x181825))
            .border_b_1()
            .border_color(rgb(0x333333));

        let mut row = div().flex().flex_row().items_start().gap_2();

        // Column dropdown
        let mut col_dd = div().flex().flex_col().w(px(160.));
        col_dd = col_dd.child(
            div()
                .id("filter-col-btn")
                .flex()
                .justify_between()
                .items_center()
                .bg(rgb(0x313244))
                .rounded(px(4.))
                .px(px(10.))
                .py(px(6.))
                .text_size(px(12.))
                .text_color(rgb(0xcdd6f4))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.column_dropdown_open = !this.column_dropdown_open;
                    this.op_dropdown_open = false;
                    cx.notify();
                }))
                .child(col_name)
                .child(
                    div()
                        .text_size(px(10.))
                        .child(if col_open { "▲" } else { "▼" }),
                ),
        );
        if col_open {
            let mut list = div()
                .id("filter-col-list")
                .mt(px(2.))
                .bg(rgb(0x313244))
                .rounded(px(4.))
                .flex()
                .flex_col()
                .max_h(px(200.))
                .overflow_y_scroll();
            for (i, col) in self.columns.iter().enumerate() {
                let name = col.name.clone();
                let is_sel = i == self.filter_form_column_idx;
                list = list.child(
                    div()
                        .id(ElementId::Integer(i as u64 + 9000))
                        .px(px(10.))
                        .py(px(5.))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .when(is_sel, |d| d.bg(rgb(0x45475a)).text_color(rgb(0xcdd6f4)))
                        .when(!is_sel, |d| d.text_color(rgb(0xa6adc8)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.filter_form_column_idx = i;
                            this.column_dropdown_open = false;
                            cx.notify();
                        }))
                        .child(name),
                );
            }
            col_dd = col_dd.child(list);
        }
        row = row.child(col_dd);

        // Operator dropdown
        let mut op_dd = div().flex().flex_col().w(px(200.));
        op_dd = op_dd.child(
            div()
                .id("filter-op-btn")
                .flex()
                .justify_between()
                .items_center()
                .bg(rgb(0x313244))
                .rounded(px(4.))
                .px(px(10.))
                .py(px(6.))
                .text_size(px(12.))
                .text_color(rgb(0xcdd6f4))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.op_dropdown_open = !this.op_dropdown_open;
                    this.column_dropdown_open = false;
                    cx.notify();
                }))
                .child(selected_op.label())
                .child(
                    div()
                        .text_size(px(10.))
                        .child(if op_open { "▲" } else { "▼" }),
                ),
        );
        if op_open {
            let mut list = div()
                .id("filter-op-list")
                .mt(px(2.))
                .bg(rgb(0x313244))
                .rounded(px(4.))
                .flex()
                .flex_col()
                .max_h(px(300.))
                .overflow_y_scroll();
            for (i, op) in ops.iter().enumerate() {
                let is_sel = i == self.filter_form_op_idx;
                let label = op.label();
                list = list.child(
                    div()
                        .id(ElementId::Integer(i as u64 + 10000))
                        .px(px(10.))
                        .py(px(5.))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .when(is_sel, |d| d.bg(rgb(0x45475a)).text_color(rgb(0xcdd6f4)))
                        .when(!is_sel, |d| d.text_color(rgb(0xa6adc8)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.filter_form_op_idx = i;
                            this.op_dropdown_open = false;
                            cx.notify();
                        }))
                        .child(label),
                );
            }
            op_dd = op_dd.child(list);
        }
        row = row.child(op_dd);

        if selected_op.needs_value() {
            row = row.child(div().flex_1().child(self.filter_form_value.clone()));
        }

        row = row
            .child(
                div()
                    .id("filter-apply")
                    .bg(rgb(0x89b4fa))
                    .text_color(rgb(0x1e1e2e))
                    .font_weight(FontWeight::SEMIBOLD)
                    .rounded(px(4.))
                    .px(px(12.))
                    .py(px(6.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.add_filter(cx);
                    }))
                    .child("Add"),
            )
            .child(
                div()
                    .id("filter-cancel")
                    .bg(rgb(0x313244))
                    .text_color(rgb(0xa6adc8))
                    .rounded(px(4.))
                    .px(px(12.))
                    .py(px(6.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.filter_form_visible = false;
                        this.column_dropdown_open = false;
                        this.op_dropdown_open = false;
                        cx.notify();
                    }))
                    .child("Cancel"),
            );

        form.child(row)
    }

    fn render_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if self.loading {
            return div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .child(div().text_color(rgb(0x6c7086)).child("Loading…"))
                .into_any_element();
        }

        if let Some(err) = &self.error {
            return div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .child(div().text_color(rgb(0xf38ba8)).child(err.clone()))
                .into_any_element();
        }

        if self.columns.is_empty() {
            return div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .child(div().text_color(rgb(0x6c7086)).child("No results"))
                .into_any_element();
        }

        let mut table = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .id("table-view-grid")
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle);

        // Header
        let mut header = div()
            .flex()
            .flex_row()
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x333333))
            .flex_shrink_0();
        for col in &self.columns {
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
        table = table.child(header);

        // Rows
        for (row_idx, row) in self.rows.iter().enumerate() {
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

            for (col_idx, val) in row.iter().enumerate() {
                let is_editing = self.editing_cell == Some((row_idx, col_idx));
                let pending_value = self.pending_edits.get(&(row_idx, col_idx)).cloned();

                let cell = if is_editing {
                    div()
                        .w(px(150.))
                        .flex_shrink_0()
                        .px(px(4.))
                        .py(px(2.))
                        .bg(rgba(0x89b4fa22))
                        .child(self.edit_field.clone())
                        .into_any_element()
                } else {
                    let display = pending_value.clone().unwrap_or_else(|| val.to_string());
                    let color = if pending_value.is_some() {
                        rgb(0xf9e2af)
                    } else {
                        match val {
                            Value::Null => rgb(0x6c7086),
                            Value::Int(_) | Value::Float(_) => rgb(0xfab387),
                            Value::Bool(_) => rgb(0xcba6f7),
                            Value::DateTime(_) => rgb(0xa6e3a1),
                            _ => rgb(0xcdd6f4),
                        }
                    };

                    let col_name = self
                        .columns
                        .get(col_idx)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    let fk_info = self
                        .foreign_keys
                        .iter()
                        .find(|fk| fk.column == col_name)
                        .map(|fk| {
                            (
                                fk.ref_database.clone(),
                                fk.ref_table.clone(),
                                fk.ref_column.clone(),
                            )
                        });

                    let mut cell_div = div()
                        .id(ElementId::Name(
                            format!("cell-{}-{}", row_idx, col_idx).into(),
                        ))
                        .w(px(150.))
                        .flex_shrink_0()
                        .flex()
                        .flex_row()
                        .items_center()
                        .px(px(12.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .text_color(color)
                        .overflow_hidden()
                        .cursor(CursorStyle::IBeam)
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.click_count() >= 2 {
                                this.start_editing(row_idx, col_idx, cx);
                                let fh = this.edit_field.read(cx).focus_handle.clone();
                                window.focus(&fh, cx);
                            }
                        }))
                        .child(div().flex_1().overflow_hidden().child(display));

                    if let Some((ref_database, ref_table, ref_column)) = fk_info {
                        let val_for_fk = val.clone();
                        cell_div = cell_div.child(
                            div()
                                .id(ElementId::Name(
                                    format!("fk-{}-{}", row_idx, col_idx).into(),
                                ))
                                .flex_shrink_0()
                                .ml(px(2.))
                                .px(px(4.))
                                .py(px(1.))
                                .text_size(px(10.))
                                .text_color(rgb(0x89b4fa))
                                .cursor_pointer()
                                .on_click(cx.listener(
                                    move |_this, _event: &ClickEvent, _window, cx| {
                                        cx.stop_propagation();
                                        cx.emit(TableViewEvent::NavigateToFk {
                                            database: ref_database.clone(),
                                            table: ref_table.clone(),
                                            column: ref_column.clone(),
                                            value: val_for_fk.clone(),
                                        });
                                    },
                                ))
                                .child("→"),
                        );
                    }

                    cell_div.into_any_element()
                };

                row_el = row_el.child(cell);
            }

            table = table.child(row_el);
        }

        if self.new_row_active {
            let has_insert_error = self.new_row_insert_error.is_some();
            let mut new_row_el = div()
                .flex()
                .flex_row()
                .bg(if has_insert_error {
                    rgba(0xf38ba815u32)
                } else {
                    rgba(0xa6e3a115u32)
                })
                .border_b_1()
                .border_color(if has_insert_error {
                    rgb(0xf38ba8)
                } else {
                    rgb(0xa6e3a1)
                });

            for (col_idx, col) in self.columns.iter().enumerate() {
                let is_auto = col.is_primary_key && col.extra.contains("auto_increment");
                let is_editing_this = self.editing_new_row_col == Some(col_idx);
                let pending = self.new_row_edits.get(&col_idx).cloned();

                let cell: AnyElement = if is_auto {
                    div()
                        .w(px(150.))
                        .flex_shrink_0()
                        .px(px(12.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .text_color(rgb(0x45475a))
                        .child("auto")
                        .into_any_element()
                } else if is_editing_this {
                    div()
                        .w(px(150.))
                        .flex_shrink_0()
                        .px(px(4.))
                        .py(px(2.))
                        .bg(rgba(0xa6e3a122u32))
                        .child(self.edit_field.clone())
                        .into_any_element()
                } else {
                    let display = pending.clone().unwrap_or_else(|| col.name.clone());
                    let color = if pending.is_some() {
                        rgb(0xa6e3a1)
                    } else {
                        rgb(0x45475a)
                    };
                    div()
                        .id(ElementId::Name(format!("new-row-col-{}", col_idx).into()))
                        .w(px(150.))
                        .flex_shrink_0()
                        .px(px(12.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .text_color(color)
                        .overflow_hidden()
                        .cursor(CursorStyle::IBeam)
                        .on_click(cx.listener(move |this, _event: &ClickEvent, window, cx| {
                            this.commit_edit(cx);
                            this.commit_new_row_edit(cx);
                            this.editing_new_row_col = Some(col_idx);
                            this.new_row_dirty = true;
                            this.new_row_insert_error = None;
                            let val = this
                                .new_row_edits
                                .get(&col_idx)
                                .cloned()
                                .unwrap_or_default();
                            this.edit_field.update(cx, |f, cx| f.set_content(&val, cx));
                            let fh = this.edit_field.read(cx).focus_handle.clone();
                            window.focus(&fh, cx);
                            cx.notify();
                        }))
                        .child(display)
                        .into_any_element()
                };

                new_row_el = new_row_el.child(cell);
            }

            let insert_enabled = self.new_row_insert_error.is_none() || self.new_row_dirty;

            let insert_btn = if insert_enabled {
                div()
                    .id("new-row-insert-btn")
                    .bg(rgb(0xa6e3a1))
                    .text_color(rgb(0x1e1e2e))
                    .font_weight(FontWeight::SEMIBOLD)
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.save_new_row(cx);
                    }))
                    .child("Insert")
            } else {
                div()
                    .id("new-row-insert-btn")
                    .bg(rgb(0x45475a))
                    .text_color(rgb(0x6c7086))
                    .font_weight(FontWeight::SEMIBOLD)
                    .rounded(px(4.))
                    .px(px(10.))
                    .py(px(4.))
                    .text_size(px(11.))
                    .child("Insert")
            };

            new_row_el = new_row_el.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .px(px(8.))
                    .child(insert_btn)
                    .child(
                        div()
                            .id("new-row-cancel-btn")
                            .bg(rgb(0x313244))
                            .text_color(rgb(0xa6adc8))
                            .rounded(px(4.))
                            .px(px(10.))
                            .py(px(4.))
                            .text_size(px(11.))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_new_row(cx);
                            }))
                            .child("Cancel"),
                    ),
            );

            table = table.child(new_row_el);
        }

        table.into_any_element()
    }
}
