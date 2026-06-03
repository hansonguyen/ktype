# CLAUDE.md

Guidance for Claude Code (claude.ai/code) when working in this repository.

## What ktype is

ktype is a terminal-native typing test inspired by Monkeytype — fast, minimal,
and offline-first. It is driven through an interactive terminal UI, with user
preferences persisted to a TOML config file.

- **Modes**: time-limited and word-count tests, with preset and custom values,
  including an unbounded "infinite" variant.
- **Word bank toggles**: punctuation and numbers can be mixed into generated text.
- **Live metrics + results**: WPM, raw WPM, accuracy, time, and a WPM-over-time
  chart on the results screen.
- **Themeable**: colors and caret style are configurable.
- **Persistent stats**: finished tests are appended to a local stats file.
- **Update hint**: a background check surfaces a "new version available" banner;
  no network access happens while typing.

## Commands

```bash
cargo build              # build
cargo run                # run ktype
cargo test               # run tests (std runner)
cargo nextest run        # run tests with nextest (preferred)
cargo nextest run <name> # run a single test by name substring
cargo clippy -- -D warnings  # lint
cargo fmt                # format
```

## Architecture

ktype follows **The Elm Architecture (TEA)**: all state lives in one model, every
event is a message, a single update step is the only place state changes, and
rendering only reads state. Side effects never happen inside update — it returns
a description of the effect, which the I/O layer carries out.

```
src/
  lib.rs, main.rs, cli.rs — crate root, binary shim, CLI entry
  app.rs                  — the runtime: terminal setup, event loop, and the
                            I/O-bearing infrastructure it owns
  input.rs                — translates terminal events into messages
  domain/                 — pure TEA core: model, messages, update logic,
                            metrics, and test settings. No I/O.
  io/                     — all side effects: effect execution and stats persistence
  config/                 — user preferences loaded from / written to TOML
  generator/              — word generation and the word list
  ui/                      — pure rendering, one module per screen and widget
```

Each frame the runtime renders the current state, polls for input, runs the
update step, executes any resulting effect, advances a timer, and exits when the
model signals it should quit.

## Key Conventions

- The update step is the **only** place the model mutates; it stays pure (no I/O)
  and expresses any effect by returning a value for the I/O layer to run.
- Rendering reads the model only — it never mutates state.
- Route every side effect (word generation, stats writes) through the effect
  boundary in `io/`. Never inline I/O in update or rendering code.
- Keep visibility as tight as possible — expose only what the binary entry needs.
- Separate the runtime/session lifecycle from what is currently on screen; they
  are distinct concerns and should stay distinct.
- Treat infrastructure (RNG, timers, the terminal) as owned by the runtime layer,
  not as application state.
- Use `thiserror` for typed domain errors and `anyhow` at the CLI boundary.
- Comment the *why* of non-obvious invariants, not the *what*.

## Adding a feature

Pick the seam that matches the change, and keep the domain and rendering layers
pure with all effects in `io/`:

- **New screen**: add a screen state, a rendering module for it, an update branch,
  and a render-dispatch arm.
- **New modal / overlay**: add an overlay state with its own render and
  input-handling arms.
- **New side effect**: add an effect variant and handle it in the I/O execution
  layer — never perform the effect inside update.
- **New persisted setting**: extend the config model and re-seed runtime state
  when the config loads.
- **Richer test options**: extend the test-settings model.

Always add the matching test for the behavior, then run `cargo fmt`,
`cargo clippy -- -D warnings`, and `cargo nextest run` before finishing — and
review (e.g. `cargo insta review`) any new or changed snapshots.

## Testing Strategy

- **Unit**: metric calculations and character classification — pure, deterministic.
- **Property** (`proptest`): update-step invariants, generator output, and
  persistence round-trips.
- **Snapshot** (`insta`): rendered terminal frames — catch UI regressions across
  modes, themes, caret styles, and modals.
- **Integration**: full session flows (typing → results → restart), config
  toggles, and CLI behavior.
