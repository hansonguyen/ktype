# Phase 7: Polish + OSS Release — Design Spec

**Date:** 2026-05-17
**Scope:** Option A — minimal crates.io-ready polish. No new features.

---

## Goals

Get kern to a state where:

- `cargo publish --dry-run` passes cleanly
- A GitHub visitor understands what kern is and how to install it within 30 seconds
- CI covers both Linux and macOS with nextest, clippy, and fmt

## Out of Scope

- Demo GIF
- GitHub issue/PR templates
- CHANGELOG.md
- Makefile or docs/ user guide
- Homebrew formula

---

## 1. `Cargo.toml` Changes

### Package Metadata

Add the following fields to `[package]`:

```toml
description = "A terminal-native typing test inspired by Monkeytype — fast, minimal, and offline-first."
license = "MIT"
repository = "https://github.com/hansonguyen/kern"
homepage = "https://github.com/hansonguyen/kern"
keywords = ["typing", "terminal", "wpm", "tui", "monkeytype"]
categories = ["command-line-utilities"]
exclude = [".github", "docs"]
```

- `license` must match the SPDX identifier for the LICENSE file (MIT ✓)
- `keywords` capped at 5 by crates.io
- `exclude` keeps the published crate lean

### Release Profile

Add a `[profile.release]` section:

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
```

- `lto = true` enables link-time optimization across crates
- `codegen-units = 1` maximizes optimization (slower compile, smaller/faster binary)
- `strip = true` removes debug symbols from the release binary

---

## 2. GitHub Actions CI

Replace `.github/workflows/ci.yml` entirely. Changes from current:

- Matrix across `ubuntu-latest` and `macos-latest`
- Use `taiki-e/install-action@nextest` to install cargo-nextest
- Replace `cargo test` with `cargo nextest run`
- Single unified job (fmt → clippy → test) per OS
- Cache is scoped per OS via `Swatinem/rust-cache@v2`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  ci:
    name: ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@nextest

      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo nextest run
```

---

## 3. README.md

Full replacement of the current skeleton. Sections in order:

### Header
- `# kern` title
- Three shields.io badges inline: CI status, crates.io version, MIT license
- One-line description

### Features
Bullet list covering:
- Word count mode (10 / 25 / 50 words) and timed mode (15s / 30s / 60s)
- Punctuation and numbers toggles
- Live WPM, raw WPM, accuracy, and character breakdown
- Persistent stats saved to `~/.config/kern/stats.json`
- Zero config, zero network — runs entirely offline

### Install
Two subsections:
1. **From crates.io (recommended):** `cargo install kern`
2. **From source:** clone + `cargo install --path .`

### Usage
- Default invocation and all flag combinations (`--time`, `--words`, `--punctuation`, `--numbers`)

### Keybindings
Markdown table: Tab / Backspace / Esc / Enter with their actions

### Results
One paragraph: what the results screen shows (WPM, raw WPM, accuracy, char breakdown), where stats persist

### Contributing
One line linking to `CONTRIBUTING.md`

### License
One line: MIT

---

## Verification

After implementation, verify:

1. `cargo publish --dry-run` exits 0
2. CI workflow runs on both `ubuntu-latest` and `macos-latest` in a push/PR
3. `cargo nextest run` passes locally (already confirmed: 73/73)
4. `cargo clippy -- -D warnings` passes (already confirmed)
5. README renders correctly in GitHub Markdown preview
