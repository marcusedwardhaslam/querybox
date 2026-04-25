# macOS Bundled Application — Design Spec

Date: 2026-04-25

## Goal

Produce a proper macOS `.app` bundle and a `.dmg` installer for QueryBox releases, so users can double-click to open, see the app in Applications, and get a dock icon. Distribution is via GitHub Releases as a downloadable `.dmg`.

## Approach

`cargo-bundle` + `hdiutil`. `cargo-bundle` reads bundle metadata from `Cargo.toml` and produces a `.app` with a correct `Info.plist`. macOS-built-in `hdiutil` wraps it in a compressed `.dmg`. No extra runtime dependencies on CI.

Code signing and notarization are deferred (no Apple Developer account yet — tracked separately in TODO).

## Components

### 1. Bundle metadata (`Cargo.toml`)

Add a `[package.metadata.bundle]` section:

```toml
[package.metadata.bundle]
name = "QueryBox"
identifier = "uk.co.marcusehaslam.querybox"
icon = ["assets/icons/icon.icns"]
category = "public.app-category.developer-tools"
short_description = "Free and open source SQL GUI"
```

### 2. App icon (`assets/icons/icon.icns`)

A placeholder `.icns` generated locally using a Python stdlib script (`dev/gen_icon.py`) and macOS `iconutil`. The script produces solid-colour PNGs at all required sizes (16–1024), packages them into an `.iconset`, and converts to `.icns`. The resulting `assets/icons/icon.icns` is committed to the repo. The generation script is kept in `dev/` for future use when a real icon is ready.

### 3. Release workflow (`.github/workflows/release.yml`)

Replace the current build + rename steps with:

1. `cargo install cargo-bundle --locked`
2. `cargo bundle --release` → outputs `target/release/bundle/osx/QueryBox.app`
3. `hdiutil create -volname QueryBox -srcfolder "target/release/bundle/osx/QueryBox.app" -ov -format UDZO QueryBox-macos-aarch64.dmg`
4. Upload `QueryBox-macos-aarch64.dmg` as the GitHub Release artifact

## Out of Scope

- Code signing / notarization (deferred — Apple Developer account needed)
- Custom DMG background or Applications shortcut
- Windows support
