use std::time::Duration;

use crate::stats::SessionResult;

pub const DURATION_OPTIONS: [u64; 3] = [15, 30, 60];
pub const WORD_COUNT_OPTIONS: [usize; 4] = [10, 25, 50, 100];

#[derive(Debug, Clone, PartialEq)]
pub enum TestMode {
    Time,
    Words,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Typing,
    Done,
    Quitting,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Waiting,
    Running,
    Done,
}

#[derive(Debug, Clone)]
pub struct Word {
    pub chars: Vec<char>,
    pub typed: String,
    pub committed: bool,
}

impl Word {
    pub fn new(text: &str) -> Self {
        Word {
            chars: text.chars().collect(),
            typed: String::new(),
            committed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CursorStyle {
    Block,
    #[expect(dead_code)]
    Underline,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub test_mode: TestMode,
    pub cursor_style: CursorStyle,
    // time mode
    pub time_limit: Duration,
    // invariant: always a valid index into DURATION_OPTIONS
    pub selected_duration_idx: usize,
    // words mode
    pub word_count: usize,
    // invariant: always a valid index into WORD_COUNT_OPTIONS
    pub selected_word_count_idx: usize,
    #[expect(dead_code)]
    pub punctuation: bool,
    #[expect(dead_code)]
    pub numbers: bool,
}

impl Config {
    /// Words to generate on test start. Time mode uses a fixed buffer that
    /// grows dynamically; words mode uses the configured word count.
    pub fn initial_word_count(&self) -> usize {
        match self.test_mode {
            TestMode::Time => 50,
            TestMode::Words => self.word_count,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            test_mode: TestMode::Time,
            cursor_style: CursorStyle::Block,
            time_limit: Duration::from_secs(15),
            selected_duration_idx: 0,
            word_count: WORD_COUNT_OPTIONS[1], // 25
            selected_word_count_idx: 1,
            punctuation: false,
            numbers: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub words: Vec<Word>,
    pub current_word: usize,
    pub status: TestStatus,
    pub elapsed: Duration,
    pub total_chars_typed: u64,
    pub total_errors: u64,
    pub wpm_history: Vec<f64>,
    pub error_history: Vec<u64>,
}

impl SessionState {
    pub fn new(words: Vec<Word>) -> Self {
        SessionState {
            words,
            current_word: 0,
            status: TestStatus::Waiting,
            elapsed: Duration::ZERO,
            total_chars_typed: 0,
            total_errors: 0,
            wpm_history: Vec::new(),
            error_history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Model {
    pub screen: Screen,
    pub session: SessionState,
    pub config: Config,
    pub history: Vec<SessionResult>,
    pub pending_update: Option<String>,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            screen: Screen::Typing,
            session: SessionState::new(Vec::new()),
            config: Config::default(),
            history: Vec::new(),
            pending_update: None,
        }
    }
}
