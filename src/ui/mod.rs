mod char_state;
mod chart;
mod modal;
mod results;
mod typing;
mod widgets;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::Span,
    widgets::{Block, Paragraph},
};

use crate::domain::model::{Model, Overlay, Screen, TestStatus};
use widgets::fg;

pub fn view(model: &Model, frame: &mut Frame) {
    frame.render_widget(
        Block::new().style(Style::new().bg(model.theme.bg.to_ratatui_color())),
        frame.area(),
    );

    match model.screen {
        Screen::Results => results::render_results(model, frame),
        Screen::Typing => typing::render_typing(model, frame),
    }

    if model.screen == Screen::Typing
        && model.session.status == TestStatus::Waiting
        && let Some(version) = &model.pending_update
    {
        let area = frame.area();
        let banner_area = Rect {
            x: 0,
            y: area.bottom().saturating_sub(1),
            width: area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("new version {version} available — cargo install ktype"),
                fg(&model.theme.sub),
            ))
            .alignment(Alignment::Center),
            banner_area,
        );
    }

    if model.overlay != Overlay::None {
        modal::render_modal(model, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::{Model, Screen, SessionState, Word};
    use crate::domain::test_config::DURATION_OPTIONS;
    use std::time::Duration;

    fn test_model(words: &[&str], current_word: usize, typed: &[&str]) -> Model {
        let mut session_words: Vec<Word> = words.iter().map(|w| Word::new(w)).collect();
        for (i, t) in typed.iter().enumerate() {
            if let Some(w) = session_words.get_mut(i) {
                w.typed = t.to_string();
            }
        }
        Model {
            screen: Screen::Typing,
            session: SessionState {
                words: session_words,
                current_word,
                status: crate::domain::model::TestStatus::Running,
                elapsed: Duration::ZERO,
                total_chars_typed: 0,
                total_errors: 0,
                wpm_history: Vec::new(),
                error_history: Vec::new(),
            },
            ..Model::default()
        }
    }

    fn render_to_string(model: &Model, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| view(model, frame)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let area = buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn typing_screen_renders_without_panic() {
        let model = test_model(&["hello", "world"], 0, &["hel"]);
        render_to_string(&model, 80, 24);
    }

    #[test]
    fn done_screen_renders_without_panic() {
        let mut model = test_model(&["hi"], 0, &["hi"]);
        model.screen = Screen::Results;
        render_to_string(&model, 80, 24);
    }

    #[test]
    fn typing_screen_snapshot() {
        let model = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn typing_screen_waiting_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::domain::model::TestStatus::Waiting;
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn punctuation_on_header_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::domain::model::TestStatus::Waiting;
        model.config.punctuation = true;
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("punctuation_on_header", output);
    }

    #[test]
    fn numbers_on_header_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::domain::model::TestStatus::Waiting;
        model.config.numbers = true;
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("numbers_on_header", output);
    }

    #[test]
    fn idle_screen_update_banner_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::domain::model::TestStatus::Waiting;
        model.pending_update = Some("v1.0.0".into());
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn results_screen_snapshot() {
        let words = vec![
            {
                let mut w = Word::new("the");
                w.typed = "the".to_string();
                w.committed = true;
                w
            },
            {
                let mut w = Word::new("quick");
                w.typed = "quikc".to_string(); // wrong — not counted in wpm
                w.committed = true;
                w
            },
            {
                let mut w = Word::new("brown");
                w.typed = "brown".to_string();
                w.committed = true;
                w
            },
        ];
        let model = Model {
            screen: Screen::Results,
            session: SessionState {
                words,
                current_word: 2,
                status: crate::domain::model::TestStatus::Done,
                elapsed: Duration::from_secs(10),
                total_chars_typed: 0,
                total_errors: 0,
                wpm_history: Vec::new(),
                error_history: Vec::new(),
            },
            ..Model::default()
        };
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn results_screen_punctuation_numbers_label_snapshot() {
        let words = vec![
            {
                let mut w = Word::new("The");
                w.typed = "The".to_string();
                w.committed = true;
                w
            },
            {
                let mut w = Word::new("quick,");
                w.typed = "quick,".to_string();
                w.committed = true;
                w
            },
        ];
        let mut model = Model {
            screen: Screen::Results,
            session: SessionState {
                words,
                current_word: 1,
                status: crate::domain::model::TestStatus::Done,
                elapsed: Duration::from_secs(5),
                total_chars_typed: 10,
                total_errors: 0,
                wpm_history: Vec::new(),
                error_history: Vec::new(),
            },
            ..Model::default()
        };
        model.config.punctuation = true;
        model.config.numbers = true;
        let output = render_to_string(&model, 160, 24);
        insta::assert_snapshot!("results_punctuation_numbers_label", output);
    }

    #[test]
    fn typing_screen_running_variants_snapshot() {
        // elapsed = 0: countdown shows full 15s
        let model = test_model(&["the", "quick", "brown"], 1, &["the", "qu"]);
        // test_model sets status = Running and elapsed = ZERO already
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("running_elapsed_zero", output);

        // elapsed = 5s: countdown shows 10s (15s − 5s)
        let mut model = test_model(&["the", "quick", "brown"], 1, &["the", "qu"]);
        model.session.elapsed = Duration::from_secs(5);
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("running_elapsed_5s", output);
    }

    #[test]
    fn typing_screen_duration_variants_snapshot() {
        let render_with_duration = |idx: usize| {
            let mut model = test_model(&["the", "quick", "brown"], 0, &[]);
            model.session.status = crate::domain::model::TestStatus::Waiting;
            model.config.selected_duration_idx = idx;
            model.config.time_limit = Duration::from_secs(DURATION_OPTIONS[idx]);
            render_to_string(&model, 80, 24)
        };

        insta::assert_snapshot!("duration_15s", render_with_duration(0));
        insta::assert_snapshot!("duration_30s", render_with_duration(1));
        insta::assert_snapshot!("duration_60s", render_with_duration(2));
    }

    #[test]
    fn words_mode_waiting_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::domain::model::TestStatus::Waiting;
        model.config.test_mode = crate::domain::test_config::TestMode::Words;
        model.config.selected_word_count_idx = 1; // 25
        model.config.word_count = crate::domain::test_config::WORD_COUNT_OPTIONS[1];
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("words_mode_waiting", output);
    }

    #[test]
    fn words_mode_running_snapshot() {
        let model = {
            let mut m = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
            m.config.test_mode = crate::domain::test_config::TestMode::Words;
            m.config.selected_word_count_idx = 1;
            m.config.word_count = crate::domain::test_config::WORD_COUNT_OPTIONS[1];
            m
        };
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("words_mode_running", output);
    }

    #[test]
    fn typing_screen_running_focus_mode_snapshot() {
        // Running state: only counter + word block should appear; no header, no footer.
        let model = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
        // test_model already sets status = Running
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("running_focus_mode", output);
    }

    #[test]
    fn caret_style_off_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
        model.config.caret_style = crate::domain::model::CaretStyle::Off;
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("caret_off", output);
    }

    #[test]
    fn caret_style_underline_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
        model.config.caret_style = crate::domain::model::CaretStyle::Underline;
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("caret_underline", output);
    }

    #[test]
    fn idle_wide_terminal_snapshot() {
        // Snapshot at 160 cols: exercises the centering path (words_area.x = 40 > 0)
        // that 80-col tests cannot reach. Header and footer should be centered;
        // words block should occupy the middle 80 cols.
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::domain::model::TestStatus::Waiting;
        let output = render_to_string(&model, 160, 24);
        insta::assert_snapshot!("idle_wide_terminal", output);
    }

    #[test]
    fn results_screen_with_chart_snapshot() {
        let words = vec![
            {
                let mut w = Word::new("the");
                w.typed = "the".to_string();
                w.committed = true;
                w
            },
            {
                let mut w = Word::new("quick");
                w.typed = "quick".to_string();
                w.committed = true;
                w
            },
        ];
        let model = Model {
            screen: Screen::Results,
            session: SessionState {
                words,
                current_word: 1,
                status: crate::domain::model::TestStatus::Done,
                elapsed: Duration::from_secs(5),
                total_chars_typed: 10,
                total_errors: 1,
                wpm_history: vec![0.0, 20.0, 24.0, 24.0, 24.0],
                error_history: vec![0, 1, 1, 1, 1],
            },
            ..Model::default()
        };
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn modal_time_snapshot() {
        use crate::domain::model::TestStatus;
        let mut model = test_model(&["the", "quick", "brown"], 0, &[]);
        model.session.status = TestStatus::Waiting;
        model.overlay = crate::domain::model::Overlay::CustomTime {
            input: "1h30m".to_string(),
        };
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("modal_time", output);
    }

    #[test]
    fn modal_words_snapshot() {
        use crate::domain::model::TestStatus;
        let mut model = test_model(&["the", "quick", "brown"], 0, &[]);
        model.session.status = TestStatus::Waiting;
        model.overlay = crate::domain::model::Overlay::CustomWords {
            input: "42".to_string(),
        };
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("modal_words", output);
    }
}
