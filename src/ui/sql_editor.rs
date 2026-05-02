use std::ops::Range;

use gpui::prelude::FluentBuilder;
use gpui::*;
use unicode_segmentation::UnicodeSegmentation;

use crate::query::highlight;

actions!(
    sql_editor,
    [
        Backspace, Delete, Left, Right, Up, Down, SelectAll, Home, End, Paste, Cut, Copy, Enter,
        Tab
    ]
);

pub fn register_sql_editor_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("SqlEditor")),
        KeyBinding::new("delete", Delete, Some("SqlEditor")),
        KeyBinding::new("left", Left, Some("SqlEditor")),
        KeyBinding::new("right", Right, Some("SqlEditor")),
        KeyBinding::new("up", Up, Some("SqlEditor")),
        KeyBinding::new("down", Down, Some("SqlEditor")),
        KeyBinding::new("cmd-a", SelectAll, Some("SqlEditor")),
        KeyBinding::new("home", Home, Some("SqlEditor")),
        KeyBinding::new("end", End, Some("SqlEditor")),
        KeyBinding::new("cmd-v", Paste, Some("SqlEditor")),
        KeyBinding::new("cmd-c", Copy, Some("SqlEditor")),
        KeyBinding::new("cmd-x", Cut, Some("SqlEditor")),
        KeyBinding::new("enter", Enter, Some("SqlEditor")),
        KeyBinding::new("tab", Tab, Some("SqlEditor")),
    ]);
}

pub struct SqlEditor {
    pub focus_handle: FocusHandle,
    pub content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    // (start_byte_offset, shaped_line) per line, filled during prepaint
    last_line_layouts: Vec<(usize, ShapedLine)>,
    last_bounds: Option<Bounds<Pixels>>,
    last_line_height: Pixels,
    is_selecting: bool,
}

impl SqlEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_line_layouts: vec![],
            last_bounds: None,
            last_line_height: px(20.),
            is_selecting: false,
        }
    }

    pub fn set_content(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = text.into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn selected_sql(&self) -> Option<&str> {
        if self.selected_range.is_empty() {
            None
        } else {
            Some(&self.content[self.selected_range.clone()])
        }
    }

    // ── content helpers ──────────────────────────────────────────────────────

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    // Returns (line_idx, byte_col_within_line, line_start_byte_offset)
    fn cursor_line_col(&self) -> (usize, usize, usize) {
        let cursor = self.cursor_offset();
        let mut line_start = 0usize;
        for (i, line) in self.content.split('\n').enumerate() {
            let line_end = line_start + line.len();
            if cursor <= line_end {
                return (i, cursor - line_start, line_start);
            }
            line_start = line_end + 1;
        }
        // cursor at very end
        let last_line = self.content.split('\n').next_back().unwrap_or("");
        let ls = self.content.len().saturating_sub(last_line.len());
        let n = self.content.split('\n').count();
        (n.saturating_sub(1), self.content.len() - ls, ls)
    }

    fn offset_at_line_col(&self, target_line: usize, target_col: usize) -> usize {
        let mut offset = 0usize;
        for (i, line) in self.content.split('\n').enumerate() {
            if i == target_line {
                // clamp to valid UTF-8 grapheme boundary
                let clamped = target_col.min(line.len());
                // walk back to nearest grapheme start
                let adjusted = line
                    .grapheme_indices(true)
                    .map(|(idx, _)| idx)
                    .rfind(|&idx| idx <= clamped)
                    .unwrap_or(0);
                return offset + adjusted;
            }
            offset += line.len() + 1;
        }
        self.content.len()
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };
        let lh = self.last_line_height;
        let rel_y = (position.y - bounds.top()).max(px(0.));
        let lh_f = f32::from(lh);
        let line_idx = if lh_f > 0.0 {
            ((f32::from(rel_y) / lh_f).floor() as usize)
                .min(self.last_line_layouts.len().saturating_sub(1))
        } else {
            0
        };
        let Some((start_offset, line)) = self.last_line_layouts.get(line_idx) else {
            return 0;
        };
        let rel_x = (position.x - bounds.left()).max(px(0.));
        start_offset + line.closest_index_for_x(rel_x)
    }

    // ── UTF-16 offset helpers (for EntityInputHandler) ───────────────────────

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(r.start)..self.offset_from_utf16(r.end)
    }

    // ── action handlers ──────────────────────────────────────────────────────

    fn on_backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                return;
            }
            self.select_to(prev, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn on_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                return;
            }
            self.select_to(next, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn on_left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn on_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn on_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        let (line_idx, col, _) = self.cursor_line_col();
        if line_idx == 0 {
            self.move_to(0, cx);
            return;
        }
        let new_offset = self.offset_at_line_col(line_idx - 1, col);
        self.move_to(new_offset, cx);
    }

    fn on_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.content.split('\n').count();
        let (line_idx, col, _) = self.cursor_line_col();
        if line_idx >= count - 1 {
            self.move_to(self.content.len(), cx);
            return;
        }
        let new_offset = self.offset_at_line_col(line_idx + 1, col);
        self.move_to(new_offset, cx);
    }

    fn on_home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        let (_, _, line_start) = self.cursor_line_col();
        self.move_to(line_start, cx);
    }

    fn on_end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        let (line_idx, _, line_start) = self.cursor_line_col();
        let line_len = self
            .content
            .split('\n')
            .nth(line_idx)
            .map(|l| l.len())
            .unwrap_or(0);
        self.move_to(line_start + line_len, cx);
    }

    fn on_select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn on_enter(&mut self, _: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    fn on_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "  ", window, cx);
    }

    fn on_paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.is_selecting = true;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }
}

// ── EntityInputHandler ────────────────────────────────────────────────────────

impl EntityInputHandler for SqlEditor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range.as_ref().map(|r| self.range_to_utf16(r))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range = None;
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.marked_range = if !new_text.is_empty() {
            Some(range.start..range.start + new_text.len())
        } else {
            None
        };
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .map(|r| r.start + range.start..r.end + range.start)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let (line_idx, col, _) = {
            let cursor = range.start;
            let mut ls = 0usize;
            let mut found = (0, 0, 0);
            for (i, line) in self.content.split('\n').enumerate() {
                let le = ls + line.len();
                if cursor <= le {
                    found = (i, cursor - ls, ls);
                    break;
                }
                ls = le + 1;
            }
            found
        };
        let (start_offset, ref line) = self.last_line_layouts.get(line_idx)?;
        let _ = start_offset;
        let bounds = self.last_bounds?;
        let x = bounds.left() + line.x_for_index(col);
        let y = bounds.top() + self.last_line_height * line_idx as f32;
        Some(Bounds::new(
            point(x, y),
            size(px(1.), self.last_line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        pt: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.offset_to_utf16(self.index_for_mouse_position(pt)))
    }
}

// ── Focusable + Render ────────────────────────────────────────────────────────

impl Focusable for SqlEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SqlEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let is_empty = self.content.is_empty();

        div()
            .key_context("SqlEditor")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::on_backspace))
            .on_action(cx.listener(Self::on_delete))
            .on_action(cx.listener(Self::on_left))
            .on_action(cx.listener(Self::on_right))
            .on_action(cx.listener(Self::on_up))
            .on_action(cx.listener(Self::on_down))
            .on_action(cx.listener(Self::on_select_all))
            .on_action(cx.listener(Self::on_home))
            .on_action(cx.listener(Self::on_end))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_enter))
            .on_action(cx.listener(Self::on_tab))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .size_full()
            .bg(rgb(0x181825))
            .when(focused, |d| d.bg(rgb(0x1e1e2e)))
            .p(px(12.))
            .text_size(px(13.))
            .text_color(rgb(0xcdd6f4))
            .font_family("monospace")
            .child(SqlEditorElement {
                editor: cx.entity(),
                show_placeholder: is_empty,
            })
    }
}

#[allow(dead_code)]
fn when<E: IntoElement>(cond: bool, f: impl FnOnce() -> E) -> Option<E> {
    if cond {
        Some(f())
    } else {
        None
    }
}

// ── Custom element ────────────────────────────────────────────────────────────

struct SqlEditorElement {
    editor: Entity<SqlEditor>,
    show_placeholder: bool,
}

struct SqlEditorPrepaint {
    line_layouts: Vec<(usize, ShapedLine)>,
    line_height: Pixels,
    cursor: Option<PaintQuad>,
    selection_quads: Vec<PaintQuad>,
}

impl IntoElement for SqlEditorElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for SqlEditorElement {
    type RequestLayoutState = ();
    type PrepaintState = SqlEditorPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let editor = self.editor.read(cx);
        let line_count = editor.content.split('\n').count().max(1) as f32;
        let lh = window.line_height();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (lh * line_count).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> SqlEditorPrepaint {
        let editor = self.editor.read(cx);
        let content = editor.content.clone();
        let selected_range = editor.selected_range.clone();
        let cursor_offset = editor.cursor_offset();
        let style = window.text_style();
        let lh = window.line_height();
        let font_size = style.font_size.to_pixels(window.rem_size());

        let mut line_layouts = vec![];
        let mut line_start = 0usize;

        // Spans are recomputed every prepaint; fast enough for typical SQL lengths.
        let highlight_spans = highlight::highlight(&content);
        let default_color = style.color;

        for raw_line in content.split('\n') {
            let display: SharedString = raw_line.to_string().into();
            let runs = build_text_runs(
                raw_line,
                line_start,
                &highlight_spans,
                style.font(),
                default_color,
            );
            let shaped = window
                .text_system()
                .shape_line(display.clone(), font_size, &runs, None);
            line_layouts.push((line_start, shaped));
            line_start += raw_line.len() + 1; // +1 for '\n'
        }

        // Selection quads — one per line that overlaps the selection
        let mut selection_quads = vec![];
        if !selected_range.is_empty() {
            for (line_idx, (ls, ref line)) in line_layouts.iter().enumerate() {
                let line_text = content.split('\n').nth(line_idx).unwrap_or("");
                let le = ls + line_text.len();
                let sel_start = selected_range.start.max(*ls).min(le) - ls;
                let sel_end = selected_range.end.max(*ls).min(le) - ls;
                if sel_start < sel_end {
                    let x1 = bounds.left() + line.x_for_index(sel_start);
                    let x2 = bounds.left() + line.x_for_index(sel_end);
                    let y = bounds.top() + lh * line_idx as f32;
                    selection_quads.push(fill(
                        Bounds::from_corners(point(x1, y), point(x2, y + lh)),
                        rgba(0x89b4fa33),
                    ));
                }
            }
        }

        // Cursor quad
        let cursor = {
            // find which line the cursor is on
            let mut cursor_line = 0;
            let mut cursor_col = 0;
            let mut ls = 0usize;
            for (i, raw) in content.split('\n').enumerate() {
                let le = ls + raw.len();
                if cursor_offset <= le {
                    cursor_line = i;
                    cursor_col = cursor_offset - ls;
                    break;
                }
                ls = le + 1;
            }
            if let Some((_, ref line)) = line_layouts.get(cursor_line) {
                let cx_pos = bounds.left() + line.x_for_index(cursor_col);
                let cy = bounds.top() + lh * cursor_line as f32;
                Some(fill(
                    Bounds::new(point(cx_pos, cy), size(px(2.), lh)),
                    rgb(0x89b4fa),
                ))
            } else {
                None
            }
        };

        SqlEditorPrepaint {
            line_layouts,
            line_height: lh,
            cursor,
            selection_quads,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut SqlEditorPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );

        for quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(quad);
        }

        let lh = prepaint.line_height;
        if self.show_placeholder {
            let style = window.text_style();
            let font_size = style.font_size.to_pixels(window.rem_size());
            let placeholder: SharedString = "SELECT * FROM table_name".into();
            let run = TextRun {
                len: placeholder.len(),
                font: style.font(),
                color: rgba(0x45475a88).into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = window
                .text_system()
                .shape_line(placeholder, font_size, &[run], None);
            shaped
                .paint(bounds.origin, lh, TextAlign::Left, None, window, cx)
                .ok();
        } else {
            for (i, (_offset, line)) in prepaint.line_layouts.iter().enumerate() {
                let origin = point(bounds.left(), bounds.top() + lh * i as f32);
                line.paint(origin, lh, TextAlign::Left, None, window, cx)
                    .ok();
            }
        }

        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }

        self.editor.update(cx, |editor, _| {
            let layouts = std::mem::take(&mut prepaint.line_layouts);
            editor.last_line_layouts = layouts;
            editor.last_bounds = Some(bounds);
            editor.last_line_height = lh;
        });
    }
}

pub(crate) fn build_text_runs(
    line_text: &str,
    line_start: usize,
    spans: &[(std::ops::Range<usize>, gpui::Rgba)],
    font: gpui::Font,
    default_color: gpui::Hsla,
) -> Vec<gpui::TextRun> {
    let line_end = line_start + line_text.len();
    let mut runs: Vec<gpui::TextRun> = Vec::new();
    let mut pos = 0usize; // byte position within line_text

    for (range, color) in spans {
        let span_start = range.start.max(line_start);
        let span_end = range.end.min(line_end);
        if span_start >= span_end {
            continue;
        }
        let local_start = span_start - line_start;
        let local_end = span_end - line_start;

        debug_assert!(
            local_start >= pos,
            "spans must be non-overlapping and ordered by start; \
             got local_start={local_start} but pos={pos}"
        );

        if local_start > pos {
            runs.push(gpui::TextRun {
                len: local_start - pos,
                font: font.clone(),
                color: default_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            });
        }
        runs.push(gpui::TextRun {
            len: local_end - local_start,
            font: font.clone(),
            color: (*color).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        });
        pos = local_end;
    }

    if pos < line_text.len() {
        runs.push(gpui::TextRun {
            len: line_text.len() - pos,
            font: font.clone(),
            color: default_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }

    // Empty line: one zero-length run so shape_line gets a valid (empty) slice.
    if runs.is_empty() {
        runs.push(gpui::TextRun {
            len: 0,
            font: font.clone(),
            color: default_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }

    runs
}

#[cfg(test)]
mod tests {
    use super::build_text_runs;
    use gpui::{black, rgba, Font, Hsla, Rgba};

    fn default_font() -> Font {
        Font::default()
    }

    fn default_color() -> Hsla {
        black()
    }

    #[test]
    fn test_build_text_runs_empty_line() {
        let runs = build_text_runs("", 0, &[], default_font(), default_color());
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].len, 0);
    }

    #[test]
    fn test_build_text_runs_no_spans() {
        let runs = build_text_runs("hello", 0, &[], default_font(), default_color());
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].len, 5);
        assert_eq!(runs[0].color, default_color());
    }

    #[test]
    fn test_build_text_runs_single_span_full_line() {
        let blue: Rgba = rgba(0x89b4faff);
        let spans = vec![(0..5, blue)];
        let runs = build_text_runs("hello", 0, &spans, default_font(), default_color());
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].len, 5);
        assert_eq!(runs[0].color, blue.into());
    }

    #[test]
    fn test_build_text_runs_span_with_gap_before_and_after() {
        // line: "  hi  " (6 bytes), span covers bytes 2..4
        let blue: Rgba = rgba(0x89b4faff);
        let spans = vec![(2..4, blue)];
        let runs = build_text_runs("  hi  ", 0, &spans, default_font(), default_color());
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].len, 2); // gap before
        assert_eq!(runs[0].color, default_color());
        assert_eq!(runs[1].len, 2); // span
        assert_eq!(runs[1].color, blue.into());
        assert_eq!(runs[2].len, 2); // gap after
        assert_eq!(runs[2].color, default_color());
        assert_eq!(runs.iter().map(|r| r.len).sum::<usize>(), 6);
    }

    #[test]
    fn test_build_text_runs_span_on_second_line() {
        // Full content: "ab\ncd" — second line "cd" starts at byte 3
        // A span covering bytes 3..5 (= "cd") should color the full line
        let green: Rgba = rgba(0xa6e3a1ff);
        let spans = vec![(3..5, green)];
        let runs = build_text_runs("cd", 3, &spans, default_font(), default_color());
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].len, 2);
        assert_eq!(runs[0].color, green.into());
    }

    #[test]
    fn test_build_text_runs_span_clipped_to_line() {
        // Span covers bytes 0..10 but line is only 5 bytes
        let blue: Rgba = rgba(0x89b4faff);
        let spans = vec![(0..10, blue)];
        let runs = build_text_runs("hello", 0, &spans, default_font(), default_color());
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].len, 5);
        assert_eq!(runs.iter().map(|r| r.len).sum::<usize>(), 5);
    }

    #[test]
    fn test_build_text_runs_lens_sum_to_line_len() {
        let blue: Rgba = rgba(0x89b4faff);
        let green: Rgba = rgba(0xa6e3a1ff);
        // "SELECT 42" — span 0..6 (keyword), span 7..9 (number)
        let spans = vec![(0..6, blue), (7..9, green)];
        let runs = build_text_runs("SELECT 42", 0, &spans, default_font(), default_color());
        assert_eq!(runs.iter().map(|r| r.len).sum::<usize>(), 9);
    }

    fn selected_sql_from(content: &str, range: std::ops::Range<usize>) -> Option<&str> {
        if range.is_empty() {
            None
        } else {
            Some(&content[range])
        }
    }

    #[test]
    fn test_selected_sql_no_selection() {
        assert_eq!(selected_sql_from("SELECT 1", 0..0), None);
    }

    #[test]
    fn test_selected_sql_with_selection() {
        assert_eq!(selected_sql_from("SELECT 1", 0..6), Some("SELECT"));
    }
}
