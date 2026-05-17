mod commands;
mod generator;
mod input;
mod metrics;
mod model;
mod msg;
mod stats;
mod update;
mod view;

use std::time::{Duration, Instant};

use anyhow::Result;
use commands::{Command, execute_command};
use model::{Model, TestStatus};
use msg::Msg;
use rand::rngs::SmallRng;
use update::update;
use view::view;

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut rng: SmallRng = rand::make_rng();
    let mut model = Model::default();
    // timer_start is infrastructure — not app state. Owned here alongside rng.
    let mut timer_start: Option<Instant> = None;

    let word_count = model.config.word_count;
    execute_command(
        &mut model,
        Command::GenerateWords { count: word_count },
        &mut rng,
    );

    loop {
        terminal.draw(|frame| view(&model, frame))?;

        // Process one pending input event (16ms timeout = ~60fps frame budget).
        if crossterm::event::poll(Duration::from_millis(16))?
            && let Some(msg) = input::event_to_msg(crossterm::event::read()?)
        {
            let cmd = update(&mut model, msg);
            execute_command(&mut model, cmd, &mut rng);
        }

        // Start timer on Waiting → Running transition.
        if timer_start.is_none() && model.session.status == TestStatus::Running {
            timer_start = Some(Instant::now());
        }
        // Clear timer when session resets to Waiting (Tab restart).
        if timer_start.is_some() && model.session.status == TestStatus::Waiting {
            timer_start = None;
        }
        // Freeze timer when test finishes so elapsed is stable on the results screen.
        if timer_start.is_some() && model.session.status == TestStatus::Done {
            timer_start = None;
        }

        // Drive countdown — fire Tick every frame with current elapsed.
        let elapsed = timer_start.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
        let cmd = update(&mut model, Msg::Tick(elapsed));
        execute_command(&mut model, cmd, &mut rng);

        if model.screen == model::Screen::Quitting {
            break;
        }
    }

    Ok(())
}
