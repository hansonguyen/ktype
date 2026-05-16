# Phase 4: Time Duration Selector — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a 15/30/60s duration selector to the typing header and results screen, cycled via Tab before or after a test (not during).

**Architecture:** `DURATION_OPTIONS: [u64; 3]` in `model.rs` is the single source of truth; `Config.selected_duration_idx` indexes into it and drives both `time_limit` (for timer expiry in `update.rs`) and the visual tab strip (in `view.rs`). Tab while Waiting or Done cycles the index and resets the session; Tab while Running restarts without cycling.

**Tech Stack:** Rust, ratatui (TUI), crossterm, cargo nextest, insta (snapshot tests)

---

### Task 1: Add `DURATION_OPTIONS` and `selected_duration_idx` to `Config`

**Files:**
- Modify: `src/model.rs`

- [ ] **Step 1: Add the constant and field**

In `src/model.rs`, add this constant immediately before `pub enum Screen`:

```rust
pub const DURATION_OPTIONS: [u64; 3] = [15, 30, 60];
```

Add `selected_duration_idx: usize` to the `Config` struct after `time_limit`:

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub word_count: usize,
    pub cursor_style: CursorStyle,
    pub time_limit: Duration,
    pub selected_duration_idx: usize,
    #[expect(dead_code)]
    pub punctuation: bool,
    #[expect(dead_code)]
    pub numbers: bool,
}
```

Add `selected_duration_idx: 0,` to `Config::default()` after `time_limit`:

```rust
impl Default for Config {
    fn default() -> Self {
        Config {
            word_count: 25,
            cursor_style: CursorStyle::Block,
            time_limit: Duration::from_secs(15),
            selected_duration_idx: 0,
            punctuation: false,
            numbers: false,
        }
    }
}
```

- [ ] **Step 2: Verify build passes**

```bash
cargo build 2>&1
```

Expected: `Finished` with no errors or warnings.

- [ ] **Step 3: Commit**

```bash
git add src/model.rs
git commit -m "feat(model): add DURATION_OPTIONS constant and selected_duration_idx to Config"
```

---

### Task 2: Write failing tests for Tab cycling behavior

**Files:**
- Modify: `src/update.rs` (tests module only)

- [ ] **Step 1: Add four failing tests inside the existing `#[cfg(test)] mod tests` block**

Place these after the existing `tab_resets_screen_to_typing` test (around line 194):

```rust
#[test]
fn tab_while_waiting_cycles_to_next_duration() {
    let mut model = model_with_words(&["hello"]);
    assert_eq!(model.session.status, TestStatus::Waiting);
    update(&mut model, Msg::Tab);
    assert_eq!(model.config.selected_duration_idx, 1);
    assert_eq!(model.config.time_limit, Duration::from_secs(30));
}

#[test]
fn tab_cycles_through_all_durations() {
    // Note: in unit tests Command::GenerateWords is not executed,
    // so session.status stays Waiting across all three Tab calls.
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Tab); // 0 → 1 (30s)
    assert_eq!(model.config.selected_duration_idx, 1);
    update(&mut model, Msg::Tab); // 1 → 2 (60s)
    assert_eq!(model.config.selected_duration_idx, 2);
    update(&mut model, Msg::Tab); // 2 → 0 (15s, wraps)
    assert_eq!(model.config.selected_duration_idx, 0);
    assert_eq!(model.config.time_limit, Duration::from_secs(15));
}

#[test]
fn tab_while_running_does_not_cycle() {
    let mut model = model_with_words(&["hello"]);
    update(&mut model, Msg::Char('h')); // transitions status to Running
    assert_eq!(model.session.status, TestStatus::Running);
    update(&mut model, Msg::Tab);
    assert_eq!(model.config.selected_duration_idx, 0);
    assert_eq!(model.config.time_limit, Duration::from_secs(15));
}

#[test]
fn tab_while_done_cycles_duration() {
    let mut model = model_with_words(&["hi"]);
    update(&mut model, Msg::Char('h'));
    update(&mut model, Msg::Space); // transitions to Done
    assert_eq!(model.screen, Screen::Done);
    update(&mut model, Msg::Tab);
    assert_eq!(model.config.selected_duration_idx, 1);
    assert_eq!(model.config.time_limit, Duration::from_secs(30));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo nextest run 2>&1 | tail -5
```

Expected: 4 failures (`tab_while_waiting_cycles_to_next_duration`, `tab_cycles_through_all_durations`, `tab_while_running_does_not_cycle`, `tab_while_done_cycles_duration`).

---

### Task 3: Implement Tab cycling logic in `update.rs`

**Files:**
- Modify: `src/update.rs`

- [ ] **Step 1: Add `Duration` and `DURATION_OPTIONS` imports**

The existing imports at the top of `update.rs` are:

```rust
use crate::commands::Command;
use crate::model::{Model, Screen, TestStatus};
use crate::msg::Msg;
```

Replace with:

```rust
use std::time::Duration;

use crate::commands::Command;
use crate::model::{Model, Screen, TestStatus, DURATION_OPTIONS};
use crate::msg::Msg;
```

- [ ] **Step 2: Replace the `Msg::Tab` arm**

Current arm (around line 11–15):

```rust
Msg::Tab => {
    model.screen = Screen::Typing;
    return Command::GenerateWords {
        count: model.config.word_count,
    };
}
```

Replace with:

```rust
Msg::Tab => {
    if model.session.status != TestStatus::Running {
        let next_idx =
            (model.config.selected_duration_idx + 1) % DURATION_OPTIONS.len();
        model.config.selected_duration_idx = next_idx;
        model.config.time_limit =
            Duration::from_secs(DURATION_OPTIONS[next_idx]);
    }
    model.screen = Screen::Typing;
    return Command::GenerateWords {
        count: model.config.word_count,
    };
}
```

- [ ] **Step 3: Run all tests to verify 57 pass**

```bash
cargo nextest run 2>&1 | tail -5
```

Expected: `57 passed` (53 original + 4 new). Zero failures.

- [ ] **Step 4: Commit**

```bash
git add src/update.rs
git commit -m "feat(update): cycle selected duration on Tab when not Running"
```

---

### Task 4: Add `duration_strip_spans` helper and update typing header in `view.rs`

**Files:**
- Modify: `src/view.rs`

- [ ] **Step 1: Add `DURATION_OPTIONS` to the import line**

Current import (line 11):

```rust
use crate::model::{CursorStyle, Model, Screen, TestStatus};
```

Replace with:

```rust
use crate::model::{CursorStyle, Model, Screen, TestStatus, DURATION_OPTIONS};
```

- [ ] **Step 2: Add the `duration_strip_spans` helper function**

Add this function immediately before `fn render_typing`:

```rust
fn duration_strip_spans<'a>(selected_idx: usize, dimmed: bool) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    for (i, &secs) in DURATION_OPTIONS.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let label: String = if i == selected_idx {
            format!("[{}]", secs)
        } else {
            secs.to_string()
        };
        let style = if i == selected_idx && !dimmed {
            Style::new().add_modifier(Modifier::BOLD)
        } else {
            Style::new().dim()
        };
        spans.push(Span::styled(label, style));
    }
    spans
}
```

- [ ] **Step 3: Replace the header rendering block in `render_typing`**

Find the header block (lines 116–131 of the current file). It currently reads:

```rust
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
```

Replace the entire block with:

```rust
// Header: duration strip always visible; countdown appended only while Running.
let is_running = model.session.status == TestStatus::Running;
let mut header_spans: Vec<Span> = vec![
    Span::styled("kern", Style::new().add_modifier(Modifier::BOLD)),
    Span::raw("  "),
];
header_spans.extend(duration_strip_spans(
    model.config.selected_duration_idx,
    is_running,
));
if is_running {
    let countdown = model
        .config
        .time_limit
        .saturating_sub(model.session.elapsed);
    header_spans.push(Span::raw("  ·  "));
    header_spans.push(Span::styled(
        format!("{}s", countdown.as_secs()),
        Style::new().dim(),
    ));
}
header_spans.push(Span::raw("  "));
header_spans.push(Span::styled("[tab] restart", Style::new().dim()));
let header = Paragraph::new(Line::from(header_spans));
frame.render_widget(header, header_area);
```

- [ ] **Step 4: Run tests — expect the typing snapshot to fail**

```bash
cargo nextest run typing_screen_snapshot 2>&1
```

Expected: FAIL — the header row now contains the strip instead of `15s`.

- [ ] **Step 5: Update the typing snapshot**

```bash
INSTA_UPDATE=always cargo nextest run typing_screen_snapshot 2>&1
```

Expected: PASS after the snapshot file is rewritten.

- [ ] **Step 6: Verify the updated snapshot looks correct**

```bash
cat src/snapshots/kern__view__tests__typing_screen_snapshot.snap
```

The header row should now contain the strip and countdown (the test model is Running with elapsed=ZERO and selected_duration_idx=0, so countdown=15s):

```
kern  [15]  30  60  ·  15s  [tab] restart
```

(`[15]` is dim since status=Running; `30` and `60` are dim; `·  15s` is the live countdown.)

---

### Task 5: Update `render_results` to show the duration strip

**Files:**
- Modify: `src/view.rs`

- [ ] **Step 1: Replace the entire `render_results` function**

The current function starts at `fn render_results` and ends before `fn render_typing`. Replace it entirely with:

```rust
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
        Constraint::Length(1), // duration strip
        Constraint::Length(1), // spacer
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
        Paragraph::new(Line::from(duration_strip_spans(
            model.config.selected_duration_idx,
            false,
        )))
        .alignment(Alignment::Center),
        vertical[1],
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            "kern",
            Style::new().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        vertical[3],
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            "  wpm       raw wpm        acc",
            Style::new().dim(),
        ))
        .alignment(Alignment::Center),
        vertical[5],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(format!("{:>5.0}", wpm_val)),
            Span::raw("       "),
            Span::raw(format!("{:>5.0}", raw_val)),
            Span::raw("       "),
            Span::raw(format!("{:>4.0}%", acc_val)),
        ]))
        .alignment(Alignment::Center),
        vertical[6],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[tab] change/restart", Style::new().dim()),
            Span::raw("   "),
            Span::styled("[esc] quit", Style::new().dim()),
        ]))
        .alignment(Alignment::Center),
        vertical[8],
    );
}
```

- [ ] **Step 2: Run tests — expect the results snapshot to fail**

```bash
cargo nextest run results_screen_snapshot 2>&1
```

Expected: FAIL — layout now has the strip row; footer text changed from `[tab] restart` to `[tab] change/restart`.

- [ ] **Step 3: Update the results snapshot**

```bash
INSTA_UPDATE=always cargo nextest run results_screen_snapshot 2>&1
```

Expected: PASS after snapshot is rewritten.

- [ ] **Step 4: Verify the updated results snapshot looks correct**

```bash
cat src/snapshots/kern__view__tests__results_screen_snapshot.snap
```

The strip row (centered) should appear two rows above `kern`:

```
                                   [15]  30  60
                                                
                                      kern
```

(`[15]` is bold since the results screen is not Running. `30` and `60` are dim.)

The footer should read:

```
                        [tab] change/restart   [esc] quit
```

- [ ] **Step 5: Run the full test suite — expect 57 tests to pass**

```bash
cargo nextest run 2>&1 | tail -5
```

Expected: `57 passed`, zero failures.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```

Expected: no warnings or errors.

- [ ] **Step 7: Commit**

```bash
git add src/view.rs src/snapshots/kern__view__tests__typing_screen_snapshot.snap src/snapshots/kern__view__tests__results_screen_snapshot.snap
git commit -m "feat(view): add duration strip to typing header and results screen"
```
