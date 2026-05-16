use crate::model::Word;
use rand::Rng;
use rand::seq::IndexedRandom;

const WORD_LIST: &[&str] = &[
    "the",
    "be",
    "to",
    "of",
    "and",
    "a",
    "in",
    "that",
    "have",
    "it",
    "for",
    "not",
    "on",
    "with",
    "he",
    "as",
    "you",
    "do",
    "at",
    "this",
    "but",
    "his",
    "by",
    "from",
    "they",
    "we",
    "say",
    "her",
    "she",
    "or",
    "an",
    "will",
    "my",
    "one",
    "all",
    "would",
    "there",
    "their",
    "what",
    "so",
    "up",
    "out",
    "if",
    "about",
    "who",
    "get",
    "which",
    "go",
    "me",
    "when",
    "make",
    "can",
    "like",
    "time",
    "no",
    "just",
    "him",
    "know",
    "take",
    "people",
    "into",
    "year",
    "your",
    "good",
    "some",
    "could",
    "them",
    "see",
    "other",
    "than",
    "then",
    "now",
    "look",
    "only",
    "come",
    "its",
    "over",
    "think",
    "also",
    "back",
    "after",
    "use",
    "two",
    "how",
    "our",
    "work",
    "first",
    "well",
    "way",
    "even",
    "new",
    "want",
    "because",
    "any",
    "these",
    "give",
    "day",
    "most",
    "us",
    "great",
    "between",
    "need",
    "large",
    "often",
    "hand",
    "high",
    "place",
    "hold",
    "turn",
    "been",
    "here",
    "why",
    "ask",
    "went",
    "men",
    "read",
    "land",
    "different",
    "home",
    "move",
    "try",
    "kind",
    "picture",
    "again",
    "change",
    "off",
    "play",
    "spell",
    "air",
    "away",
    "animal",
    "house",
    "point",
    "page",
    "letter",
    "mother",
    "answer",
    "found",
    "study",
    "still",
    "learn",
    "plant",
    "cover",
    "food",
    "sun",
    "four",
    "state",
    "keep",
    "eye",
    "never",
    "last",
    "let",
    "thought",
    "city",
    "tree",
    "cross",
    "farm",
    "hard",
    "start",
    "might",
    "story",
    "saw",
    "far",
    "sea",
    "draw",
    "left",
    "late",
    "run",
    "while",
    "press",
    "close",
    "night",
    "real",
    "life",
    "few",
    "open",
    "seem",
    "together",
    "next",
    "white",
    "children",
    "begin",
    "got",
    "walk",
    "example",
    "ease",
    "paper",
];

pub fn generate(count: usize, rng: &mut impl Rng) -> Vec<Word> {
    (0..count)
        .map(|_| {
            // choose panics on empty slice, which is impossible here since WORD_LIST is non-empty
            let word = WORD_LIST.choose(rng).unwrap();
            Word::new(word)
        })
        .collect()
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
        let words = generate(25, &mut rng);
        assert_eq!(words.len(), 25);
    }

    #[test]
    fn all_words_are_non_empty() {
        let mut rng = seeded_rng();
        let words = generate(10, &mut rng);
        assert!(words.iter().all(|w| !w.chars.is_empty()));
    }

    #[test]
    fn words_start_untyped() {
        let mut rng = seeded_rng();
        let words = generate(5, &mut rng);
        assert!(words.iter().all(|w| w.typed.is_empty() && !w.committed));
    }

    #[test]
    fn zero_count_returns_empty() {
        let mut rng = seeded_rng();
        assert_eq!(generate(0, &mut rng).len(), 0);
    }
}
