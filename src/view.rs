use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Paragraph,
        canvas::{Canvas, Line as CanvasLine},
    },
};

use crate::input::{CharState, char_state};
use crate::metrics;
use crate::model::{
    CursorStyle, DURATION_OPTIONS, Model, Screen, TestMode, TestStatus, WORD_COUNT_OPTIONS,
};

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
    result_header.extend(mode_selector_spans(&model.config.test_mode));
    result_header.push(Span::raw("   "));
    match model.config.test_mode {
        TestMode::Time => {
            result_header.extend(duration_strip_spans(model.config.selected_duration_idx))
        }
        TestMode::Words => {
            result_header.extend(word_count_strip_spans(model.config.selected_word_count_idx))
        }
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
        Paragraph::new(Span::styled("wpm", Style::new().dim())),
        left[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}", wpm_val),
            Style::new().add_modifier(Modifier::BOLD).fg(Color::Yellow),
        )),
        left[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("acc", Style::new().dim())),
        left[3],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}%", acc_val),
            Style::new().add_modifier(Modifier::BOLD),
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
        Paragraph::new(Span::styled("test type", Style::new().dim())),
        bottom_left[0],
    );
    let mode_detail = match model.config.test_mode {
        TestMode::Time => format!(
            "time {}",
            DURATION_OPTIONS[model.config.selected_duration_idx]
        ),
        TestMode::Words => format!("words {}", model.config.word_count),
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            mode_detail,
            Style::new().add_modifier(Modifier::BOLD),
        )),
        bottom_left[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("english", Style::new().dim())),
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
        Paragraph::new(Span::styled("raw", Style::new().dim())),
        br_raw[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}", raw_val),
            Style::new().add_modifier(Modifier::BOLD),
        )),
        br_raw[1],
    );

    frame.render_widget(
        Paragraph::new(Span::styled("time", Style::new().dim())),
        br_time[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{}s", elapsed.as_secs()),
            Style::new().add_modifier(Modifier::BOLD),
        )),
        br_time[1],
    );

    // Footer
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[tab] change/restart", Style::new().dim()),
            Span::raw("   "),
            Span::styled("[esc] quit", Style::new().dim()),
        ]))
        .alignment(Alignment::Center),
        vertical[6],
    );
}

fn render_chart(model: &Model, frame: &mut Frame, area: Rect) {
    let wpm_history = &model.session.wpm_history;

    if wpm_history.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("no data", Style::new().dim()))
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
            y_lines[row] = Line::from(Span::styled(format!("{:>4.0}", value), Style::new().dim()));
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
            Style::new().dim(),
        )),
        x_labels_area,
    );

    // Canvas: WPM line + error markers
    frame.render_widget(
        Canvas::default()
            .x_bounds([0.0, max_t])
            .y_bounds([0.0, y_bound_max])
            .marker(Marker::Braille)
            .paint(|ctx| {
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
                        color: Color::LightBlue,
                    });
                }
                // Error markers (× in red, scaled into WPM range)
                for (i, &delta) in error_deltas.iter().enumerate() {
                    if delta > 0 {
                        let scaled_y = (delta as f64 / max_error_delta as f64) * max_wpm;
                        ctx.print(
                            (i + 1) as f64,
                            scaled_y,
                            Line::from(Span::styled("×", Style::new().fg(Color::Red))),
                        );
                    }
                }
            }),
        canvas_area,
    );
}

fn options_strip_spans(labels: Vec<String>, selected_idx: usize) -> Vec<Span<'static>> {
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
            Style::new().add_modifier(Modifier::BOLD)
        } else {
            Style::new().dim()
        };
        spans.push(Span::styled(display, style));
    }
    spans
}

fn duration_strip_spans(selected_idx: usize) -> Vec<Span<'static>> {
    let labels = DURATION_OPTIONS.iter().map(|s| s.to_string()).collect();
    options_strip_spans(labels, selected_idx)
}

fn word_count_strip_spans(selected_idx: usize) -> Vec<Span<'static>> {
    let labels = WORD_COUNT_OPTIONS.iter().map(|s| s.to_string()).collect();
    options_strip_spans(labels, selected_idx)
}

fn mode_selector_spans(mode: &TestMode) -> Vec<Span<'static>> {
    let selected_style = Style::new().add_modifier(Modifier::BOLD);
    let unselected_style = Style::new().dim();
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

    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(80),
        Constraint::Fill(1),
    ])
    .split(area);
    let content = horizontal[1];

    if is_running {
        render_typing_running(model, frame, content);
    } else {
        render_typing_idle(model, frame, content);
    }
}

fn render_typing_running(model: &Model, frame: &mut Frame, content: Rect) {
    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1), // counter (countdown or word progress)
        Constraint::Length(1), // spacer
        Constraint::Length(3), // word block
        Constraint::Fill(1),
    ])
    .split(content);

    let counter_area = vertical[1];
    let words_area = vertical[3];

    let counter_text = match model.config.test_mode {
        TestMode::Time => {
            let remaining = model
                .config
                .time_limit
                .saturating_sub(model.session.elapsed);
            format!("{}", remaining.as_secs())
        }
        TestMode::Words => {
            // current_word is 0-indexed; shows completed word count (0 at start,
            // increments to 1 only after the first word is committed and space pressed)
            let total = model.session.words.len();
            format!("{}/{}", model.session.current_word, total)
        }
    };

    frame.render_widget(
        Paragraph::new(Span::styled(
            counter_text,
            Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        counter_area,
    );

    let word_lines = build_word_lines(model, words_area.width);
    frame.render_widget(Paragraph::new(word_lines), words_area);
}

fn render_typing_idle(model: &Model, frame: &mut Frame, content: Rect) {
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

    let mut header_spans: Vec<Span> = vec![
        Span::styled("ktype", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
    ];
    header_spans.extend(mode_selector_spans(&model.config.test_mode));
    header_spans.push(Span::raw("   "));
    match model.config.test_mode {
        TestMode::Time => {
            header_spans.extend(duration_strip_spans(model.config.selected_duration_idx))
        }
        TestMode::Words => {
            header_spans.extend(word_count_strip_spans(model.config.selected_word_count_idx))
        }
    }
    header_spans.push(Span::raw("   "));
    header_spans.push(Span::styled("[←→] cycle", Style::new().dim()));
    header_spans.push(Span::raw("  "));
    header_spans.push(Span::styled("[tab] restart", Style::new().dim()));
    header_spans.push(Span::raw("  "));
    let mode_hint = match model.config.test_mode {
        TestMode::Time => "[shift+tab] → word mode",
        TestMode::Words => "[shift+tab] → time mode",
    };
    header_spans.push(Span::styled(mode_hint, Style::new().dim()));

    frame.render_widget(Paragraph::new(Line::from(header_spans)), header_area);

    let word_lines = build_word_lines(model, words_area.width);
    frame.render_widget(Paragraph::new(word_lines), words_area);

    frame.render_widget(
        Paragraph::new(Span::styled("[esc] quit", Style::new().dim())),
        footer_area,
    );
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
    let scroll = current_line.saturating_sub(1);

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
    use crate::model::{Config, DURATION_OPTIONS, Model, Screen, SessionState, Word};
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
            config: Config::default(),
            history: Vec::new(),
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
            config: Config::default(),
            history: Vec::new(),
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
            config: Config::default(),
            history: Vec::new(),
        };
        let output = render_to_string(&model, 80, 24);
        insta::assert_snapshot!(output);
    }
}
