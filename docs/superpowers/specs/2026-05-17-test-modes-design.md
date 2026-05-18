# Test Modes Design: Endless Time Mode + Word-Count Mode

**Date:** 2026-05-17
**Status:** Approved

## Overview

Two features are added to ktype:

1. **Endless time mode** — time tests no longer have a fixed word pool. Words are appended dynamically as the user types so the test never runs out of words before the timer expires. The 3-line scrolling view already exists; this change makes the word pool behind it truly infinite.

2. **Word-count mode** — a new test mode where the user types a fixed number of words with no timer. The test ends automatically when the last character of the last word is typed (no Space required). A `X/Y` counter replaces the countdown in the header.

---

## Data Model (`model.rs`)

### New types

```rust
pub enum TestMode { Time, Words }

pub const WORD_COUNT_OPTIONS: [usize; 4] = [10, 25, 50, 100];
```

### `Config` changes

Remove `word_count: 25`. Add:

```rust
pub struct Config {
    pub test_mode: TestMode,
    // time mode
    pub time_limit: Duration,
    pub selected_duration_idx: usize,   // valid index into DURATION_OPTIONS
    // words mode
    pub word_count: usize,              // actual count used (custom-friendly)
    pub selected_word_count_idx: usize, // valid index into WORD_COUNT_OPTIONS
    // shared
    pub cursor_style: CursorStyle,
    pub punctuation: bool,              // stubbed
    pub numbers: bool,                  // stubbed
}
```

**Default:** time mode, `selected_duration_idx = 0` (15 s), `word_count = 25`, `selected_word_count_idx = 1`.

**Custom-value readiness:** `time_limit` and `word_count` store the ground-truth values used at runtime. The `selected_*_idx` fields only track which preset button is highlighted. Adding a custom option in the future is a UI-only change — the model already handles arbitrary values.

`Config` gets a helper:

```rust
impl Config {
    pub fn initial_word_count(&self) -> usize {
        match self.test_mode {
            TestMode::Time => 25,   // initial buffer; more are appended per word commit
            TestMode::Words => self.word_count,
        }
    }
}
```

---

## Message Types (`msg.rs`)

```rust
pub enum Msg {
    Tick(Duration),
    Char(char),
    Backspace,
    Space,
    Tab,
    ShiftTab,   // new: toggle test mode
    Left,       // new: cycle option backward + regenerate
    Right,      // new: cycle option forward  + regenerate
    Esc,
}
```

---

## Input Mapping (`input.rs`)

```rust
KeyCode::BackTab          => Some(Msg::ShiftTab),
KeyCode::Left             => Some(Msg::Left),
KeyCode::Right            => Some(Msg::Right),
```

Arrow keys and ShiftTab only generate messages unconditionally; the guard logic lives in `update`.

---

## Command Extension (`commands.rs`)

```rust
pub enum Command {
    None,
    GenerateWords { count: usize },
    AppendWords { count: usize },   // new
    SaveStats(StatsPayload),
}
```

`AppendWords { count }` appends `count` freshly-generated words to `session.words` without resetting any other session state.

---

## Update Logic (`update.rs`)

### `Msg::Tab`
Simplified — cycles no config options. Always resets to `Screen::Typing` and returns `Command::GenerateWords { count: model.config.initial_word_count() }`. No guard for Running state (interrupting a running test to restart is intentional, matching MonkeyType behavior).

### `Msg::ShiftTab`
Toggles `test_mode` (`Time ↔ Words`). Blocked while Running (same semantics as the old Tab cycling guard). Resets `screen = Screen::Typing`. Returns `Command::GenerateWords { count: config.initial_word_count() }`.

### `Msg::Right`
Blocked while Running. Advances the relevant option index (wrapping):
- Time mode: `selected_duration_idx = (idx + 1) % DURATION_OPTIONS.len()`, updates `time_limit`.
- Words mode: `selected_word_count_idx = (idx + 1) % WORD_COUNT_OPTIONS.len()`, updates `word_count`.
Resets `screen = Screen::Typing`. Returns `Command::GenerateWords { count: config.initial_word_count() }`.

### `Msg::Left`
Same as `Msg::Right` but cycles backward: `(idx + options.len() - 1) % options.len()`.

### `Msg::Char` — mode-gated last-word behavior

```
if is_last && word_full:
    commit word, set status = Done, screen = Done, return SaveStats  -- WORDS mode only
    advance current_word, return AppendWords { count: 1 }            -- TIME mode
```

In time mode the "last word" condition never permanently ends the test — it appends one more word and advances. In words mode it ends the test without requiring Space (matches MonkeyType behavior).

### `Msg::Space` — mode-gated last-word behavior

```
if is_last (and typed is non-empty):
    commit word, Done                   -- WORDS mode
    advance + AppendWords { count: 1 }  -- TIME mode (no end)
```

### `Msg::Tick`
- Time mode: unchanged — expires test when `elapsed >= time_limit`.
- Words mode: updates `elapsed` for WPM tracking but **does not** expire the test.

### `build_stats_payload`
- Time mode: `duration_secs = DURATION_OPTIONS[selected_duration_idx]` (unchanged).
- Words mode: `duration_secs = elapsed.as_secs()` (actual time taken).

---

## View (`view.rs`)

### Header layouts

**Time mode — waiting:**
```
kern  [time] words   [15]  30  60   [←→] cycle  [tab] restart  [shift+tab] → word mode
```

**Time mode — running:**
```
kern  [time] words   [15]  30  60  ·  12s   [tab] restart
```

**Words mode — waiting:**
```
kern  time [words]   [10]  25  50  100   [←→] cycle  [tab] restart  [shift+tab] → time mode
```

**Words mode — running:**
```
kern  time [words]   [10]  25  50  100   5/25   [tab] restart
```

**Footer (all states):**
```
[esc] quit
```

### Refactored helpers

`duration_strip_spans` is generalized into `options_strip_spans(labels, selected_idx, dimmed)` that renders any slice of string labels with one highlighted. Both duration and word-count option strips use it.

A new `mode_selector_spans(mode, is_running)` renders `[time] words` or `time [words]`, dimmed while running.

The `X/Y` counter in words-mode-running uses `current_word + 1` / `session.words.len()` and is rendered bold.

---

## Testing Strategy

### Unit tests (`update.rs`)

- ShiftTab toggles `test_mode` and returns `GenerateWords`
- ShiftTab while Running is a no-op (mode stays, no restart)
- Right cycles `selected_duration_idx` in time mode
- Right cycles `selected_word_count_idx` in words mode
- Left cycles backward (wraps correctly)
- Arrows while Running are no-ops
- Tab no longer cycles duration index
- Tick in words mode does not transition to Done
- Last char of last word in words mode ends test (auto-end, no Space required)
- Last char of last word in time mode advances word and returns `AppendWords`
- Space on last word in words mode ends test
- Space on last word in time mode advances and returns `AppendWords`

### Property tests (`proptest`)

- `current_word` stays in bounds through arbitrary inputs in both modes
- `words.len()` never shrinks (words are only appended, never removed)

### Snapshot tests (`insta`)

Update existing typing-screen snapshots to reflect new header. Add:
- Words mode waiting (with options strip + hints)
- Words mode running with `X/Y` counter

### Integration tests

Full words-mode session flow: start → type all words → results screen lands with correct stats.

---

## Commit Strategy

Implementation is grouped into 3–4 logical commits that can be reviewed independently:

1. **Model + message layer** — `TestMode`, `Config` fields, `Msg` variants, `input.rs` mappings, `Command::AppendWords`
2. **Update logic** — all `update.rs` changes: mode-gated handlers, arrow/ShiftTab/Tab rework, `AppendWords` return paths
3. **View** — mode selector, generalized options strip, `X/Y` counter, updated header layouts
4. **Tests** — new unit/property/snapshot/integration tests; update existing snapshots

Each commit should leave the codebase in a working, buildable state.
