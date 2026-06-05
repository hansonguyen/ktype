use std::time::Duration;

use crate::domain::model::Word;

#[derive(Debug, Clone, PartialEq)]
pub struct CharBreakdown {
    pub correct: usize,
    pub incorrect: usize,
    pub extra: usize,
    pub missed: usize,
}

pub fn char_breakdown(words: &[Word]) -> CharBreakdown {
    let mut correct = 0;
    let mut incorrect = 0;
    let mut extra = 0;
    let mut missed = 0;

    for word in words {
        if !word.committed && word.typed.is_empty() {
            continue;
        }
        let typed: Vec<char> = word.typed.chars().collect();
        let expected_len = word.chars.len();
        let typed_len = typed.len();

        for (i, &typed_char) in typed.iter().enumerate().take(typed_len.min(expected_len)) {
            if typed_char == word.chars[i] {
                correct += 1;
            } else {
                incorrect += 1;
            }
        }
        extra += typed_len.saturating_sub(expected_len);
        missed += expected_len.saturating_sub(typed_len);
    }

    CharBreakdown {
        correct,
        incorrect,
        extra,
        missed,
    }
}

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

pub fn raw_accuracy(total_chars_typed: u64, total_errors: u64) -> f64 {
    if total_chars_typed == 0 {
        return 0.0;
    }
    (total_chars_typed - total_errors) as f64 / total_chars_typed as f64 * 100.0
}

/// Consistency as Monkeytype computes it: map the coefficient of variation of
/// per-second raw-WPM samples through `kogasa`. `chars_history` is the cumulative
/// typed-char count per second; consecutive differences are the per-second raw
/// samples (`delta_chars / 5 * 60`). Returns a percentage in `[0, 100]`.
pub fn consistency(chars_history: &[u64]) -> f64 {
    if chars_history.is_empty() {
        return 0.0;
    }
    let mut prev = 0u64;
    let samples: Vec<f64> = chars_history
        .iter()
        .map(|&cumulative| {
            let delta = cumulative.saturating_sub(prev);
            prev = cumulative;
            delta as f64 / 5.0 * 60.0
        })
        .collect();

    let avg = mean(&samples);
    if avg == 0.0 {
        // cov would be NaN; Monkeytype's isNaN guard maps this to 0.
        return 0.0;
    }
    let cov = std_dev(&samples) / avg;
    let value = round_to_2(kogasa(cov));
    if value.is_nan() { 0.0 } else { value }
}

fn mean(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f64>() / samples.len() as f64
}

fn std_dev(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let m = mean(samples);
    let variance = samples.iter().map(|x| (x - m).powi(2)).sum::<f64>() / samples.len() as f64;
    variance.sqrt()
}

/// Maps a coefficient of variation in `[0, +inf)` to consistency in `(0, 100]`.
/// A tanh-based sigmoid that stays close to the identity on `[0, 1)`.
fn kogasa(cov: f64) -> f64 {
    100.0 * (1.0 - (cov + cov.powi(3) / 3.0 + cov.powi(5) / 5.0).tanh())
}

/// Mirrors Monkeytype's `roundTo2`: round half-up to two decimals.
fn round_to_2(num: f64) -> f64 {
    ((num + f64::EPSILON) * 100.0).round() / 100.0
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
    use crate::domain::model::Word;

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

    #[test]
    fn consistency_empty_history_is_zero() {
        assert_eq!(consistency(&[]), 0.0);
    }

    #[test]
    fn consistency_single_sample_is_full() {
        // One window => stddev 0 => cov 0 => kogasa(0) = 100.
        assert!((consistency(&[60]) - 100.0).abs() < 0.01);
    }

    #[test]
    fn consistency_constant_per_second_rate_is_full() {
        // Cumulative [60, 120, 180] => equal deltas => perfectly consistent.
        assert!((consistency(&[60, 120, 180]) - 100.0).abs() < 0.01);
    }

    #[test]
    fn consistency_all_zero_history_is_zero() {
        // No chars typed => mean 0 => cov NaN => guarded to 0 (mirrors Monkeytype).
        assert_eq!(consistency(&[0, 0, 0]), 0.0);
    }

    #[test]
    fn consistency_varied_rate_is_between_zero_and_full() {
        // Cumulative [5, 15] => deltas [5, 10] => raw [60, 120] => cov = 1/3 => kogasa ~= 66.68.
        let c = consistency(&[5, 15]);
        assert!(c > 0.0 && c < 100.0);
        assert!((c - 66.68).abs() < 0.5);
    }

    #[test]
    fn char_breakdown_all_correct() {
        let b = char_breakdown(&[word("hi", "hi", true)]);
        assert_eq!(
            b,
            CharBreakdown {
                correct: 2,
                incorrect: 0,
                extra: 0,
                missed: 0
            }
        );
    }

    #[test]
    fn char_breakdown_all_incorrect() {
        let b = char_breakdown(&[word("hi", "xx", true)]);
        assert_eq!(
            b,
            CharBreakdown {
                correct: 0,
                incorrect: 2,
                extra: 0,
                missed: 0
            }
        );
    }

    #[test]
    fn char_breakdown_partial_commit_counts_missed() {
        // User pressed space before finishing "hello"
        let b = char_breakdown(&[word("hello", "hel", true)]);
        assert_eq!(
            b,
            CharBreakdown {
                correct: 3,
                incorrect: 0,
                extra: 0,
                missed: 2
            }
        );
    }

    #[test]
    fn char_breakdown_extra_chars_counted() {
        let b = char_breakdown(&[word("hi", "hixxx", true)]);
        assert_eq!(
            b,
            CharBreakdown {
                correct: 2,
                incorrect: 0,
                extra: 3,
                missed: 0
            }
        );
    }

    #[test]
    fn char_breakdown_in_progress_word_included() {
        // Non-committed word with partial typing (current active word at test end)
        let b = char_breakdown(&[word("be", "b", false)]);
        assert_eq!(
            b,
            CharBreakdown {
                correct: 1,
                incorrect: 0,
                extra: 0,
                missed: 1
            }
        );
    }

    #[test]
    fn char_breakdown_untouched_buffered_words_excluded() {
        let words = vec![Word::new("hello"), Word::new("world")];
        let b = char_breakdown(&words);
        assert_eq!(
            b,
            CharBreakdown {
                correct: 0,
                incorrect: 0,
                extra: 0,
                missed: 0
            }
        );
    }

    #[test]
    fn char_breakdown_mixed_session() {
        let words = vec![
            word("hi", "hi", true),   // correct: 2
            word("ok", "ox", true),   // correct: 1 (o=o), incorrect: 1 (x≠k)
            word("go", "goxx", true), // correct: 2 (g,o), extra: 2 (xx)
            word("be", "b", false),   // correct: 1 (b), missed: 1 (e)
            Word::new("do"),          // untouched — excluded
        ];
        let b = char_breakdown(&words);
        assert_eq!(
            b,
            CharBreakdown {
                correct: 6,
                incorrect: 1,
                extra: 2,
                missed: 1
            }
        );
    }
}
