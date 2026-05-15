use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::Paragraph,
};

pub struct App {
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self { should_quit: false }
    }

    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(crossterm::event::KeyEvent { kind: KeyEventKind::Press, code: KeyCode::Esc, .. }) = event {
            self.should_quit = true;
        }
        Ok(())
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        let vertical = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(area);

        let text = Paragraph::new("kern — press Esc to quit")
            .alignment(Alignment::Center);

        frame.render_widget(text, vertical[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

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
