use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::input::{CharState, char_state};
use crate::model::{CursorStyle, Model, Screen, TestStatus};

pub fn view(model: &Model, frame: &mut Frame) {
    match model.screen {
        Screen::Done => render_done(frame),
        Screen::Typing => render_typing(model, frame),
        // Quitting is handled by the main loop (terminal restore + exit).
        // Rendering one last frame is unnecessary and could cause flicker.
        Screen::Quitting => {}
    }
}

fn render_done(frame: &mut Frame) {
    let area = frame.area();
    let msg = Paragraph::new("test complete — press tab to restart").alignment(Alignment::Center);

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(area);

    frame.render_widget(msg, vertical[1]);
}

fn render_typing(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    // Constrain content width to 80 chars and center it horizontally.
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

    // Header
    let status_text = match model.session.status {
        TestStatus::Waiting | TestStatus::Running => {
            format!("words: {}", model.config.word_count)
        }
        TestStatus::Done => String::from("done"),
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled("kern", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(status_text, Style::new().dim()),
        Span::raw("  "),
        Span::styled("[tab] restart", Style::new().dim()),
    ]));
    frame.render_widget(header, header_area);

    // Word block
    let word_lines = build_word_lines(model, words_area.width);
    let words_widget = Paragraph::new(word_lines);
    frame.render_widget(words_widget, words_area);

    // Footer
    let footer = Paragraph::new(Span::styled("[esc] quit", Style::new().dim()));
    frame.render_widget(footer, footer_area);
}

// Returns a vec where result[i] is the line index for words[i], given the available width.
// Words that don't fit on a line start a new one; spaces between words count as 1 char.
fn word_line_indices(words: &[crate::model::Word], width: u16) -> Vec<usize> {
    let max_width = width as usize;
    let mut line_for_word = vec![0usize; words.len()];
    let mut current_line = 0usize;
    let mut line_width = 0usize;

    for (i, word) in words.iter().enumerate() {
        // Clamp to available width so a single oversized word doesn't
        // cascade every subsequent word onto its own line.
        let word_len = word.chars.len().min(max_width.max(1));
        // Words at the start of a line need no preceding space.
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

// Builds the 3 visible lines of the word block for rendering.
// Scroll position is derived here from current_word's line index — not stored in Model.
fn build_word_lines<'a>(model: &Model, width: u16) -> Vec<Line<'a>> {
    let words = &model.session.words;
    if words.is_empty() {
        return vec![Line::default(); 3];
    }

    let line_indices = word_line_indices(words, width);
    // Clamp in case session state briefly has current_word beyond the word list
    // (e.g., a stale model during a Tab restart before new words are generated).
    let current_word = model.session.current_word.min(words.len() - 1);
    let current_line = line_indices[current_word];

    // Once the cursor reaches line 2, scroll so it stays at the bottom of the 3-line
    // window. For lines 0 and 1 no scroll occurs, showing context ahead of the cursor.
    let scroll = current_line.saturating_sub(2);

    // Collect all rendered lines, then take the visible slice.
    let total_lines = line_indices.last().copied().unwrap_or(0) + 1;
    let mut all_lines: Vec<Vec<Span<'a>>> = vec![Vec::new(); total_lines];

    for (word_idx, (word, &line_idx)) in words.iter().zip(line_indices.iter()).enumerate() {
        let spans = &mut all_lines[line_idx];

        // Space separator — dim so it doesn't distract from the typed text.
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

            // ch.to_string() allocates per character per frame; acceptable at 25 words
            // (~125 chars) but worth revisiting in Phase 6 polish.
            spans.push(Span::styled(ch.to_string(), style));
        }
    }

    // Pad to 3 lines so the word block never collapses.
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
}
