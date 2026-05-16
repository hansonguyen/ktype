# Phase 3: Timer + Results Screen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a 15-second countdown timer (starts on first keypress, always-on time mode) and a results screen showing WPM, raw WPM, and accuracy.

**Architecture:** Time flows into `update` via `Msg::Tick(Duration)` — `main.rs` holds `timer_start: Option<Instant>` as infrastructure, computes elapsed each frame, and fires `Msg::Tick(elapsed)` every tick. `update` stays 100% pure. A new `src/metrics.rs` module provides pure functions over `&[Word]` and `Duration`; `view` calls them on demand when rendering the results screen.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, insta (snapshots), proptest (property tests), cargo-nextest

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/metrics.rs` | Create | 6 pure metric functions + unit tests |
| `src/model.rs` | Modify | Add `elapsed: Duration` to `SessionState`; `time_limit: Duration` to `Config` |
| `src/msg.rs` | Modify | `Tick` → `Tick(Duration)` |
| `src/update.rs` | Modify | Handle `Tick(Duration)` with timer expiry; update proptest |
| `src/main.rs` | Modify | Add `timer_start: Option<Instant>`; rework loop to fire `Msg::Tick(elapsed)` |
| `src/view.rs` | Modify | Countdown in header; `render_done` → `render_results` with metrics |

---

## Task 1: Create `src/metrics.rs`

**Files:**
- Create: `src/metrics.rs`
- Modify: `src/main.rs` (add `mod metrics;`)

- [ ] **Step 1: Write `src/metrics.rs` with tests first, stubs second**

Create `src/metrics.rs` with this full content (tests + implementations together — all functions are pure so TDD collapses to write-and-verify):

```rust
use std::time::Duration;

use crate::model::Word;

pub fn wpm(correct_words: usize, elapsed: Duration) -> f64 {
    if elapsed < Duration::from_millis(1) {
        return 0.0;
    }
    correct_words as f64 / elapsed.as_secs_f64() * 60.0
}

pub fn raw_wpm(committed_words: usize, elapsed: Duration) -> f64 {
    if elapsed < Duration::from_millis(1) {
        return 0.0;
    }
    committed_words as f64 / elapsed.as_secs_f64() * 60.0
}

pub fn accuracy(correct_chars: u64, total_chars_typed: u64) -> f64 {
    if total_chars_typed == 0 {
        return 0.0;
    }
    correct_chars as f64 / total_chars_typed as f64 * 100.0
}

pub fn count_correct_words(words: &[Word]) -> usize {
    words
        .iter()
        .filter(|w| w.committed && w.typed == w.chars.iter().collect::<String>())
        .count()
}

pub fn count_committed_words(words: &[Word]) -> usize {
    words.iter().filter(|w| w.committed).count()
}

pub fn count_correct_chars(words: &[Word]) -> u64 {
    words
        .iter()
        .map(|w| {
            w.typed
                .chars()
                .enumerate()
                .filter(|(i, c)| w.chars.get(*i) == Some(c))
                .count() as u64
        })
        .sum()
}

pub fn count_total_chars_typed(words: &[Word]) -> u64 {
    words.iter().map(|w| w.typed.len() as u64).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Word;

    fn word(text: &str, typed: &str, committed: bool) -> Word {
        let mut w = Word::new(text);
        w.typed = typed.to_string();
        w.committed = committed;
        w
    }

    #[test]
    fn wpm_zero_elapsed_returns_zero() {
        assert_eq!(wpm(10, Duration::ZERO), 0.0);
    }

    #[test]
    fn wpm_correct_calculation() {
        // 30 words in 60 seconds = 30 wpm
        assert!((wpm(30, Duration::from_secs(60)) - 30.0).abs() < 0.01);
    }

    #[test]
    fn wpm_fractional_minutes() {
        // 10 words in 30 seconds = 20 wpm
        assert!((wpm(10, Duration::from_secs(30)) - 20.0).abs() < 0.01);
    }

    #[test]
    fn raw_wpm_zero_elapsed_returns_zero() {
        assert_eq!(raw_wpm(10, Duration::ZERO), 0.0);
    }

    #[test]
    fn raw_wpm_correct_calculation() {
        assert!((raw_wpm(35, Duration::from_secs(60)) - 35.0).abs() < 0.01);
    }

    #[test]
    fn accuracy_zero_total_returns_zero() {
        assert_eq!(accuracy(0, 0), 0.0);
    }

    #[test]
    fn accuracy_all_correct() {
        assert!((accuracy(100, 100) - 100.0).abs() < 0.01);
    }

    #[test]
    fn accuracy_partial_correct() {
        assert!((accuracy(90, 100) - 90.0).abs() < 0.01);
    }

    #[test]
    fn count_correct_words_committed_exact_match() {
        let words = vec![
            word("hello", "hello", true),  // correct
            word("world", "world", true),  // correct
        ];
        assert_eq!(count_correct_words(&words), 2);
    }

    #[test]
    fn count_correct_words_committed_with_mistake_excluded() {
        let words = vec![
            word("hello", "hellx", true),  // committed but wrong
            word("world", "world", true),  // correct
        ];
        assert_eq!(count_correct_words(&words), 1);
    }

    #[test]
    fn count_correct_words_uncommitted_excluded() {
        let words = vec![
            word("hello", "hello", false), // not committed
            word("world", "world", true),
        ];
        assert_eq!(count_correct_words(&words), 1);
    }

    #[test]
    fn count_correct_words_partial_typing_excluded() {
        let words = vec![
            word("hello", "hel", true), // committed but incomplete
        ];
        assert_eq!(count_correct_words(&words), 0);
    }

    #[test]
    fn count_committed_words_counts_committed_only() {
        let words = vec![
            word("a", "a", true),
            word("b", "x", true),  // wrong but still committed
            word("c", "c", false), // not committed
        ];
        assert_eq!(count_committed_words(&words), 2);
    }

    #[test]
    fn count_correct_chars_matches_correct_positions() {
        let words = vec![
            word("hello", "hxllo", false), // 4 correct ('h','l','l','o'), 1 wrong ('e'→'x')
        ];
        assert_eq!(count_correct_chars(&words), 4);
    }

    #[test]
    fn count_correct_chars_sums_across_words() {
        let words = vec![
            word("hi", "hi", true),  // 2 correct
            word("ok", "ox", false), // 1 correct
        ];
        assert_eq!(count_correct_chars(&words), 3);
    }

    #[test]
    fn count_total_chars_typed_sums_all_typed() {
        let words = vec![
            word("hello", "hel", true),
            word("world", "wo", false),
        ];
        assert_eq!(count_total_chars_typed(&words), 5);
    }
}
```

- [ ] **Step 2: Declare `mod metrics;` in `src/main.rs`**

In `src/main.rs`, add `mod metrics;` alongside the other module declarations:

```rust
mod commands;
mod generator;
mod input;
mod metrics;
mod model;
mod msg;
mod update;
mod view;
```

- [ ] **Step 3: Run tests**

```bash
cargo nextest run
```

Expected: all 31 existing tests pass + 17 new metrics tests pass (48 total).

- [ ] **Step 4: Commit**

```bash
git add src/metrics.rs src/main.rs
git commit -m "feat: add metrics module with wpm, raw_wpm, accuracy calculations"
```

---

## Task 2: Extend `src/model.rs` with `elapsed` and `time_limit`

**Files:**
- Modify: `src/model.rs`
- Modify: `src/view.rs` (fix test helper that constructs `SessionState` directly)

- [ ] **Step 1: Add `use std::time::Duration;` and `elapsed` field to `model.rs`**

Replace the top of `src/model.rs` with the full updated file:

```rust
use std::time::Duration;

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
    pub word_count: usize,
    pub cursor_style: CursorStyle,
    pub time_limit: Duration,
    #[expect(dead_code)]
    pub punctuation: bool,
    #[expect(dead_code)]
    pub numbers: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            word_count: 25,
            cursor_style: CursorStyle::Block,
            time_limit: Duration::from_secs(15),
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
}

impl Default for Model {
    fn default() -> Self {
        Model {
            screen: Screen::Typing,
            session: SessionState::new(Vec::new()),
            config: Config::default(),
        }
    }
}
```

- [ ] **Step 2: Fix `test_model` in `src/view.rs` to include `elapsed`**

In `src/view.rs`, find the `test_model` helper (around line 198) and update the `SessionState` struct literal to add `elapsed`:

```rust
fn test_model(words: &[&str], current_word: usize, typed: &[&str]) -> Model {
    let mut session_words: Vec<Word> = words.iter().map(|w| Word::new(w)).collect();
    for (i, t) in typed.iter().enumerate() {
        if let Some(w) = session_words.get_mut(i) {
            w.typed = t.to_string();
        }
    }
    Model {
        screen: Screen::Typing,
        session: SessionState {
            words: session_words,
            current_word,
            status: crate::model::TestStatus::Running,
            elapsed: std::time::Duration::ZERO,
        },
        config: Config::default(),
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo nextest run
```

Expected: all 48 tests pass. The new fields don't break any existing logic.

- [ ] **Step 4: Commit**

```bash
git add src/model.rs src/view.rs
git commit -m "feat: add elapsed and time_limit fields to model"
```

---

## Task 3: Wire `Msg::Tick(Duration)` and timer expiry in `update.rs`

**Files:**
- Modify: `src/msg.rs`
- Modify: `src/update.rs`

This task changes the `Tick` variant signature in `msg.rs` and implements timer expiry logic in `update.rs`. Both files must change together because `Msg::Tick(Duration)` breaks compilation of the existing `Msg::Tick` match arm.

- [ ] **Step 1: Rewrite `src/msg.rs`**

Replace `src/msg.rs` with:

```rust
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Tick(Duration), // elapsed since test start; fired every frame by main.rs
    Char(char),
    Backspace,
    Space,
    Tab,
    Esc,
}
```

- [ ] **Step 2: Update the `Tick` handler and add imports in `src/update.rs`**

Add `use std::time::Duration;` to the imports at the top of `src/update.rs`:

```rust
use std::time::Duration;

use crate::commands::Command;
use crate::model::{Model, Screen, TestStatus};
use crate::msg::Msg;
```

Replace the existing `Msg::Tick` match arm (currently a no-op) with:

```rust
Msg::Tick(elapsed) => {
    if model.session.status != TestStatus::Running {
        return Command::None;
    }
    model.session.elapsed = elapsed;
    if elapsed >= model.config.time_limit {
        model.session.status = TestStatus::Done;
        model.screen = Screen::Done;
    }
}
```

- [ ] **Step 3: Add timer tests to the `tests` module in `src/update.rs`**

Inside the existing `#[cfg(test)] mod tests { ... }` block, add these tests after the existing ones:

```rust
#[test]
fn tick_before_running_is_noop() {
    let mut model = model_with_words(&["hello"]);
    assert_eq!(model.session.status, TestStatus::Waiting);
    update(&mut model, Msg::Tick(Duration::from_secs(5)));
    assert_eq!(model.session.status, TestStatus::Waiting);
    assert_eq!(model.screen, Screen::Typing);
}

#[test]
fn tick_updates_elapsed_when_running() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Char('h'));
    update(&mut model, Msg::Tick(Duration::from_secs(5)));
    assert_eq!(model.session.elapsed, Duration::from_secs(5));
}

#[test]
fn tick_at_time_limit_transitions_to_done() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Char('h'));
    update(&mut model, Msg::Tick(Duration::from_secs(15)));
    assert_eq!(model.session.status, TestStatus::Done);
    assert_eq!(model.screen, Screen::Done);
}

#[test]
fn tick_past_time_limit_transitions_to_done() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Char('h'));
    update(&mut model, Msg::Tick(Duration::from_secs(16)));
    assert_eq!(model.session.status, TestStatus::Done);
    assert_eq!(model.screen, Screen::Done);
}

#[test]
fn tick_after_done_is_noop() {
    let mut model = model_with_words(&["hi"]);
    update(&mut model, Msg::Char('h'));
    update(&mut model, Msg::Space);
    assert_eq!(model.screen, Screen::Done);
    let elapsed_before = model.session.elapsed;
    update(&mut model, Msg::Tick(Duration::from_secs(100)));
    assert_eq!(model.session.elapsed, elapsed_before);
    assert_eq!(model.screen, Screen::Done);
}
```

Also add `use std::time::Duration;` inside the `mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;
    use crate::model::{Config, SessionState, Word};
    // ... existing tests unchanged ...
}
```

- [ ] **Step 4: Update `arb_msg` in the proptest module**

In `src/update.rs`, find `mod prop_tests` and update `arb_msg` to use `Msg::Tick(Duration::ZERO)` and add the Duration import:

```rust
#[cfg(test)]
mod prop_tests {
    use std::time::Duration;
    use super::*;
    use crate::model::{Config, SessionState, Word};
    use proptest::prelude::*;

    fn arb_msg() -> impl Strategy<Value = Msg> {
        prop_oneof![
            Just(Msg::Char('a')),
            Just(Msg::Char('z')),
            Just(Msg::Backspace),
            Just(Msg::Space),
            Just(Msg::Tick(Duration::ZERO)), // zero elapsed won't expire the 15s timer
        ]
    }

    fn model_with_words(words: &[&str]) -> Model {
        Model {
            screen: Screen::Typing,
            session: SessionState::new(words.iter().map(|w| Word::new(w)).collect()),
            config: Config::default(),
        }
    }

    proptest! {
        #[test]
        fn current_word_stays_in_bounds(actions in prop::collection::vec(arb_msg(), 0..100)) {
            let mut model = model_with_words(&["hello", "world", "test", "kern", "rust"]);
            for msg in actions {
                update(&mut model, msg);
                prop_assert!(model.session.current_word < model.session.words.len());
            }
        }

        #[test]
        fn typed_len_never_exceeds_word_len(actions in prop::collection::vec(arb_msg(), 0..100)) {
            let mut model = model_with_words(&["hi", "ok", "go", "be", "do"]);
            for msg in actions {
                update(&mut model, msg);
                for word in &model.session.words {
                    prop_assert!(word.typed.len() <= word.chars.len());
                }
            }
        }
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo nextest run
```

Expected: all tests pass, including the 5 new timer tests (53 total).

- [ ] **Step 6: Commit**

```bash
git add src/msg.rs src/update.rs
git commit -m "feat: wire Msg::Tick(Duration) with timer expiry in update"
```

---

## Task 4: Rework the event loop in `src/main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Rewrite `src/main.rs`**

Replace `src/main.rs` with:

```rust
mod commands;
mod generator;
mod input;
mod metrics;
mod model;
mod msg;
mod update;
mod view;

use std::time::{Duration, Instant};

use anyhow::Result;
use commands::{Command, execute_command};
use model::{Model, TestStatus};
use msg::Msg;
use rand::rngs::SmallRng;
use update::update;
use view::view;

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut rng: SmallRng = rand::make_rng();
    let mut model = Model::default();
    // timer_start is infrastructure — not app state. Owned here alongside rng.
    let mut timer_start: Option<Instant> = None;

    let word_count = model.config.word_count;
    execute_command(
        &mut model,
        Command::GenerateWords { count: word_count },
        &mut rng,
    );

    loop {
        terminal.draw(|frame| view(&model, frame))?;

        // Process one pending input event (16ms timeout = ~60fps frame budget).
        if crossterm::event::poll(Duration::from_millis(16))?
            && let Some(msg) = input::event_to_msg(crossterm::event::read()?)
        {
            let cmd = update(&mut model, msg);
            execute_command(&mut model, cmd, &mut rng);
        }

        // Start timer on Waiting → Running transition.
        if timer_start.is_none() && model.session.status == TestStatus::Running {
            timer_start = Some(Instant::now());
        }
        // Clear timer when session resets to Waiting (Tab restart).
        if timer_start.is_some() && model.session.status == TestStatus::Waiting {
            timer_start = None;
        }

        // Drive countdown — fire Tick every frame with current elapsed.
        let elapsed = timer_start.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
        let cmd = update(&mut model, Msg::Tick(elapsed));
        execute_command(&mut model, cmd, &mut rng);

        if model.screen == model::Screen::Quitting {
            break;
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Build and smoke test**

```bash
cargo build
```

Expected: compiles with no errors.

```bash
cargo nextest run
```

Expected: all 53 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add timer_start infrastructure and Tick dispatch to event loop"
```

---

## Task 5: Update `src/view.rs` — countdown header and results screen

**Files:**
- Modify: `src/view.rs`

- [ ] **Step 1: Rewrite `src/view.rs`**

Replace the full content of `src/view.rs` with:

```rust
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::input::{CharState, char_state};
use crate::model::{CursorStyle, Model, Screen, TestStatus};
use crate::metrics;

pub fn view(model: &Model, frame: &mut Frame) {
    match model.screen {
        Screen::Done => render_results(model, frame),
        Screen::Typing => render_typing(model, frame),
        Screen::Quitting => {}
    }
}

fn render_results(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let correct_words = metrics::count_correct_words(&model.session.words);
    let committed_words = metrics::count_committed_words(&model.session.words);
    let correct_chars = metrics::count_correct_chars(&model.session.words);
    let total_chars = metrics::count_total_chars_typed(&model.session.words);
    let elapsed = model.session.elapsed;

    let wpm_val = metrics::wpm(correct_words, elapsed);
    let raw_val = metrics::raw_wpm(committed_words, elapsed);
    let acc_val = metrics::accuracy(correct_chars, total_chars);

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // "kern"
        Constraint::Length(1), // spacer
        Constraint::Length(1), // metric labels
        Constraint::Length(1), // metric values
        Constraint::Length(1), // spacer
        Constraint::Length(1), // footer
        Constraint::Fill(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled("kern", Style::new().add_modifier(Modifier::BOLD)))
            .alignment(Alignment::Center),
        vertical[1],
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            "wpm         raw wpm         acc",
            Style::new().dim(),
        ))
        .alignment(Alignment::Center),
        vertical[3],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(format!("{:.0}", wpm_val)),
            Span::raw("           "),
            Span::raw(format!("{:.0}", raw_val)),
            Span::raw("           "),
            Span::raw(format!("{:.0}%", acc_val)),
        ]))
        .alignment(Alignment::Center),
        vertical[4],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[tab] restart", Style::new().dim()),
            Span::raw("   "),
            Span::styled("[esc] quit", Style::new().dim()),
        ]))
        .alignment(Alignment::Center),
        vertical[6],
    );
}

fn render_typing(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(80),
        Constraint::Fill(1),
    ])
    .split(area);
    let content = horizontal[1];

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // header
        Constraint::Length(1), // spacer
        Constraint::Length(3), // word block
        Constraint::Length(1), // spacer
        Constraint::Length(1), // footer
        Constraint::Fill(1),
    ])
    .split(content);

    let header_area = vertical[1];
    let words_area = vertical[3];
    let footer_area = vertical[5];

    // Header: show countdown timer. Static when Waiting, live when Running.
    let countdown = model
        .config
        .time_limit
        .saturating_sub(model.session.elapsed);
    let time_text = match model.session.status {
        TestStatus::Waiting | TestStatus::Running => format!("{}s", countdown.as_secs()),
        TestStatus::Done => String::from("done"),
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled("kern", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(time_text, Style::new().dim()),
        Span::raw("  "),
        Span::styled("[tab] restart", Style::new().dim()),
    ]));
    frame.render_widget(header, header_area);

    let word_lines = build_word_lines(model, words_area.width);
    let words_widget = Paragraph::new(word_lines);
    frame.render_widget(words_widget, words_area);

    let footer = Paragraph::new(Span::styled("[esc] quit", Style::new().dim()));
    frame.render_widget(footer, footer_area);
}

fn word_line_indices(words: &[crate::model::Word], width: u16) -> Vec<usize> {
    let max_width = width as usize;
    let mut line_for_word = vec![0usize; words.len()];
    let mut current_line = 0usize;
    let mut line_width = 0usize;

    for (i, word) in words.iter().enumerate() {
        let word_len = word.chars.len().min(max_width.max(1));
        let needed = if line_width == 0 {
            word_len
        } else {
            1 + word_len
        };

        if line_width > 0 && line_width + 1 + word_len > max_width {
            current_line += 1;
            line_width = word_len;
        } else {
            line_width += needed;
        }
        line_for_word[i] = current_line;
    }
    line_for_word
}

fn build_word_lines<'a>(model: &Model, width: u16) -> Vec<Line<'a>> {
    let words = &model.session.words;
    if words.is_empty() {
        return vec![Line::default(); 3];
    }

    let line_indices = word_line_indices(words, width);
    let current_word = model.session.current_word.min(words.len() - 1);
    let current_line = line_indices[current_word];
    let scroll = current_line.saturating_sub(2);

    let total_lines = line_indices.last().copied().unwrap_or(0) + 1;
    let mut all_lines: Vec<Vec<Span<'a>>> = vec![Vec::new(); total_lines];

    for (word_idx, (word, &line_idx)) in words.iter().zip(line_indices.iter()).enumerate() {
        let spans = &mut all_lines[line_idx];

        if !spans.is_empty() {
            spans.push(Span::styled(" ", Style::new().dim()));
        }

        for (char_idx, &ch) in word.chars.iter().enumerate() {
            let is_cursor =
                word_idx == current_word && char_idx == word.typed.len() && !word.committed;

            let style = if is_cursor {
                cursor_style(&model.config.cursor_style)
            } else {
                match char_state(word, char_idx) {
                    CharState::Correct => Style::new(),
                    CharState::Incorrect => Style::new().fg(Color::Red),
                    CharState::Untyped => Style::new().dim(),
                }
            };

            spans.push(Span::styled(ch.to_string(), style));
        }
    }

    let mut visible: Vec<Line<'a>> = all_lines
        .into_iter()
        .skip(scroll)
        .take(3)
        .map(Line::from)
        .collect();
    while visible.len() < 3 {
        visible.push(Line::default());
    }
    visible
}

fn cursor_style(style: &CursorStyle) -> Style {
    match style {
        CursorStyle::Block => Style::new().add_modifier(Modifier::REVERSED),
        CursorStyle::Underline => Style::new().add_modifier(Modifier::UNDERLINED),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;
    use crate::model::{Config, Model, Screen, SessionState, Word};

    fn test_model(words: &[&str], current_word: usize, typed: &[&str]) -> Model {
        let mut session_words: Vec<Word> = words.iter().map(|w| Word::new(w)).collect();
        for (i, t) in typed.iter().enumerate() {
            if let Some(w) = session_words.get_mut(i) {
                w.typed = t.to_string();
            }
        }
        Model {
            screen: Screen::Typing,
            session: SessionState {
                words: session_words,
                current_word,
                status: crate::model::TestStatus::Running,
                elapsed: Duration::ZERO,
            },
            config: Config::default(),
        }
    }

    fn render_to_string(model: &Model, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| view(model, frame)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let area = buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn typing_screen_renders_without_panic() {
        let model = test_model(&["hello", "world"], 0, &["hel"]);
        render_to_string(&model, 80, 24);
    }

    #[test]
    fn done_screen_renders_without_panic() {
        let mut model = test_model(&["hi"], 0, &["hi"]);
        model.screen = Screen::Done;
        render_to_string(&model, 80, 24);
    }

    #[test]
    fn typing_screen_snapshot() {
        let model = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn results_screen_snapshot() {
        let words = vec![
            {
                let mut w = Word::new("the");
                w.typed = "the".to_string();
                w.committed = true;
                w
            },
            {
                let mut w = Word::new("quick");
                w.typed = "quikc".to_string(); // wrong — not counted in wpm
                w.committed = true;
                w
            },
            {
                let mut w = Word::new("brown");
                w.typed = "brown".to_string();
                w.committed = true;
                w
            },
        ];
        let model = Model {
            screen: Screen::Done,
            session: SessionState {
                words,
                current_word: 2,
                status: crate::model::TestStatus::Done,
                elapsed: Duration::from_secs(10),
            },
            config: Config::default(),
        };
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }
}
```

- [ ] **Step 2: Update the existing typing screen snapshot**

The header now shows `"15s"` instead of `"words: 25"`. The old snapshot will no longer match. Accept the new snapshot:

```bash
INSTA_UPDATE=always cargo nextest run view::tests::typing_screen_snapshot
```

Expected: snapshot file updated with the new header.

- [ ] **Step 3: Run all tests**

```bash
cargo nextest run
```

Expected: all tests pass. Two snapshots accepted (typing screen updated, results screen new).

- [ ] **Step 4: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/view.rs src/snapshots/
git commit -m "feat: add countdown header and results screen with wpm/raw/accuracy"
```

> Note: insta snapshot files are stored in `src/snapshots/`. The typing screen snap will be updated; the results screen snap will be new. Both go in this commit.

---

## Verification

After all tasks complete, run the full suite:

```bash
cargo nextest run
cargo clippy -- -D warnings
cargo fmt
cargo fmt --check
```

Then do a manual smoke test:

```bash
cargo run
```

- [ ] Confirm header shows `15s` on launch
- [ ] Confirm countdown starts and ticks down on first keypress
- [ ] Confirm timer expiry transitions to results screen with non-zero WPM
- [ ] Confirm Tab on results restarts with fresh `15s` header
- [ ] Confirm typing all words before timer also shows results screen
- [ ] Confirm `[esc]` quits from both typing and results screens
