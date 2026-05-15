# Kern MVP Specification

## Project Overview

**Kern** is an open-source terminal-native typing test inspired by Monkeytype.

The goal is to create a fast, minimal, visually polished CLI typing experience with real-time feedback, local stat tracking, and strong extensibility.

### Vision

Kern should feel like:

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

Initial durations:

- 15s
- 30s
- 60s

#### Word Count Mode

Fixed word-count tests.

Initial sizes:

- 10 words
- 25 words
- 50 words

---

### 2. Input Configuration Toggles

Configurable generation options:

- Include punctuation
- Include numbers

Examples:

```text
kern --time 15 --punctuation
kern --words 25 --numbers
```

---

### 3. Keyboard Shortcuts

Default Monkeytype-inspired flow:

- `Tab` → restart
- `Enter` → confirm restart / continue
- `Esc` → quit
- `Backspace` → delete
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
- Cursor highlight
- Smooth redraws

---

### 5. Post-Test Summary

Display:

## Primary Metrics

- WPM
- Raw WPM
- Accuracy
- Time elapsed

## Secondary Metrics

- Character breakdown:
  - Correct
  - Incorrect
  - Extra
  - Missed

- Consistency

## Optional MVP Stretch

ASCII graph of typing speed over time.

Inspired by Monkeytype result screen.

---

### 6. Local Statistics Persistence

Store locally.

Tracked data:

- Total tests completed
- Average WPM
- Personal best
- Accuracy average
- Historical sessions

Storage location:

```text
~/.config/kern/stats.json
```

Future:

SQLite if scaling demands it.

---

### 7. Thorough Testing

Required before public MVP release.

---

# Non-MVP (Later)

Not included initially:

- Multiplayer
- Online sync
- Themes
- Quotes mode
- Language packs
- Custom test generation
- Plugin system

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

### Serialization

- serde
- serde_json

Purpose:
Stats persistence

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

### Testing

- cargo-nextest
- proptest
- insta

Purpose:

- test execution
- property testing
- snapshot testing

---

# Architecture

Kern should follow **The Elm Architecture (TEA)**.

## Why TEA

It gives:

- deterministic state transitions
- testability
- pure update logic
- easier reasoning for terminal UIs

---

## TEA Structure

## Model

Application state.

```rust
pub struct Model {
    pub screen: Screen,
    pub session: SessionState,
    pub stats: Stats,
    pub config: Config,
}
```

Contains all application state.

---

## Msg

All events.

```rust
pub enum Msg {
    Tick,
    KeyPressed(KeyEvent),
    Restart,
    TestCompleted,
    TogglePunctuation,
    ToggleNumbers,
}
```

No side effects here.

---

## Update

Pure state transitions.

```rust
fn update(model: &mut Model, msg: Msg) -> Command
```

Responsibilities:

- apply state changes
- produce commands

---

## View

Pure rendering.

```rust
fn view(model: &Model) -> Frame
```

Only renders.

No mutation.

---

## Commands

Side effects:

- save stats
- load config
- start timer
- generate test

---

# Proposed Project Layout

```text
src/
  main.rs
  app.rs
  model.rs
  msg.rs
  update.rs
  view.rs
  commands.rs
  input.rs
  stats.rs
  persistence.rs
  generator.rs
  metrics.rs

tests/
  integration/
```

---

# Rendering Goals

Inspired by Monkeytype:

## Main Test Screen

Sections:

- Header
- Test text
- Live metrics
- Footer shortcuts

---

## Results Screen

Layout inspired by reference image:

Left:

- WPM
- Accuracy

Center:

- Character breakdown
- Raw score

Right:

- Time
- Consistency

Optional:

ASCII trend graph

---

# Testing Strategy

## Unit Tests

For:

- WPM calculations
- Accuracy calculations
- Character classification
- State transitions

---

## Property Tests

Validate:

- update function invariants
- stat persistence correctness
- generator output correctness

---

## Snapshot Tests

Validate terminal rendering.

Ensure UI regressions are caught.

---

## Integration Tests

Simulate:

- full typing sessions
- restart flow
- config toggles
- persistence writes

---

# MVP Success Criteria

Kern MVP is complete when:

- Time mode works
- Word mode works
- Restart shortcuts work
- Results screen renders
- Stats persist locally
- Tests are comprehensive
- Clean open-source structure exists

---

# Build Order

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

**Kern should feel like Monkeytype translated into a terminal-native Rust experience — fast, beautiful, deterministic, and hackable.**
