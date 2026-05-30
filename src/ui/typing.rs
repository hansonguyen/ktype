use crossterm::cursor::SetCursorStyle;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::domain::model::{CaretStyle, Model, TestStatus};
use crate::domain::test_config::{TestMode, WORD_COUNT_OPTIONS};
use crate::ui::char_state::{CharState, char_state};
use crate::ui::widgets::{
    duration_strip_spans, fg, mode_selector_spans, toggle_span, word_count_strip_spans,
};

pub(crate) fn render_typing(model: &Model, frame: &mut Frame) {
    let area = frame.area();
    let is_running = model.session.status == TestStatus::Running;

    if is_running {
        let [_, content, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Max(80),
            Constraint::Fill(1),
        ])
        .areas(area);
        render_typing_running(model, frame, content);
    } else {
        render_typing_idle(model, frame, area);
    }
}

fn render_typing_running(model: &Model, frame: &mut Frame, content: Rect) {
    let is_infinite = model.config.is_infinite_time() || model.config.is_infinite_words();

    let [_, counter_area, _, words_area, hint_area, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // counter (countdown or word progress)
        Constraint::Length(1), // spacer
        Constraint::Length(3), // word block
        Constraint::Length(1), // hint (ctrl+enter) or empty
        Constraint::Fill(1),
    ])
    .areas(content);

    let counter_text = match model.config.test_mode {
        TestMode::Time => {
            if model.config.is_infinite_time() {
                format!("{}", model.session.elapsed.as_secs())
            } else {
                let remaining = model
                    .config
                    .time_limit
                    .saturating_sub(model.session.elapsed);
                format!("{}", remaining.as_secs())
            }
        }
        TestMode::Words => {
            let is_infinite = model.config.selected_word_count_idx == WORD_COUNT_OPTIONS.len()
                && model.config.custom_word_count == Some(0);
            if is_infinite {
                format!("{}", model.session.current_word)
            } else {
                // current_word is 0-indexed; shows completed word count (0 at start,
                // increments to 1 only after the first word is committed and space pressed)
                let total = model.session.words.len();
                format!("{}/{}", model.session.current_word, total)
            }
        }
    };

    frame.render_widget(
        Paragraph::new(Span::styled(
            counter_text,
            fg(&model.theme.main).add_modifier(Modifier::BOLD),
        )),
        counter_area,
    );

    let word_lines = build_word_lines(model, words_area.width);
    frame.render_widget(Paragraph::new(word_lines), words_area);
    apply_terminal_cursor(&model.config.caret_style, model, frame, words_area);

    if is_infinite {
        frame.render_widget(
            Paragraph::new(Span::styled("[ctrl+e] end", fg(&model.theme.sub)))
                .alignment(Alignment::Center),
            hint_area,
        );
    }
}

fn render_typing_idle(model: &Model, frame: &mut Frame, area: Rect) {
    let [_, header_area, _, words_row, _, footer_area, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // header (full terminal width)
        Constraint::Length(2), // spacer
        Constraint::Length(3), // word block
        Constraint::Length(2), // spacer
        Constraint::Length(1), // footer
        Constraint::Fill(1),
    ])
    .areas(area);

    // Center word block at 80 cols max, matching the running view.
    let [_, words_area, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(80),
        Constraint::Fill(1),
    ])
    .areas(words_row);

    let mut header_spans: Vec<Span> = vec![
        Span::styled("ktype", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        toggle_span("@", "punctuation", model.config.punctuation, &model.theme),
        Span::raw("  "),
        toggle_span("#", "numbers", model.config.numbers, &model.theme),
        Span::raw("   "),
    ];
    header_spans.extend(mode_selector_spans(&model.config.test_mode, &model.theme));
    header_spans.push(Span::raw("   "));
    match model.config.test_mode {
        TestMode::Time => header_spans.extend(duration_strip_spans(
            model.config.selected_duration_idx,
            model.config.custom_time_secs,
            &model.theme,
        )),
        TestMode::Words => header_spans.extend(word_count_strip_spans(
            model.config.selected_word_count_idx,
            model.config.custom_word_count,
            &model.theme,
        )),
    }
    header_spans.push(Span::raw("   "));
    header_spans.push(Span::styled("[←→] cycle", fg(&model.theme.sub)));
    header_spans.push(Span::raw("  "));
    header_spans.push(Span::styled("[tab] restart", fg(&model.theme.sub)));
    header_spans.push(Span::raw("  "));
    let mode_hint = match model.config.test_mode {
        TestMode::Time => "[shift+tab] → word mode",
        TestMode::Words => "[shift+tab] → time mode",
    };
    header_spans.push(Span::styled(mode_hint, fg(&model.theme.sub)));

    frame.render_widget(
        Paragraph::new(Line::from(header_spans)).alignment(Alignment::Center),
        header_area,
    );

    let word_lines = build_word_lines(model, words_area.width);
    frame.render_widget(Paragraph::new(word_lines), words_area);
    apply_terminal_cursor(&model.config.caret_style, model, frame, words_area);

    frame.render_widget(
        Paragraph::new(Span::styled("[esc] quit", fg(&model.theme.sub)))
            .alignment(Alignment::Center),
        footer_area,
    );
}

fn scroll_for_line(cursor_line: usize) -> usize {
    cursor_line.saturating_sub(1)
}

fn word_line_indices(words: &[crate::domain::model::Word], width: u16) -> Vec<usize> {
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
    // Scroll once the cursor reaches line 2 (0-indexed), keeping the cursor on the
    // second visible line so the user never types on line 3 and always reads ahead.
    let scroll = scroll_for_line(current_line);

    let total_lines = line_indices.last().copied().unwrap_or(0) + 1;
    let mut all_lines: Vec<Vec<Span<'a>>> = vec![Vec::new(); total_lines];

    for (word_idx, (word, &line_idx)) in words.iter().zip(line_indices.iter()).enumerate() {
        let spans = &mut all_lines[line_idx];

        if !spans.is_empty() {
            spans.push(Span::styled(" ", fg(&model.theme.sub)));
        }

        for (char_idx, &ch) in word.chars.iter().enumerate() {
            // Cursor sits at the next untyped character of the active word.
            // When a word is fully typed, typed.len() == chars.len(), so
            // this condition is never true and the cursor naturally disappears
            // until Space is pressed to commit the word.
            let is_cursor =
                word_idx == current_word && char_idx == word.typed.len() && !word.committed;

            let style = if is_cursor {
                match model.config.caret_style {
                    // Off and Default show no span-level cursor indicator;
                    // Default relies on the terminal cursor via set_cursor_position.
                    CaretStyle::Off | CaretStyle::Default => fg(&model.theme.sub),
                    _ => cursor_style(&model.config.caret_style, &model.theme),
                }
            } else {
                match char_state(word, char_idx) {
                    CharState::Correct => fg(&model.theme.text),
                    CharState::Incorrect => fg(&model.theme.error),
                    CharState::Untyped => fg(&model.theme.sub),
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

fn cursor_style(style: &CaretStyle, theme: &crate::config::theme::Theme) -> Style {
    match style {
        CaretStyle::Block => Style::new()
            .fg(theme.caret.to_ratatui_color())
            .add_modifier(Modifier::REVERSED),
        CaretStyle::Underline => Style::new()
            .fg(theme.sub.to_ratatui_color())
            .underline_color(theme.caret.to_ratatui_color())
            .add_modifier(Modifier::UNDERLINED),
        // Off and Default never reach here — build_word_lines handles them before calling this.
        CaretStyle::Off | CaretStyle::Default => {
            unreachable!("cursor_style called for non-span style")
        }
    }
}

fn apply_terminal_cursor(
    caret_style: &CaretStyle,
    model: &Model,
    frame: &mut Frame,
    words_area: Rect,
) {
    let shape = match caret_style {
        CaretStyle::Default => SetCursorStyle::DefaultUserShape,
        _ => return,
    };
    if let Some((col, row)) = cursor_screen_pos(model, words_area) {
        let _ = crossterm::execute!(std::io::stdout(), shape);
        frame.set_cursor_position((col, row));
    }
}

/// Computes the terminal cursor position (col, row) within `words_area` for the
/// `Default` caret style, which delegates cursor rendering to the terminal.
/// Returns `None` if the cursor is not visible (word committed or fully typed).
fn cursor_screen_pos(model: &Model, words_area: Rect) -> Option<(u16, u16)> {
    let words = &model.session.words;
    if words.is_empty() {
        return None;
    }
    let current_word = model.session.current_word.min(words.len() - 1);
    let word = &words[current_word];
    if word.committed || word.typed.len() >= word.chars.len() {
        return None;
    }

    let max_width = words_area.width as usize;
    let line_indices = word_line_indices(words, words_area.width);
    let cursor_line = line_indices[current_word];
    let scroll = scroll_for_line(cursor_line);
    let visible_row = cursor_line.saturating_sub(scroll) as u16;

    let mut col = 0u16;
    let mut is_first_on_line = true;
    for (i, w) in words.iter().enumerate() {
        if line_indices[i] != cursor_line {
            continue;
        }
        if !is_first_on_line {
            col += 1; // space separator
        }
        if i == current_word {
            break;
        }
        // Mirror word_line_indices' max_width.max(1) clamp so column math stays in sync.
        col += w.chars.len().min(max_width.max(1)) as u16;
        is_first_on_line = false;
    }
    col += word.typed.len() as u16;

    Some((words_area.x + col, words_area.y + visible_row))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::{Model, Screen, SessionState, Word};
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

    // cursor_style() unit tests — verify span-level styling for each rendered variant.
    // Snapshot tests can't catch style regressions (TestBackend strips modifiers);
    // these assert on the Style directly.
    #[test]
    fn cursor_style_block_applies_reversed_modifier() {
        let theme = crate::config::theme::Theme::default();
        let style = cursor_style(&CaretStyle::Block, &theme);
        assert!(
            style.add_modifier.contains(Modifier::REVERSED),
            "Block caret must use REVERSED modifier"
        );
    }

    #[test]
    fn cursor_style_underline_applies_underline_modifier_and_color() {
        let theme = crate::config::theme::Theme::default();
        let style = cursor_style(&CaretStyle::Underline, &theme);
        assert!(
            style.add_modifier.contains(Modifier::UNDERLINED),
            "Underline caret must use UNDERLINED modifier"
        );
        assert!(
            style.underline_color.is_some(),
            "Underline caret must set an underline color"
        );
    }

    // cursor_screen_pos() unit tests — verify column arithmetic in the Default caret path.
    #[test]
    fn cursor_screen_pos_col_accounts_for_preceding_word() {
        // "the" committed, cursor at start of "quick" (nothing typed yet).
        // Expected col = 3 (len("the")) + 1 (space separator) + 0 (nothing typed) = 4.
        let mut model = test_model(&["the", "quick"], 1, &["the"]);
        model.session.words[0].committed = true;
        let area = ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 3,
        };
        assert_eq!(cursor_screen_pos(&model, area), Some((4, 0)));
    }

    #[test]
    fn cursor_screen_pos_col_includes_typed_chars() {
        // "the" committed, "qu" typed in "quick".
        // Expected col = 3 + 1 (space) + 2 (typed) = 6.
        let mut model = test_model(&["the", "quick"], 1, &["the", "qu"]);
        model.session.words[0].committed = true;
        let area = ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 3,
        };
        assert_eq!(cursor_screen_pos(&model, area), Some((6, 0)));
    }

    #[test]
    fn cursor_screen_pos_applies_words_area_x_offset() {
        // At terminal width 160 the words block is centered: x = (160 - 80) / 2 = 40.
        // Verify that words_area.x is added to the column result, not silently dropped.
        // "the" committed, "qu" typed: logical col = 3 + 1 + 2 = 6 → absolute = 40 + 6 = 46.
        let mut model = test_model(&["the", "quick"], 1, &["the", "qu"]);
        model.session.words[0].committed = true;
        let area = ratatui::layout::Rect {
            x: 40,
            y: 5,
            width: 80,
            height: 3,
        };
        assert_eq!(cursor_screen_pos(&model, area), Some((46, 5)));
    }
}
