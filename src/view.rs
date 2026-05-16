use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::input::{CharState, char_state};
use crate::metrics;
use crate::model::{CursorStyle, DURATION_OPTIONS, Model, Screen, TestStatus};

pub fn view(model: &Model, frame: &mut Frame) {
    match model.screen {
        Screen::Done => render_results(model, frame),
        Screen::Typing => render_typing(model, frame),
        // Quitting is handled by the main loop (terminal restore + exit).
        // Rendering one last frame is unnecessary and could cause flicker.
        Screen::Quitting => {}
    }
}

fn render_results(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let correct_words = metrics::count_correct_words(&model.session.words);
    let committed_words = metrics::count_committed_words(&model.session.words);
    let correct_chars = metrics::count_correct_chars(&model.session.words);
    let total_chars = metrics::count_total_chars_typed(&model.session.words);
    let elapsed = model.session.elapsed;

    let wpm_val = metrics::wpm(correct_words, elapsed);
    let raw_val = metrics::raw_wpm(committed_words, elapsed);
    let acc_val = metrics::accuracy(correct_chars, total_chars);

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // duration strip
        Constraint::Length(1), // spacer
        Constraint::Length(1), // "kern"
        Constraint::Length(1), // spacer
        Constraint::Length(1), // metric labels
        Constraint::Length(1), // metric values
        Constraint::Length(1), // spacer
        Constraint::Length(1), // footer
        Constraint::Fill(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(duration_strip_spans(
            model.config.selected_duration_idx,
            false,
        )))
        .alignment(Alignment::Center),
        vertical[1],
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            "kern",
            Style::new().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        vertical[3],
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            "  wpm       raw wpm        acc",
            Style::new().dim(),
        ))
        .alignment(Alignment::Center),
        vertical[5],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(format!("{:>5.0}", wpm_val)),
            Span::raw("       "),
            Span::raw(format!("{:>5.0}", raw_val)),
            Span::raw("       "),
            Span::raw(format!("{:>4.0}%", acc_val)),
        ]))
        .alignment(Alignment::Center),
        vertical[6],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[tab] change/restart", Style::new().dim()),
            Span::raw("   "),
            Span::styled("[esc] quit", Style::new().dim()),
        ]))
        .alignment(Alignment::Center),
        vertical[8],
    );
}

fn duration_strip_spans<'a>(selected_idx: usize, dimmed: bool) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    for (i, &secs) in DURATION_OPTIONS.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let label = if i == selected_idx {
            format!("[{}]", secs)
        } else {
            secs.to_string()
        };
        let style = if i == selected_idx && !dimmed {
            Style::new().add_modifier(Modifier::BOLD)
        } else {
            Style::new().dim()
        };
        spans.push(Span::styled(label, style));
    }
    spans
}

fn render_typing(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(80),
        Constraint::Fill(1),
    ])
    .split(area);
    let content = horizontal[1];

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // header
        Constraint::Length(1), // spacer
        Constraint::Length(3), // word block
        Constraint::Length(1), // spacer
        Constraint::Length(1), // footer
        Constraint::Fill(1),
    ])
    .split(content);

    let header_area = vertical[1];
    let words_area = vertical[3];
    let footer_area = vertical[5];

    // Header: duration strip always visible; countdown appended only while Running.
    let is_running = model.session.status == TestStatus::Running;
    let mut header_spans: Vec<Span> = vec![
        Span::styled("kern", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
    ];
    header_spans.extend(duration_strip_spans(
        model.config.selected_duration_idx,
        is_running,
    ));
    if is_running {
        let countdown = model
            .config
            .time_limit
            .saturating_sub(model.session.elapsed);
        header_spans.push(Span::raw("  ·  "));
        header_spans.push(Span::styled(
            format!("{}s", countdown.as_secs()),
            Style::new().dim(),
        ));
    }
    header_spans.push(Span::raw("  "));
    header_spans.push(Span::styled("[tab] restart", Style::new().dim()));
    let header = Paragraph::new(Line::from(header_spans));
    frame.render_widget(header, header_area);

    let word_lines = build_word_lines(model, words_area.width);
    let words_widget = Paragraph::new(word_lines);
    frame.render_widget(words_widget, words_area);

    let footer = Paragraph::new(Span::styled("[esc] quit", Style::new().dim()));
    frame.render_widget(footer, footer_area);
}

fn word_line_indices(words: &[crate::model::Word], width: u16) -> Vec<usize> {
    let max_width = width as usize;
    let mut line_for_word = vec![0usize; words.len()];
    let mut current_line = 0usize;
    let mut line_width = 0usize;

    for (i, word) in words.iter().enumerate() {
        // Clamp to available width so a single oversized word doesn't
        // cascade every subsequent word onto its own line.
        let word_len = word.chars.len().min(max_width.max(1));
        let needed = if line_width == 0 {
            word_len
        } else {
            1 + word_len
        };

        if line_width > 0 && line_width + 1 + word_len > max_width {
            current_line += 1;
            line_width = word_len;
        } else {
            line_width += needed;
        }
        line_for_word[i] = current_line;
    }
    line_for_word
}

fn build_word_lines<'a>(model: &Model, width: u16) -> Vec<Line<'a>> {
    let words = &model.session.words;
    if words.is_empty() {
        return vec![Line::default(); 3];
    }

    let line_indices = word_line_indices(words, width);
    let current_word = model.session.current_word.min(words.len() - 1);
    let current_line = line_indices[current_word];
    // Once the cursor reaches line 2, scroll so it stays at the bottom of the 3-line
    // window. For lines 0 and 1 no scroll occurs, showing context ahead of the cursor.
    let scroll = current_line.saturating_sub(2);

    let total_lines = line_indices.last().copied().unwrap_or(0) + 1;
    let mut all_lines: Vec<Vec<Span<'a>>> = vec![Vec::new(); total_lines];

    for (word_idx, (word, &line_idx)) in words.iter().zip(line_indices.iter()).enumerate() {
        let spans = &mut all_lines[line_idx];

        if !spans.is_empty() {
            spans.push(Span::styled(" ", Style::new().dim()));
        }

        for (char_idx, &ch) in word.chars.iter().enumerate() {
            // Cursor sits at the next untyped character of the active word.
            // When a word is fully typed, typed.len() == chars.len(), so
            // this condition is never true and the cursor naturally disappears
            // until Space is pressed to commit the word.
            let is_cursor =
                word_idx == current_word && char_idx == word.typed.len() && !word.committed;

            let style = if is_cursor {
                cursor_style(&model.config.cursor_style)
            } else {
                match char_state(word, char_idx) {
                    CharState::Correct => Style::new(),
                    CharState::Incorrect => Style::new().fg(Color::Red),
                    CharState::Untyped => Style::new().dim(),
                }
            };

            spans.push(Span::styled(ch.to_string(), style));
        }
    }

    let mut visible: Vec<Line<'a>> = all_lines
        .into_iter()
        .skip(scroll)
        .take(3)
        .map(Line::from)
        .collect();
    while visible.len() < 3 {
        visible.push(Line::default());
    }
    visible
}

fn cursor_style(style: &CursorStyle) -> Style {
    match style {
        CursorStyle::Block => Style::new().add_modifier(Modifier::REVERSED),
        CursorStyle::Underline => Style::new().add_modifier(Modifier::UNDERLINED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Config, Model, Screen, SessionState, Word};
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
                status: crate::model::TestStatus::Running,
                elapsed: Duration::ZERO,
            },
            config: Config::default(),
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
        model.screen = Screen::Done;
        render_to_string(&model, 80, 24);
    }

    #[test]
    fn typing_screen_snapshot() {
        let model = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
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
            screen: Screen::Done,
            session: SessionState {
                words,
                current_word: 2,
                status: crate::model::TestStatus::Done,
                elapsed: Duration::from_secs(10),
            },
            config: Config::default(),
        };
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }
}
