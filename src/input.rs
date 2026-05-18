use crate::model::Word;
use crate::msg::Msg;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

#[derive(Debug, Clone, PartialEq)]
pub enum CharState {
    Untyped,
    Correct,
    Incorrect,
}

// Derives the display state of word.chars[idx] from what was typed.
// State is never stored in the model — derived fresh on every render.
pub fn char_state(word: &Word, idx: usize) -> CharState {
    match word.typed.chars().nth(idx) {
        None => CharState::Untyped,
        Some(typed_c) => {
            if word.chars.get(idx) == Some(&typed_c) {
                CharState::Correct
            } else {
                CharState::Incorrect
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn make_word(text: &str, typed: &str) -> Word {
        let mut w = Word::new(text);
        w.typed = typed.to_string();
        w
    }

    // --- char_state ---

    #[test]
    fn untyped_when_not_yet_reached() {
        let word = make_word("hello", "he");
        assert_eq!(char_state(&word, 2), CharState::Untyped);
        assert_eq!(char_state(&word, 4), CharState::Untyped);
    }

    #[test]
    fn correct_when_typed_char_matches() {
        let word = make_word("hello", "he");
        assert_eq!(char_state(&word, 0), CharState::Correct);
        assert_eq!(char_state(&word, 1), CharState::Correct);
    }

    #[test]
    fn incorrect_when_typed_char_differs() {
        let word = make_word("hello", "hx");
        assert_eq!(char_state(&word, 1), CharState::Incorrect);
    }

    // --- event_to_msg ---

    fn key_press(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        })
    }

    fn key_release(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Release,
            state: crossterm::event::KeyEventState::empty(),
        })
    }

    #[test]
    fn char_key_maps_to_char_msg() {
        assert_eq!(
            event_to_msg(key_press(KeyCode::Char('a'))),
            Some(Msg::Char('a'))
        );
    }

    #[test]
    fn space_key_maps_to_space_msg() {
        assert_eq!(
            event_to_msg(key_press(KeyCode::Char(' '))),
            Some(Msg::Space)
        );
    }

    #[test]
    fn backspace_maps_to_backspace_msg() {
        assert_eq!(
            event_to_msg(key_press(KeyCode::Backspace)),
            Some(Msg::Backspace)
        );
    }

    #[test]
    fn tab_maps_to_tab_msg() {
        assert_eq!(event_to_msg(key_press(KeyCode::Tab)), Some(Msg::Tab));
    }

    #[test]
    fn esc_maps_to_esc_msg() {
        assert_eq!(event_to_msg(key_press(KeyCode::Esc)), Some(Msg::Esc));
    }

    #[test]
    fn key_release_returns_none() {
        // Only Press events produce messages — Release events are ignored.
        assert_eq!(event_to_msg(key_release(KeyCode::Char('a'))), None);
    }

    #[test]
    fn unknown_key_returns_none() {
        assert_eq!(event_to_msg(key_press(KeyCode::F(1))), None);
    }

    #[test]
    fn shift_tab_maps_to_shifttab_msg() {
        assert_eq!(
            event_to_msg(key_press(KeyCode::BackTab)),
            Some(Msg::ShiftTab)
        );
    }

    #[test]
    fn left_maps_to_left_msg() {
        assert_eq!(event_to_msg(key_press(KeyCode::Left)), Some(Msg::Left));
    }

    #[test]
    fn right_maps_to_right_msg() {
        assert_eq!(event_to_msg(key_press(KeyCode::Right)), Some(Msg::Right));
    }
}
