use std::time::SystemTime;

use rand::rngs::SmallRng;

use crate::generator;
use crate::model::{Model, SessionState};
use crate::persistence;
use crate::stats::SessionResult;

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
            model.session = SessionState::new(generator::generate(count, rng));
        }
        Command::AppendWords { count } => {
            model.session.words.extend(generator::generate(count, rng));
            // Advance to the newly appended word if the current word is committed.
            // This handles the last-word case where update deferred the advance.
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
                eprintln!("kern: failed to save stats: {e}");
            }
        }
    }
}
