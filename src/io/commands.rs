use std::time::SystemTime;

use rand::rngs::SmallRng;

use crate::domain::model::{Model, SessionState};
use crate::generator;
use crate::io::persistence;
use crate::io::stats::SessionResult;

#[derive(Debug)]
pub struct StatsPayload {
    pub duration_secs: u64,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
}

#[derive(Debug)]
pub enum Command {
    None,
    GenerateWords { count: usize },
    AppendWords { count: usize },
    SaveStats(StatsPayload),
}

pub fn execute_command(model: &mut Model, cmd: Command, rng: &mut SmallRng) {
    match cmd {
        Command::None => {}
        Command::GenerateWords { count } => {
            let words = generator::generate(
                count,
                rng,
                model.config.punctuation,
                model.config.numbers,
                '.',
            );
            model.session = SessionState::new(words);
        }
        Command::AppendWords { count } => {
            let prev_last = model
                .session
                .words
                .last()
                .and_then(|w| w.chars.last().copied())
                .unwrap_or('.');
            let new_words = generator::generate(
                count,
                rng,
                model.config.punctuation,
                model.config.numbers,
                prev_last,
            );
            model.session.words.extend(new_words);
            if model.session.current_word + 1 < model.session.words.len()
                && model.session.words[model.session.current_word].committed
            {
                model.session.current_word += 1;
            }
        }
        Command::SaveStats(payload) => {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let result = SessionResult {
                timestamp,
                duration_secs: payload.duration_secs,
                wpm: payload.wpm,
                raw_wpm: payload.raw_wpm,
                accuracy: payload.accuracy,
            };
            model.history.push(result);
            if let Err(e) = persistence::append(model.history.last().unwrap()) {
                eprintln!("ktype: failed to save stats: {e}");
            }
        }
    }
}
