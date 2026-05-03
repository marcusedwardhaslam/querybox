# Text Editor Keybindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add standard macOS text editor keybindings to `SqlEditor` and `TextField`, including cmd+arrow navigation, shift-selection extension, option word navigation, cmd-backspace, and undo/redo.

**Architecture:** A new `text_motion` module provides pure movement-calculation functions (line start/end, word boundaries) shared by both components. Each component gains undo/redo stacks via snapshots in `replace_text_in_range`, and new GPUI action types for every keybinding combination.

**Tech Stack:** Rust, GPUI (Zed's UI framework), `unicode-segmentation` crate (already in use)

---

## File Map

| File | Change |
|------|--------|
| `src/ui/text_motion.rs` | **Create** — pure functions: `line_start`, `line_end`, `prev_word_start`, `next_word_end` |
| `src/ui/mod.rs` | **Modify** — add `pub mod text_motion;` |
| `src/ui/sql_editor.rs` | **Modify** — undo/redo fields, new actions, keybindings, handlers |
| `src/ui/text_field.rs` | **Modify** — undo/redo fields, new actions, keybindings, handlers |

---

### Task 1: Create `src/ui/text_motion.rs`

**Files:**
- Create: `src/ui/text_motion.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create `src/ui/text_motion.rs` with tests and `todo!()` stubs**

```rust
pub fn line_start(_text: &str, _offset: usize) -> usize { todo!() }
pub fn line_end(_text: &str, _offset: usize) -> usize { todo!() }
pub fn prev_word_start(_text: &str, _offset: usize) -> usize { todo!() }
pub fn next_word_end(_text: &str, _offset: usize) -> usize { todo!() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_start() {
        assert_eq!(line_start("hello\nworld", 8), 6); // inside "world"
        assert_eq!(line_start("hello\nworld", 3), 0); // inside "hello"
        assert_eq!(line_start("hello\nworld", 6), 6); // at start of "world"
        assert_eq!(line_start("hello", 3), 0);         // single line
        assert_eq!(line_start("hello\nworld", 0), 0);  // at document start
    }

    #[test]
    fn test_line_end() {
        assert_eq!(line_end("hello\nworld", 3), 5);   // inside "hello"
        assert_eq!(line_end("hello\nworld", 8), 11);  // inside "world"
        assert_eq!(line_end("hello\nworld", 5), 5);   // at end of "hello"
        assert_eq!(line_end("hello", 3), 5);           // single line
    }

    #[test]
    fn test_prev_word_start() {
        assert_eq!(prev_word_start("hello world", 11), 6);  // after "world"
        assert_eq!(prev_word_start("hello world", 5), 0);   // after "hello"
        assert_eq!(prev_word_start("hello  world", 12), 7); // after "world" with double space
        assert_eq!(prev_word_start("hello world", 0), 0);   // at start
        assert_eq!(prev_word_start("hello world", 6), 0);   // at start of "world"
    }

    #[test]
    fn test_next_word_end() {
        assert_eq!(next_word_end("hello world", 0), 5);    // before "hello"
        assert_eq!(next_word_end("hello world", 5), 11);   // at space before "world"
        assert_eq!(next_word_end("hello  world", 0), 5);   // before "hello" with double space
        assert_eq!(next_word_end("hello world", 11), 11);  // at document end
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test text_motion
```

Expected: 4 tests fail/panic with `not yet implemented`. This confirms the test harness is wired up correctly.

- [ ] **Step 3: Implement the four functions**

Replace the four `todo!()` stubs in `src/ui/text_motion.rs`:

```rust
pub fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

pub fn line_end(text: &str, offset: usize) -> usize {
    text[offset..].find('\n').map(|i| offset + i).unwrap_or(text.len())
}

pub fn prev_word_start(text: &str, offset: usize) -> usize {
    let s = &text[..offset];
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let mut i = n;

    // skip trailing whitespace (moving backwards)
    while i > 0 && chars[i - 1].1.is_whitespace() {
        i -= 1;
    }
    if i == 0 {
        return 0;
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let class = is_word(chars[i - 1].1);

    while i > 0 && is_word(chars[i - 1].1) == class {
        i -= 1;
    }

    chars.get(i).map(|(idx, _)| *idx).unwrap_or(0)
}

pub fn next_word_end(text: &str, offset: usize) -> usize {
    let s = &text[offset..];
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let n = chars.len();
    if n == 0 {
        return text.len();
    }
    let mut i = 0;

    // skip leading whitespace
    while i < n && chars[i].1.is_whitespace() {
        i += 1;
    }
    if i == n {
        return text.len();
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let class = is_word(chars[i].1);

    while i < n && is_word(chars[i].1) == class {
        i += 1;
    }

    if i == n {
        text.len()
    } else {
        offset + chars[i].0
    }
}
```

- [ ] **Step 4: Run tests to verify they all pass**

```
cargo test text_motion
```

Expected:
```
test text_motion::tests::test_line_end ... ok
test text_motion::tests::test_line_start ... ok
test text_motion::tests::test_next_word_end ... ok
test text_motion::tests::test_prev_word_start ... ok
```

- [ ] **Step 5: Expose the module in `src/ui/mod.rs`**

Add one line at the end of `src/ui/mod.rs`:

```rust
pub mod app_view;
pub mod connection_dialog;
pub mod editor_view;
pub mod filter_panel;
pub mod schema_view;
pub mod sidebar;
pub mod sql_editor;
pub mod tab_bar;
pub mod table_view;
pub mod text_field;
pub mod text_motion;
```

- [ ] **Step 6: Verify clean build**

```
cargo build
```

Expected: compiles with no errors.

- [ ] **Step 7: Commit**

```bash
git add src/ui/text_motion.rs src/ui/mod.rs
git commit -m "feat: add text_motion module with line/word boundary functions"
```

---

### Task 2: Update `SqlEditor` with undo/redo and new keybindings

**Files:**
- Modify: `src/ui/sql_editor.rs`

- [ ] **Step 1: Add undo/redo fields to the struct, `new()`, and `set_content()`**

In `src/ui/sql_editor.rs`, replace the `SqlEditor` struct definition (lines 36–47):

```rust
pub struct SqlEditor {
    pub focus_handle: FocusHandle,
    pub content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_line_layouts: Vec<(usize, ShapedLine)>,
    last_bounds: Option<Bounds<Pixels>>,
    last_line_height: Pixels,
    is_selecting: bool,
    undo_stack: Vec<(SharedString, Range<usize>)>,
    redo_stack: Vec<(SharedString, Range<usize>)>,
}
```

Replace `new()` (lines 50–62):

```rust
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
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
    }
}
```

Replace `set_content()` (lines 64–70):

```rust
pub fn set_content(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
    self.content = text.into();
    self.selected_range = 0..0;
    self.selection_reversed = false;
    self.marked_range = None;
    self.undo_stack.clear();
    self.redo_stack.clear();
    cx.notify();
}
```

- [ ] **Step 2: Modify `replace_text_in_range` to snapshot before every mutation**

Find `replace_text_in_range` in the `EntityInputHandler` impl block (around line 386). Add two lines at the very top of the method body, before the `let range = ...` line:

```rust
fn replace_text_in_range(
    &mut self,
    range_utf16: Option<Range<usize>>,
    new_text: &str,
    _: &mut Window,
    cx: &mut Context<Self>,
) {
    self.undo_stack.push((self.content.clone(), self.selected_range.clone()));
    self.redo_stack.clear();

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
```

- [ ] **Step 3: Expand the `actions!()` macro**

Replace the `actions!()` call at lines 9–15:

```rust
actions!(
    sql_editor,
    [
        Backspace, Delete, Left, Right, Up, Down, SelectAll, Home, End, Paste, Cut, Copy, Enter,
        Tab,
        MovePrevWord, MoveNextWord, MoveDocStart, MoveDocEnd,
        SelectLeft, SelectRight, SelectUp, SelectDown,
        SelectLineStart, SelectLineEnd, SelectDocStart, SelectDocEnd,
        SelectPrevWord, SelectNextWord,
        DeleteWordBack, DeleteWordForward, DeleteToLineStart,
        Undo, Redo,
    ]
);
```

- [ ] **Step 4: Replace `register_sql_editor_actions` with all new bindings**

Replace the entire `register_sql_editor_actions` function (lines 17–34):

```rust
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
        KeyBinding::new("cmd-left", Home, Some("SqlEditor")),
        KeyBinding::new("cmd-right", End, Some("SqlEditor")),
        KeyBinding::new("cmd-v", Paste, Some("SqlEditor")),
        KeyBinding::new("cmd-c", Copy, Some("SqlEditor")),
        KeyBinding::new("cmd-x", Cut, Some("SqlEditor")),
        KeyBinding::new("enter", Enter, Some("SqlEditor")),
        KeyBinding::new("tab", Tab, Some("SqlEditor")),
        KeyBinding::new("alt-left", MovePrevWord, Some("SqlEditor")),
        KeyBinding::new("alt-right", MoveNextWord, Some("SqlEditor")),
        KeyBinding::new("cmd-up", MoveDocStart, Some("SqlEditor")),
        KeyBinding::new("cmd-down", MoveDocEnd, Some("SqlEditor")),
        KeyBinding::new("shift-left", SelectLeft, Some("SqlEditor")),
        KeyBinding::new("shift-right", SelectRight, Some("SqlEditor")),
        KeyBinding::new("shift-up", SelectUp, Some("SqlEditor")),
        KeyBinding::new("shift-down", SelectDown, Some("SqlEditor")),
        KeyBinding::new("shift-home", SelectLineStart, Some("SqlEditor")),
        KeyBinding::new("shift-cmd-left", SelectLineStart, Some("SqlEditor")),
        KeyBinding::new("shift-end", SelectLineEnd, Some("SqlEditor")),
        KeyBinding::new("shift-cmd-right", SelectLineEnd, Some("SqlEditor")),
        KeyBinding::new("shift-cmd-up", SelectDocStart, Some("SqlEditor")),
        KeyBinding::new("shift-cmd-down", SelectDocEnd, Some("SqlEditor")),
        KeyBinding::new("shift-alt-left", SelectPrevWord, Some("SqlEditor")),
        KeyBinding::new("shift-alt-right", SelectNextWord, Some("SqlEditor")),
        KeyBinding::new("alt-backspace", DeleteWordBack, Some("SqlEditor")),
        KeyBinding::new("alt-delete", DeleteWordForward, Some("SqlEditor")),
        KeyBinding::new("cmd-backspace", DeleteToLineStart, Some("SqlEditor")),
        KeyBinding::new("cmd-z", Undo, Some("SqlEditor")),
        KeyBinding::new("cmd-shift-z", Redo, Some("SqlEditor")),
    ]);
}
```

- [ ] **Step 5: Add the `text_motion` import**

Add after the existing `use crate::query::highlight;` line near the top of `src/ui/sql_editor.rs`:

```rust
use crate::ui::text_motion;
```

- [ ] **Step 6: Add all new handler methods**

Add the following methods to the `impl SqlEditor` block, after `on_cut` (around line 329) and before `on_mouse_down`:

```rust
fn on_undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
    if let Some((content, selection)) = self.undo_stack.pop() {
        self.redo_stack.push((self.content.clone(), self.selected_range.clone()));
        self.content = content;
        self.selected_range = selection;
        self.marked_range = None;
        cx.notify();
    }
}

fn on_redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
    if let Some((content, selection)) = self.redo_stack.pop() {
        self.undo_stack.push((self.content.clone(), self.selected_range.clone()));
        self.content = content;
        self.selected_range = selection;
        self.marked_range = None;
        cx.notify();
    }
}

fn on_move_prev_word(&mut self, _: &MovePrevWord, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::prev_word_start(&self.content, self.cursor_offset());
    self.move_to(offset, cx);
}

fn on_move_next_word(&mut self, _: &MoveNextWord, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::next_word_end(&self.content, self.cursor_offset());
    self.move_to(offset, cx);
}

fn on_move_doc_start(&mut self, _: &MoveDocStart, _: &mut Window, cx: &mut Context<Self>) {
    self.move_to(0, cx);
}

fn on_move_doc_end(&mut self, _: &MoveDocEnd, _: &mut Window, cx: &mut Context<Self>) {
    self.move_to(self.content.len(), cx);
}

fn on_select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
    let offset = self.previous_boundary(self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
    let offset = self.next_boundary(self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
    let (line_idx, col, _) = self.cursor_line_col();
    if line_idx == 0 {
        self.select_to(0, cx);
        return;
    }
    let offset = self.offset_at_line_col(line_idx - 1, col);
    self.select_to(offset, cx);
}

fn on_select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
    let count = self.content.split('\n').count();
    let (line_idx, col, _) = self.cursor_line_col();
    if line_idx >= count - 1 {
        self.select_to(self.content.len(), cx);
        return;
    }
    let offset = self.offset_at_line_col(line_idx + 1, col);
    self.select_to(offset, cx);
}

fn on_select_line_start(&mut self, _: &SelectLineStart, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::line_start(&self.content, self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_select_line_end(&mut self, _: &SelectLineEnd, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::line_end(&self.content, self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_select_doc_start(&mut self, _: &SelectDocStart, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(0, cx);
}

fn on_select_doc_end(&mut self, _: &SelectDocEnd, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(self.content.len(), cx);
}

fn on_select_prev_word(&mut self, _: &SelectPrevWord, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::prev_word_start(&self.content, self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_select_next_word(&mut self, _: &SelectNextWord, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::next_word_end(&self.content, self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_delete_word_back(&mut self, _: &DeleteWordBack, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
        let offset = text_motion::prev_word_start(&self.content, self.cursor_offset());
        self.select_to(offset, cx);
    }
    self.replace_text_in_range(None, "", window, cx);
}

fn on_delete_word_forward(&mut self, _: &DeleteWordForward, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
        let offset = text_motion::next_word_end(&self.content, self.cursor_offset());
        self.select_to(offset, cx);
    }
    self.replace_text_in_range(None, "", window, cx);
}

fn on_delete_to_line_start(&mut self, _: &DeleteToLineStart, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
        let offset = text_motion::line_start(&self.content, self.cursor_offset());
        self.select_to(offset, cx);
    }
    self.replace_text_in_range(None, "", window, cx);
}
```

- [ ] **Step 7: Wire all new handlers in `render()`**

In the `render()` method, add the following `.on_action` calls immediately after `.on_action(cx.listener(Self::on_tab))`:

```rust
.on_action(cx.listener(Self::on_tab))
.on_action(cx.listener(Self::on_undo))
.on_action(cx.listener(Self::on_redo))
.on_action(cx.listener(Self::on_move_prev_word))
.on_action(cx.listener(Self::on_move_next_word))
.on_action(cx.listener(Self::on_move_doc_start))
.on_action(cx.listener(Self::on_move_doc_end))
.on_action(cx.listener(Self::on_select_left))
.on_action(cx.listener(Self::on_select_right))
.on_action(cx.listener(Self::on_select_up))
.on_action(cx.listener(Self::on_select_down))
.on_action(cx.listener(Self::on_select_line_start))
.on_action(cx.listener(Self::on_select_line_end))
.on_action(cx.listener(Self::on_select_doc_start))
.on_action(cx.listener(Self::on_select_doc_end))
.on_action(cx.listener(Self::on_select_prev_word))
.on_action(cx.listener(Self::on_select_next_word))
.on_action(cx.listener(Self::on_delete_word_back))
.on_action(cx.listener(Self::on_delete_word_forward))
.on_action(cx.listener(Self::on_delete_to_line_start))
```

- [ ] **Step 8: Run clippy**

```
cargo clippy -- -D warnings
```

Expected: no warnings or errors. Fix any that appear before continuing.

- [ ] **Step 9: Commit**

```bash
git add src/ui/sql_editor.rs
git commit -m "feat: add undo/redo and full keybindings to SqlEditor"
```

---

### Task 3: Update `TextField` with undo/redo and new keybindings

**Files:**
- Modify: `src/ui/text_field.rs`

- [ ] **Step 1: Add undo/redo fields to the struct, `new()`, and `set_content()`**

In `src/ui/text_field.rs`, replace the `TextField` struct definition (lines 27–38):

```rust
pub struct TextField {
    pub focus_handle: FocusHandle,
    pub content: SharedString,
    pub placeholder: SharedString,
    pub masked: bool,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    undo_stack: Vec<(SharedString, Range<usize>)>,
    redo_stack: Vec<(SharedString, Range<usize>)>,
}
```

Replace `new()` (lines 41–54):

```rust
pub fn new(cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
    Self {
        focus_handle: cx.focus_handle(),
        content: "".into(),
        placeholder: placeholder.into(),
        masked: false,
        selected_range: 0..0,
        selection_reversed: false,
        marked_range: None,
        last_layout: None,
        last_bounds: None,
        is_selecting: false,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
    }
}
```

Replace `set_content()` (lines 56–61):

```rust
pub fn set_content(&mut self, content: impl Into<SharedString>, cx: &mut Context<Self>) {
    self.content = content.into();
    let len = self.content.len();
    self.selected_range = len..len;
    self.undo_stack.clear();
    self.redo_stack.clear();
    cx.notify();
}
```

- [ ] **Step 2: Modify `replace_text_in_range` to snapshot before every mutation**

Find `replace_text_in_range` in the `EntityInputHandler` impl block (around line 282). Add two lines at the top of the method body:

```rust
fn replace_text_in_range(
    &mut self,
    range_utf16: Option<Range<usize>>,
    new_text: &str,
    _: &mut Window,
    cx: &mut Context<Self>,
) {
    self.undo_stack.push((self.content.clone(), self.selected_range.clone()));
    self.redo_stack.clear();

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
```

- [ ] **Step 3: Expand the `actions!()` macro**

Replace the `actions!()` call at lines 7–10:

```rust
actions!(
    text_field,
    [
        Backspace, Delete, Left, Right, SelectAll, Home, End, Paste, Cut, Copy,
        MovePrevWord, MoveNextWord,
        SelectLeft, SelectRight,
        SelectLineStart, SelectLineEnd,
        SelectPrevWord, SelectNextWord,
        DeleteWordBack, DeleteWordForward, DeleteToLineStart,
        Undo, Redo,
    ]
);
```

- [ ] **Step 4: Replace `register_text_field_actions` with all new bindings**

Replace the entire `register_text_field_actions` function (lines 12–25):

```rust
pub fn register_text_field_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("TextField")),
        KeyBinding::new("delete", Delete, Some("TextField")),
        KeyBinding::new("left", Left, Some("TextField")),
        KeyBinding::new("right", Right, Some("TextField")),
        KeyBinding::new("cmd-a", SelectAll, Some("TextField")),
        KeyBinding::new("home", Home, Some("TextField")),
        KeyBinding::new("end", End, Some("TextField")),
        KeyBinding::new("cmd-left", Home, Some("TextField")),
        KeyBinding::new("cmd-right", End, Some("TextField")),
        KeyBinding::new("cmd-v", Paste, Some("TextField")),
        KeyBinding::new("cmd-c", Copy, Some("TextField")),
        KeyBinding::new("cmd-x", Cut, Some("TextField")),
        KeyBinding::new("alt-left", MovePrevWord, Some("TextField")),
        KeyBinding::new("alt-right", MoveNextWord, Some("TextField")),
        KeyBinding::new("shift-left", SelectLeft, Some("TextField")),
        KeyBinding::new("shift-right", SelectRight, Some("TextField")),
        KeyBinding::new("shift-home", SelectLineStart, Some("TextField")),
        KeyBinding::new("shift-cmd-left", SelectLineStart, Some("TextField")),
        KeyBinding::new("shift-end", SelectLineEnd, Some("TextField")),
        KeyBinding::new("shift-cmd-right", SelectLineEnd, Some("TextField")),
        KeyBinding::new("shift-alt-left", SelectPrevWord, Some("TextField")),
        KeyBinding::new("shift-alt-right", SelectNextWord, Some("TextField")),
        KeyBinding::new("alt-backspace", DeleteWordBack, Some("TextField")),
        KeyBinding::new("alt-delete", DeleteWordForward, Some("TextField")),
        KeyBinding::new("cmd-backspace", DeleteToLineStart, Some("TextField")),
        KeyBinding::new("cmd-z", Undo, Some("TextField")),
        KeyBinding::new("cmd-shift-z", Redo, Some("TextField")),
    ]);
}
```

- [ ] **Step 5: Add the `text_motion` import**

Add after the existing use statements at the top of `src/ui/text_field.rs`:

```rust
use crate::ui::text_motion;
```

- [ ] **Step 6: Add all new handler methods**

Add the following methods to the `impl TextField` block, after `on_cut` (around line 226) and before `on_mouse_down`:

```rust
fn on_undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
    if let Some((content, selection)) = self.undo_stack.pop() {
        self.redo_stack.push((self.content.clone(), self.selected_range.clone()));
        self.content = content;
        self.selected_range = selection;
        self.marked_range = None;
        cx.notify();
    }
}

fn on_redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
    if let Some((content, selection)) = self.redo_stack.pop() {
        self.undo_stack.push((self.content.clone(), self.selected_range.clone()));
        self.content = content;
        self.selected_range = selection;
        self.marked_range = None;
        cx.notify();
    }
}

fn on_move_prev_word(&mut self, _: &MovePrevWord, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::prev_word_start(&self.content, self.cursor_offset());
    self.move_to(offset, cx);
}

fn on_move_next_word(&mut self, _: &MoveNextWord, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::next_word_end(&self.content, self.cursor_offset());
    self.move_to(offset, cx);
}

fn on_select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
    let offset = self.previous_boundary(self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
    let offset = self.next_boundary(self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_select_line_start(&mut self, _: &SelectLineStart, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(0, cx);
}

fn on_select_line_end(&mut self, _: &SelectLineEnd, _: &mut Window, cx: &mut Context<Self>) {
    self.select_to(self.content.len(), cx);
}

fn on_select_prev_word(&mut self, _: &SelectPrevWord, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::prev_word_start(&self.content, self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_select_next_word(&mut self, _: &SelectNextWord, _: &mut Window, cx: &mut Context<Self>) {
    let offset = text_motion::next_word_end(&self.content, self.cursor_offset());
    self.select_to(offset, cx);
}

fn on_delete_word_back(&mut self, _: &DeleteWordBack, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
        let offset = text_motion::prev_word_start(&self.content, self.cursor_offset());
        self.select_to(offset, cx);
    }
    self.replace_text_in_range(None, "", window, cx);
}

fn on_delete_word_forward(&mut self, _: &DeleteWordForward, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
        let offset = text_motion::next_word_end(&self.content, self.cursor_offset());
        self.select_to(offset, cx);
    }
    self.replace_text_in_range(None, "", window, cx);
}

fn on_delete_to_line_start(&mut self, _: &DeleteToLineStart, window: &mut Window, cx: &mut Context<Self>) {
    if self.selected_range.is_empty() {
        self.select_to(0, cx);
    }
    self.replace_text_in_range(None, "", window, cx);
}
```

Note: `on_select_line_start`, `on_select_line_end`, and `on_delete_to_line_start` use `0` / `self.content.len()` directly — `text_motion` line functions would return the same values since `TextField` never contains `\n`.

- [ ] **Step 7: Wire all new handlers in `render()`**

In the `render()` method (around line 367), add the following `.on_action` calls after `.on_action(cx.listener(Self::on_cut))`:

```rust
.on_action(cx.listener(Self::on_cut))
.on_action(cx.listener(Self::on_undo))
.on_action(cx.listener(Self::on_redo))
.on_action(cx.listener(Self::on_move_prev_word))
.on_action(cx.listener(Self::on_move_next_word))
.on_action(cx.listener(Self::on_select_left))
.on_action(cx.listener(Self::on_select_right))
.on_action(cx.listener(Self::on_select_line_start))
.on_action(cx.listener(Self::on_select_line_end))
.on_action(cx.listener(Self::on_select_prev_word))
.on_action(cx.listener(Self::on_select_next_word))
.on_action(cx.listener(Self::on_delete_word_back))
.on_action(cx.listener(Self::on_delete_word_forward))
.on_action(cx.listener(Self::on_delete_to_line_start))
```

- [ ] **Step 8: Run clippy**

```
cargo clippy -- -D warnings
```

Expected: no warnings or errors. Fix any that appear before continuing.

- [ ] **Step 9: Commit**

```bash
git add src/ui/text_field.rs
git commit -m "feat: add undo/redo and full keybindings to TextField"
```
