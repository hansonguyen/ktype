use std::time::Duration;

use crate::domain::model::CaretStyle;

pub const DURATION_OPTIONS: [u64; 3] = [15, 30, 60];
pub const WORD_COUNT_OPTIONS: [usize; 4] = [10, 25, 50, 100];

#[derive(Debug, Clone, PartialEq)]
pub enum TestMode {
    Time,
    Words,
}

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub test_mode: TestMode,
    pub caret_style: CaretStyle,
    // time mode
    pub time_limit: Duration,
    // valid in 0..=DURATION_OPTIONS.len(); DURATION_OPTIONS.len() = custom slot
    pub selected_duration_idx: usize,
    pub custom_time_secs: Option<u64>,
    // words mode
    pub word_count: usize,
    // valid in 0..=WORD_COUNT_OPTIONS.len(); WORD_COUNT_OPTIONS.len() = custom slot
    pub selected_word_count_idx: usize,
    pub custom_word_count: Option<usize>,
    pub punctuation: bool,
    pub numbers: bool,
}

impl TestConfig {
    pub fn initial_word_count(&self) -> usize {
        match self.test_mode {
            TestMode::Time => 50,
            TestMode::Words => {
                if self.selected_word_count_idx == WORD_COUNT_OPTIONS.len() {
                    match self.custom_word_count {
                        None | Some(0) => 50,
                        Some(n) => n,
                    }
                } else {
                    self.word_count
                }
            }
        }
    }

    pub fn on_custom_time_slot(&self) -> bool {
        self.selected_duration_idx == DURATION_OPTIONS.len()
    }

    pub fn on_custom_words_slot(&self) -> bool {
        self.selected_word_count_idx == WORD_COUNT_OPTIONS.len()
    }

    pub fn is_infinite_words(&self) -> bool {
        self.on_custom_words_slot() && self.custom_word_count == Some(0)
    }

    pub fn is_infinite_time(&self) -> bool {
        self.on_custom_time_slot() && self.custom_time_secs == Some(0)
    }

    pub fn word_bank_label(&self) -> &'static str {
        match (self.punctuation, self.numbers) {
            (true, true) => "english + punctuation + numbers",
            (true, false) => "english + punctuation",
            (false, true) => "english + numbers",
            (false, false) => "english",
        }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            test_mode: TestMode::Time,
            caret_style: CaretStyle::Block,
            time_limit: Duration::from_secs(15),
            selected_duration_idx: 0,
            custom_time_secs: None,
            word_count: WORD_COUNT_OPTIONS[1], // 25
            selected_word_count_idx: 1,
            custom_word_count: None,
            punctuation: false,
            numbers: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_style_default_is_block() {
        let cfg = TestConfig::default();
        assert_eq!(cfg.caret_style, CaretStyle::Block);
    }

    #[test]
    fn custom_fields_default_to_none() {
        let cfg = TestConfig::default();
        assert!(cfg.custom_time_secs.is_none());
        assert!(cfg.custom_word_count.is_none());
    }

    #[test]
    fn initial_word_count_infinite_words_returns_50() {
        let cfg = TestConfig {
            test_mode: TestMode::Words,
            selected_word_count_idx: WORD_COUNT_OPTIONS.len(), // custom slot
            custom_word_count: Some(0),                        // infinite
            ..TestConfig::default()
        };
        assert_eq!(cfg.initial_word_count(), 50);
    }

    #[test]
    fn initial_word_count_custom_words_returns_count() {
        let cfg = TestConfig {
            test_mode: TestMode::Words,
            selected_word_count_idx: WORD_COUNT_OPTIONS.len(),
            custom_word_count: Some(42),
            ..TestConfig::default()
        };
        assert_eq!(cfg.initial_word_count(), 42);
    }
}
