# ktype MVP Specification

## Project Overview

**ktype** is an open-source terminal-native typing test inspired by Monkeytype.

The goal is to create a fast, minimal, visually polished CLI typing experience with real-time feedback, local stat tracking, and strong extensibility.

> Status: the MVP described here has shipped. This document reflects the product as built; the build order at the end is kept as a record of how it came together.

### Vision

ktype should feel like:

- **Monkeytype's UX philosophy**
- **Terminal-native performance**
- **Composable Rust architecture**
- **Highly testable and maintainable**

Core principles:

- Fast startup
- Zero unnecessary dependencies
- Deterministic behavior
- Beautiful terminal rendering
- Offline-first
- Open source friendly

---

# MVP Scope

## Included

### 1. Typing Test Modes

#### Time Mode

Timed typing sessions.

Durations:

- 15s
- 30s
- 60s
- Custom (e.g. `1h30m`) and an unbounded "infinite" variant

#### Word Count Mode

Fixed word-count tests.

Sizes:

- 10 words
- 25 words
- 50 words
- 100 words
- Custom count and an unbounded "infinite" variant

---

### 2. Word Bank Toggles

Generation options that mix extra tokens into the text:

- Include punctuation
- Include numbers

Both are toggled live from the idle screen — there are no CLI flags for them.

---

### 3. Keyboard Shortcuts

Monkeytype-inspired flow:

- `←` / `→` → cycle the selected option (duration or word count), when idle
- `Shift+Tab` → switch between time and words mode, when idle
- `Tab` → restart with fresh words
- `Space` / `Enter` → commit the current word; on the custom slot, `Space` opens a prompt for a custom value
- `Backspace` → delete a character, or step back to the previous word
- `@` → toggle punctuation; `#` → toggle numbers
- `Ctrl+E` → end the current test
- `Esc` → quit (or close the custom prompt)
- Character input

Future extensibility:

- Configurable keybindings

---

### 4. Real-Time Typing Feedback

Per-character state rendering:

- Untyped
- Correct
- Incorrect
- Active cursor position

Visual behavior:

- Monkeytype-style inline correctness coloring
- Configurable caret style (off / default / block / underline)
- Smooth redraws

---

### 5. Post-Test Summary

The results screen displays:

## Primary Metrics

- WPM
- Accuracy

## Secondary Metrics

- Raw WPM
- Time elapsed
- Test type and active word bank (english / + punctuation / + numbers)

## Visualization

WPM-over-time chart, inspired by the Monkeytype result screen.

> Future: a correct / incorrect / extra / missed character breakdown and a
> consistency metric (not yet shipped).

---

### 6. Local Statistics Persistence

Each finished test is appended to a local JSON file.

Storage location:

```text
~/.config/ktype/stats.json
```

Future:

SQLite if scaling demands it.

---

### 7. Thorough Testing

Required before public release (see [Testing Strategy](#testing-strategy)).

---

# Non-MVP (Later)

Not included initially:

- Multiplayer
- Online sync
- Quotes mode
- Language packs
- Plugin system
- Character breakdown and consistency metrics

(Themes and custom test generation, originally deferred, have since shipped.)

---

# Technical Stack

## Language

Rust (stable)

Target edition:

```toml
edition = "2024"
```

---

## Core Dependencies

### Terminal Rendering

- ratatui

Purpose:
Layout, rendering, widgets

---

### Terminal Events

- crossterm

Purpose:
Keyboard input, raw mode, event handling

---

### CLI

- clap

Purpose:
Argument parsing and the `--version` / `--help` entry point

---

### Serialization

- serde
- serde_json
- toml

Purpose:
Stats persistence (JSON) and user config (TOML)

---

### Random Generation

- rand

Purpose:
Word generation

---

### Error Handling

- anyhow
- thiserror

Purpose:
Robust CLI errors

---

### Update Notifications

- update-informer

Purpose:
Background "new version available" check

---

### Testing

- cargo-nextest
- proptest
- insta
- tempfile

Purpose:

- test execution
- property testing
- snapshot testing
- temporary dirs for persistence/config tests

---

# Architecture

ktype follows **The Elm Architecture (TEA)**.

## Why TEA

It gives:

- deterministic state transitions
- testability
- pure update logic
- easier reasoning for terminal UIs

---

## TEA Structure

### Model

One struct holds all application state — the current screen, the active typing
session, runtime test settings, persisted history, theme, any open overlay, and
the quit signal. Nothing else owns application state.

### Msg

An enum of every event (key presses, timer ticks, toggles, update
notifications). Messages describe *what happened*; they carry no side effects.

### Update

The single place state changes:

```rust
fn update(model: &mut Model, msg: Msg) -> Command
```

It applies state transitions and returns a `Command` describing any effect to
run. It performs no I/O itself.

### View

Pure rendering — reads the model and draws into the frame, never mutating state:

```rust
fn view(model: &Model, frame: &mut Frame)
```

### Commands

Effects, executed by the I/O layer (never inside `update`):

- generate / append words
- save stats

Infrastructure such as the RNG and timer is owned by the runtime layer, not by
the model.

---

# Project Layout

Source is grouped by responsibility rather than kept flat:

```text
src/
  lib.rs, main.rs, cli.rs — crate root, binary shim, CLI entry
  app.rs                  — runtime: terminal setup, event loop, owned infrastructure
  input.rs                — terminal events → messages
  domain/                 — pure TEA core: model, messages, update, metrics, test settings
  io/                     — side effects: effect execution, stats persistence
  config/                 — user preferences loaded from / written to TOML
  generator/              — word generation and the word list
  ui/                     — pure rendering, one module per screen and widget

tests/
  cli_flags.rs            — CLI behavior (--version / --help)
```

(Integration flows also live as `#[cfg(test)]` modules inside `src/`.)

---

# Rendering Goals

Inspired by Monkeytype:

## Main Test Screen

Sections:

- Mode / option strip (idle)
- Test text with inline correctness coloring and caret
- Live countdown / word counter
- Footer shortcuts (idle)

---

## Results Screen

- Mode / option strip across the top
- Left: WPM and accuracy
- Right: WPM-over-time chart
- Bottom-left: test type and active word bank
- Bottom-right: raw WPM and time elapsed
- Footer: change/restart and quit hints

---

# Testing Strategy

## Unit Tests

For:

- WPM / accuracy calculations
- Character classification

---

## Property Tests

Validate:

- update function invariants
- stat persistence round-trips
- generator output correctness

---

## Snapshot Tests

Validate terminal rendering across modes, themes, caret styles, and modals to
catch UI regressions.

---

## Integration Tests

Simulate:

- full typing sessions
- restart flow
- config toggles
- persistence writes
- CLI behavior (`--version` / `--help`)

---

# MVP Success Criteria

ktype MVP is complete when:

- Time mode works
- Word mode works
- Restart shortcuts work
- Results screen renders
- Stats persist locally
- Tests are comprehensive
- Clean open-source structure exists

---

# Build Order

How the MVP was built, in order:

## Phase 1
Core event loop

## Phase 2
Typing engine

## Phase 3
Rendering

## Phase 4
Metrics

## Phase 5
Persistence

## Phase 6
Testing

## Phase 7
Polish + OSS release

---

# Open Source Standards

Before public release:

- README
- CONTRIBUTING
- LICENSE (MIT preferred)
- Example screenshots
- CI pipeline
- Formatting + linting

Recommended:

- rustfmt
- clippy
- nextest
- GitHub Actions

---

# Guiding Principle

**ktype should feel like Monkeytype translated into a terminal-native Rust experience — fast, beautiful, deterministic, and hackable.**
