use std::time::Duration;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use tempfile::TempDir;

use crate::domain::model::{Model, Screen, SessionState, TestStatus, Word};
use crate::domain::msg::Msg;
use crate::domain::test_config::{TestConfig, TestMode};
use crate::domain::update::update;
use crate::io::commands::{Command, execute_command};
use crate::io::persistence;
use crate::io::stats::SessionResult;

fn two_word_time_mode_model() -> Model {
    Model {
        session: SessionState::new(vec![Word::new("hi"), Word::new("ok")]),
        ..Model::default()
    }
}

fn two_word_model() -> Model {
    // Integration tests exercise the word-completion end-of-session path;
    // use Words mode so Space on the last word ends the test rather than
    // appending more words (which is the Time mode behavior).
    let config = TestConfig {
        test_mode: TestMode::Words,
        ..TestConfig::default()
    };
    Model {
        session: SessionState::new(vec![Word::new("hi"), Word::new("ok")]),
        config,
        ..Model::default()
    }
}

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

    assert_eq!(model.screen, Screen::Results);
    assert_eq!(model.history.len(), 1);
}

#[test]
fn timer_expiry_saves_stats() {
    let mut rng = SmallRng::seed_from_u64(0);
    // Timer expiry is a time-mode-only event; use Time mode for this test.
    let mut model = two_word_model();
    model.config.test_mode = TestMode::Time;

    // Start the session (Waiting → Running)
    update(&mut model, Msg::Char('h'));
    assert_eq!(model.session.status, TestStatus::Running);

    // Tick exactly at the time limit → triggers Done via timer path (not Space)
    let time_limit = model.config.time_limit;
    let cmd = update(&mut model, Msg::Tick(time_limit));
    assert!(matches!(cmd, Command::SaveStats(_)));
    execute_command(&mut model, cmd, &mut rng);

    assert_eq!(model.screen, Screen::Results);
    assert_eq!(model.session.status, TestStatus::Done);
    assert_eq!(model.history.len(), 1);
}

#[test]
fn tab_from_done_resets_session() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_model();

    // Drive to Done via word completion
    update(&mut model, Msg::Char('h'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);
    update(&mut model, Msg::Char('o'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);
    assert_eq!(model.screen, Screen::Results);

    // Tab from Done: restart-only, does not cycle duration
    let cmd = update(&mut model, Msg::Tab);
    assert!(matches!(cmd, Command::GenerateWords { .. }));
    execute_command(&mut model, cmd, &mut rng);

    assert_eq!(model.screen, Screen::Typing);
    assert_eq!(model.session.status, TestStatus::Waiting);
    // Tab no longer cycles duration — idx stays at 0
    assert_eq!(model.config.selected_duration_idx, 0);
}

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

#[test]
fn words_mode_full_session() {
    let mut rng = SmallRng::seed_from_u64(0);
    let mut model = two_word_model(); // Words mode

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
    assert_eq!(model.screen, Screen::Results);
    assert!(matches!(cmd, Command::SaveStats(_)));
    execute_command(&mut model, cmd, &mut rng);
    assert_eq!(model.history.len(), 1);
    assert_eq!(model.config.test_mode, TestMode::Words);
}

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

    // Commit the second word via Space (it is not the last word now)
    update(&mut model, Msg::Char('o'));
    update(&mut model, Msg::Char('k'));
    let cmd = update(&mut model, Msg::Space);
    execute_command(&mut model, cmd, &mut rng);
    // Pool grew again; test is still running
    assert_eq!(model.session.words.len(), initial_len + 2);
    assert_eq!(model.session.status, TestStatus::Running);
    assert_eq!(model.screen, Screen::Typing);
}

#[test]
fn raw_accuracy_includes_corrected_errors() {
    let mut rng = SmallRng::seed_from_u64(0);
    let config = TestConfig {
        test_mode: TestMode::Words,
        ..TestConfig::default()
    };
    let mut model = Model {
        session: SessionState::new(vec![Word::new("hi"), Word::new("ok")]),
        config,
        ..Model::default()
    };

    // Type wrong char, backspace, then correct — error must persist
    update(&mut model, Msg::Char('x')); // wrong ('h' expected)
    update(&mut model, Msg::Backspace);
    update(&mut model, Msg::Char('h')); // correct
    let cmd = update(&mut model, Msg::Space); // commit "hi" (partially typed)
    execute_command(&mut model, cmd, &mut rng);

    update(&mut model, Msg::Char('o'));
    let cmd = update(&mut model, Msg::Space); // commit "ok" → Done
    execute_command(&mut model, cmd, &mut rng);

    // total_chars_typed = 3 ('x', 'h', 'o'), total_errors = 1 ('x')
    // accuracy = (3 - 1) / 3 * 100 ≈ 66.67%
    assert_eq!(model.screen, Screen::Results);
    let result = model.history.last().expect("stats saved");
    assert!(
        (result.accuracy - 66.67).abs() < 0.1,
        "accuracy was {}",
        result.accuracy
    );
}

#[test]
fn version_check_sends_on_channel() {
    use std::sync::mpsc;
    use std::time::Duration;
    use update_informer::registry;

    let (tx, rx) = mpsc::channel();
    let informer = update_informer::fake(registry::Crates, "ktype", "0.4.0", "99.0.0");
    crate::app::spawn_version_check(tx, informer);
    let version = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(version, "v99.0.0");
}
