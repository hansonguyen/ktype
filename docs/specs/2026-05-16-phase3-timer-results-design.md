# Phase 3 Design: Live Countdown Timer + Results Screen

## Overview

Phase 3 adds a 15-second countdown timer and a basic results screen to Kern. The test ends when the timer hits zero or all words are completed, whichever comes first. Results show WPM, raw WPM, and accuracy. Tab on the results screen restarts a fresh test. All results are in-memory only — no persistence.

**Mode:** Phase 3 is always time mode. The existing 25-word set acts as a ceiling (test also ends if all words are typed before time expires), but the timer is the primary end condition. A word/time mode toggle is deferred to a later phase. There are no CLI flags or in-app mode selectors in this phase.

---

## Timer Architecture

**Approach: Pure TEA (time flows through `Msg::Tick(Duration)`).**

`main.rs` owns all `Instant` usage as infrastructure, alongside `rng`. It computes elapsed time and passes it into `update` via `Msg::Tick(Duration)`. `update` stays 100% pure — it only receives values, never calls `Instant::now()`.

`main.rs` detects the `Waiting → Running` status transition after each input update and manages `timer_start: Option<Instant>` directly — no new `Command` variant needed.

**Loop structure:**
```
draw → process input → detect status transition → fire Msg::Tick(elapsed) → check quit
```

**Timer management in `main.rs`:**
- After input update: if `timer_start.is_none() && status == Running` → `timer_start = Some(Instant::now())`
- After input update: if `timer_start.is_some() && status == Waiting` → `timer_start = None` (Tab reset)
- Every frame: `elapsed = timer_start.map(|t| t.elapsed()).unwrap_or_default()`; fire `Msg::Tick(elapsed)`

**Why B over A (pragmatic Instant in session):** `update` tests can drive timer logic with synthetic `Duration` values — no wall clock, fully deterministic. `Duration` implements `PartialEq` so `Msg` retains all derives.

---

## Model Changes (`model.rs`)

```rust
pub struct SessionState {
    pub words: Vec<Word>,
    pub current_word: usize,
    pub status: TestStatus,
    pub elapsed: Duration,  // new: updated each Tick when Running
}

pub struct Config {
    pub word_count: usize,
    pub cursor_style: CursorStyle,
    pub time_limit: Duration,  // new: default Duration::from_secs(15)
    pub punctuation: bool,
    pub numbers: bool,
}
```

`SessionState::new` zero-initializes `elapsed: Duration::ZERO`. Tab restarts via `Command::GenerateWords` → `SessionState::new` → elapsed resets automatically.

---

## Message Changes (`msg.rs`)

```rust
pub enum Msg {
    Tick(Duration),  // was: Tick (no-op); now carries elapsed since test start
    Char(char),
    Backspace,
    Space,
    Tab,
    Esc,
}
```

`Duration` implements `Debug + Clone + PartialEq`, so all derives on `Msg` are preserved.

---

## Update Changes (`update.rs`)

```
Msg::Tick(elapsed):
  - if status != Running → return Command::None (ignore before first keypress and after done)
  - session.elapsed = elapsed
  - if elapsed >= config.time_limit → session.status = Done, screen = Done

Msg::Space (last word):
  - unchanged: status = Done, screen = Done
  - session.elapsed holds the most-recent Tick value (accurate to ±16ms) — no special handling needed

Msg::Char:
  - unchanged: sets status = Running on Waiting → Running transition
  - main.rs detects this transition and records timer_start

Msg::Tab:
  - unchanged: screen = Typing, return Command::GenerateWords
  - fresh SessionState from GenerateWords resets elapsed = Duration::ZERO
  - main.rs detects Waiting status after reset and clears timer_start
```

---

## New Module: `metrics.rs`

Six pure functions over `&[Word]` and `Duration`. No I/O, no state.

```rust
// Primary metrics
pub fn wpm(correct_words: usize, elapsed: Duration) -> f64
pub fn raw_wpm(committed_words: usize, elapsed: Duration) -> f64
pub fn accuracy(correct_chars: u64, total_chars_typed: u64) -> f64

// Helpers (pub for tests)
pub fn count_correct_words(words: &[Word]) -> usize     // committed AND all chars match
pub fn count_committed_words(words: &[Word]) -> usize   // any committed word
pub fn count_correct_chars(words: &[Word]) -> u64       // positions where typed[i] == word.chars[i]
pub fn count_total_chars_typed(words: &[Word]) -> u64   // sum of typed.len() across all words
```

**Definitions:**
- **Correct word**: `word.committed == true` AND every `word.typed.chars().nth(i) == Some(word.chars[i])` for all i
- **WPM**: `correct_words as f64 / elapsed.as_secs_f64() * 60.0` — returns `0.0` if `elapsed < 1ms`
- **Raw WPM**: `committed_words as f64 / elapsed.as_secs_f64() * 60.0` — returns `0.0` if `elapsed < 1ms`
- **Accuracy**: `correct_chars as f64 / total_chars_typed as f64 * 100.0` — returns `0.0` if nothing typed

Metrics are computed in `view` on demand — not stored in `Model`. The session's word and elapsed state is available on the results screen before Tab resets it.

---

## View Changes (`view.rs`)

### Typing screen header

Replaces the static `"words: N"` info slot with a live countdown:

- `Waiting`: show `"15"` dimmed (static preview of the time limit)
- `Running`: show `(config.time_limit.saturating_sub(session.elapsed)).as_secs()` — counts down to 0

Header format: `kern  12  [tab] restart`

### Results screen (`render_results`, replaces `render_done`)

```
                 kern

   wpm      raw wpm    acc
   72.4      78.1      94%

         [tab] restart   [esc] quit
```

Three centered columns. Metrics computed inline by calling `metrics::*` with `&model.session.words` and `model.session.elapsed`. View remains pure — no mutation.

---

## Commands (`commands.rs`)

No changes. `Command::GenerateWords` already handles restart via `SessionState::new`.

---

## Testing Strategy

### `metrics.rs` — unit tests
Pure functions with fixed word slices and durations. Cover: zero elapsed edge case (returns 0.0), zero typed chars (accuracy = 0.0), all correct, all wrong, mixed.

### `update.rs` — timer tests
```rust
// Simulate expiry with synthetic Duration — no Instant needed
update(&mut model, Msg::Tick(Duration::from_secs(16)));
assert_eq!(model.screen, Screen::Done);
assert_eq!(model.session.status, TestStatus::Done);

// Tick before Running is a no-op
update(&mut model, Msg::Tick(Duration::from_secs(5)));
assert_eq!(model.session.status, TestStatus::Waiting);
```

### Proptest
Add `Msg::Tick(Duration::ZERO)` to the arbitrary message set. Zero duration won't expire the timer, preserving existing invariants.

### Snapshot tests
Results screen render (`render_results`) with known word/elapsed inputs.

---

## File Checklist

| File | Change |
|------|--------|
| `src/model.rs` | Add `elapsed: Duration` to `SessionState`; add `time_limit: Duration` to `Config` |
| `src/msg.rs` | `Tick` → `Tick(Duration)` |
| `src/update.rs` | Handle `Tick(Duration)` with expiry logic; update tests |
| `src/main.rs` | Add `timer_start: Option<Instant>`; rework loop to fire `Msg::Tick(elapsed)` every frame |
| `src/metrics.rs` | New module: 6 pure functions + unit tests |
| `src/view.rs` | Countdown in header; replace `render_done` with `render_results` |
