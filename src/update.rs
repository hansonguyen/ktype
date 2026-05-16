use std::time::Duration;

use crate::commands::Command;
use crate::model::{Model, Screen, TestStatus, DURATION_OPTIONS};
use crate::msg::Msg;

pub fn update(model: &mut Model, msg: Msg) -> Command {
    match msg {
        Msg::Esc => {
            model.screen = Screen::Quitting;
        }

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

        Msg::Char(c) => {
            let session = &mut model.session;
            if session.words.is_empty() {
                return Command::None;
            }
            if session.status == TestStatus::Waiting {
                session.status = TestStatus::Running;
            }
            let word = &mut session.words[session.current_word];
            // Block input when the word is full — overtyping support is deferred to Phase 4.
            if word.typed.len() < word.chars.len() {
                word.typed.push(c);
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
            let session = &mut model.session;
            if session.words.is_empty() || session.words[session.current_word].typed.is_empty() {
                return Command::None;
            }
            let is_last = session.current_word == session.words.len() - 1;
            session.words[session.current_word].committed = true;

            if is_last {
                session.status = TestStatus::Done;
                model.screen = Screen::Done;
            } else {
                session.current_word += 1;
            }
        }

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
    }

    Command::None
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::model::{Config, SessionState, Word};

    fn model_with_words(words: &[&str]) -> Model {
        Model {
            screen: Screen::Typing,
            session: SessionState::new(words.iter().map(|w| Word::new(w)).collect()),
            config: Config::default(),
        }
    }

    #[test]
    fn esc_sets_quitting() {
        let mut model = model_with_words(&["hello", "world"]);
        update(&mut model, Msg::Esc);
        assert_eq!(model.screen, Screen::Quitting);
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
        update(&mut model, Msg::Char('h'));
        update(&mut model, Msg::Space);
        assert_eq!(model.session.status, TestStatus::Done);
        assert_eq!(model.screen, Screen::Done);
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
        model.screen = Screen::Done;
        update(&mut model, Msg::Tab);
        assert_eq!(model.screen, Screen::Typing);
    }

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
}

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
