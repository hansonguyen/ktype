# Phase 6 Testing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close integration, property, and snapshot test coverage gaps in kern before the Phase 7 OSS release.

**Architecture:** All tests live inside the binary crate's module tree using `#[cfg(test)]`, giving access to `pub(crate)` functions without promoting them to `pub`. Integration tests get a dedicated `src/integration_tests.rs` file registered from `main.rs`. Property tests extend existing `#[cfg(test)]` blocks in `generator.rs` and `persistence.rs`. Snapshot additions go into `view.rs` using named `assert_snapshot!` calls.

**Tech Stack:** Rust 2024 edition, `proptest 1.11`, `insta 1.47` (yaml feature), `tempfile 3`, `rand 0.10` (`SmallRng`, `SeedableRng`), `cargo nextest`.

---

## File Map

| File | Change |
|---|---|
| `src/main.rs` | Add `#[cfg(test)] mod integration_tests;` |
| `src/integration_tests.rs` | **Create** — 4 integration tests |
| `src/generator.rs` | Add `mod prop_tests` block with 3 proptest cases |
| `src/persistence.rs` | Add proptest + `arb_result` strategy to existing `mod tests` |
| `src/view.rs` | Add 2 new snapshot test functions (5 named snapshots total) |

---

## Task 1: Wire the integration_tests module

**Files:**
- Modify: `src/main.rs`
- Create: `src/integration_tests.rs`

- [ ] **Step 1: Add module declaration to main.rs**

Open `src/main.rs`. After the last existing `mod` declaration (currently `mod view;` on line 10), add:

```rust
#[cfg(test)]
mod integration_tests;
```

The top of `main.rs` should look like this after the edit:

```rust
mod commands;
mod generator;
mod input;
mod metrics;
mod model;
mod msg;
mod persistence;
mod stats;
mod update;
mod view;

#[cfg(test)]
mod integration_tests;
```

- [ ] **Step 2: Create the integration_tests file with a placeholder**

Create `src/integration_tests.rs` with all imports the tests will need:

```rust
use std::time::Duration;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use tempfile::TempDir;

use crate::commands::{Command, execute_command};
use crate::model::{Config, Model, Screen, SessionState, TestStatus, Word};
use crate::msg::Msg;
use crate::persistence;
use crate::stats::SessionResult;
use crate::update::update;

fn two_word_model() -> Model {
    Model {
        screen: Screen::Typing,
        session: SessionState::new(vec![Word::new("hi"), Word::new("ok")]),
        config: Config::default(),
        history: Vec::new(),
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build
```

Expected: no errors. (Unused import warnings are fine at this stage — they will be resolved when tests are added.)

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/integration_tests.rs
git commit -m "test(integration): wire integration_tests module"
```

---

## Task 2: Integration test — full session via word completion

**Files:**
- Modify: `src/integration_tests.rs`

- [ ] **Step 1: Write the failing test**

Append this test to `src/integration_tests.rs`:

```rust
#[test]
fn full_session_via_word_completion() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_model();

    // Type one char of "hi", commit with Space → advances to "ok"
    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);

    // Type one char of "ok", commit with Space → last word → Done + SaveStats
    update(&mut model, Msg::Char('o'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);

    assert_eq!(model.screen, Screen::Done);
    assert_eq!(model.history.len(), 1);
}
```

- [ ] **Step 2: Run and verify it passes**

```bash
cargo nextest run integration_tests::full_session_via_word_completion
```

Expected output: `PASS`.

Note: `execute_command(SaveStats)` writes to `~/.config/kern/stats.json` as a side effect — this is expected behavior and won't cause the test to fail even if that write fails (errors are printed to stderr only).

- [ ] **Step 3: Commit**

```bash
git add src/integration_tests.rs
git commit -m "test(integration): full session via word completion populates history"
```

---

## Task 3: Integration test — timer expiry saves stats

**Files:**
- Modify: `src/integration_tests.rs`

- [ ] **Step 1: Write the failing test**

Append this test to `src/integration_tests.rs`:

```rust
#[test]
fn timer_expiry_saves_stats() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_model();

    // Start the session (Waiting → Running)
    update(&mut model, Msg::Char('h'));
    assert_eq!(model.session.status, TestStatus::Running);

    // Tick exactly at the time limit → triggers Done via timer path (not Space)
    let cmd = update(&mut model, Msg::Tick(model.config.time_limit));
    assert!(matches!(cmd, Command::SaveStats(_)));
    execute_command(&mut model, cmd, &mut rng);

    assert_eq!(model.screen, Screen::Done);
    assert_eq!(model.session.status, TestStatus::Done);
    assert_eq!(model.history.len(), 1);
}
```

- [ ] **Step 2: Run and verify it passes**

```bash
cargo nextest run integration_tests::timer_expiry_saves_stats
```

Expected output: `PASS`.

- [ ] **Step 3: Commit**

```bash
git add src/integration_tests.rs
git commit -m "test(integration): timer expiry path saves stats and transitions to Done"
```

---

## Task 4: Integration test — Tab from Done resets session

**Files:**
- Modify: `src/integration_tests.rs`

- [ ] **Step 1: Write the failing test**

Append this test to `src/integration_tests.rs`:

```rust
#[test]
fn tab_from_done_resets_session() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_model();

    // Drive to Done via word completion
    update(&mut model, Msg::Char('h'));
    execute_command(&mut model, update(&mut model, Msg::Space), &mut rng);
    update(&mut model, Msg::Char('o'));
    execute_command(&mut model, update(&mut model, Msg::Space), &mut rng);
    assert_eq!(model.screen, Screen::Done);

    // Tab from Done: cycles duration (Done status != Running) and resets session
    let cmd = update(&mut model, Msg::Tab);
    assert!(matches!(cmd, Command::GenerateWords { .. }));
    execute_command(&mut model, cmd, &mut rng);

    assert_eq!(model.screen, Screen::Typing);
    assert_eq!(model.session.status, TestStatus::Waiting);
    assert_eq!(model.session.words.len(), model.config.word_count);
    // Duration cycles: 0 → 1 (15s → 30s)
    assert_eq!(model.config.selected_duration_idx, 1);
}
```

- [ ] **Step 2: Run and verify it passes**

```bash
cargo nextest run integration_tests::tab_from_done_resets_session
```

Expected output: `PASS`.

- [ ] **Step 3: Commit**

```bash
git add src/integration_tests.rs
git commit -m "test(integration): Tab from Done resets session and cycles duration"
```

---

## Task 5: Integration test — persistence end-to-end

**Files:**
- Modify: `src/integration_tests.rs`

- [ ] **Step 1: Write the failing test**

Append this test to `src/integration_tests.rs`:

```rust
#[test]
fn persistence_end_to_end() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("stats.json");

    let result = SessionResult {
        timestamp: 1_234_567_890,
        duration_secs: 15,
        wpm: 42.5,
        raw_wpm: 48.0,
        accuracy: 88.5,
    };

    persistence::append_to(&path, &result).unwrap();
    let loaded = persistence::load_from(&path).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].timestamp, result.timestamp);
    assert_eq!(loaded[0].duration_secs, result.duration_secs);
    assert!((loaded[0].wpm - result.wpm).abs() < 1e-9);
    assert!((loaded[0].raw_wpm - result.raw_wpm).abs() < 1e-9);
    assert!((loaded[0].accuracy - result.accuracy).abs() < 1e-9);
}
```

- [ ] **Step 2: Run and verify it passes**

```bash
cargo nextest run integration_tests::persistence_end_to_end
```

Expected output: `PASS`.

- [ ] **Step 3: Run all integration tests together**

```bash
cargo nextest run integration_tests
```

Expected: all 4 integration tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/integration_tests.rs
git commit -m "test(integration): persistence end-to-end round-trip with temp path"
```

---

## Task 6: Property tests for generator

**Files:**
- Modify: `src/generator.rs`

- [ ] **Step 1: Write the failing tests**

At the bottom of `src/generator.rs`, after the existing `mod tests { ... }` block, add a new block:

```rust
#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    proptest! {
        #[test]
        fn generate_count_is_exact(count in 1usize..=50, seed in any::<[u8; 32]>()) {
            let mut rng = SmallRng::from_seed(seed);
            prop_assert_eq!(generate(count, &mut rng).len(), count);
        }

        #[test]
        fn generate_all_non_empty(count in 1usize..=50, seed in any::<[u8; 32]>()) {
            let mut rng = SmallRng::from_seed(seed);
            let words = generate(count, &mut rng);
            for word in &words {
                prop_assert!(!word.chars.is_empty(), "word had empty chars");
            }
        }

        #[test]
        fn generate_all_lowercase_ascii(count in 1usize..=50, seed in any::<[u8; 32]>()) {
            let mut rng = SmallRng::from_seed(seed);
            let words = generate(count, &mut rng);
            for word in &words {
                for &c in &word.chars {
                    prop_assert!(
                        c.is_ascii_lowercase(),
                        "char '{}' is not lowercase ASCII", c
                    );
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run and verify they pass**

```bash
cargo nextest run generator::prop_tests
```

Expected output: all 3 proptest cases pass. Each runs 256 random seeds by default.

- [ ] **Step 3: Commit**

```bash
git add src/generator.rs
git commit -m "test(generator): proptest invariants for count, non-empty, lowercase ASCII"
```

---

## Task 7: Property test for persistence round-trip

**Files:**
- Modify: `src/persistence.rs`

- [ ] **Step 1: Add proptest imports and strategy**

In `src/persistence.rs`, the existing `mod tests` block starts at line 52. Add proptest imports and an `arb_result` strategy inside that block. The full updated `mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn sample_result() -> SessionResult {
        SessionResult {
            timestamp: 1_000_000,
            duration_secs: 15,
            wpm: 60.0,
            raw_wpm: 65.0,
            accuracy: 92.0,
        }
    }

    fn arb_result() -> impl Strategy<Value = SessionResult> {
        (
            any::<i64>(),
            any::<u64>(),
            0.0f64..200.0f64,
            0.0f64..200.0f64,
            0.0f64..100.0f64,
        )
            .prop_map(|(timestamp, duration_secs, wpm, raw_wpm, accuracy)| {
                SessionResult {
                    timestamp,
                    duration_secs,
                    wpm,
                    raw_wpm,
                    accuracy,
                }
            })
    }

    #[test]
    fn load_missing_file_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("stats.json");
        let result = load_from(&path).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn append_creates_file_and_loads_back() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("stats.json");
        let r = sample_result();
        append_to(&path, &r).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].timestamp, r.timestamp);
        assert!((loaded[0].wpm - r.wpm).abs() < 0.01);
    }

    #[test]
    fn append_accumulates_multiple_results() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("stats.json");
        append_to(&path, &sample_result()).unwrap();
        append_to(&path, &sample_result()).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    proptest! {
        #[test]
        fn append_n_load_n_round_trip(results in prop::collection::vec(arb_result(), 1..=20)) {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("stats.json");

            for result in &results {
                append_to(&path, result).unwrap();
            }

            let loaded = load_from(&path).unwrap();
            prop_assert_eq!(loaded.len(), results.len());

            for (orig, loaded_entry) in results.iter().zip(loaded.iter()) {
                prop_assert_eq!(orig.timestamp, loaded_entry.timestamp);
                prop_assert_eq!(orig.duration_secs, loaded_entry.duration_secs);
                prop_assert!((orig.wpm - loaded_entry.wpm).abs() < 1e-9);
                prop_assert!((orig.raw_wpm - loaded_entry.raw_wpm).abs() < 1e-9);
                prop_assert!((orig.accuracy - loaded_entry.accuracy).abs() < 1e-9);
            }
        }
    }
}
```

- [ ] **Step 2: Run and verify all persistence tests pass**

```bash
cargo nextest run persistence::tests
```

Expected: all 4 tests pass (3 existing + 1 new proptest).

- [ ] **Step 3: Commit**

```bash
git add src/persistence.rs
git commit -m "test(persistence): proptest round-trip for N results with field equality"
```

---

## Task 8: Snapshot — typing screen running variants

**Files:**
- Modify: `src/view.rs`

- [ ] **Step 1: Write the new snapshot test function**

In `src/view.rs`, inside the `mod tests { ... }` block (after the last existing `#[test]` function), add:

```rust
#[test]
fn typing_screen_running_variants_snapshot() {
    // elapsed = 0: countdown shows full 15s
    let model = test_model(&["the", "quick", "brown"], 1, &["the", "qu"]);
    // test_model sets status = Running and elapsed = ZERO already
    let output = render_to_string(&model, 80, 24);
    insta::assert_snapshot!("running_elapsed_zero", output);

    // elapsed = 5s: countdown shows 10s (15s − 5s)
    let mut model = test_model(&["the", "quick", "brown"], 1, &["the", "qu"]);
    model.session.elapsed = Duration::from_secs(5);
    let output = render_to_string(&model, 80, 24);
    insta::assert_snapshot!("running_elapsed_5s", output);
}
```

- [ ] **Step 2: Run to generate pending snapshots**

```bash
cargo nextest run view::tests::typing_screen_running_variants_snapshot
```

Expected: **FAIL** with "snapshot assertion failed" — insta creates `.snap.new` files for review. This is expected on first run.

- [ ] **Step 3: Review and accept the snapshots**

```bash
cargo insta review
```

Insta opens an interactive review. For each snapshot:
- `running_elapsed_zero`: header should contain `kern  [15]  30  60  ·  15s  [tab] restart`
- `running_elapsed_5s`: header should contain `kern  [15]  30  60  ·  10s  [tab] restart`

Press `a` to accept each one.

- [ ] **Step 4: Run again to confirm they pass**

```bash
cargo nextest run view::tests::typing_screen_running_variants_snapshot
```

Expected: `PASS`.

- [ ] **Step 5: Commit**

```bash
git add src/view.rs src/snapshots/
git commit -m "test(view): snapshot variants for running state with elapsed 0s and 5s"
```

---

## Task 9: Snapshot — typing screen duration variants

**Files:**
- Modify: `src/view.rs`

- [ ] **Step 1: Add the required import**

The test needs `DURATION_OPTIONS` from the model. The existing import at the top of `mod tests` is:

```rust
use crate::model::{Config, Model, Screen, SessionState, Word};
```

Update this line to also import `DURATION_OPTIONS`:

```rust
use crate::model::{Config, DURATION_OPTIONS, Model, Screen, SessionState, Word};
```

- [ ] **Step 2: Write the new snapshot test function**

Inside `mod tests`, after `typing_screen_running_variants_snapshot`, add:

```rust
#[test]
fn typing_screen_duration_variants_snapshot() {
    let render_with_duration = |idx: usize| {
        let mut model = test_model(&["the", "quick", "brown"], 0, &[]);
        model.session.status = crate::model::TestStatus::Waiting;
        model.config.selected_duration_idx = idx;
        model.config.time_limit = Duration::from_secs(DURATION_OPTIONS[idx]);
        render_to_string(&model, 80, 24)
    };

    insta::assert_snapshot!("duration_15s", render_with_duration(0));
    insta::assert_snapshot!("duration_30s", render_with_duration(1));
    insta::assert_snapshot!("duration_60s", render_with_duration(2));
}
```

- [ ] **Step 3: Run to generate pending snapshots**

```bash
cargo nextest run view::tests::typing_screen_duration_variants_snapshot
```

Expected: **FAIL** — insta creates `.snap.new` files.

- [ ] **Step 4: Review and accept the snapshots**

```bash
cargo insta review
```

For each snapshot, the header duration strip should show the selected duration in brackets:
- `duration_15s`: header contains `[15]  30  60`
- `duration_30s`: header contains `15  [30]  60`
- `duration_60s`: header contains `15  30  [60]`

No countdown should appear (status is Waiting, not Running).

Press `a` to accept each.

- [ ] **Step 5: Run again to confirm they pass**

```bash
cargo nextest run view::tests::typing_screen_duration_variants_snapshot
```

Expected: `PASS`.

- [ ] **Step 6: Commit**

```bash
git add src/view.rs src/snapshots/
git commit -m "test(view): snapshot variants for all three duration strip selections"
```

---

## Task 10: Final verification

**Files:** none

- [ ] **Step 1: Run the full test suite**

```bash
cargo nextest run
```

Expected: all tests pass. Count should be 63 (existing) + 4 (integration) + 3 (generator proptest) + 1 (persistence proptest) + 2 (snapshot functions) = **73 test functions** (plus 5 named snapshots).

Note: proptest cases each count as 1 test in nextest output.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: no warnings. If there are unused import warnings from `integration_tests.rs`, all imports should be used by the 4 tests. If any remain unused, remove them.

- [ ] **Step 3: Run formatter check**

```bash
cargo fmt --check
```

Expected: no diff. If there is a diff, run `cargo fmt` and commit:

```bash
cargo fmt
git add -p
git commit -m "style: apply cargo fmt"
```

- [ ] **Step 4: Final commit if needed**

If clippy or fmt required changes:

```bash
git add src/
git commit -m "style: apply cargo fmt after Phase 6 testing"
```
