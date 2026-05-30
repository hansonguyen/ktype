use std::time::Duration;

use crate::domain::metrics;
use crate::domain::model::{Model, Overlay, Screen, TestStatus};
use crate::domain::msg::Msg;
use crate::domain::test_config::{DURATION_OPTIONS, TestMode, WORD_COUNT_OPTIONS};
use crate::io::commands::{Command, StatsPayload};

fn regenerate(model: &mut Model) -> Command {
    model.screen = Screen::Typing;
    Command::GenerateWords {
        count: model.config.initial_word_count(),
    }
}

enum CycleDir {
    Prev,
    Next,
}

fn cycle_selection(model: &mut Model, dir: CycleDir) -> Command {
    if model.session.status == TestStatus::Running {
        return Command::None;
    }
    match model.config.test_mode {
        TestMode::Time => {
            let n = DURATION_OPTIONS.len() + 1;
            let cur = model.config.selected_duration_idx;
            let next = match dir {
                CycleDir::Next => (cur + 1) % n,
                CycleDir::Prev => (cur + n - 1) % n,
            };
            model.config.selected_duration_idx = next;
            if next < DURATION_OPTIONS.len() {
                model.config.time_limit = Duration::from_secs(DURATION_OPTIONS[next]);
            } else if let Some(secs) = model.config.custom_time_secs {
                if secs > 0 {
                    model.config.time_limit = Duration::from_secs(secs);
                }
            }
        }
        TestMode::Words => {
            let n = WORD_COUNT_OPTIONS.len() + 1;
            let cur = model.config.selected_word_count_idx;
            let next = match dir {
                CycleDir::Next => (cur + 1) % n,
                CycleDir::Prev => (cur + n - 1) % n,
            };
            model.config.selected_word_count_idx = next;
            if next < WORD_COUNT_OPTIONS.len() {
                model.config.word_count = WORD_COUNT_OPTIONS[next];
            }
        }
    }
    regenerate(model)
}

pub(crate) fn parse_custom_time(input: &str) -> u64 {
    let mut total: u64 = 0;
    let mut num_buf = String::new();

    for ch in input.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else if !num_buf.is_empty() {
            let n: u64 = num_buf.parse().unwrap_or(0);
            num_buf.clear();
            total = total.saturating_add(match ch {
                'h' => n.saturating_mul(3600),
                'm' => n.saturating_mul(60),
                _ => n,
            });
        }
        // non-digit with empty num_buf: skip
    }

    if !num_buf.is_empty() {
        total = total.saturating_add(num_buf.parse().unwrap_or(0));
    }

    total
}

fn build_stats_payload(model: &Model) -> StatsPayload {
    let correct_words = metrics::count_correct_words(&model.session.words);
    let committed_words = metrics::count_committed_words(&model.session.words);
    let duration_secs = match model.config.test_mode {
        TestMode::Time => {
            if model.config.selected_duration_idx < DURATION_OPTIONS.len() {
                DURATION_OPTIONS[model.config.selected_duration_idx]
            } else {
                model.session.elapsed.as_secs()
            }
        }
        TestMode::Words => model.session.elapsed.as_secs(),
    };
    let accuracy =
        metrics::raw_accuracy(model.session.total_chars_typed, model.session.total_errors);
    StatsPayload {
        duration_secs,
        wpm: metrics::wpm(correct_words, model.session.elapsed),
        raw_wpm: metrics::raw_wpm(committed_words, model.session.elapsed),
        accuracy,
    }
}

fn update_overlay(model: &mut Model, msg: Msg) -> Command {
    match msg {
        Msg::Char(c) => {
            if let Overlay::CustomTime { input } | Overlay::CustomWords { input } =
                &mut model.overlay
            {
                input.push(c);
            }
            Command::None
        }
        Msg::Backspace => {
            if let Overlay::CustomTime { input } | Overlay::CustomWords { input } =
                &mut model.overlay
            {
                input.pop();
            }
            Command::None
        }
        Msg::Space => {
            let overlay = std::mem::replace(&mut model.overlay, Overlay::None);
            match overlay {
                Overlay::CustomTime { input } => {
                    let secs = parse_custom_time(&input);
                    model.config.custom_time_secs = Some(secs);
                    if secs > 0 {
                        model.config.time_limit = Duration::from_secs(secs);
                    }
                }
                Overlay::CustomWords { input } => {
                    let count: usize = input.parse().unwrap_or(0);
                    model.config.custom_word_count = Some(count);
                }
                Overlay::None => return Command::None,
            }
            Command::GenerateWords {
                count: model.config.initial_word_count(),
            }
        }
        Msg::Esc => {
            model.overlay = Overlay::None;
            Command::None
        }
        _ => Command::None,
    }
}

pub fn update(model: &mut Model, msg: Msg) -> Command {
    if model.overlay != Overlay::None {
        return update_overlay(model, msg);
    }
    match msg {
        Msg::Esc => {
            model.should_quit = true;
        }

        Msg::EndTest => {
            if model.session.status == TestStatus::Running {
                model.session.status = TestStatus::Done;
                model.screen = Screen::Results;
                return Command::SaveStats(build_stats_payload(model));
            }
        }

        Msg::Tab => {
            return regenerate(model);
        }

        Msg::Char(c) => {
            if model.session.words.is_empty() {
                return Command::None;
            }
            if model.session.status == TestStatus::Waiting {
                model.session.status = TestStatus::Running;
            }
            let pushed = {
                let word = &mut model.session.words[model.session.current_word];
                if word.typed.len() < word.chars.len() {
                    word.typed.push(c);
                    let idx = word.typed.len() - 1;
                    let is_error = word.chars.get(idx) != Some(&c);
                    Some(is_error)
                } else {
                    None
                }
            };
            if let Some(is_error) = pushed {
                model.session.total_chars_typed += 1;
                if is_error {
                    model.session.total_errors += 1;
                }
            }

            let is_last = model.session.current_word == model.session.words.len() - 1;
            let word_full = model.session.words[model.session.current_word].typed.len()
                == model.session.words[model.session.current_word].chars.len();

            if is_last && word_full {
                let ends_test = matches!(model.config.test_mode, TestMode::Words)
                    && !model.config.is_infinite_words();

                model.session.words[model.session.current_word].committed = true;
                if ends_test {
                    model.session.status = TestStatus::Done;
                    model.screen = Screen::Results;
                    return Command::SaveStats(build_stats_payload(model));
                } else {
                    return Command::AppendWords { count: 1 };
                }
            }
        }

        Msg::Backspace => {
            let session = &mut model.session;
            let word = &mut session.words[session.current_word];
            if !word.typed.is_empty() {
                word.typed.pop();
            } else if session.current_word > 0 {
                // Retreat to previous word so the user can correct it.
                // Un-commit so it accepts input again.
                session.current_word -= 1;
                session.words[session.current_word].committed = false;
            }
        }

        Msg::Space => {
            // Open custom modal when on custom slot and idle
            if model.session.status == TestStatus::Waiting {
                let on_custom = match model.config.test_mode {
                    TestMode::Time => model.config.on_custom_time_slot(),
                    TestMode::Words => model.config.on_custom_words_slot(),
                };
                if on_custom {
                    let prefill = match model.config.test_mode {
                        TestMode::Time => model
                            .config
                            .custom_time_secs
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                        TestMode::Words => model
                            .config
                            .custom_word_count
                            .map(|c| c.to_string())
                            .unwrap_or_default(),
                    };
                    model.overlay = match model.config.test_mode {
                        TestMode::Time => Overlay::CustomTime { input: prefill },
                        TestMode::Words => Overlay::CustomWords { input: prefill },
                    };
                    return Command::None;
                }
            }
            if model.session.words.is_empty()
                || model.session.words[model.session.current_word]
                    .typed
                    .is_empty()
                || model.session.words[model.session.current_word].committed
            {
                return Command::None;
            }
            let is_last = model.session.current_word == model.session.words.len() - 1;
            model.session.words[model.session.current_word].committed = true;

            if is_last {
                let ends_test = matches!(model.config.test_mode, TestMode::Words)
                    && !model.config.is_infinite_words();

                if ends_test {
                    model.session.status = TestStatus::Done;
                    model.screen = Screen::Results;
                    return Command::SaveStats(build_stats_payload(model));
                } else {
                    return Command::AppendWords { count: 1 };
                }
            } else {
                model.session.current_word += 1;
                if matches!(model.config.test_mode, TestMode::Time) {
                    return Command::AppendWords { count: 1 };
                }
                if model.config.is_infinite_words() {
                    return Command::AppendWords { count: 1 };
                }
            }
        }

        Msg::Tick(elapsed) => {
            if model.session.status != TestStatus::Running {
                return Command::None;
            }
            model.session.elapsed = elapsed;
            // One snapshot per crossed second boundary
            if elapsed.as_secs() as usize > model.session.wpm_history.len() {
                let correct_words = metrics::count_correct_words(&model.session.words);
                let wpm = metrics::wpm(correct_words, elapsed);
                model.session.wpm_history.push(wpm);
                model.session.error_history.push(model.session.total_errors);
            }
            // Only expire in time mode with a finite limit.
            if matches!(model.config.test_mode, TestMode::Time)
                && !model.config.is_infinite_time()
                && elapsed >= model.config.time_limit
            {
                model.session.status = TestStatus::Done;
                model.screen = Screen::Results;
                return Command::SaveStats(build_stats_payload(model));
            }
        }

        Msg::ShiftTab => {
            if model.session.status == TestStatus::Running {
                return Command::None;
            }
            model.config.test_mode = match model.config.test_mode {
                TestMode::Time => TestMode::Words,
                TestMode::Words => TestMode::Time,
            };
            return regenerate(model);
        }

        Msg::Right => return cycle_selection(model, CycleDir::Next),

        Msg::Left => return cycle_selection(model, CycleDir::Prev),

        Msg::TogglePunctuation => {
            if model.session.status == TestStatus::Running {
                return Command::None;
            }
            model.config.punctuation = !model.config.punctuation;
            return regenerate(model);
        }

        Msg::ToggleNumbers => {
            if model.session.status == TestStatus::Running {
                return Command::None;
            }
            model.config.numbers = !model.config.numbers;
            return regenerate(model);
        }

        // Update notifications arrive asynchronously from the background version-check
        // thread. Store the version string so the view can display a banner; the
        // notification intentionally survives Tab restarts (the update is still available).
        Msg::UpdateAvailable(version) => {
            model.pending_update = Some(version);
        }
    }

    Command::None
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::domain::model::{Overlay, Screen, SessionState, TestStatus, Word};
    use crate::domain::test_config::{DURATION_OPTIONS, TestMode, WORD_COUNT_OPTIONS};

    fn model_with_words(words: &[&str]) -> Model {
        Model {
            session: SessionState::new(words.iter().map(|w| Word::new(w)).collect()),
            ..Model::default()
        }
    }

    #[test]
    fn esc_sets_should_quit() {
        let mut model = model_with_words(&["hello", "world"]);
        update(&mut model, Msg::Esc);
        assert!(model.should_quit);
    }

    #[test]
    fn end_test_while_running_goes_to_done() {
        let mut model = model_with_words(&["hello", "world"]);
        model.session.status = TestStatus::Running;
        update(&mut model, Msg::EndTest);
        assert_eq!(model.screen, Screen::Results);
        assert_eq!(model.session.status, TestStatus::Done);
    }

    #[test]
    fn end_test_while_waiting_is_ignored() {
        let mut model = model_with_words(&["hello", "world"]);
        assert_eq!(model.session.status, TestStatus::Waiting);
        update(&mut model, Msg::EndTest);
        assert_eq!(model.screen, Screen::Typing);
        assert_eq!(model.session.status, TestStatus::Waiting);
    }

    #[test]
    fn end_test_while_modal_open_is_swallowed() {
        let mut model = model_with_words(&["hello", "world"]);
        model.session.status = TestStatus::Running;
        model.overlay = Overlay::CustomTime {
            input: String::new(),
        };
        update(&mut model, Msg::EndTest);
        // overlay stays open, test still running
        assert!(model.overlay != Overlay::None);
        assert_eq!(model.session.status, TestStatus::Running);
        assert_eq!(model.screen, Screen::Typing);
    }

    #[test]
    fn char_transitions_waiting_to_running() {
        let mut model = model_with_words(&["hello"]);
        assert_eq!(model.session.status, TestStatus::Waiting);
        update(&mut model, Msg::Char('h'));
        assert_eq!(model.session.status, TestStatus::Running);
    }

    #[test]
    fn char_appends_to_current_word() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Char('e'));
        assert_eq!(model.session.words[0].typed, "he");
    }

    #[test]
    fn char_capped_at_word_length() {
        let mut model = model_with_words(&["hi"]);
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Char('i'));
        update(&mut model, Msg::Char('x')); // overtype — must be ignored
        assert_eq!(model.session.words[0].typed, "hi");
    }

    #[test]
    fn backspace_pops_last_char() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Char('e'));
        update(&mut model, Msg::Backspace);
        assert_eq!(model.session.words[0].typed, "h");
    }

    #[test]
    fn backspace_at_start_retreats_to_previous_word() {
        let mut model = model_with_words(&["hello", "world"]);
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Space); // commit "hello" (partially), advance to "world"
        assert_eq!(model.session.current_word, 1);
        update(&mut model, Msg::Backspace); // retreat back to "hello"
        assert_eq!(model.session.current_word, 0);
        // previous word is un-committed so it can be edited again
        assert!(!model.session.words[0].committed);
    }

    #[test]
    fn backspace_at_first_word_start_is_noop() {
        let mut model = model_with_words(&["hello"]);
        // typed is empty, current_word is 0 — nothing to retreat to
        update(&mut model, Msg::Backspace);
        assert_eq!(model.session.current_word, 0);
        assert!(model.session.words[0].typed.is_empty());
    }

    #[test]
    fn space_advances_to_next_word() {
        let mut model = model_with_words(&["hello", "world"]);
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Space);
        assert_eq!(model.session.current_word, 1);
        assert!(model.session.words[0].committed);
    }

    #[test]
    fn space_on_last_word_sets_done() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Space);
        assert_eq!(model.session.status, TestStatus::Done);
        assert_eq!(model.screen, Screen::Results);
    }

    #[test]
    fn last_char_of_last_word_auto_ends_test() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        update(&mut model, Msg::Char('h'));
        assert_eq!(model.session.status, TestStatus::Running);
        update(&mut model, Msg::Char('i'));
        assert_eq!(model.session.status, TestStatus::Done);
        assert_eq!(model.screen, Screen::Results);
        assert!(model.session.words[0].committed);
    }

    #[test]
    fn last_char_of_last_word_returns_save_stats_command() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        update(&mut model, Msg::Char('h'));
        let cmd = update(&mut model, Msg::Char('i'));
        assert!(matches!(cmd, Command::SaveStats(_)));
    }

    #[test]
    fn last_char_on_multi_word_test_auto_ends() {
        let mut model = model_with_words(&["go", "hi"]);
        model.config.test_mode = TestMode::Words;
        update(&mut model, Msg::Char('g'));
        update(&mut model, Msg::Space); // commit first word, advance
        update(&mut model, Msg::Char('h'));
        assert_eq!(model.session.status, TestStatus::Running);
        update(&mut model, Msg::Char('i')); // last char of last word
        assert_eq!(model.session.status, TestStatus::Done);
        assert_eq!(model.screen, Screen::Results);
    }

    #[test]
    fn tab_returns_generate_words_command() {
        let mut model = model_with_words(&["hello"]);
        let cmd = update(&mut model, Msg::Tab);
        assert!(matches!(cmd, Command::GenerateWords { .. }));
    }

    #[test]
    fn space_on_empty_typed_is_noop() {
        let mut model = model_with_words(&["hello", "world"]);
        // no chars typed — Space should be ignored
        update(&mut model, Msg::Space);
        assert_eq!(model.session.current_word, 0);
        assert!(!model.session.words[0].committed);
    }

    #[test]
    fn tab_resets_screen_to_typing() {
        let mut model = model_with_words(&["hi"]);
        model.screen = Screen::Results;
        update(&mut model, Msg::Tab);
        assert_eq!(model.screen, Screen::Typing);
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
    fn tick_before_running_is_noop() {
        let mut model = model_with_words(&["hello"]);
        assert_eq!(model.session.status, TestStatus::Waiting);
        update(&mut model, Msg::Tick(Duration::from_secs(5)));
        assert_eq!(model.session.status, TestStatus::Waiting);
        assert_eq!(model.screen, Screen::Typing);
        assert_eq!(model.session.elapsed, Duration::ZERO);
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
        assert_eq!(model.screen, Screen::Results);
    }

    #[test]
    fn tick_past_time_limit_transitions_to_done() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Tick(Duration::from_secs(16)));
        assert_eq!(model.session.status, TestStatus::Done);
        assert_eq!(model.screen, Screen::Results);
    }

    #[test]
    fn tick_after_done_is_noop() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Space);
        assert_eq!(model.screen, Screen::Results);
        let elapsed_before = model.session.elapsed;
        update(&mut model, Msg::Tick(Duration::from_secs(100)));
        assert_eq!(model.session.elapsed, elapsed_before);
        assert_eq!(model.screen, Screen::Results);
    }

    #[test]
    fn space_on_last_word_returns_save_stats_command() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        update(&mut model, Msg::Char('h'));
        let cmd = update(&mut model, Msg::Space);
        assert!(matches!(cmd, Command::SaveStats(_)));
    }

    #[test]
    fn tick_at_time_limit_returns_save_stats_command() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('h'));
        let cmd = update(&mut model, Msg::Tick(Duration::from_secs(15)));
        assert!(matches!(cmd, Command::SaveStats(_)));
    }

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
        update(&mut model, Msg::Right); // 2 → custom slot (3)
        assert_eq!(model.config.selected_duration_idx, DURATION_OPTIONS.len());
        // wrap back to 0
        update(&mut model, Msg::Right);
        assert_eq!(model.config.selected_duration_idx, 0);
        assert_eq!(model.config.time_limit, Duration::from_secs(15));
    }

    #[test]
    fn left_cycles_duration_backward() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Left); // 0 → custom slot (wraps to DURATION_OPTIONS.len())
        assert_eq!(model.config.selected_duration_idx, DURATION_OPTIONS.len());
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

    #[test]
    fn time_mode_last_char_appends_not_ends() {
        // In time mode, completing the last word should NOT end the test.
        // It should return AppendWords.
        let mut model = model_with_words(&["hi"]);
        // model_with_words uses TestConfig::default() which is TestMode::Time
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

    #[test]
    fn words_mode_last_char_ends_test() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        update(&mut model, Msg::Char('h'));
        let cmd = update(&mut model, Msg::Char('i'));
        assert_eq!(model.session.status, TestStatus::Done);
        assert_eq!(model.screen, Screen::Results);
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

    #[test]
    fn char_increments_total_chars_typed() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('h'));
        assert_eq!(model.session.total_chars_typed, 1);
        assert_eq!(model.session.total_errors, 0);
    }

    #[test]
    fn char_wrong_increments_total_errors() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('x')); // 'h' expected
        assert_eq!(model.session.total_chars_typed, 1);
        assert_eq!(model.session.total_errors, 1);
    }

    #[test]
    fn errors_not_decremented_on_backspace() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('x')); // wrong
        update(&mut model, Msg::Backspace); // correct it
        update(&mut model, Msg::Char('h')); // now correct
        // total_chars_typed counts keystrokes (not current buffer length)
        assert_eq!(model.session.total_chars_typed, 2);
        assert_eq!(model.session.total_errors, 1); // error persists despite correction
    }

    #[test]
    fn tick_snapshots_wpm_at_each_second_boundary() {
        let mut model = model_with_words(&["hello", "world"]);
        update(&mut model, Msg::Char('h')); // start running
        // First second boundary
        update(&mut model, Msg::Tick(Duration::from_secs(1)));
        assert_eq!(model.session.wpm_history.len(), 1);
        // Mid-second: no new snapshot
        update(&mut model, Msg::Tick(Duration::from_millis(1500)));
        assert_eq!(model.session.wpm_history.len(), 1);
        // Second boundary
        update(&mut model, Msg::Tick(Duration::from_secs(2)));
        assert_eq!(model.session.wpm_history.len(), 2);
    }

    #[test]
    fn save_stats_accuracy_counts_corrected_errors() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        update(&mut model, Msg::Char('x')); // wrong ('h' expected) → errors=1, typed=1
        update(&mut model, Msg::Backspace);
        update(&mut model, Msg::Char('h')); // correct → errors=1, typed=2
        let cmd = update(&mut model, Msg::Space); // commit, last word → Done + SaveStats
        // accuracy = (2 - 1) / 2 * 100 = 50.0
        if let Command::SaveStats(payload) = cmd {
            assert!((payload.accuracy - 50.0).abs() < 0.01);
        } else {
            panic!("expected SaveStats command");
        }
    }

    #[test]
    fn tick_snapshot_records_cumulative_error_count() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('x')); // wrong — total_errors becomes 1
        model.session.status = TestStatus::Running;
        model.session.total_errors = 3; // set directly to test recording
        update(&mut model, Msg::Tick(Duration::from_secs(1)));
        assert_eq!(model.session.error_history, vec![3]);
    }

    #[test]
    fn parse_empty_string_returns_zero() {
        assert_eq!(parse_custom_time(""), 0);
    }

    #[test]
    fn parse_plain_seconds() {
        assert_eq!(parse_custom_time("30"), 30);
        assert_eq!(parse_custom_time("0"), 0);
        assert_eq!(parse_custom_time("90"), 90);
    }

    #[test]
    fn parse_minutes_suffix() {
        assert_eq!(parse_custom_time("1m"), 60);
        assert_eq!(parse_custom_time("2m"), 120);
    }

    #[test]
    fn parse_hours_suffix() {
        assert_eq!(parse_custom_time("1h"), 3600);
    }

    #[test]
    fn parse_hours_and_minutes() {
        assert_eq!(parse_custom_time("1h30m"), 5400);
    }

    #[test]
    fn parse_monkeytype_example() {
        assert_eq!(parse_custom_time("10h5dm14asd3asdas2d"), 36024);
    }

    #[test]
    fn parse_trailing_number_is_seconds() {
        assert_eq!(parse_custom_time("1m30"), 90);
    }

    #[test]
    fn right_from_custom_slot_wraps_to_zero() {
        let mut model = model_with_words(&["hello"]);
        model.config.selected_duration_idx = DURATION_OPTIONS.len(); // custom slot
        update(&mut model, Msg::Right);
        assert_eq!(model.config.selected_duration_idx, 0);
        assert_eq!(
            model.config.time_limit,
            Duration::from_secs(DURATION_OPTIONS[0])
        );
    }

    #[test]
    fn right_to_custom_slot_applies_stored_value() {
        let mut model = model_with_words(&["hello"]);
        model.config.selected_duration_idx = DURATION_OPTIONS.len() - 1; // last preset
        model.config.custom_time_secs = Some(45);
        update(&mut model, Msg::Right); // → custom slot
        assert_eq!(model.config.selected_duration_idx, DURATION_OPTIONS.len());
        assert_eq!(model.config.time_limit, Duration::from_secs(45));
    }

    #[test]
    fn right_to_custom_slot_no_stored_value_keeps_time_limit() {
        let mut model = model_with_words(&["hello"]);
        model.config.selected_duration_idx = DURATION_OPTIONS.len() - 1;
        let limit_before = model.config.time_limit;
        update(&mut model, Msg::Right); // → custom slot, no value stored
        assert_eq!(model.config.selected_duration_idx, DURATION_OPTIONS.len());
        assert_eq!(model.config.time_limit, limit_before); // unchanged
    }

    #[test]
    fn right_cycles_word_count_to_custom_slot() {
        let mut model = model_with_words(&["hello"]);
        model.config.test_mode = TestMode::Words;
        model.config.selected_word_count_idx = WORD_COUNT_OPTIONS.len() - 1;
        update(&mut model, Msg::Right);
        assert_eq!(
            model.config.selected_word_count_idx,
            WORD_COUNT_OPTIONS.len()
        );
    }

    #[test]
    fn right_from_custom_word_slot_wraps_to_zero() {
        let mut model = model_with_words(&["hello"]);
        model.config.test_mode = TestMode::Words;
        model.config.selected_word_count_idx = WORD_COUNT_OPTIONS.len();
        update(&mut model, Msg::Right);
        assert_eq!(model.config.selected_word_count_idx, 0);
    }

    #[test]
    fn space_on_custom_time_slot_while_waiting_opens_modal() {
        let mut model = model_with_words(&["hello"]);
        model.config.selected_duration_idx = DURATION_OPTIONS.len();
        assert_eq!(model.session.status, TestStatus::Waiting);
        update(&mut model, Msg::Space);
        assert!(matches!(model.overlay, Overlay::CustomTime { .. }));
    }

    #[test]
    fn space_on_custom_words_slot_while_waiting_opens_modal() {
        let mut model = model_with_words(&["hello"]);
        model.config.test_mode = TestMode::Words;
        model.config.selected_word_count_idx = WORD_COUNT_OPTIONS.len();
        update(&mut model, Msg::Space);
        assert!(matches!(model.overlay, Overlay::CustomWords { .. }));
    }

    #[test]
    fn modal_char_appends_to_input() {
        let mut model = model_with_words(&["hello"]);
        model.overlay = Overlay::CustomTime {
            input: String::new(),
        };
        update(&mut model, Msg::Char('3'));
        update(&mut model, Msg::Char('0'));
        if let Overlay::CustomTime { input } = &model.overlay {
            assert_eq!(input, "30");
        } else {
            panic!("expected Overlay::CustomTime");
        }
    }

    #[test]
    fn modal_backspace_removes_char() {
        let mut model = model_with_words(&["hello"]);
        model.overlay = Overlay::CustomTime { input: "30".into() };
        update(&mut model, Msg::Backspace);
        if let Overlay::CustomTime { input } = &model.overlay {
            assert_eq!(input, "3");
        } else {
            panic!("expected Overlay::CustomTime");
        }
    }

    #[test]
    fn modal_esc_closes_without_applying() {
        let mut model = model_with_words(&["hello"]);
        model.config.custom_time_secs = None;
        model.overlay = Overlay::CustomTime { input: "45".into() };
        update(&mut model, Msg::Esc);
        assert_eq!(model.overlay, Overlay::None);
        assert!(model.config.custom_time_secs.is_none()); // unchanged
    }

    #[test]
    fn modal_space_applies_custom_time_and_closes() {
        let mut model = model_with_words(&["hello"]);
        model.config.selected_duration_idx = DURATION_OPTIONS.len();
        model.overlay = Overlay::CustomTime { input: "45".into() };
        update(&mut model, Msg::Space);
        assert_eq!(model.overlay, Overlay::None);
        assert_eq!(model.config.custom_time_secs, Some(45));
        assert_eq!(model.config.time_limit, Duration::from_secs(45));
    }

    #[test]
    fn modal_space_applies_custom_words_and_closes() {
        let mut model = model_with_words(&["hello"]);
        model.config.test_mode = TestMode::Words;
        model.config.selected_word_count_idx = WORD_COUNT_OPTIONS.len();
        model.overlay = Overlay::CustomWords { input: "42".into() };
        update(&mut model, Msg::Space);
        assert_eq!(model.overlay, Overlay::None);
        assert_eq!(model.config.custom_word_count, Some(42));
    }

    #[test]
    fn modal_space_zero_input_sets_infinite_time() {
        let mut model = model_with_words(&["hello"]);
        model.config.selected_duration_idx = DURATION_OPTIONS.len();
        model.overlay = Overlay::CustomTime { input: "0".into() };
        update(&mut model, Msg::Space);
        assert_eq!(model.config.custom_time_secs, Some(0));
    }

    #[test]
    fn modal_space_zero_input_sets_infinite_words() {
        let mut model = model_with_words(&["hello"]);
        model.config.test_mode = TestMode::Words;
        model.config.selected_word_count_idx = WORD_COUNT_OPTIONS.len();
        model.overlay = Overlay::CustomWords { input: "0".into() };
        update(&mut model, Msg::Space);
        assert_eq!(model.config.custom_word_count, Some(0));
    }

    #[test]
    fn infinite_time_tick_does_not_expire() {
        let mut model = model_with_words(&["hello"]);
        model.config.selected_duration_idx = DURATION_OPTIONS.len();
        model.config.custom_time_secs = Some(0);
        update(&mut model, Msg::Char('h')); // → Running
        update(&mut model, Msg::Tick(Duration::from_secs(9999)));
        assert_eq!(model.session.status, TestStatus::Running);
        assert_eq!(model.screen, Screen::Typing);
    }

    #[test]
    fn infinite_words_space_on_last_word_appends_not_ends() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        model.config.selected_word_count_idx = WORD_COUNT_OPTIONS.len();
        model.config.custom_word_count = Some(0);
        update(&mut model, Msg::Char('h'));
        let cmd = update(&mut model, Msg::Space);
        assert!(matches!(cmd, Command::AppendWords { count: 1 }));
        assert_eq!(model.session.status, TestStatus::Running);
    }

    #[test]
    fn infinite_words_last_char_appends_not_ends() {
        let mut model = model_with_words(&["hi"]);
        model.config.test_mode = TestMode::Words;
        model.config.selected_word_count_idx = WORD_COUNT_OPTIONS.len();
        model.config.custom_word_count = Some(0);
        update(&mut model, Msg::Char('h'));
        let cmd = update(&mut model, Msg::Char('i'));
        assert!(matches!(cmd, Command::AppendWords { count: 1 }));
        assert_eq!(model.session.status, TestStatus::Running);
    }

    #[test]
    fn build_stats_uses_elapsed_for_custom_time_slot() {
        let mut model = model_with_words(&["hi"]);
        model.config.selected_duration_idx = DURATION_OPTIONS.len();
        model.config.custom_time_secs = Some(45);
        model.config.time_limit = Duration::from_secs(45);
        update(&mut model, Msg::Char('h'));
        model.session.elapsed = Duration::from_secs(20);
        let cmd = update(&mut model, Msg::Tick(Duration::from_secs(45)));
        if let Command::SaveStats(payload) = cmd {
            assert_eq!(payload.duration_secs, 45);
        } else {
            panic!("expected SaveStats");
        }
    }

    #[test]
    fn toggle_punctuation_noop_while_running() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('h')); // → Running
        assert_eq!(model.session.status, TestStatus::Running);
        let flag_before = model.config.punctuation;
        let cmd = update(&mut model, Msg::TogglePunctuation);
        assert!(matches!(cmd, Command::None));
        assert_eq!(model.config.punctuation, flag_before);
    }

    #[test]
    fn toggle_numbers_noop_while_running() {
        let mut model = model_with_words(&["hello"]);
        update(&mut model, Msg::Char('h'));
        assert_eq!(model.session.status, TestStatus::Running);
        let flag_before = model.config.numbers;
        let cmd = update(&mut model, Msg::ToggleNumbers);
        assert!(matches!(cmd, Command::None));
        assert_eq!(model.config.numbers, flag_before);
    }

    #[test]
    fn toggle_punctuation_flips_flag_and_generates() {
        let mut model = model_with_words(&["hello"]);
        assert!(!model.config.punctuation);
        let cmd = update(&mut model, Msg::TogglePunctuation);
        assert!(model.config.punctuation);
        assert!(matches!(cmd, Command::GenerateWords { .. }));
        // toggle back
        let cmd2 = update(&mut model, Msg::TogglePunctuation);
        assert!(!model.config.punctuation);
        assert!(matches!(cmd2, Command::GenerateWords { .. }));
    }

    #[test]
    fn toggle_numbers_flips_flag_and_generates() {
        let mut model = model_with_words(&["hello"]);
        assert!(!model.config.numbers);
        let cmd = update(&mut model, Msg::ToggleNumbers);
        assert!(model.config.numbers);
        assert!(matches!(cmd, Command::GenerateWords { .. }));
        let cmd2 = update(&mut model, Msg::ToggleNumbers);
        assert!(!model.config.numbers);
        assert!(matches!(cmd2, Command::GenerateWords { .. }));
    }

    #[test]
    fn toggle_punctuation_on_done_screen_navigates_away() {
        let mut model = model_with_words(&["hello", "world"]);
        model.screen = Screen::Results;
        model.session.status = TestStatus::Done;
        let cmd = update(&mut model, Msg::TogglePunctuation);
        assert_ne!(
            model.screen,
            Screen::Results,
            "toggle must not leave screen on Done"
        );
        assert!(matches!(cmd, Command::GenerateWords { .. }));
    }

    #[test]
    fn toggle_numbers_on_done_screen_navigates_away() {
        let mut model = model_with_words(&["hello", "world"]);
        model.screen = Screen::Results;
        model.session.status = TestStatus::Done;
        let cmd = update(&mut model, Msg::ToggleNumbers);
        assert_ne!(
            model.screen,
            Screen::Results,
            "toggle must not leave screen on Done"
        );
        assert!(matches!(cmd, Command::GenerateWords { .. }));
    }
}

#[cfg(test)]
mod prop_tests {
    use std::time::Duration;

    use super::*;
    use crate::domain::model::{SessionState, Word};
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
            session: SessionState::new(words.iter().map(|w| Word::new(w)).collect()),
            ..Model::default()
        }
    }

    proptest! {
        #[test]
        fn current_word_stays_in_bounds(actions in prop::collection::vec(arb_msg(), 0..100)) {
            let mut model = model_with_words(&["hello", "world", "test", "kern", "rust"]);
            for msg in actions {
                update(&mut model, msg);
                // current_word must always index a valid word
                prop_assert!(model.session.current_word < model.session.words.len());
            }
        }

        #[test]
        fn typed_len_never_exceeds_word_len(actions in prop::collection::vec(arb_msg(), 0..100)) {
            let mut model = model_with_words(&["hi", "ok", "go", "be", "do"]);
            for msg in actions {
                update(&mut model, msg);
                for word in &model.session.words {
                    // Overtype cap must hold under any input sequence.
                    prop_assert!(word.typed.len() <= word.chars.len());
                }
            }
        }
    }
}
