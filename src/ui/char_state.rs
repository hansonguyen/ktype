use crate::domain::model::Word;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CharState {
    Untyped,
    Correct,
    Incorrect,
}

// Derives the display state of word.chars[idx] from what was typed.
// State is never stored in the model — derived fresh on every render.
pub(crate) fn char_state(word: &Word, idx: usize) -> CharState {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_word(text: &str, typed: &str) -> Word {
        let mut w = Word::new(text);
        w.typed = typed.to_string();
        w
    }

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
}
