# Phase 7: OSS Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring kern to a publish-ready state: accurate Cargo.toml metadata, release-optimized binary, macOS+Linux CI with nextest, and a production-quality README that reflects what the binary actually does.

**Architecture:** Three independent file changes — `Cargo.toml`, `.github/workflows/ci.yml`, and `README.md` — each committed separately. No new source files. No new dependencies.

**Tech Stack:** Cargo (crates.io metadata, release profiles), GitHub Actions (matrix CI), shields.io (badges)

---

## File Map

| File | Change |
|------|--------|
| `Cargo.toml` | Add `description`, `license`, `repository`, `homepage`, `keywords`, `categories`, `exclude`; add `[profile.release]` |
| `.github/workflows/ci.yml` | Replace single ubuntu job with OS-matrix job using nextest |
| `README.md` | Full rewrite — accurate features, install, usage, keybindings, results |

---

## Task 1: Cargo.toml — Package Metadata + Release Profile

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add package metadata fields**

Open `Cargo.toml`. The current `[package]` section ends at `edition = "2024"`. Add these fields immediately after `edition`:

```toml
[package]
name = "kern"
version = "0.1.0"
edition = "2024"
description = "A terminal-native typing test inspired by Monkeytype — fast, minimal, and offline-first."
license = "MIT"
repository = "https://github.com/hansonguyen/kern"
homepage = "https://github.com/hansonguyen/kern"
keywords = ["typing", "terminal", "wpm", "tui", "monkeytype"]
categories = ["command-line-utilities"]
exclude = [".github", "docs"]
```

- [ ] **Step 2: Add release profile**

Append this section at the end of `Cargo.toml`, after `[dev-dependencies]`:

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
```

- [ ] **Step 3: Verify metadata is complete**

Run:
```bash
cargo publish --dry-run --allow-dirty 2>&1
```

Expected output ends with something like:
```
Uploading kern v0.1.0
```
with no "missing field" or "invalid" errors. If it complains about a missing field, add it.

- [ ] **Step 4: Verify release build still compiles**

```bash
cargo build --release 2>&1 | tail -3
```

Expected: `Finished release [optimized] target(s)` with no errors.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add crates.io metadata and release profile to Cargo.toml"
```

---

## Task 2: GitHub Actions CI — Matrix + Nextest

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Replace the workflow file entirely**

Write the following content to `.github/workflows/ci.yml` (overwrite the existing file):

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

- [ ] **Step 2: Verify all three checks pass locally**

Run each command and confirm all pass:

```bash
cargo fmt --check
```
Expected: no output, exit 0.

```bash
cargo clippy -- -D warnings
```
Expected: no warnings or errors.

```bash
cargo nextest run
```
Expected: `73 tests run: 73 passed, 0 skipped`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: upgrade to nextest and add macOS matrix"
```

---

## Task 3: README.md — Production-Quality Rewrite

**Files:**
- Modify: `README.md`

> **Important:** The binary has no CLI flags. `--time`, `--words`, `--punctuation`, and `--numbers` do not exist. The current README documents flags that don't work — this rewrite fixes that. All features described below are confirmed present in the codebase.

- [ ] **Step 1: Rewrite README.md with the following content**

Replace the entire contents of `README.md`:

```markdown
# kern

[![CI](https://github.com/hansonguyen/kern/actions/workflows/ci.yml/badge.svg)](https://github.com/hansonguyen/kern/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/kern.svg)](https://crates.io/crates/kern)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A terminal-native typing test inspired by Monkeytype — fast, minimal, and offline-first.

## Features

- Timed tests: 15s, 30s, or 60s (cycle with `Tab`)
- Live WPM, raw WPM, accuracy, and character breakdown
- Persistent stats saved to `~/.config/kern/stats.json`
- Zero config, zero network — runs entirely offline

## Install

### From crates.io

```bash
cargo install kern
```

### From source

```bash
git clone https://github.com/hansonguyen/kern
cd kern
cargo install --path .
```

## Usage

```bash
kern
```

kern starts a 15-second timed test immediately. Press `Tab` to cycle through duration options (15s → 30s → 60s) before typing begins.

## Keybindings

| Key               | Action                                    |
|-------------------|-------------------------------------------|
| `Tab`             | Cycle duration (waiting) / Restart test   |
| `Space` / `Enter` | Commit current word                       |
| `Backspace`       | Delete last character                     |
| `Esc`             | Quit                                      |

## Results

After each test, kern shows:

- **WPM** — words per minute (correctly typed words only)
- **Raw WPM** — all keystrokes, including errors
- **Accuracy** — percentage of correct keystrokes
- **Breakdown** — correct / incorrect / extra / missed characters

Stats are saved automatically to `~/.config/kern/stats.json`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
```

- [ ] **Step 2: Verify no phantom flags remain**

```bash
grep -n "\-\-time\|\-\-words\|\-\-punctuation\|\-\-numbers" README.md
```

Expected: no output (zero matches). If any appear, remove them.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: rewrite README with accurate features, install, and keybindings"
```

---

## Task 4: Final Verification

**Files:** none (read-only verification)

- [ ] **Step 1: Full test suite**

```bash
cargo nextest run
```

Expected: `73 tests run: 73 passed, 0 skipped`

- [ ] **Step 2: Lint**

```bash
cargo clippy -- -D warnings
```

Expected: no output, exit 0.

- [ ] **Step 3: Format check**

```bash
cargo fmt --check
```

Expected: no output, exit 0.

- [ ] **Step 4: Publish dry-run**

```bash
cargo publish --dry-run
```

Expected: completes without errors. The crates.io upload line appears at the end. Note: the crates.io version badge in the README will show "not found" until the first real publish — that is expected.

- [ ] **Step 5: Confirm README accuracy**

Read through `README.md` and verify:
- No CLI flags mentioned (`--time`, `--words`, etc.)
- Install instructions use the correct repo URL (`https://github.com/hansonguyen/kern`)
- Keybindings match `src/input.rs` (Tab, Space/Enter, Backspace, Esc)
- Duration options match `DURATION_OPTIONS` in `src/model.rs` (15, 30, 60)
