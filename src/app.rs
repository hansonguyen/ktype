use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;

pub struct App {
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self { should_quit: false }
    }

    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc {
                self.should_quit = true;
            }
        }
        Ok(())
    }

    pub fn draw(&self, _frame: &mut Frame) {
        // implemented in Task 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_app_is_not_quitting() {
        let app = App::new();
        assert!(!app.should_quit);
    }

    #[test]
    fn esc_sets_should_quit() {
        let mut app = App::new();
        let event = Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        });
        app.handle_event(event).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn other_key_does_not_quit() {
        let mut app = App::new();
        let event = Event::Key(KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        });
        app.handle_event(event).unwrap();
        assert!(!app.should_quit);
    }

    #[test]
    fn key_release_does_not_quit() {
        let mut app = App::new();
        let event = Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Release,
            state: crossterm::event::KeyEventState::empty(),
        });
        app.handle_event(event).unwrap();
        assert!(!app.should_quit);
    }
}
