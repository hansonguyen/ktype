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

pub fn count_correct_words(words: &[Word]) -> usize {
    words
        .iter()
        .filter(|w| w.committed && w.typed == w.chars.iter().collect::<String>())
        .count()
}

pub fn count_committed_words(words: &[Word]) -> usize {
    words.iter().filter(|w| w.committed).count()
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
    fn count_correct_words_committed_exact_match() {
        let words = vec![
            word("hello", "hello", true), // correct
            word("world", "world", true), // correct
        ];
        assert_eq!(count_correct_words(&words), 2);
    }

    #[test]
    fn count_correct_words_committed_with_mistake_excluded() {
        let words = vec![
            word("hello", "hellx", true), // committed but wrong
            word("world", "world", true), // correct
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

}
