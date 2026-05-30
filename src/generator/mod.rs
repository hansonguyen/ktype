mod word_list;

use crate::domain::model::Word;
use rand::Rng;
use rand::RngExt;
use rand::seq::IndexedRandom;
use word_list::{CONTRACTIONS, WORD_LIST};

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn try_contract(word: &str, rng: &mut impl Rng) -> Option<String> {
    CONTRACTIONS
        .iter()
        .find(|(base, _)| *base == word)
        .map(|(_, forms)| forms.choose(rng).unwrap().to_string())
}

fn punctuate(
    raw: String,
    last_char: char,
    index: usize,
    count: usize,
    prev_was_hyphen: bool,
    rng: &mut impl Rng,
) -> String {
    // 1. Capitalize: word following a sentence-ending mark.
    //    index==0 is NOT special here; GenerateWords seeds prev_last_char='.' so the first
    //    word of a fresh batch is still capitalized via this branch.
    if ".?!".contains(last_char) {
        return capitalize(&raw);
    }

    // 2. Sentence end: forced on the last word of a multi-word batch, or ~10% chance.
    //    Skipped for the second-to-last word (monkeytype avoids consecutive end marks).
    //    "Force on last" only fires for count > 1; AppendWords always uses count=1 so never forces.
    let force_end = count > 1 && index == count - 1;
    if force_end
        || (rng.random::<f64>() < 0.1 && last_char != '.' && last_char != ',' && index + 2 != count)
    {
        let r: f64 = rng.random();
        let suffix = if r <= 0.8 {
            "."
        } else if r < 0.9 {
            "?"
        } else {
            "!"
        };
        return format!("{}{}", raw, suffix);
    }

    // 3. Double quotes ~1%
    if rng.random::<f64>() < 0.01 && last_char != ',' && last_char != '.' {
        return format!("\"{}\"", raw);
    }

    // 4. Single quotes ~1.1%
    if rng.random::<f64>() < 0.011 && last_char != ',' && last_char != '.' {
        return format!("'{}'", raw);
    }

    // 5. Parentheses ~1.2%
    if rng.random::<f64>() < 0.012 && last_char != ',' && last_char != '.' {
        return format!("({})", raw);
    }

    // 6. Colon ~1.3%
    if rng.random::<f64>() < 0.013
        && last_char != ','
        && last_char != '.'
        && last_char != ';'
        && last_char != ':'
    {
        return format!("{}:", raw);
    }

    // 7. Hyphen ~1.4% — standalone word, never consecutive (last_char != '-' guards cross-call)
    if rng.random::<f64>() < 0.014
        && last_char != ','
        && last_char != '.'
        && last_char != '-'
        && !prev_was_hyphen
    {
        return "-".to_string();
    }

    // 8. Semicolon ~1.5%
    if rng.random::<f64>() < 0.015 && last_char != ',' && last_char != '.' && last_char != ';' {
        return format!("{};", raw);
    }

    // 9. Comma ~20%
    if rng.random::<f64>() < 0.2 && last_char != ',' {
        return format!("{},", raw);
    }

    // 10. English contraction ~50% if word is eligible
    if rng.random::<f64>() < 0.5 {
        if let Some(contracted) = try_contract(&raw, rng) {
            return contracted;
        }
    }

    raw
}

pub fn generate(
    count: usize,
    rng: &mut impl Rng,
    punctuation: bool,
    numbers: bool,
    prev_last_char: char,
) -> Vec<Word> {
    let mut words = Vec::with_capacity(count);
    let mut last_char = prev_last_char;
    let mut prev_was_hyphen = false;

    for i in 0..count {
        let raw: String = if numbers && rng.random::<f64>() < 0.1 {
            generate_number(rng)
        } else {
            // choose panics on empty slice, which is impossible here since WORD_LIST is non-empty
            WORD_LIST.choose(rng).unwrap().to_string()
        };

        let word_str = if punctuation {
            punctuate(raw, last_char, i, count, prev_was_hyphen, rng)
        } else {
            raw
        };

        prev_was_hyphen = word_str == "-";
        last_char = word_str.chars().last().unwrap_or('.');
        words.push(Word::new(&word_str));
    }

    words
}

fn generate_number(rng: &mut impl Rng) -> String {
    let len = rng.random_range(1usize..=4);
    let mut s = String::with_capacity(len);
    s.push((b'0' + rng.random_range(1u8..=9)) as char);
    for _ in 1..len {
        s.push((b'0' + rng.random_range(0u8..=9)) as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn seeded_rng() -> SmallRng {
        SmallRng::seed_from_u64(42)
    }

    #[test]
    fn returns_correct_word_count() {
        let mut rng = seeded_rng();
        let words = generate(25, &mut rng, false, false, '.');
        assert_eq!(words.len(), 25);
    }

    #[test]
    fn all_words_are_non_empty() {
        let mut rng = seeded_rng();
        let words = generate(10, &mut rng, false, false, '.');
        assert!(words.iter().all(|w| !w.chars.is_empty()));
    }

    #[test]
    fn words_start_untyped() {
        let mut rng = seeded_rng();
        let words = generate(5, &mut rng, false, false, '.');
        assert!(words.iter().all(|w| w.typed.is_empty() && !w.committed));
    }

    #[test]
    fn zero_count_returns_empty() {
        let mut rng = seeded_rng();
        assert_eq!(generate(0, &mut rng, false, false, '.').len(), 0);
    }

    #[test]
    fn numbers_flag_off_produces_no_digit_words() {
        let mut rng = seeded_rng();
        let words = generate(50, &mut rng, false, false, '.');
        let any_all_digits = words
            .iter()
            .any(|w| !w.chars.is_empty() && w.chars.iter().all(|c| c.is_ascii_digit()));
        assert!(
            !any_all_digits,
            "numbers=false must not produce digit-only words"
        );
    }

    #[test]
    fn numbers_flag_produces_digit_strings() {
        let mut rng = seeded_rng();
        let words = generate(200, &mut rng, false, true, '.');
        let has_number = words
            .iter()
            .any(|w| !w.chars.is_empty() && w.chars.iter().all(|c| c.is_ascii_digit()));
        assert!(
            has_number,
            "expected at least one numeric word in 200 words with numbers=true"
        );
    }

    #[test]
    fn numbers_never_start_with_zero() {
        let mut rng = seeded_rng();
        let words = generate(200, &mut rng, false, true, '.');
        for word in &words {
            let s: String = word.chars.iter().collect();
            if s.chars().all(|c| c.is_ascii_digit()) {
                assert_ne!(
                    word.chars.first().copied(),
                    Some('0'),
                    "number {:?} must not start with 0",
                    s
                );
            }
        }
    }

    #[test]
    fn numbers_are_one_to_four_digits() {
        let mut rng = seeded_rng();
        let words = generate(200, &mut rng, false, true, '.');
        for word in &words {
            let s: String = word.chars.iter().collect();
            if s.chars().all(|c| c.is_ascii_digit()) {
                assert!(
                    (1..=4).contains(&s.len()),
                    "number {:?} has {} digits, expected 1–4",
                    s,
                    s.len()
                );
            }
        }
    }

    #[test]
    fn punctuation_off_first_word_lowercase() {
        let mut rng = seeded_rng();
        let words = generate(10, &mut rng, false, false, '.');
        let first: String = words[0].chars.iter().collect();
        // without punctuation, words stay lowercase
        assert!(
            first
                .chars()
                .next()
                .map(|c| c.is_lowercase())
                .unwrap_or(false),
            "punctuation=false must keep words lowercase, got {:?}",
            first
        );
    }

    #[test]
    fn punctuation_first_word_is_capitalized() {
        let mut rng = seeded_rng();
        let words = generate(10, &mut rng, true, false, '.');
        let first: String = words[0].chars.iter().collect();
        assert!(
            first
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false),
            "first word must be capitalized when punctuation=true, got {:?}",
            first
        );
    }

    #[test]
    fn punctuation_capitalizes_after_sentence_end() {
        // In any 200-word punctuated batch, every word that follows a .?! word must be capitalized.
        let mut rng = seeded_rng();
        let words = generate(200, &mut rng, true, false, '.');
        for i in 1..words.len() {
            let prev: String = words[i - 1].chars.iter().collect();
            let curr: String = words[i].chars.iter().collect();
            if let Some(last) = prev.chars().last() {
                if ".?!".contains(last) {
                    assert!(
                        curr.chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false),
                        "word after {:?} should be capitalized, got {:?}",
                        prev,
                        curr
                    );
                }
            }
        }
    }

    #[test]
    fn punctuation_produces_commas_and_sentence_ends() {
        let mut rng = seeded_rng();
        let words = generate(100, &mut rng, true, false, '.');
        let any_comma = words.iter().any(|w| w.chars.last() == Some(&','));
        let any_sentence_end = words
            .iter()
            .any(|w| matches!(w.chars.last().copied(), Some('.') | Some('?') | Some('!')));
        assert!(
            any_comma,
            "expected at least one comma in 100 punctuated words"
        );
        assert!(
            any_sentence_end,
            "expected at least one sentence-ending mark in 100 punctuated words"
        );
    }

    #[test]
    fn punctuation_last_word_ends_with_sentence_mark() {
        // In a multi-word fresh batch, the last word must end with .?!
        let mut rng = seeded_rng();
        let words = generate(50, &mut rng, true, false, '.');
        let last: String = words.last().unwrap().chars.iter().collect();
        assert!(
            last.ends_with('.') || last.ends_with('?') || last.ends_with('!'),
            "last word of fresh batch must end with a sentence mark, got {:?}",
            last
        );
    }

    #[test]
    fn appended_word_not_capitalized_mid_sentence() {
        // AppendWords passes count=1; prev_last=',' means mid-sentence.
        // The word must not be capitalized since ',' is not a sentence-ending mark.
        for seed in 0..100u64 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let words = generate(1, &mut rng, true, false, ',');
            let w: String = words[0].chars.iter().collect();
            assert!(
                w.chars().next().map(|c| !c.is_uppercase()).unwrap_or(true),
                "seed {}: mid-sentence appended word must not be capitalized, got {:?}",
                seed,
                w
            );
        }
    }

    #[test]
    fn punctuation_last_word_ends_sentence_across_seeds() {
        // force_end must fire even when the penultimate word already ended with .?!
        for seed in 0..500u64 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let words = generate(10, &mut rng, true, false, '.');
            let last: String = words.last().unwrap().chars.iter().collect();
            assert!(
                last.ends_with('.') || last.ends_with('?') || last.ends_with('!'),
                "seed {}: last word must end with sentence mark, got {:?}",
                seed,
                last
            );
        }
    }

    #[test]
    fn no_consecutive_hyphen_words() {
        // After a hyphen word (prev_last='-'), generate must not produce another hyphen.
        for seed in 0..500u64 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let words = generate(1, &mut rng, true, false, '-');
            let w: String = words[0].chars.iter().collect();
            assert_ne!(
                w, "-",
                "seed {}: consecutive hyphen words must be prevented",
                seed
            );
        }
    }

    #[test]
    fn punctuation_numbers_first_word_capitalized() {
        // When both numbers=true and punctuation=true, the first word must be capitalized
        // or be a digit. This test catches the bug where a number like "42" can appear
        // as the first word without any capitalization (since digits have no uppercase form).
        for seed in 0..1000u64 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let words = generate(10, &mut rng, true, true, '.');
            let first: String = words[0].chars.iter().collect();
            assert!(
                first
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase() || c.is_ascii_digit())
                    .unwrap_or(false),
                "first word with numbers=true, punctuation=true must be capitalized or digit, got {:?}",
                first
            );
        }
    }
}

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
            prop_assert_eq!(generate(count, &mut rng, false, false, '.').len(), count);
        }

        #[test]
        fn generate_all_non_empty(count in 1usize..=50, seed in any::<[u8; 32]>()) {
            let mut rng = SmallRng::from_seed(seed);
            let words = generate(count, &mut rng, false, false, '.');
            for word in &words {
                prop_assert!(!word.chars.is_empty(), "word had empty chars");
            }
        }

        #[test]
        fn generate_all_lowercase_ascii(count in 1usize..=50, seed in any::<[u8; 32]>()) {
            let mut rng = SmallRng::from_seed(seed);
            // punctuation=false, numbers=false: all chars must remain lowercase ASCII
            let words = generate(count, &mut rng, false, false, '.');
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
