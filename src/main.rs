mod app;

use anyhow::Result;
use app::App;
use std::time::Duration;

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if crossterm::event::poll(Duration::from_millis(16))? {
            app.handle_event(crossterm::event::read()?)?;
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
