# CMD+1–9 Tab Switching Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add CMD+1–9 keyboard shortcuts to switch between open tabs by position, with CMD+9 always jumping to the last tab.

**Architecture:** Nine unit-struct GPUI actions (`SwitchToTab1`–`SwitchToTab9`) are declared in `main.rs` and bound globally to `cmd-1`–`cmd-9`. `AppView` handles them via nine thin methods that delegate to a shared `switch_to_tab_by_position` helper, which clamps the index so CMD+9 always resolves to the last tab.

**Tech Stack:** Rust, GPUI (`actions!`, `KeyBinding`, `on_action`, `cx.listener`)

---

## File Map

| File | Change |
|------|--------|
| `src/main.rs` | Extend `actions!()` with 9 new action types; add 9 keybindings to `bind_keys()` |
| `src/ui/app_view.rs` | Add `switch_to_tab_by_position` helper + 9 handler methods; register 9 `.on_action` calls in `render()` |

---

### Task 1: Declare actions and keybindings in `src/main.rs`

**Files:**
- Modify: `src/main.rs:11` (actions macro)
- Modify: `src/main.rs:30-33` (bind_keys call)

- [ ] **Step 1: Verify the build is clean before touching anything**

```bash
cargo build 2>&1 | tail -5
```

Expected: zero errors.

- [ ] **Step 2: Replace the `actions!` call on line 11**

Replace:
```rust
actions!(querybox, [Quit, CloseTab]);
```

With:
```rust
actions!(
    querybox,
    [
        Quit,
        CloseTab,
        SwitchToTab1,
        SwitchToTab2,
        SwitchToTab3,
        SwitchToTab4,
        SwitchToTab5,
        SwitchToTab6,
        SwitchToTab7,
        SwitchToTab8,
        SwitchToTab9,
    ]
);
```

- [ ] **Step 3: Replace the `bind_keys` call on lines 30–33**

Replace:
```rust
cx.bind_keys([
    KeyBinding::new("cmd-q", Quit, None),
    KeyBinding::new("cmd-w", CloseTab, None),
]);
```

With:
```rust
cx.bind_keys([
    KeyBinding::new("cmd-q", Quit, None),
    KeyBinding::new("cmd-w", CloseTab, None),
    KeyBinding::new("cmd-1", SwitchToTab1, None),
    KeyBinding::new("cmd-2", SwitchToTab2, None),
    KeyBinding::new("cmd-3", SwitchToTab3, None),
    KeyBinding::new("cmd-4", SwitchToTab4, None),
    KeyBinding::new("cmd-5", SwitchToTab5, None),
    KeyBinding::new("cmd-6", SwitchToTab6, None),
    KeyBinding::new("cmd-7", SwitchToTab7, None),
    KeyBinding::new("cmd-8", SwitchToTab8, None),
    KeyBinding::new("cmd-9", SwitchToTab9, None),
]);
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build 2>&1 | grep -E "^error"
```

Expected: no output (zero errors). If warnings appear about unused actions, that's fine — they'll be resolved in Task 2.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: declare SwitchToTab1-9 actions and cmd-1-9 keybindings"
```

---

### Task 2: Add handler methods and `on_action` registrations in `src/ui/app_view.rs`

**Files:**
- Modify: `src/ui/app_view.rs:686` (render — add on_action calls)
- Modify: `src/ui/app_view.rs:712` (impl block — add helper + 9 handler methods)

- [ ] **Step 1: Add nine `.on_action` registrations in `render()`**

In `render()`, find the existing line (around line 686):
```rust
.on_action(cx.listener(Self::close_active_tab))
```

Add nine more lines immediately after it:
```rust
.on_action(cx.listener(Self::close_active_tab))
.on_action(cx.listener(Self::on_switch_to_tab_1))
.on_action(cx.listener(Self::on_switch_to_tab_2))
.on_action(cx.listener(Self::on_switch_to_tab_3))
.on_action(cx.listener(Self::on_switch_to_tab_4))
.on_action(cx.listener(Self::on_switch_to_tab_5))
.on_action(cx.listener(Self::on_switch_to_tab_6))
.on_action(cx.listener(Self::on_switch_to_tab_7))
.on_action(cx.listener(Self::on_switch_to_tab_8))
.on_action(cx.listener(Self::on_switch_to_tab_9))
```

- [ ] **Step 2: Add helper and handler methods to the `impl AppView` block**

In the second `impl AppView` block (the one that starts around line 712, containing `close_active_tab`), add the following after `close_active_tab`:

```rust
fn switch_to_tab_by_position(&mut self, position: usize, cx: &mut Context<Self>) {
    let tabs = self.tab_bar.read(cx).tabs.clone();
    if tabs.is_empty() {
        return;
    }
    let idx = position.min(tabs.len() - 1);
    let tab_id = tabs[idx].id;
    self.tab_bar.update(cx, |bar, cx| bar.set_active(tab_id, cx));
    cx.notify();
}

fn on_switch_to_tab_1(&mut self, _: &crate::SwitchToTab1, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(0, cx);
}

fn on_switch_to_tab_2(&mut self, _: &crate::SwitchToTab2, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(1, cx);
}

fn on_switch_to_tab_3(&mut self, _: &crate::SwitchToTab3, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(2, cx);
}

fn on_switch_to_tab_4(&mut self, _: &crate::SwitchToTab4, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(3, cx);
}

fn on_switch_to_tab_5(&mut self, _: &crate::SwitchToTab5, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(4, cx);
}

fn on_switch_to_tab_6(&mut self, _: &crate::SwitchToTab6, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(5, cx);
}

fn on_switch_to_tab_7(&mut self, _: &crate::SwitchToTab7, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(6, cx);
}

fn on_switch_to_tab_8(&mut self, _: &crate::SwitchToTab8, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(7, cx);
}

fn on_switch_to_tab_9(&mut self, _: &crate::SwitchToTab9, _: &mut Window, cx: &mut Context<Self>) {
    self.switch_to_tab_by_position(8, cx);
}
```

- [ ] **Step 3: Verify it compiles cleanly**

```bash
cargo build 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 4: Run clippy**

```bash
cargo clippy 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 5: Manual verification**

Run the app:
```bash
cargo run
```

Connect to the dev database (`cd dev && docker compose up -d` if not already running — host `localhost:3306`, user `queryuser`, pass `querypass`, db `querybox`).

Open several table tabs and a query tab (3–4 total), then verify:
- `CMD+1` activates the first tab
- `CMD+2` activates the second tab
- `CMD+3` activates the third tab
- `CMD+9` activates the last tab (regardless of how many tabs are open)
- `CMD+5` with only 3 tabs open activates the last (third) tab

- [ ] **Step 6: Commit**

```bash
git add src/ui/app_view.rs
git commit -m "feat: implement CMD+1-9 tab switching in AppView"
```

---

### Task 3: Mark TODO complete

**Files:**
- Modify: `TODO.md`

- [ ] **Step 1: Mark the item as done**

In `TODO.md`, change:
```
- [ ] CMD+1, CMD+2... key bindings for switching open tabs
```
to:
```
- [x] CMD+1, CMD+2... key bindings for switching open tabs
```

- [ ] **Step 2: Commit**

```bash
git add TODO.md
git commit -m "chore: mark CMD+1-9 tab switching as complete in TODO"
```
