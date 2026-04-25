# Build Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two GitHub Actions workflows — CI (check + lint on push/PR) and Release (tag-triggered binary build + GitHub Release).

**Architecture:** Two separate workflow files under `.github/workflows/`. CI runs `cargo check` and `cargo clippy` on `macos-latest`. Release triggers on `v*.*.*` tags, builds a release binary with `cargo build --release`, and publishes it to a GitHub Release using `softprops/action-gh-release`. Both workflows share a cargo cache strategy keyed on `Cargo.lock`.

**Tech Stack:** GitHub Actions, Rust/Cargo, `actions/checkout@v4`, `actions/cache@v4`, `softprops/action-gh-release@v2`

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `.github/workflows/ci.yml` | Create | CI workflow: check + clippy on push/PR |
| `.github/workflows/release.yml` | Create | Release workflow: build binary, publish GitHub Release on tag |

---

### Task 1: Create CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create the `.github/workflows/` directory and `ci.yml`**

Create `.github/workflows/ci.yml` with this exact content:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Check
        run: cargo check

      - name: Clippy
        run: cargo clippy -- -D warnings
```

- [ ] **Step 2: Verify the YAML is valid**

Run:
```sh
brew install yamllint  # skip if already installed
yamllint .github/workflows/ci.yml
```
Expected: no errors

- [ ] **Step 3: Commit**

```sh
git add .github/workflows/ci.yml
git commit -m "ci: add CI workflow (check + clippy on push/PR)"
```

- [ ] **Step 4: Push to main and verify workflow runs**

```sh
git push origin main
```

Go to `https://github.com/marcusedwardhaslam/querybox/actions` and confirm the `CI` workflow appears and runs. It will take several minutes on first run due to GPUI compile time — subsequent runs will be faster once the cache is warm.

Expected: workflow completes with a green checkmark on the `check` job.

---

### Task 2: Create Release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create `.github/workflows/release.yml`**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            artifact: querybox-macos-aarch64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Build release binary
        run: cargo build --release

      - name: Rename binary
        run: mv target/release/querybox ${{ matrix.artifact }}

      - name: Publish GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: ${{ matrix.artifact }}
          generate_release_notes: true
```

- [ ] **Step 2: Verify the YAML is valid**

```sh
yamllint .github/workflows/release.yml
```
Expected: no errors

- [ ] **Step 3: Commit**

```sh
git add .github/workflows/release.yml
git commit -m "ci: add release workflow (tag-triggered binary + GitHub Release)"
```

---

### Task 3: Cut a test release and verify end-to-end

- [ ] **Step 1: Update version in `Cargo.toml`**

Open `Cargo.toml` and change:
```toml
version = "0.1.0"
```
to:
```toml
version = "0.1.1"
```

- [ ] **Step 2: Commit the version bump**

```sh
git add Cargo.toml
git commit -m "chore: bump version to v0.1.1"
```

- [ ] **Step 3: Tag and push**

```sh
git tag v0.1.1
git push origin main
git push origin v0.1.1
```

- [ ] **Step 4: Verify the release workflow runs**

Go to `https://github.com/marcusedwardhaslam/querybox/actions` and confirm the `Release` workflow is triggered by the `v0.1.1` tag.

Expected: workflow completes, a release named `v0.1.1` appears at `https://github.com/marcusedwardhaslam/querybox/releases` with `querybox-macos-aarch64` attached as a downloadable asset and auto-generated release notes.

- [ ] **Step 5: Download and smoke-test the binary**

```sh
# Download from the release page and make executable
chmod +x ~/Downloads/querybox-macos-aarch64
~/Downloads/querybox-macos-aarch64
```

Expected: QueryBox launches normally. (macOS will show a Gatekeeper warning on first run since the binary is unsigned — right-click → Open to bypass.)
