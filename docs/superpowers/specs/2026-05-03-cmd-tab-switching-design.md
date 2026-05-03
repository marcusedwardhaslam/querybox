# CMD+1–9 Tab Switching — Design Spec

**Date:** 2026-05-03
**Status:** Approved

## Summary

Add CMD+1 through CMD+9 keyboard shortcuts to switch between open tabs by position. CMD+9 always jumps to the last tab regardless of tab count, matching browser conventions (Chrome, Firefox, Safari).

## Behaviour

- `CMD+1` activates the first tab (leftmost)
- `CMD+2` through `CMD+8` activate tabs at those 1-based positions
- `CMD+9` always activates the last tab, regardless of how many tabs are open
- If `CMD+N` is pressed and fewer than N tabs are open (and N ≠ 9), the last tab is activated (same `min` clamp)
- If no tabs are open, the shortcut is a no-op

## Architecture

### `src/main.rs`

1. Extend the existing `actions!()` macro call to include `SwitchToTab1` through `SwitchToTab9` alongside `Quit` and `CloseTab`.
2. Add nine `KeyBinding::new` entries to the existing `bind_keys([...])` call — all with `None` context (global scope):
   - `"cmd-1"` → `SwitchToTab1`
   - ...
   - `"cmd-9"` → `SwitchToTab9`

### `src/ui/app_view.rs`

**Shared helper method on `AppView`:**

```rust
fn switch_to_tab_by_position(&mut self, position: usize, cx: &mut Context<Self>) {
    let tabs = self.tab_bar.read(cx).tabs.clone();
    if tabs.is_empty() { return; }
    let idx = position.min(tabs.len() - 1);
    let tab_id = tabs[idx].id;
    self.tab_bar.update(cx, |bar, cx| bar.set_active(tab_id, cx));
    cx.notify();
}
```

**Nine thin action handlers** (`on_switch_to_tab_1` through `on_switch_to_tab_9`), each delegating to the helper with their 0-based index (0–8).

**Nine `.on_action` registrations** in the `render()` method, placed alongside the existing `CloseTab` handler.

## Key Design Decisions

- **Nine unit struct actions** (Option A) — idiomatic GPUI pattern, matches `CloseTab`/`Quit` in the codebase. Preferred over a single parameterised action (more boilerplate, diverges from existing patterns) or direct key listeners (bypasses the action system).
- **`position.min(tabs.len() - 1)`** — single expression handles both "index out of range" and "CMD+9 = last tab" with no special-casing needed.
- **Global scope (`None` context)** — tab switching should work regardless of which UI element has focus, consistent with `CloseTab` and `Quit`.

## Files Changed

| File | Change |
|------|--------|
| `src/main.rs` | Add 9 actions to `actions!()`, add 9 keybindings to `bind_keys()` |
| `src/ui/app_view.rs` | Add `switch_to_tab_by_position()` helper, 9 handler methods, 9 `.on_action()` registrations in `render()` |
