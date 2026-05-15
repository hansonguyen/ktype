# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build              # build
cargo run                # run kern
cargo test               # run tests (std runner)
cargo nextest run        # run tests with nextest (preferred)
cargo nextest run <name> # run a single test by name substring
cargo clippy -- -D warnings  # lint
cargo fmt                # format
```

## Architecture

Kern follows **The Elm Architecture (TEA)**. All state lives in `Model`, all events flow as `Msg`, `update` is the only place state changes, and `view` is pure rendering.

```
main.rs        — terminal init/restore, 60fps event loop
app.rs         — current skeleton (will be replaced by TEA modules)
model.rs       — Model struct: Screen, SessionState, Stats, Config
msg.rs         — Msg enum: all events, no side effects
update.rs      — pure fn update(model, msg) -> Command
view.rs        — pure fn view(model, frame): only renders, no mutation
commands.rs    — side effects: save stats, generate words, start timer
input.rs       — keystroke routing and raw character handling
generator.rs   — random word generation (with punctuation/numbers toggles)
metrics.rs     — WPM, raw WPM, accuracy, consistency calculations
stats.rs       — session stats types
persistence.rs — JSON R/W to ~/.config/kern/stats.json
```

The loop in `main.rs` polls events at 16ms (≈60fps), dispatches to `handle_event`, then redraws. `should_quit` on the app struct is the exit signal.

## Key Conventions

- `update` must remain pure — no I/O, no side effects; return a `Command` for anything async/effectful
- `view` must remain pure — read `Model` only, never mutate
- Character state (untyped / correct / incorrect / cursor) is the core data primitive for the test screen
- Stats persist to `~/.config/kern/stats.json`; future migration path is SQLite
- Use `thiserror` for domain errors, `anyhow` for top-level CLI error propagation
- Property tests (`proptest`) cover `update` invariants and metric calculations; snapshot tests (`insta`) cover rendered UI frames

## Testing Strategy

- **Unit**: WPM/accuracy/character classification — pure functions, deterministic inputs
- **Property** (`proptest`): `update` state machine invariants, generator output, persistence round-trips
- **Snapshot** (`insta`): terminal frame rendering — catch UI regressions
- **Integration**: full session flows (typing → result screen → restart), config toggles, persistence writes

## Build Phases (from spec)

1. ✅ Core event loop (`main.rs` + `app.rs`)
2. → Typing engine (`model`, `msg`, `update`, `input`, `generator`)
3. Rendering (`view`, ratatui widgets)
4. Metrics (`metrics.rs`)
5. Persistence (`persistence.rs`, `stats.rs`)
6. Testing (nextest, proptest, insta)
7. Polish + OSS release (README, CI, rustfmt, clippy)
