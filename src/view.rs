use crossterm::cursor::SetCursorStyle;
use ratatui::widgets::{BorderType, Borders, Clear};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Block, Paragraph,
        canvas::{Canvas, Line as CanvasLine},
    },
};

fn fg(color: &crate::theme::HexColor) -> Style {
    Style::new().fg(color.to_ratatui_color())
}

use crate::input::{CharState, char_state};
use crate::metrics;
use crate::model::ModalKind;
use crate::model::{
    CaretStyle, DURATION_OPTIONS, Model, Screen, TestMode, TestStatus, WORD_COUNT_OPTIONS,
};
use crate::update::parse_custom_time;

pub fn view(model: &Model, frame: &mut Frame) {
    frame.render_widget(
        Block::new().style(Style::new().bg(model.theme.bg.to_ratatui_color())),
        frame.area(),
    );

    match model.screen {
        Screen::Done => render_results(model, frame),
        Screen::Typing => render_typing(model, frame),
        // Quitting is handled by the main loop (terminal restore + exit).
        // Rendering one last frame is unnecessary and could cause flicker.
        Screen::Quitting => {}
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

    if model.modal.is_some() {
        render_modal(model, frame);
    }
}

fn render_results(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let correct_words = metrics::count_correct_words(&model.session.words);
    let committed_words = metrics::count_committed_words(&model.session.words);
    let elapsed = model.session.elapsed;

    let wpm_val = metrics::wpm(correct_words, elapsed);
    let raw_val = metrics::raw_wpm(committed_words, elapsed);
    let acc_val =
        metrics::raw_accuracy(model.session.total_chars_typed, model.session.total_errors);

    // Horizontally center the results block
    let outer = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(120),
        Constraint::Fill(1),
    ])
    .split(area);
    let area = outer[1];

    let vertical = Layout::vertical([
        Constraint::Length(1), // mode strip
        Constraint::Length(1), // spacer
        Constraint::Fill(1),   // top padding (vertical centering)
        Constraint::Max(18),   // main content: left stats + chart
        Constraint::Length(3), // bottom stats row
        Constraint::Fill(1),   // bottom padding (vertical centering)
        Constraint::Length(1), // footer
    ])
    .split(area);

    // Mode strip
    let mut result_header: Vec<Span> = Vec::new();
    result_header.extend(mode_selector_spans(&model.config.test_mode, &model.theme));
    result_header.push(Span::raw("   "));
    match model.config.test_mode {
        TestMode::Time => result_header.extend(duration_strip_spans(
            model.config.selected_duration_idx,
            model.config.custom_time_secs,
            &model.theme,
        )),
        TestMode::Words => result_header.extend(word_count_strip_spans(
            model.config.selected_word_count_idx,
            model.config.custom_word_count,
            &model.theme,
        )),
    }
    frame.render_widget(
        Paragraph::new(Line::from(result_header)).alignment(Alignment::Center),
        vertical[0],
    );

    // Main content: left stats panel | chart
    let content =
        Layout::horizontal([Constraint::Length(14), Constraint::Fill(1)]).split(vertical[3]);

    // Left stats panel
    let left = Layout::vertical([
        Constraint::Length(1), // "wpm" label
        Constraint::Length(1), // wpm value
        Constraint::Length(1), // spacer
        Constraint::Length(1), // "acc" label
        Constraint::Length(1), // acc value
        Constraint::Fill(1),   // fill
    ])
    .split(content[0]);

    frame.render_widget(
        Paragraph::new(Span::styled("wpm", fg(&model.theme.sub))),
        left[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}", wpm_val),
            fg(&model.theme.main).add_modifier(Modifier::BOLD),
        )),
        left[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("acc", fg(&model.theme.sub))),
        left[3],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}%", acc_val),
            fg(&model.theme.text).add_modifier(Modifier::BOLD),
        )),
        left[4],
    );

    // Chart fills the right side
    render_chart(model, frame, content[1]);

    // Bottom stats: left (test type) | right (raw + time)
    let bottom =
        Layout::horizontal([Constraint::Length(14), Constraint::Fill(1)]).split(vertical[4]);

    // Bottom-left: test type info
    let bottom_left = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(bottom[0]);

    frame.render_widget(
        Paragraph::new(Span::styled("test type", fg(&model.theme.sub))),
        bottom_left[0],
    );
    let mode_detail = match model.config.test_mode {
        TestMode::Time => {
            let idx = model.config.selected_duration_idx;
            if idx < DURATION_OPTIONS.len() {
                format!("time {}", DURATION_OPTIONS[idx])
            } else {
                match model.config.custom_time_secs {
                    Some(0) => "time \u{221e}".to_string(),
                    Some(n) => format!("time {}s", n),
                    None => "time custom".to_string(),
                }
            }
        }
        TestMode::Words => {
            let idx = model.config.selected_word_count_idx;
            if idx < WORD_COUNT_OPTIONS.len() {
                format!("words {}", model.config.word_count)
            } else {
                match model.config.custom_word_count {
                    Some(0) => "words \u{221e}".to_string(),
                    Some(n) => format!("words {}", n),
                    None => "words custom".to_string(),
                }
            }
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            mode_detail,
            fg(&model.theme.text).add_modifier(Modifier::BOLD),
        )),
        bottom_left[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("english", fg(&model.theme.sub))),
        bottom_left[2],
    );

    // Bottom-right: raw wpm | time
    let bottom_right =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).split(bottom[1]);

    let br_raw = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(bottom_right[0]);

    let br_time = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(bottom_right[1]);

    frame.render_widget(
        Paragraph::new(Span::styled("raw", fg(&model.theme.sub))),
        br_raw[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}", raw_val),
            fg(&model.theme.text).add_modifier(Modifier::BOLD),
        )),
        br_raw[1],
    );

    frame.render_widget(
        Paragraph::new(Span::styled("time", fg(&model.theme.sub))),
        br_time[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{}s", elapsed.as_secs()),
            fg(&model.theme.text).add_modifier(Modifier::BOLD),
        )),
        br_time[1],
    );

    // Footer
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[tab] change/restart", fg(&model.theme.sub)),
            Span::raw("   "),
            Span::styled("[esc] quit", fg(&model.theme.sub)),
        ]))
        .alignment(Alignment::Center),
        vertical[6],
    );
}

fn render_chart(model: &Model, frame: &mut Frame, area: Rect) {
    let wpm_history = &model.session.wpm_history;

    if wpm_history.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("no data", fg(&model.theme.sub)))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let max_wpm = wpm_history.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
    let y_bound_max = max_wpm * 1.1;
    let max_t = wpm_history.len() as f64;

    // Per-second error deltas from cumulative history
    let error_history = &model.session.error_history;
    debug_assert_eq!(
        wpm_history.len(),
        error_history.len(),
        "wpm_history and error_history must be kept in sync"
    );
    let error_deltas: Vec<u64> = error_history
        .iter()
        .enumerate()
        .map(|(i, &cum)| {
            if i == 0 {
                cum
            } else {
                cum.saturating_sub(error_history[i - 1])
            }
        })
        .collect();
    let max_error_delta = error_deltas.iter().cloned().max().unwrap_or(1).max(1);

    // Layout: y-labels strip | canvas area
    let y_label_width = 5u16;
    let chart_h =
        Layout::horizontal([Constraint::Length(y_label_width), Constraint::Fill(1)]).split(area);

    let canvas_v = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // x-axis labels
    ])
    .split(chart_h[1]);

    let canvas_area = canvas_v[0];
    let x_labels_area = canvas_v[1];

    // Y-axis labels: 5 evenly-spaced values top-to-bottom
    let canvas_height = canvas_area.height as usize;
    let mut y_lines: Vec<Line> = vec![Line::default(); canvas_height];
    for i in 0..5usize {
        let value = y_bound_max * (4 - i) as f64 / 4.0;
        let row = i * canvas_height.saturating_sub(1) / 4;
        if row < canvas_height {
            y_lines[row] = Line::from(Span::styled(
                format!("{:>4.0}", value),
                fg(&model.theme.sub),
            ));
        }
    }
    frame.render_widget(Paragraph::new(y_lines), chart_h[0]);

    // X-axis labels: second markers spaced to canvas width
    let canvas_width = canvas_area.width as usize;
    let n_secs = wpm_history.len();
    let interval = if n_secs <= 15 {
        1
    } else if n_secs <= 60 {
        5
    } else {
        10
    };
    let mut x_buf = vec![b' '; canvas_width];
    for t in (interval..=n_secs).step_by(interval) {
        let col = (t * canvas_width) / n_secs;
        let label = t.to_string();
        let start = col
            .saturating_sub(label.len() / 2)
            .min(canvas_width.saturating_sub(label.len()));
        for (j, b) in label.bytes().enumerate() {
            if start + j < canvas_width {
                x_buf[start + j] = b;
            }
        }
    }
    frame.render_widget(
        Paragraph::new(Span::styled(
            String::from_utf8_lossy(&x_buf).into_owned(),
            fg(&model.theme.sub),
        )),
        x_labels_area,
    );

    // Canvas: WPM line + error markers
    let main_color = model.theme.main.to_ratatui_color();
    let error_style = fg(&model.theme.error);
    frame.render_widget(
        Canvas::default()
            .x_bounds([0.0, max_t])
            .y_bounds([0.0, y_bound_max])
            .marker(Marker::Braille)
            .paint(move |ctx| {
                // WPM line segments; wpm_history[i] is the WPM at the end of second i+1.
                // i=0: flat segment from x=0 to x=1 fills the opening gap.
                // i>0: connects wpm[i-1] at x=i to wpm[i] at x=i+1.
                for i in 0..wpm_history.len() {
                    let y1 = if i == 0 {
                        wpm_history[0]
                    } else {
                        wpm_history[i - 1]
                    };
                    ctx.draw(&CanvasLine {
                        x1: i as f64,
                        y1,
                        x2: (i + 1) as f64,
                        y2: wpm_history[i],
                        color: main_color,
                    });
                }
                // Error markers (×, scaled into WPM range)
                for (i, &delta) in error_deltas.iter().enumerate() {
                    if delta > 0 {
                        let scaled_y = (delta as f64 / max_error_delta as f64) * max_wpm;
                        ctx.print(
                            (i + 1) as f64,
                            scaled_y,
                            Line::from(Span::styled("×", error_style)),
                        );
                    }
                }
            }),
        canvas_area,
    );
}

fn options_strip_spans(
    labels: Vec<String>,
    selected_idx: usize,
    theme: &crate::theme::Theme,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, label) in labels.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let display = if i == selected_idx {
            format!("[{}]", label)
        } else {
            label
        };
        let style = if i == selected_idx {
            fg(&theme.text).add_modifier(Modifier::BOLD)
        } else {
            fg(&theme.sub)
        };
        spans.push(Span::styled(display, style));
    }
    spans
}

fn duration_strip_spans(
    selected_idx: usize,
    custom_time_secs: Option<u64>,
    theme: &crate::theme::Theme,
) -> Vec<Span<'static>> {
    let mut labels: Vec<String> = DURATION_OPTIONS.iter().map(|s| s.to_string()).collect();
    labels.push(match custom_time_secs {
        None => "custom".to_string(),
        Some(0) => "\u{221e}".to_string(),
        Some(n) => n.to_string(),
    });
    options_strip_spans(labels, selected_idx, theme)
}

fn word_count_strip_spans(
    selected_idx: usize,
    custom_word_count: Option<usize>,
    theme: &crate::theme::Theme,
) -> Vec<Span<'static>> {
    let mut labels: Vec<String> = WORD_COUNT_OPTIONS.iter().map(|s| s.to_string()).collect();
    labels.push(match custom_word_count {
        None => "custom".to_string(),
        Some(0) => "\u{221e}".to_string(),
        Some(n) => n.to_string(),
    });
    options_strip_spans(labels, selected_idx, theme)
}

fn mode_selector_spans(mode: &TestMode, theme: &crate::theme::Theme) -> Vec<Span<'static>> {
    let selected_style = fg(&theme.text).add_modifier(Modifier::BOLD);
    let unselected_style = fg(&theme.sub);
    match mode {
        TestMode::Time => vec![
            Span::styled("[time]", selected_style),
            Span::raw(" "),
            Span::styled("words", unselected_style),
        ],
        TestMode::Words => vec![
            Span::styled("time", unselected_style),
            Span::raw(" "),
            Span::styled("[words]", selected_style),
        ],
    }
}

fn render_typing(model: &Model, frame: &mut Frame) {
    let area = frame.area();
    let is_running = model.session.status == TestStatus::Running;

    if is_running {
        let horizontal = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Max(80),
            Constraint::Fill(1),
        ])
        .split(area);
        render_typing_running(model, frame, horizontal[1]);
    } else {
        render_typing_idle(model, frame, area);
    }
}

fn render_typing_running(model: &Model, frame: &mut Frame, content: Rect) {
    let is_infinite = model.config.is_infinite_time() || model.config.is_infinite_words();

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // counter (countdown or word progress)
        Constraint::Length(1), // spacer
        Constraint::Length(3), // word block
        Constraint::Length(1), // hint (ctrl+enter) or empty
        Constraint::Fill(1),
    ])
    .split(content);

    let counter_area = vertical[1];
    let words_area = vertical[3];

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
            vertical[4],
        );
    }
}

fn render_typing_idle(model: &Model, frame: &mut Frame, area: Rect) {
    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // header (full terminal width)
        Constraint::Length(2), // spacer
        Constraint::Length(3), // word block
        Constraint::Length(2), // spacer
        Constraint::Length(1), // footer
        Constraint::Fill(1),
    ])
    .split(area);

    let header_area = vertical[1];
    let footer_area = vertical[5];

    // Center word block at 80 cols max, matching the running view.
    let words_h = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(80),
        Constraint::Fill(1),
    ])
    .split(vertical[3]);
    let words_area = words_h[1];

    let mut header_spans: Vec<Span> = vec![
        Span::styled("ktype", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
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

fn cursor_style(style: &CaretStyle, theme: &crate::theme::Theme) -> Style {
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

fn plural(n: u64, unit: &str) -> String {
    if n == 1 {
        format!("1 {}", unit)
    } else {
        format!("{} {}s", n, unit)
    }
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    match (h, m, s) {
        (0, 0, s) => plural(s, "second"),
        (0, m, 0) => plural(m, "minute"),
        (0, m, s) => format!("{} and {}", plural(m, "minute"), plural(s, "second")),
        (h, 0, 0) => plural(h, "hour"),
        (h, 0, s) => format!("{} and {}", plural(h, "hour"), plural(s, "second")),
        (h, m, 0) => format!("{} and {}", plural(h, "hour"), plural(m, "minute")),
        (h, m, s) => format!(
            "{}, {} and {}",
            plural(h, "hour"),
            plural(m, "minute"),
            plural(s, "second")
        ),
    }
}

fn render_modal(model: &Model, frame: &mut Frame) {
    let modal_state = model.modal.as_ref().expect("modal must be Some");
    let area = frame.area();

    let is_time = matches!(modal_state.kind, ModalKind::CustomTime);
    let height: u16 = if is_time { 10 } else { 8 };
    let width: u16 = 54;
    let modal_area = Rect {
        x: area.width.saturating_sub(width) / 2,
        y: area.height.saturating_sub(height) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    frame.render_widget(Clear, modal_area);
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(fg(&model.theme.main))
        .style(Style::new().bg(model.theme.bg.to_ratatui_color()));
    let inner_area = outer_block.inner(modal_area);
    frame.render_widget(outer_block, modal_area);

    let constraints: Vec<Constraint> = if is_time {
        vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ]
    };
    let rows = Layout::vertical(constraints).split(inner_area);

    let title = if is_time {
        "Test Duration"
    } else {
        "Custom word amount"
    };
    frame.render_widget(
        Paragraph::new(Span::styled(title, fg(&model.theme.sub))).alignment(Alignment::Center),
        rows[0],
    );

    let preview = if modal_state.input.is_empty() {
        String::new()
    } else if is_time {
        let secs = parse_custom_time(&modal_state.input);
        if secs == 0 {
            "infinite".to_string()
        } else {
            format_duration(secs)
        }
    } else {
        match modal_state.input.parse::<usize>() {
            Ok(0) => "infinite".to_string(),
            Ok(n) => format!("{} words", n),
            Err(_) => String::new(),
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(preview, fg(&model.theme.sub))).alignment(Alignment::Center),
        rows[1],
    );

    let input_display = format!("{}_", modal_state.input);
    frame.render_widget(
        Paragraph::new(Span::styled(input_display, fg(&model.theme.text))).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(fg(&model.theme.main)),
        ),
        rows[2],
    );

    if is_time {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "use h for hours, m for minutes (e.g. 1h30m). 0 = infinite.",
                fg(&model.theme.sub),
            ))
            .alignment(Alignment::Center),
            rows[3],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                "[enter] apply   [esc] cancel",
                fg(&model.theme.sub),
            ))
            .alignment(Alignment::Center),
            rows[4],
        );
    } else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "[enter] apply   [esc] cancel",
                fg(&model.theme.sub),
            ))
            .alignment(Alignment::Center),
            rows[3],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DURATION_OPTIONS, Model, Screen, SessionState, Word};
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
    fn typing_screen_waiting_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::model::TestStatus::Waiting;
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn idle_screen_update_banner_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::model::TestStatus::Waiting;
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
            screen: Screen::Done,
            session: SessionState {
                words,
                current_word: 2,
                status: crate::model::TestStatus::Done,
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
            model.session.status = crate::model::TestStatus::Waiting;
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
        model.session.status = crate::model::TestStatus::Waiting;
        model.config.test_mode = crate::model::TestMode::Words;
        model.config.selected_word_count_idx = 1; // 25
        model.config.word_count = crate::model::WORD_COUNT_OPTIONS[1];
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("words_mode_waiting", output);
    }

    #[test]
    fn words_mode_running_snapshot() {
        let model = {
            let mut m = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
            m.config.test_mode = crate::model::TestMode::Words;
            m.config.selected_word_count_idx = 1;
            m.config.word_count = crate::model::WORD_COUNT_OPTIONS[1];
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
        model.config.caret_style = crate::model::CaretStyle::Off;
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("caret_off", output);
    }

    #[test]
    fn caret_style_underline_snapshot() {
        let mut model = test_model(&["the", "quick", "brown", "fox"], 1, &["the", "qu"]);
        model.config.caret_style = crate::model::CaretStyle::Underline;
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("caret_underline", output);
    }

    // cursor_style() unit tests — verify span-level styling for each rendered variant.
    // Snapshot tests can't catch style regressions (TestBackend strips modifiers);
    // these assert on the Style directly.
    #[test]
    fn cursor_style_block_applies_reversed_modifier() {
        let theme = crate::theme::Theme::default();
        let style = cursor_style(&CaretStyle::Block, &theme);
        assert!(
            style.add_modifier.contains(Modifier::REVERSED),
            "Block caret must use REVERSED modifier"
        );
    }

    #[test]
    fn cursor_style_underline_applies_underline_modifier_and_color() {
        let theme = crate::theme::Theme::default();
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

    #[test]
    fn idle_wide_terminal_snapshot() {
        // Snapshot at 160 cols: exercises the centering path (words_area.x = 40 > 0)
        // that 80-col tests cannot reach. Header and footer should be centered;
        // words block should occupy the middle 80 cols.
        let mut model = test_model(&["the", "quick", "brown", "fox"], 0, &[]);
        model.session.status = crate::model::TestStatus::Waiting;
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
            screen: Screen::Done,
            session: SessionState {
                words,
                current_word: 1,
                status: crate::model::TestStatus::Done,
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
        use crate::model::{ModalKind, ModalState, TestStatus};
        let mut model = test_model(&["the", "quick", "brown"], 0, &[]);
        model.session.status = TestStatus::Waiting;
        model.modal = Some(ModalState {
            kind: ModalKind::CustomTime,
            input: "1h30m".to_string(),
        });
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("modal_time", output);
    }

    #[test]
    fn modal_words_snapshot() {
        use crate::model::{ModalKind, ModalState, TestStatus};
        let mut model = test_model(&["the", "quick", "brown"], 0, &[]);
        model.session.status = TestStatus::Waiting;
        model.modal = Some(ModalState {
            kind: ModalKind::CustomWords,
            input: "42".to_string(),
        });
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!("modal_words", output);
    }
}
