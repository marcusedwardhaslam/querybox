---
name: rust-clippy-gate
description: Use when any Rust source files have been modified and you are about to commit, finish a task, call a completion skill, or claim work is done - requires running cargo clippy and resolving all warnings before proceeding
---

# Rust Clippy Gate

## Overview

Any Rust changes must pass `cargo clippy` with zero warnings before committing or completing work. This is a hard gate — not advisory.

## When This Applies

- You modified any `.rs` file
- You are about to run `git commit`
- You are about to invoke `superpowers:finishing-a-development-branch`, `superpowers:verification-before-completion`, or any completion skill
- You are about to tell the user the work is done

## Required Steps

1. Run `cargo clippy -- -D warnings`
2. If warnings exist → fix them, then go back to step 1
3. Only proceed to commit / completion once clippy exits 0 with no warnings

```bash
cargo clippy -- -D warnings
```

`-D warnings` promotes all warnings to errors so the exit code is non-zero on any issue — no ambiguity.

## Red Flags — Stop and Run Clippy

These thoughts mean you are rationalizing. Stop.

| Thought | Reality |
|---|---|
| "Clippy warnings are non-critical" | The project requires a clean build; warnings become tech debt immediately |
| "I'll fix warnings in a follow-up" | There is no follow-up. Fix them now. |
| "The code compiles, that's enough" | `rustc` and `clippy` are different tools. Compiling ≠ passing clippy. |
| "These are just style suggestions" | Clippy catches real bugs, not just style. |
| "I only changed one line" | One line can introduce a warning. Run it anyway. |
| "I ran it earlier and it was clean" | Earlier ≠ now. Run it again after your latest change. |

## Common Clippy Fixes

```rust
// needless_return
return x;        // ❌
x                // ✅

// clone_on_copy
val.clone()      // ❌ for Copy types
val              // ✅

// unused_variable
let x = foo();   // ❌ if x unused
let _x = foo();  // ✅ or remove entirely

// map_unwrap_or → use unwrap_or_else for lazy eval
opt.map(f).unwrap_or(default)   // ❌
opt.map_or(default, f)          // ✅
```

## What to Do With `#[allow(...)]`

Only suppress a warning with `#[allow(...)]` if:
- You have a concrete reason the lint is a false positive here, AND
- You add an inline comment explaining why

Never suppress warnings just to make clippy pass silently.
