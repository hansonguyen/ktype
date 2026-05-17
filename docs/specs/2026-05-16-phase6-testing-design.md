# Phase 6 Testing Design

**Date:** 2026-05-16
**Status:** Approved

## Context

Phases 1–5 are complete: TEA event loop, typing engine, rendering, metrics, duration selector, and JSON persistence. 63 tests pass. Phase 6 closes coverage gaps before the OSS release in Phase 7.

## Gaps Being Closed

| Category | Gap |
|---|---|
| Integration | No test exercises `execute_command` — `model.history` population, GenerateWords, timer-expiry pipeline |
| Property | Generator: no arbitrary-seed coverage; Persistence: no field-level round-trip proof |
| Snapshot | No countdown rendering (elapsed > 0); no duration strip variant coverage |

Already covered (no action needed):
- `persistence.rs` has `append_to` + `load_from` round-trip unit tests
- `results_screen_snapshot` already renders non-zero metrics (elapsed=10s, mixed words)

## Architecture

### File Organization (Approach B)

```
src/
  integration_tests.rs   ← NEW: cross-module flows, pub(crate) access
  generator.rs           ← add prop_tests block
  persistence.rs         ← extend tests block with proptest
  view.rs                ← add two new snapshot test functions
main.rs                  ← add `#[cfg(test)] mod integration_tests;`
```

The `tests/` directory is not used — kern is a binary crate (`[[bin]]`), so `tests/` cannot access `pub(crate)` items without promoting them to `pub`. All tests stay within the binary's module tree using `#[cfg(test)]`.

## Section 1 — Integration Tests (`src/integration_tests.rs`)

Registered in `main.rs`:
```rust
#[cfg(test)]
mod integration_tests;
```

All tests use real functions, real `TempDir` paths, and no mocks.

### Test: `full_session_via_word_completion`

Drive a model through a complete typing session using only the `update` + `execute_command` pipeline:

1. Build a model with known words via `execute_command(GenerateWords)`
2. For each word: type one char, then Space
3. Capture the returned `Command::SaveStats` from the final Space
4. Call `execute_command(SaveStats(...))` with a `TempDir`-backed persistence path
5. Assert: `model.history.len() == 1`, `model.screen == Screen::Done`

**Why:** Verifies the complete word-completion path populates `model.history` end-to-end.

### Test: `timer_expiry_saves_stats`

1. Start a model, type one char (status → Running)
2. Issue `Msg::Tick(config.time_limit)` — captures `Command::SaveStats`
3. Call `execute_command(SaveStats(...))`
4. Assert: `model.history.len() == 1`, `model.screen == Screen::Done`

**Why:** Verifies the timer-expiry code path (not word completion) saves stats.

### Test: `tab_from_done_resets_session`

1. Drive model to Done via Space on last word
2. Issue `Msg::Tab` — captures `Command::GenerateWords { count }`
3. Call `execute_command(GenerateWords { count })`
4. Assert: `model.session.status == TestStatus::Waiting`, `model.session.words.len() == count`

**Why:** Verifies Tab from Done screen resets the session so a new test can begin.

### Test: `persistence_end_to_end`

1. Drive model to Done, capture `SaveStats` payload
2. Build a `SessionResult` from the payload fields
3. Call `persistence::append_to(&tmp_path, &result)` directly
4. Call `persistence::load_from(&tmp_path)`
5. Assert: loaded len == 1, all fields match (`wpm`, `raw_wpm`, `accuracy`, `duration_secs`, `timestamp`)

**Why:** Confirms the persistence layer correctly serializes and deserializes a real result from the full pipeline.

## Section 2 — Property Tests

### Generator (`generator.rs` — new `prop_tests` block)

Uses `SmallRng::from_seed(seed)` with `proptest::array::uniform32(any::<u8>())` as the seed strategy so arbitrary seeds are exercised.

```
fn generate_count_is_exact(count in 1usize..=50, seed: [u8; 32])
    → generate(count, &mut SmallRng::from_seed(seed)).len() == count

fn generate_all_non_empty(count in 1usize..=50, seed: [u8; 32])
    → all words: chars.len() > 0

fn generate_all_lowercase_ascii(count in 1usize..=50, seed: [u8; 32])
    → all chars in every word: c.is_ascii_lowercase()
```

### Persistence (`persistence.rs` — extend existing `tests` with proptest)

Generates `n` `SessionResult` values via `prop::collection::vec(arb_result(), 1..=20)` where `arb_result()` produces results with arbitrary (but deterministic) field values.

```
fn append_n_load_n_round_trip(results: Vec<SessionResult>)
    → append each to TempDir path
    → load_from returns exactly results.len() entries
    → each entry matches the original on all fields:
         timestamp, duration_secs, wpm, raw_wpm, accuracy
```

Field equality for floats uses `(a - b).abs() < 1e-9` to handle JSON float round-trips.

## Section 3 — Snapshot Additions (`view.rs`)

Uses named `assert_snapshot!("name", output)` calls so multiple variants live in one test function. Each named snapshot gets its own insta snapshot file and review cycle.

### Test function: `typing_screen_running_variants_snapshot`

Two named snapshots using `status: Running`:

| Snapshot name | `elapsed` | Expected |
|---|---|---|
| `elapsed_zero` | `Duration::ZERO` | No countdown in header |
| `elapsed_5s` | `Duration::from_secs(5)` | "10s" countdown in header (15s limit − 5s) |

### Test function: `typing_screen_duration_variants_snapshot`

Three named snapshots using `status: Waiting`, `elapsed: ZERO`:

| Snapshot name | `selected_duration_idx` | Expected |
|---|---|---|
| `duration_15s` | 0 | `[15]` bold, `30` and `60` dim |
| `duration_30s` | 1 | `[30]` bold, `15` and `60` dim |
| `duration_60s` | 2 | `[60]` bold, `15` and `30` dim |

## Error Handling

- Integration tests that call `persistence::append_to` / `load_from` use `.unwrap()` — failures are bugs, not recoverable conditions in tests.
- Float comparisons use `(a - b).abs() < 1e-9` rather than `==` for JSON round-trip safety.

## Testing the Tests

All 63 existing tests must keep passing. New test count targets:

| Category | New tests |
|---|---|
| Integration | 4 |
| Property (generator) | 3 proptest cases |
| Property (persistence) | 1 proptest case |
| Snapshots | 5 named snapshots in 2 functions |

Run with: `cargo nextest run` (preferred) or `cargo test`.
Snapshot review: `cargo insta review` after first run.
Lint: `cargo clippy -- -D warnings` must pass.
Format: `cargo fmt` must pass.
