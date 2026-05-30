use std::time::{Duration, Instant};

use anyhow::Result;
use rand::rngs::SmallRng;
use update_informer::{Check, registry};

use crate::domain::model::{Model, TestStatus};
use crate::domain::msg::Msg;
use crate::domain::update::update;
use crate::io::commands::{Command, execute_command};
use crate::io::persistence;
use crate::ui::view;
use crate::{config, input};

pub(crate) fn spawn_version_check(
    tx: std::sync::mpsc::Sender<String>,
    informer: impl Check + Send + 'static,
) {
    std::thread::spawn(move || {
        if let Some(version) = informer.check_version().ok().flatten() {
            let _ = tx.send(version.to_string());
        }
    });
}

pub(crate) fn run() -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let informer = update_informer::new(
        registry::Crates,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    );
    spawn_version_check(tx, informer);

    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal, rx);
    ratatui::restore();
    result
}

fn run_loop(
    terminal: &mut ratatui::DefaultTerminal,
    update_rx: std::sync::mpsc::Receiver<String>,
) -> Result<()> {
    let mut rng: SmallRng = rand::make_rng();
    let mut model = Model::default();
    match persistence::load() {
        Ok(history) => model.history = history,
        Err(e) => eprintln!("ktype: failed to load stats: {e}"),
    }
    if let Err(e) = config::write_if_missing() {
        eprintln!("ktype: failed to write default config: {e}");
    }
    config::apply_to_model(&mut model, config::load_or_default());
    // timer_start is infrastructure — not app state. Owned here alongside rng.
    let mut timer_start: Option<Instant> = None;

    let initial_count = model.config.initial_word_count();
    execute_command(
        &mut model,
        Command::GenerateWords {
            count: initial_count,
        },
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

        if let Ok(version) = update_rx.try_recv() {
            let cmd = update(&mut model, Msg::UpdateAvailable(version));
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

        if model.should_quit {
            break;
        }
    }

    Ok(())
}
