use std::time::Duration;

use crate::config::theme::Theme;
use crate::domain::test_config::TestConfig;
use crate::io::stats::SessionResult;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Typing,
    Results,
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

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaretStyle {
    Off,
    Default,
    #[default]
    Block,
    Underline,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    CustomTime { input: String },
    CustomWords { input: String },
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
    pub config: TestConfig,
    pub history: Vec<SessionResult>,
    pub pending_update: Option<String>,
    pub theme: Theme,
    pub overlay: Overlay,
    pub should_quit: bool,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            screen: Screen::Typing,
            session: SessionState::new(Vec::new()),
            config: TestConfig::default(),
            history: Vec::new(),
            pending_update: None,
            theme: Theme::default(),
            overlay: Overlay::None,
            should_quit: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_style_serializes_to_lowercase() {
        let s = serde_json::to_string(&CaretStyle::Underline).unwrap();
        assert_eq!(s, "\"underline\"");
        let s = serde_json::to_string(&CaretStyle::Off).unwrap();
        assert_eq!(s, "\"off\"");
    }

    #[test]
    fn caret_style_deserializes_from_lowercase() {
        let v: CaretStyle = serde_json::from_str("\"block\"").unwrap();
        assert_eq!(v, CaretStyle::Block);
        let v: CaretStyle = serde_json::from_str("\"underline\"").unwrap();
        assert_eq!(v, CaretStyle::Underline);
    }

    #[test]
    fn overlay_defaults_to_none() {
        let model = Model::default();
        assert_eq!(model.overlay, Overlay::None);
    }
}
