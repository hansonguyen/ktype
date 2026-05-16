mod commands;
mod generator;
mod input;
mod model;
mod msg;
mod update;
mod view;

use anyhow::Result;
use commands::{Command, execute_command};
use model::Model;
use rand::rngs::SmallRng;
use std::time::Duration;
use update::update;
use view::view;

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    // SmallRng is seeded once here and passed into execute_command for all word generation.
    // It is infrastructure, not app state — kept out of Model intentionally.
    let mut rng: SmallRng = rand::make_rng();
    let mut model = Model::default();

    // Populate words before the first frame.
    let word_count = model.config.word_count;
    execute_command(
        &mut model,
        Command::GenerateWords { count: word_count },
        &mut rng,
    );

    loop {
        terminal.draw(|frame| view(&model, frame))?;

        if crossterm::event::poll(Duration::from_millis(16))? {
            if let Some(msg) = input::event_to_msg(crossterm::event::read()?) {
                let cmd = update(&mut model, msg);
                execute_command(&mut model, cmd, &mut rng);
            }
        }

        if model.screen == model::Screen::Quitting {
            break;
        }
    }

    Ok(())
}
