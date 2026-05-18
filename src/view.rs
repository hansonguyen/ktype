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
    let acc_val = if model.session.total_chars_typed == 0 {
        0.0
    } else {
        (model.session.total_chars_typed - model.session.total_errors) as f64
            / model.session.total_chars_typed as f64
            * 100.0
    };

    let vertical = Layout::vertical([
        Constraint::Length(1), // config/mode strip
        Constraint::Length(1), // spacer
        Constraint::Length(1), // "ktype" title
        Constraint::Length(1), // spacer
        Constraint::Length(1), // metric labels
        Constraint::Length(1), // metric values
        Constraint::Length(1), // spacer
        Constraint::Fill(1),   // chart
        Constraint::Length(1), // footer
    ])
    .split(area);

    // Config/mode strip
    let mut result_header: Vec<Span> = Vec::new();
    result_header.extend(mode_selector_spans(&model.config.test_mode, false));
    result_header.push(Span::raw("   "));
    match model.config.test_mode {
        TestMode::Time => result_header.extend(duration_strip_spans(
            model.config.selected_duration_idx,
            false,
        )),
        TestMode::Words => result_header.extend(word_count_strip_spans(
            model.config.selected_word_count_idx,
            false,
        )),
    }
    frame.render_widget(
        Paragraph::new(Line::from(result_header)).alignment(Alignment::Center),
        vertical[0],
    );

    // Title
    frame.render_widget(
        Paragraph::new(Span::styled(
            "ktype",
            Style::new().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        vertical[2],
    );

    // Metric labels
    frame.render_widget(
        Paragraph::new(Span::styled(
            "  wpm       raw wpm        acc",
            Style::new().dim(),
        ))
        .alignment(Alignment::Center),
        vertical[4],
    );

    // Metric values
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(format!("{:>5.0}", wpm_val)),
            Span::raw("       "),
            Span::raw(format!("{:>5.0}", raw_val)),
            Span::raw("       "),
            Span::raw(format!("{:>4.0}%", acc_val)),
        ]))
        .alignment(Alignment::Center),
        vertical[5],
    );

    // Chart
    render_chart(model, frame, vertical[7]);

    // Footer
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

    debug_assert_eq!(
        wpm_history.len(),
        error_history.len(),
        "wpm_history and error_history must be kept in sync"
    );

    // Layout: y-labels strip | canvas area
    let y_label_width = 5u16;
    let chart_h = Layout::horizontal([
        Constraint::Length(y_label_width),
        Constraint::Fill(1),
    ])
    .split(area);

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
                Style::new().dim(),
            ));
        }
    }
    frame.render_widget(Paragraph::new(y_lines), chart_h[0]);

    // X-axis labels: second markers spaced to canvas width
    let canvas_width = canvas_area.width as usize;
    let n_secs = wpm_history.len();
    let interval = if n_secs <= 15 { 1 } else if n_secs <= 60 { 5 } else { 10 };
    let mut x_buf = vec![b' '; canvas_width];
    for t in (interval..=n_secs).step_by(interval) {
        let col = (t * canvas_width) / n_secs;
        let label = t.to_string();
        let start = col.saturating_sub(label.len() / 2);
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
                // WPM line segments
                for i in 1..wpm_history.len() {
                    ctx.draw(&CanvasLine {
                        x1: (i - 1) as f64,
                        y1: wpm_history[i - 1],
                        x2: i as f64,
                        y2: wpm_history[i],
                        color: Color::LightBlue,
                    });
                }
                // Error markers (× in red, scaled into WPM range)
                for (i, &delta) in error_deltas.iter().enumerate() {
                    if delta > 0 {
                        let scaled_y =
                            (delta as f64 / max_error_delta as f64) * max_wpm;
                        ctx.print(
                            i as f64 + 0.5,
                            scaled_y,
                            Line::from(Span::styled(
                                "×",
                                Style::new().fg(Color::Red),
                            )),
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
    dimmed: bool,
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
        let style = if i == selected_idx && !dimmed {
            Style::new().add_modifier(Modifier::BOLD)
        } else {
            Style::new().dim()
        };
        spans.push(Span::styled(display, style));
    }
    spans
}

fn duration_strip_spans(selected_idx: usize, dimmed: bool) -> Vec<Span<'static>> {
    let labels = DURATION_OPTIONS.iter().map(|s| s.to_string()).collect();
    options_strip_spans(labels, selected_idx, dimmed)
}

fn word_count_strip_spans(selected_idx: usize, dimmed: bool) -> Vec<Span<'static>> {
    let labels = WORD_COUNT_OPTIONS.iter().map(|s| s.to_string()).collect();
    options_strip_spans(labels, selected_idx, dimmed)
}

fn mode_selector_spans(mode: &TestMode, is_running: bool) -> Vec<Span<'static>> {
    let selected_style = if is_running {
        Style::new().dim()
    } else {
        Style::new().add_modifier(Modifier::BOLD)
    };
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

    let is_running = model.session.status == TestStatus::Running;
    let mut header_spans: Vec<Span> = vec![
        Span::styled("ktype", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
    ];

    // Mode selector
    header_spans.extend(mode_selector_spans(&model.config.test_mode, is_running));
    header_spans.push(Span::raw("   "));

    // Options strip
    match model.config.test_mode {
        TestMode::Time => header_spans.extend(duration_strip_spans(
            model.config.selected_duration_idx,
            is_running,
        )),
        TestMode::Words => header_spans.extend(word_count_strip_spans(
            model.config.selected_word_count_idx,
            is_running,
        )),
    }

    // Context info (countdown or word counter)
    if is_running {
        match model.config.test_mode {
            TestMode::Time => {
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
            TestMode::Words => {
                let total = model.session.words.len();
                let current = (model.session.current_word + 1).min(total);
                header_spans.push(Span::raw("   "));
                header_spans.push(Span::styled(
                    format!("{}/{}", current, total),
                    Style::new().add_modifier(Modifier::BOLD),
                ));
            }
        }
    }

    // Key hints
    header_spans.push(Span::raw("   "));
    if !is_running {
        header_spans.push(Span::styled("[←→] cycle", Style::new().dim()));
        header_spans.push(Span::raw("  "));
    }
    header_spans.push(Span::styled("[tab] restart", Style::new().dim()));
    if !is_running {
        header_spans.push(Span::raw("  "));
        let mode_hint = match model.config.test_mode {
            TestMode::Time => "[shift+tab] → word mode",
            TestMode::Words => "[shift+tab] → time mode",
        };
        header_spans.push(Span::styled(mode_hint, Style::new().dim()));
    }

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
