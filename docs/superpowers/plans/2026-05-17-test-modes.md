# Test Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add endless time mode (dynamic word appending so the pool never runs dry) and word-count mode (fixed N words, no timer, auto-ends on last char, X/Y counter).

**Architecture:** Model gains `TestMode` enum and word-count config fields. A new `Command::AppendWords` enables dynamic pool growth in time mode. `update.rs` is mode-gated: time mode appends per commit, words mode ends on last char. Arrow keys cycle options; Tab becomes restart-only; Shift+Tab toggles mode. View header shows a mode selector and context-sensitive hints.

**Tech Stack:** Rust, ratatui, crossterm, proptest, insta (cargo-insta for snapshot review)

---

## File Map

| File | Change |
|------|--------|
| `src/model.rs` | Add `TestMode`, `WORD_COUNT_OPTIONS`, new `Config` fields, `initial_word_count()` |
| `src/msg.rs` | Add `ShiftTab`, `Left`, `Right` variants |
| `src/input.rs` | Map `BackTab`, `Left`, `Right` keys |
| `src/commands.rs` | Add `AppendWords` variant and execution logic |
| `src/update.rs` | Mode-gated handlers; Tab simplified; arrow/ShiftTab; endless time; words lifecycle; stats payload |
| `src/view.rs` | Mode selector, generalized options strip, X/Y counter, new header layouts |
| `src/main.rs` | Use `initial_word_count()` for initial generation |
| `src/integration_tests.rs` | Fix broken tests; add words-mode flow test |
| `src/snapshots/*.snap` | Update all 8 snapshots for new header |

---

## Task 1 — Data Foundation

**Files:**
- Modify: `src/model.rs`
- Modify: `src/msg.rs`
- Modify: `src/input.rs`
- Modify: `src/commands.rs`
- Modify: `src/main.rs`

- [ ] **Step 1.1 — Update `src/model.rs`**

Replace the entire file content:

```rust
use std::time::Duration;

use crate::stats::SessionResult;

pub const DURATION_OPTIONS: [u64; 3] = [15, 30, 60];
pub const WORD_COUNT_OPTIONS: [usize; 4] = [10, 25, 50, 100];

#[derive(Debug, Clone, PartialEq)]
pub enum TestMode {
    Time,
    Words,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Typing,
    Done,
    Quitting,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Waiting,
    Running,
    Done,
}

#[derive(Debug, Clone)]
pub struct Word {
    pub chars: Vec<char>,
    pub typed: String,
    pub committed: bool,
}

impl Word {
    pub fn new(text: &str) -> Self {
        Word {
            chars: text.chars().collect(),
            typed: String::new(),
            committed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CursorStyle {
    Block,
    #[expect(dead_code)]
    Underline,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub test_mode: TestMode,
    pub cursor_style: CursorStyle,
    // time mode
    pub time_limit: Duration,
    // invariant: always a valid index into DURATION_OPTIONS
    pub selected_duration_idx: usize,
    // words mode
    pub word_count: usize,
    // invariant: always a valid index into WORD_COUNT_OPTIONS
    pub selected_word_count_idx: usize,
    #[expect(dead_code)]
    pub punctuation: bool,
    #[expect(dead_code)]
    pub numbers: bool,
}

impl Config {
    /// Words to generate on test start. Time mode uses a fixed buffer that
    /// grows dynamically; words mode uses the configured word count.
    pub fn initial_word_count(&self) -> usize {
        match self.test_mode {
            TestMode::Time => 25,
            TestMode::Words => self.word_count,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            test_mode: TestMode::Time,
            cursor_style: CursorStyle::Block,
            time_limit: Duration::from_secs(15),
            selected_duration_idx: 0,
            word_count: WORD_COUNT_OPTIONS[1], // 25
            selected_word_count_idx: 1,
            punctuation: false,
            numbers: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub words: Vec<Word>,
    pub current_word: usize,
    pub status: TestStatus,
    pub elapsed: Duration,
}

impl SessionState {
    pub fn new(words: Vec<Word>) -> Self {
        SessionState {
            words,
            current_word: 0,
            status: TestStatus::Waiting,
            elapsed: Duration::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Model {
    pub screen: Screen,
    pub session: SessionState,
    pub config: Config,
    pub history: Vec<SessionResult>,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            screen: Screen::Typing,
            session: SessionState::new(Vec::new()),
            config: Config::default(),
            history: Vec::new(),
        }
    }
}
```

- [ ] **Step 1.2 — Update `src/msg.rs`**

```rust
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Tick(Duration),
    Char(char),
    Backspace,
    Space,
    Tab,
    ShiftTab,
    Left,
    Right,
    Esc,
}
```

- [ ] **Step 1.3 — Update `src/input.rs`**

Add three new key mappings. Replace the `event_to_msg` match arms:

```rust
pub fn event_to_msg(event: Event) -> Option<Msg> {
    match event {
        Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code,
            ..
        }) => match code {
            KeyCode::Char(' ') | KeyCode::Enter => Some(Msg::Space),
            KeyCode::Char(c) => Some(Msg::Char(c)),
            KeyCode::Backspace => Some(Msg::Backspace),
            KeyCode::Tab => Some(Msg::Tab),
            KeyCode::BackTab => Some(Msg::ShiftTab),
            KeyCode::Left => Some(Msg::Left),
            KeyCode::Right => Some(Msg::Right),
            KeyCode::Esc => Some(Msg::Esc),
            _ => None,
        },
        _ => None,
    }
}
```

Also add three new tests to the tests module in `input.rs`:

```rust
#[test]
fn shift_tab_maps_to_shifttab_msg() {
    assert_eq!(
        event_to_msg(key_press(KeyCode::BackTab)),
        Some(Msg::ShiftTab)
    );
}

#[test]
fn left_maps_to_left_msg() {
    assert_eq!(
        event_to_msg(key_press(KeyCode::Left)),
        Some(Msg::Left)
    );
}

#[test]
fn right_maps_to_right_msg() {
    assert_eq!(
        event_to_msg(key_press(KeyCode::Right)),
        Some(Msg::Right)
    );
}
```

- [ ] **Step 1.4 — Update `src/commands.rs`**

Add `AppendWords` to the enum and its execution branch:

```rust
use std::time::SystemTime;

use rand::rngs::SmallRng;

use crate::generator;
use crate::model::{Model, SessionState};
use crate::persistence;
use crate::stats::SessionResult;

#[derive(Debug)]
pub struct StatsPayload {
    pub duration_secs: u64,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
}

#[derive(Debug)]
pub enum Command {
    None,
    GenerateWords { count: usize },
    AppendWords { count: usize },
    SaveStats(StatsPayload),
}

pub fn execute_command(model: &mut Model, cmd: Command, rng: &mut SmallRng) {
    match cmd {
        Command::None => {}
        Command::GenerateWords { count } => {
            model.session = SessionState::new(generator::generate(count, rng));
        }
        Command::AppendWords { count } => {
            model.session.words.extend(generator::generate(count, rng));
            // Advance to the newly appended word if the current word is committed.
            // This handles the last-word case where update deferred the advance.
            if model.session.current_word + 1 < model.session.words.len()
                && model.session.words[model.session.current_word].committed
            {
                model.session.current_word += 1;
            }
        }
        Command::SaveStats(payload) => {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let result = SessionResult {
                timestamp,
                duration_secs: payload.duration_secs,
                wpm: payload.wpm,
                raw_wpm: payload.raw_wpm,
                accuracy: payload.accuracy,
            };
            model.history.push(result);
            if let Err(e) = persistence::append(model.history.last().unwrap()) {
                eprintln!("kern: failed to save stats: {e}");
            }
        }
    }
}
```

- [ ] **Step 1.5 — Update `src/main.rs`**

Replace `let word_count = model.config.word_count;` and the `execute_command` call below it with:

```rust
execute_command(
    &mut model,
    Command::GenerateWords {
        count: model.config.initial_word_count(),
    },
    &mut rng,
);
```

- [ ] **Step 1.6 — Verify compilation**

```bash
cargo build 2>&1
```

Expected: no errors. Warnings are OK at this stage (update.rs/view.rs still use old patterns and will emit unused-import or non-exhaustive match warnings — that's expected and will be fixed in subsequent tasks).

---

## Task 2 — Update: Tab, Arrow Keys, ShiftTab

**Files:**
- Modify: `src/update.rs`

- [ ] **Step 2.1 — Write failing tests**

Add these tests to the `tests` module in `update.rs`. The existing `tab_while_waiting_cycles_to_next_duration` and `tab_cycles_through_all_durations` and `tab_while_done_cycles_duration` tests tested the old Tab cycling behavior; replace them with tests for the new arrow-key cycling.

First, find and **delete** these three tests (they test removed behavior and will never pass):
- `tab_while_waiting_cycles_to_next_duration`
- `tab_cycles_through_all_durations`
- `tab_while_done_cycles_duration`

Then add these new tests:

```rust
#[test]
fn right_cycles_duration_forward() {
    let mut model = model_with_words(&["hello"]);
    assert_eq!(model.session.status, TestStatus::Waiting);
    update(&mut model, Msg::Right);
    assert_eq!(model.config.selected_duration_idx, 1);
    assert_eq!(model.config.time_limit, Duration::from_secs(30));
}

#[test]
fn right_cycles_duration_wraps() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Right); // 0 → 1
    update(&mut model, Msg::Right); // 1 → 2
    update(&mut model, Msg::Right); // 2 → 0
    assert_eq!(model.config.selected_duration_idx, 0);
    assert_eq!(model.config.time_limit, Duration::from_secs(15));
}

#[test]
fn left_cycles_duration_backward() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Left); // 0 → 2 (wraps)
    assert_eq!(model.config.selected_duration_idx, 2);
    assert_eq!(model.config.time_limit, Duration::from_secs(60));
}

#[test]
fn right_cycles_word_count_in_words_mode() {
    let mut model = model_with_words(&["hello"]);
    model.config.test_mode = TestMode::Words;
    update(&mut model, Msg::Right); // idx 1 (25) → idx 2 (50)
    assert_eq!(model.config.selected_word_count_idx, 2);
    assert_eq!(model.config.word_count, WORD_COUNT_OPTIONS[2]);
}

#[test]
fn left_cycles_word_count_in_words_mode() {
    let mut model = model_with_words(&["hello"]);
    model.config.test_mode = TestMode::Words;
    update(&mut model, Msg::Left); // idx 1 (25) → idx 0 (10)
    assert_eq!(model.config.selected_word_count_idx, 0);
    assert_eq!(model.config.word_count, WORD_COUNT_OPTIONS[0]);
}

#[test]
fn right_while_running_is_noop() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Char('h')); // → Running
    assert_eq!(model.session.status, TestStatus::Running);
    let idx_before = model.config.selected_duration_idx;
    update(&mut model, Msg::Right);
    assert_eq!(model.config.selected_duration_idx, idx_before);
}

#[test]
fn left_while_running_is_noop() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Char('h'));
    let idx_before = model.config.selected_duration_idx;
    update(&mut model, Msg::Left);
    assert_eq!(model.config.selected_duration_idx, idx_before);
}

#[test]
fn right_returns_generate_words_command() {
    let mut model = model_with_words(&["hello"]);
    let cmd = update(&mut model, Msg::Right);
    assert!(matches!(cmd, Command::GenerateWords { .. }));
}

#[test]
fn tab_no_longer_cycles_duration() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Tab);
    // Tab is restart-only now — selected_duration_idx must not change
    assert_eq!(model.config.selected_duration_idx, 0);
}

#[test]
fn shifttab_toggles_from_time_to_words() {
    let mut model = model_with_words(&["hello"]);
    assert_eq!(model.config.test_mode, TestMode::Time);
    update(&mut model, Msg::ShiftTab);
    assert_eq!(model.config.test_mode, TestMode::Words);
}

#[test]
fn shifttab_toggles_from_words_to_time() {
    let mut model = model_with_words(&["hello"]);
    model.config.test_mode = TestMode::Words;
    update(&mut model, Msg::ShiftTab);
    assert_eq!(model.config.test_mode, TestMode::Time);
}

#[test]
fn shifttab_while_running_is_noop() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Char('h'));
    assert_eq!(model.session.status, TestStatus::Running);
    update(&mut model, Msg::ShiftTab);
    assert_eq!(model.config.test_mode, TestMode::Time); // unchanged
}

#[test]
fn shifttab_returns_generate_words_command() {
    let mut model = model_with_words(&["hello"]);
    let cmd = update(&mut model, Msg::ShiftTab);
    assert!(matches!(cmd, Command::GenerateWords { .. }));
}
```

- [ ] **Step 2.2 — Run tests to confirm they fail**

```bash
cargo nextest run tab_no_longer_cycles_duration right_cycles_duration_forward left_cycles_duration_backward shifttab_toggles_from_time_to_words 2>&1 | head -30
```

Expected: compilation errors or test failures (handlers not yet implemented).

- [ ] **Step 2.3 — Implement the new handlers in `update.rs`**

Update the imports at the top of `update.rs`:

```rust
use crate::model::{DURATION_OPTIONS, WORD_COUNT_OPTIONS, Model, Screen, TestMode, TestStatus};
```

Replace the `Msg::Tab` arm:

```rust
Msg::Tab => {
    model.screen = Screen::Typing;
    return Command::GenerateWords {
        count: model.config.initial_word_count(),
    };
}
```

Add these arms before `Command::None` at the end (place them after `Msg::Tab`):

```rust
Msg::ShiftTab => {
    if model.session.status == TestStatus::Running {
        return Command::None;
    }
    model.config.test_mode = match model.config.test_mode {
        TestMode::Time => TestMode::Words,
        TestMode::Words => TestMode::Time,
    };
    model.screen = Screen::Typing;
    return Command::GenerateWords {
        count: model.config.initial_word_count(),
    };
}

Msg::Right => {
    if model.session.status == TestStatus::Running {
        return Command::None;
    }
    match model.config.test_mode {
        TestMode::Time => {
            let next =
                (model.config.selected_duration_idx + 1) % DURATION_OPTIONS.len();
            model.config.selected_duration_idx = next;
            model.config.time_limit =
                Duration::from_secs(DURATION_OPTIONS[next]);
        }
        TestMode::Words => {
            let next = (model.config.selected_word_count_idx + 1)
                % WORD_COUNT_OPTIONS.len();
            model.config.selected_word_count_idx = next;
            model.config.word_count = WORD_COUNT_OPTIONS[next];
        }
    }
    model.screen = Screen::Typing;
    return Command::GenerateWords {
        count: model.config.initial_word_count(),
    };
}

Msg::Left => {
    if model.session.status == TestStatus::Running {
        return Command::None;
    }
    match model.config.test_mode {
        TestMode::Time => {
            let prev = (model.config.selected_duration_idx
                + DURATION_OPTIONS.len()
                - 1)
                % DURATION_OPTIONS.len();
            model.config.selected_duration_idx = prev;
            model.config.time_limit =
                Duration::from_secs(DURATION_OPTIONS[prev]);
        }
        TestMode::Words => {
            let prev = (model.config.selected_word_count_idx
                + WORD_COUNT_OPTIONS.len()
                - 1)
                % WORD_COUNT_OPTIONS.len();
            model.config.selected_word_count_idx = prev;
            model.config.word_count = WORD_COUNT_OPTIONS[prev];
        }
    }
    model.screen = Screen::Typing;
    return Command::GenerateWords {
        count: model.config.initial_word_count(),
    };
}
```

- [ ] **Step 2.4 — Run tests**

```bash
cargo nextest run 2>&1 | tail -20
```

Expected: all tests pass. The three deleted tests are gone; the new ones pass. The existing `tab_while_running_does_not_cycle` and `tab_returns_generate_words_command` tests still pass since Tab no longer cycles but still returns GenerateWords.

---

## Task 3 — Update: Endless Time Mode

**Files:**
- Modify: `src/update.rs`

- [ ] **Step 3.1 — Write failing tests**

Add to the `tests` module in `update.rs`:

```rust
#[test]
fn time_mode_last_char_appends_not_ends() {
    // In time mode, completing the last word should NOT end the test.
    // It should return AppendWords.
    let mut model = model_with_words(&["hi"]);
    // model_with_words uses Config::default() which is TestMode::Time
    assert_eq!(model.config.test_mode, TestMode::Time);
    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Char('i')); // last char of last word
    assert!(matches!(cmd, Command::AppendWords { count: 1 }));
    // Test must NOT be done
    assert_eq!(model.session.status, TestStatus::Running);
    assert_eq!(model.screen, Screen::Typing);
}

#[test]
fn time_mode_space_on_last_word_appends_not_ends() {
    let mut model = model_with_words(&["hi"]);
    assert_eq!(model.config.test_mode, TestMode::Time);
    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Space);
    assert!(matches!(cmd, Command::AppendWords { count: 1 }));
    assert_eq!(model.session.status, TestStatus::Running);
}

#[test]
fn time_mode_non_last_space_appends_word() {
    let mut model = model_with_words(&["hi", "ok"]);
    assert_eq!(model.config.test_mode, TestMode::Time);
    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Space); // non-last word
    assert!(matches!(cmd, Command::AppendWords { count: 1 }));
    assert_eq!(model.session.current_word, 1); // advanced immediately
}

#[test]
fn time_mode_current_word_stays_in_bounds_after_last_word_commit() {
    // After last word commit (no execute_command), current_word must remain valid.
    let mut model = model_with_words(&["hi"]);
    update(&mut model, Msg::Char('h'));
    update(&mut model, Msg::Char('i')); // commits last word, defers advance
    // current_word == 0, words.len() == 1 — still valid
    assert!(model.session.current_word < model.session.words.len());
}
```

- [ ] **Step 3.2 — Run tests to confirm they fail**

```bash
cargo nextest run time_mode_last_char_appends_not_ends time_mode_space_on_last_word_appends_not_ends 2>&1 | head -20
```

Expected: failures.

- [ ] **Step 3.3 — Implement mode-gated `Msg::Char` and `Msg::Space`**

Replace the entire `Msg::Char` arm in `update.rs`:

```rust
Msg::Char(c) => {
    let session = &mut model.session;
    if session.words.is_empty() {
        return Command::None;
    }
    if session.status == TestStatus::Waiting {
        session.status = TestStatus::Running;
    }
    let word = &mut session.words[session.current_word];
    if word.typed.len() < word.chars.len() {
        word.typed.push(c);
    }

    let is_last = session.current_word == session.words.len() - 1;
    let word_full = session.words[session.current_word].typed.len()
        == session.words[session.current_word].chars.len();

    if is_last && word_full {
        match model.config.test_mode {
            TestMode::Words => {
                session.words[session.current_word].committed = true;
                session.status = TestStatus::Done;
                model.screen = Screen::Done;
                return Command::SaveStats(build_stats_payload(model));
            }
            TestMode::Time => {
                // Commit but defer advance — execute_command will advance
                // after appending so current_word is never out of bounds.
                session.words[session.current_word].committed = true;
                return Command::AppendWords { count: 1 };
            }
        }
    }
}
```

Replace the entire `Msg::Space` arm:

```rust
Msg::Space => {
    let session = &mut model.session;
    if session.words.is_empty()
        || session.words[session.current_word].typed.is_empty()
        || session.words[session.current_word].committed
    {
        return Command::None;
    }
    let is_last = session.current_word == session.words.len() - 1;
    session.words[session.current_word].committed = true;

    if is_last {
        match model.config.test_mode {
            TestMode::Words => {
                session.status = TestStatus::Done;
                model.screen = Screen::Done;
                return Command::SaveStats(build_stats_payload(model));
            }
            TestMode::Time => {
                // Defer advance — execute_command advances after appending.
                return Command::AppendWords { count: 1 };
            }
        }
    } else {
        session.current_word += 1;
        if matches!(model.config.test_mode, TestMode::Time) {
            return Command::AppendWords { count: 1 };
        }
    }
}
```

- [ ] **Step 3.4 — Fix existing tests that assumed time mode ends on last word**

Five existing tests used `Config::default()` (time mode) and asserted the test ends when the last word is committed. They now need `TestMode::Words`. Also add the new imports to the test module's `use` block:

```rust
use crate::model::{Config, DURATION_OPTIONS, WORD_COUNT_OPTIONS, Screen, SessionState, TestMode, TestStatus, Word};
```

In each of these five test bodies, add `model.config.test_mode = TestMode::Words;` right after constructing the model:

- `space_on_last_word_sets_done`
- `space_on_last_word_returns_save_stats_command`
- `last_char_of_last_word_auto_ends_test`
- `last_char_of_last_word_returns_save_stats_command`
- `last_char_on_multi_word_test_auto_ends`

For example, `space_on_last_word_sets_done` becomes:

```rust
#[test]
fn space_on_last_word_sets_done() {
    let mut model = model_with_words(&["hi"]);
    model.config.test_mode = TestMode::Words;
    update(&mut model, Msg::Char('h'));
    update(&mut model, Msg::Space);
    assert_eq!(model.session.status, TestStatus::Done);
    assert_eq!(model.screen, Screen::Done);
}
```

Apply the same one-line addition to each of the other four tests.

- [ ] **Step 3.5 — Run tests**

```bash
cargo nextest run 2>&1 | tail -20
```

Expected: all tests pass including the new time-mode tests.

---

## Task 4 — Update: Words Mode Lifecycle + Stats

**Files:**
- Modify: `src/update.rs`

- [ ] **Step 4.1 — Write failing tests**

Add to `update.rs` tests module:

```rust
#[test]
fn words_mode_last_char_ends_test() {
    let mut model = model_with_words(&["hi"]);
    model.config.test_mode = TestMode::Words;
    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Char('i'));
    assert_eq!(model.session.status, TestStatus::Done);
    assert_eq!(model.screen, Screen::Done);
    assert!(matches!(cmd, Command::SaveStats(_)));
}

#[test]
fn words_mode_space_on_last_word_ends_test() {
    let mut model = model_with_words(&["hi"]);
    model.config.test_mode = TestMode::Words;
    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Space);
    assert_eq!(model.session.status, TestStatus::Done);
    assert!(matches!(cmd, Command::SaveStats(_)));
}

#[test]
fn words_mode_tick_does_not_expire_test() {
    let mut model = model_with_words(&["hello"]);
    model.config.test_mode = TestMode::Words;
    update(&mut model, Msg::Char('h')); // → Running
    // Fire a tick well past the default time_limit
    update(&mut model, Msg::Tick(Duration::from_secs(999)));
    assert_eq!(model.session.status, TestStatus::Running);
    assert_eq!(model.screen, Screen::Typing);
}

#[test]
fn words_mode_tick_still_updates_elapsed() {
    let mut model = model_with_words(&["hello"]);
    model.config.test_mode = TestMode::Words;
    update(&mut model, Msg::Char('h'));
    update(&mut model, Msg::Tick(Duration::from_secs(5)));
    assert_eq!(model.session.elapsed, Duration::from_secs(5));
}

#[test]
fn words_mode_stats_payload_uses_elapsed() {
    let mut model = model_with_words(&["hi"]);
    model.config.test_mode = TestMode::Words;
    model.session.elapsed = Duration::from_secs(7);
    update(&mut model, Msg::Char('h'));
    // Manually set elapsed (Tick would update it but we want precise control)
    model.session.elapsed = Duration::from_secs(7);
    let cmd = update(&mut model, Msg::Space); // ends test
    if let Command::SaveStats(payload) = cmd {
        assert_eq!(payload.duration_secs, 7);
    } else {
        panic!("expected SaveStats");
    }
}
```

- [ ] **Step 4.2 — Run tests to confirm they fail**

```bash
cargo nextest run words_mode_last_char_ends_test words_mode_tick_does_not_expire_test words_mode_stats_payload_uses_elapsed 2>&1 | head -20
```

Expected: failures or compilation errors.

- [ ] **Step 4.3 — Implement words-mode Tick guard and fix `build_stats_payload`**

Replace the `Msg::Tick` arm:

```rust
Msg::Tick(elapsed) => {
    if model.session.status != TestStatus::Running {
        return Command::None;
    }
    model.session.elapsed = elapsed;
    // Only expire in time mode; words mode tracks elapsed but has no deadline.
    if matches!(model.config.test_mode, TestMode::Time)
        && elapsed >= model.config.time_limit
    {
        model.session.status = TestStatus::Done;
        model.screen = Screen::Done;
        return Command::SaveStats(build_stats_payload(model));
    }
}
```

Replace `build_stats_payload`:

```rust
fn build_stats_payload(model: &Model) -> StatsPayload {
    let correct_words = metrics::count_correct_words(&model.session.words);
    let committed_words = metrics::count_committed_words(&model.session.words);
    let correct_chars = metrics::count_correct_chars(&model.session.words);
    let total_chars = metrics::count_total_chars_typed(&model.session.words);
    let duration_secs = match model.config.test_mode {
        TestMode::Time => DURATION_OPTIONS[model.config.selected_duration_idx],
        TestMode::Words => model.session.elapsed.as_secs(),
    };
    StatsPayload {
        duration_secs,
        wpm: metrics::wpm(correct_words, model.session.elapsed),
        raw_wpm: metrics::raw_wpm(committed_words, model.session.elapsed),
        accuracy: metrics::accuracy(correct_chars, total_chars),
    }
}
```

- [ ] **Step 4.4 — Run tests**

```bash
cargo nextest run 2>&1 | tail -20
```

Expected: all tests pass.

---

## Task 5 — Integration Tests: Fix + Add Words Flow

**Files:**
- Modify: `src/integration_tests.rs`

- [ ] **Step 5.1 — Fix `full_session_via_word_completion`**

The model uses `Config::default()` (time mode). In time mode, Space on the last word now appends instead of ending. Fix the test by switching to words mode:

Replace `two_word_model()`:

```rust
fn two_word_words_mode_model() -> Model {
    let mut m = Model {
        screen: Screen::Typing,
        session: SessionState::new(vec![Word::new("hi"), Word::new("ok")]),
        config: Config::default(),
        history: Vec::new(),
    };
    m.config.test_mode = crate::model::TestMode::Words;
    m.config.word_count = 2;
    m
}

fn two_word_time_mode_model() -> Model {
    Model {
        screen: Screen::Typing,
        session: SessionState::new(vec![Word::new("hi"), Word::new("ok")]),
        config: Config::default(),
        history: Vec::new(),
    }
}
```

Update `full_session_via_word_completion` to use `two_word_words_mode_model()`:

```rust
#[test]
fn full_session_via_word_completion() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_words_mode_model();

    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);

    update(&mut model, Msg::Char('o'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);

    assert_eq!(model.screen, Screen::Done);
    assert_eq!(model.history.len(), 1);
}
```

Update `timer_expiry_saves_stats` to use `two_word_time_mode_model()`:

```rust
#[test]
fn timer_expiry_saves_stats() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_time_mode_model();
    // ... rest of test body unchanged ...
}
```

Update `tab_from_done_resets_session` — Tab no longer cycles duration, so remove the duration assertion. Also use the words mode model so we can reach Done via word completion:

```rust
#[test]
fn tab_from_done_resets_session() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_words_mode_model();

    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);
    update(&mut model, Msg::Char('o'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);
    assert_eq!(model.screen, Screen::Done);

    let cmd = update(&mut model, Msg::Tab);
    assert!(matches!(cmd, Command::GenerateWords { .. }));
    execute_command(&mut model, cmd, &mut rng);

    assert_eq!(model.screen, Screen::Typing);
    assert_eq!(model.session.status, TestStatus::Waiting);
    // Tab is restart-only: duration index must not change
    assert_eq!(model.config.selected_duration_idx, 0);
}
```

Also add the import at the top:
```rust
use crate::model::{Config, Model, Screen, SessionState, TestStatus, Word};
```

- [ ] **Step 5.2 — Add words mode session integration test**

```rust
#[test]
fn words_mode_full_session() {
    use crate::model::TestMode;
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_words_mode_model();

    // Waiting → Running
    assert_eq!(model.session.status, TestStatus::Waiting);
    update(&mut model, Msg::Char('h'));
    assert_eq!(model.session.status, TestStatus::Running);

    // Advance past first word
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);
    assert_eq!(model.session.current_word, 1);

    // Tick in words mode must NOT expire the test
    update(&mut model, Msg::Tick(Duration::from_secs(999)));
    assert_eq!(model.session.status, TestStatus::Running);

    // Last char of last word ends test immediately, no Space needed
    update(&mut model, Msg::Char('o'));
    let cmd = update(&mut model, Msg::Char('k'));
    assert_eq!(model.session.status, TestStatus::Done);
    assert_eq!(model.screen, Screen::Done);
    assert!(matches!(cmd, Command::SaveStats(_)));
    execute_command(&mut model, cmd, &mut rng);
    assert_eq!(model.history.len(), 1);
    assert_eq!(model.config.test_mode, TestMode::Words);
}
```

Also add `use std::time::Duration;` to `integration_tests.rs` if not already present.

- [ ] **Step 5.3 — Add endless time mode integration test**

```rust
#[test]
fn time_mode_words_never_run_out() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_time_mode_model();
    let initial_len = model.session.words.len();

    // Commit the first word
    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);
    // Pool grew by 1
    assert_eq!(model.session.words.len(), initial_len + 1);
    assert_eq!(model.session.current_word, 1);

    // Commit the second word (now the last)
    update(&mut model, Msg::Char('o'));
    let cmd = update(&mut model, Msg::Char('k'));
    execute_command(&mut model, cmd, &mut rng);
    // Pool grew again; test is still running
    assert_eq!(model.session.words.len(), initial_len + 2);
    assert_eq!(model.session.status, TestStatus::Running);
    assert_eq!(model.screen, Screen::Typing);
}
```

- [ ] **Step 5.4 — Run all tests**

```bash
cargo nextest run 2>&1 | tail -20
```

Expected: all tests pass.

---

## Task 6 — View: Mode Selector, Options Strip, X/Y Counter

**Files:**
- Modify: `src/view.rs`

- [ ] **Step 6.1 — Update imports**

At the top of `view.rs`, update the model import:

```rust
use crate::model::{CursorStyle, DURATION_OPTIONS, WORD_COUNT_OPTIONS, Model, Screen, TestMode, TestStatus};
```

- [ ] **Step 6.2 — Replace `duration_strip_spans` with `options_strip_spans` and two wrappers**

Delete the existing `duration_strip_spans` function and replace with:

```rust
fn options_strip_spans(labels: Vec<String>, selected_idx: usize, dimmed: bool) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, label) in labels.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let display = if i == selected_idx {
            format!("[{}]", label)
        } else {
            label
        };
        let style = if i == selected_idx && !dimmed {
            Style::new().add_modifier(Modifier::BOLD)
        } else {
            Style::new().dim()
        };
        spans.push(Span::styled(display, style));
    }
    spans
}

fn duration_strip_spans(selected_idx: usize, dimmed: bool) -> Vec<Span<'static>> {
    let labels = DURATION_OPTIONS.iter().map(|s| s.to_string()).collect();
    options_strip_spans(labels, selected_idx, dimmed)
}

fn word_count_strip_spans(selected_idx: usize, dimmed: bool) -> Vec<Span<'static>> {
    let labels = WORD_COUNT_OPTIONS.iter().map(|s| s.to_string()).collect();
    options_strip_spans(labels, selected_idx, dimmed)
}

fn mode_selector_spans(mode: &TestMode, is_running: bool) -> Vec<Span<'static>> {
    let selected_style = if is_running {
        Style::new().dim()
    } else {
        Style::new().add_modifier(Modifier::BOLD)
    };
    let unselected_style = Style::new().dim();
    match mode {
        TestMode::Time => vec![
            Span::styled("[time]", selected_style),
            Span::raw(" "),
            Span::styled("words", unselected_style),
        ],
        TestMode::Words => vec![
            Span::styled("time", unselected_style),
            Span::raw(" "),
            Span::styled("[words]", selected_style),
        ],
    }
}
```

- [ ] **Step 6.3 — Rewrite `render_typing` header**

In `render_typing`, replace the entire header-building block (from `let is_running = ...` to `frame.render_widget(header, header_area);`) with:

```rust
let is_running = model.session.status == TestStatus::Running;
let mut header_spans: Vec<Span> = vec![
    Span::styled("kern", Style::new().add_modifier(Modifier::BOLD)),
    Span::raw("  "),
];

// Mode selector
header_spans.extend(mode_selector_spans(&model.config.test_mode, is_running));
header_spans.push(Span::raw("   "));

// Options strip
match model.config.test_mode {
    TestMode::Time => header_spans.extend(duration_strip_spans(
        model.config.selected_duration_idx,
        is_running,
    )),
    TestMode::Words => header_spans.extend(word_count_strip_spans(
        model.config.selected_word_count_idx,
        is_running,
    )),
}

// Context info (countdown or word counter)
if is_running {
    match model.config.test_mode {
        TestMode::Time => {
            let countdown =
                model.config.time_limit.saturating_sub(model.session.elapsed);
            header_spans.push(Span::raw("  ·  "));
            header_spans.push(Span::styled(
                format!("{}s", countdown.as_secs()),
                Style::new().dim(),
            ));
        }
        TestMode::Words => {
            let total = model.session.words.len();
            let current = (model.session.current_word + 1).min(total);
            header_spans.push(Span::raw("   "));
            header_spans.push(Span::styled(
                format!("{}/{}", current, total),
                Style::new().add_modifier(Modifier::BOLD),
            ));
        }
    }
}

// Key hints
header_spans.push(Span::raw("   "));
if !is_running {
    header_spans.push(Span::styled("[←→] cycle", Style::new().dim()));
    header_spans.push(Span::raw("  "));
}
header_spans.push(Span::styled("[tab] restart", Style::new().dim()));
if !is_running {
    header_spans.push(Span::raw("  "));
    let mode_hint = match model.config.test_mode {
        TestMode::Time => "[shift+tab] → word mode",
        TestMode::Words => "[shift+tab] → time mode",
    };
    header_spans.push(Span::styled(mode_hint, Style::new().dim()));
}

let header = Paragraph::new(Line::from(header_spans));
frame.render_widget(header, header_area);
```

- [ ] **Step 6.4 — Update `render_results` to use the new helpers**

In `render_results`, the `duration_strip_spans` call already works since we kept the wrapper. But also add the mode selector for consistency. Replace the `frame.render_widget(Paragraph::new(Line::from(duration_strip_spans(...` block with:

```rust
let mut result_header: Vec<Span> = Vec::new();
result_header.extend(mode_selector_spans(&model.config.test_mode, false));
result_header.push(Span::raw("   "));
match model.config.test_mode {
    TestMode::Time => result_header.extend(duration_strip_spans(
        model.config.selected_duration_idx,
        false,
    )),
    TestMode::Words => result_header.extend(word_count_strip_spans(
        model.config.selected_word_count_idx,
        false,
    )),
}
frame.render_widget(
    Paragraph::new(Line::from(result_header)).alignment(Alignment::Center),
    vertical[1],
);
```

- [ ] **Step 6.5 — Build and run tests (snapshots will fail)**

```bash
cargo nextest run 2>&1 | tail -30
```

Expected: snapshot tests fail with "snapshot changed" errors. Non-snapshot tests should pass.

- [ ] **Step 6.6 — Accept updated snapshots**

```bash
cargo insta review
```

For each snapshot, press `a` to accept. There are 8 snapshots to update. All of them now include the mode selector in the header.

Alternatively, to accept all at once:

```bash
cargo insta accept
```

- [ ] **Step 6.7 — Run full test suite**

```bash
cargo nextest run 2>&1 | tail -10
```

Expected: all tests pass including snapshot tests.

- [ ] **Step 6.8 — Run clippy and fmt**

```bash
cargo clippy -- -D warnings 2>&1 | head -30
cargo fmt --check 2>&1
```

Fix any warnings before committing. Common ones: unused import if `DURATION_OPTIONS` no longer referenced from view tests. Check view test helpers use the new Config fields.

---

## Task 7 — View Test Helpers: Update for New Config

**Files:**
- Modify: `src/view.rs` (tests module only)

The `test_model` and `typing_screen_duration_variants_snapshot` helper in the `tests` module need minor updates since `Config` changed.

- [ ] **Step 7.1 — Update view test helpers**

In the `tests` module of `view.rs`, the `test_model` helper uses `Config::default()` (unchanged — still works). The `typing_screen_duration_variants_snapshot` test uses `DURATION_OPTIONS` which still exists. Verify the test compiles:

```bash
cargo test --doc 2>&1; cargo nextest run view 2>&1 | tail -20
```

Expected: passes (snapshots already updated in Task 6).

- [ ] **Step 7.2 — Add view tests for words-mode header**

Add to the view `tests` module:

```rust
#[test]
fn words_mode_waiting_snapshot() {
    let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
    model.session.status = crate::model::TestStatus::Waiting;
    model.config.test_mode = crate::model::TestMode::Words;
    model.config.selected_word_count_idx = 1; // 25
    model.config.word_count = crate::model::WORD_COUNT_OPTIONS[1];
    let output = render_to_string(&model, 80, 24);
    insta::assert_snapshot!("words_mode_waiting", output);
}

#[test]
fn words_mode_running_snapshot() {
    let model = {
        let mut m = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
        m.config.test_mode = crate::model::TestMode::Words;
        m.config.selected_word_count_idx = 1;
        m.config.word_count = crate::model::WORD_COUNT_OPTIONS[1];
        m
    };
    let output = render_to_string(&model, 80, 24);
    insta::assert_snapshot!("words_mode_running", output);
}
```

- [ ] **Step 7.3 — Run tests and accept new snapshots**

```bash
cargo nextest run words_mode_waiting_snapshot words_mode_running_snapshot 2>&1
cargo insta accept
cargo nextest run 2>&1 | tail -10
```

Expected: all pass.

---

## Task 8 — Commit Grouping

The 4 logical commits that cover this feature. Each should be a checkpoint the user reviews before moving on.

- [ ] **Commit 1: Data foundation + input + commands**

```bash
git add src/model.rs src/msg.rs src/input.rs src/commands.rs src/main.rs
git commit -m "feat: add TestMode, word-count config, AppendWords command, and ShiftTab/arrow messages"
```

- [ ] **Commit 2: Update logic**

```bash
git add src/update.rs
git commit -m "feat(update): mode-gated handlers, endless time mode via AppendWords, word-count mode lifecycle"
```

- [ ] **Commit 3: View**

```bash
git add src/view.rs src/snapshots/
git commit -m "feat(view): mode selector, generalized options strip, X/Y word counter, updated header layouts"
```

- [ ] **Commit 4: Integration tests**

```bash
git add src/integration_tests.rs
git commit -m "test: fix time-mode session tests, add words-mode and endless-time integration tests"
```

---

## Self-Review Checklist (Engineer)

Before each commit, verify:

1. `cargo nextest run` — all pass
2. `cargo clippy -- -D warnings` — zero warnings
3. `cargo fmt --check` — no formatting violations
4. Snapshot diffs look correct (mode selector appears in header, X/Y counter visible in words-mode running)
