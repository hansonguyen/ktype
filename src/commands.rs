use rand::rngs::SmallRng;

use crate::generator;
use crate::model::{Model, SessionState};

#[derive(Debug)]
pub enum Command {
    None,
    GenerateWords { count: usize },
}

// The only place side effects happen. update() returns a Command; main.rs calls this.
// rng lives in main.rs (seeded once at startup) and is passed in here — it is
// infrastructure, not app state.
pub fn execute_command(model: &mut Model, cmd: Command, rng: &mut SmallRng) {
    match cmd {
        Command::None => {}
        Command::GenerateWords { count } => {
            model.session = SessionState::new(generator::generate(count, rng));
        }
    }
}
