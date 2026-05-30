use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::domain::metrics;
use crate::domain::model::Model;
use crate::domain::test_config::{DURATION_OPTIONS, TestMode, WORD_COUNT_OPTIONS};
use crate::ui::chart::render_chart;
use crate::ui::widgets::{duration_strip_spans, fg, mode_selector_spans, word_count_strip_spans};

pub(crate) fn render_results(model: &Model, frame: &mut Frame) {
    let area = frame.area();

    let correct_words = metrics::count_correct_words(&model.session.words);
    let committed_words = metrics::count_committed_words(&model.session.words);
    let elapsed = model.session.elapsed;

    let wpm_val = metrics::wpm(correct_words, elapsed);
    let raw_val = metrics::raw_wpm(committed_words, elapsed);
    let acc_val =
        metrics::raw_accuracy(model.session.total_chars_typed, model.session.total_errors);

    // Horizontally center the results block
    let [_, area, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(120),
        Constraint::Fill(1),
    ])
    .areas(area);

    let [
        mode_strip_area,
        _,
        _,
        content_area,
        bottom_stats_area,
        _,
        footer_area,
    ] = Layout::vertical([
        Constraint::Length(1), // mode strip
        Constraint::Length(1), // spacer
        Constraint::Fill(1),   // top padding (vertical centering)
        Constraint::Max(18),   // main content: left stats + chart
        Constraint::Length(3), // bottom stats row
        Constraint::Fill(1),   // bottom padding (vertical centering)
        Constraint::Length(1), // footer
    ])
    .areas(area);

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
        mode_strip_area,
    );

    // Main content: left stats panel | chart
    let [left_panel, chart_area] =
        Layout::horizontal([Constraint::Length(14), Constraint::Fill(1)]).areas(content_area);

    // Left stats panel
    let [wpm_label, wpm_val_area, _, acc_label, acc_val_area, _] = Layout::vertical([
        Constraint::Length(1), // "wpm" label
        Constraint::Length(1), // wpm value
        Constraint::Length(1), // spacer
        Constraint::Length(1), // "acc" label
        Constraint::Length(1), // acc value
        Constraint::Fill(1),   // fill
    ])
    .areas(left_panel);

    frame.render_widget(
        Paragraph::new(Span::styled("wpm", fg(&model.theme.sub))),
        wpm_label,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}", wpm_val),
            fg(&model.theme.main).add_modifier(Modifier::BOLD),
        )),
        wpm_val_area,
    );
    frame.render_widget(
        Paragraph::new(Span::styled("acc", fg(&model.theme.sub))),
        acc_label,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}%", acc_val),
            fg(&model.theme.text).add_modifier(Modifier::BOLD),
        )),
        acc_val_area,
    );

    // Chart fills the right side
    render_chart(model, frame, chart_area);

    // Bottom stats: left (test type) | right (raw + time)
    let [bottom_left, bottom_right] =
        Layout::horizontal([Constraint::Length(32), Constraint::Fill(1)]).areas(bottom_stats_area);

    // Bottom-left: test type info
    let [test_type_label, mode_detail_area, word_bank_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(bottom_left);

    frame.render_widget(
        Paragraph::new(Span::styled("test type", fg(&model.theme.sub))),
        test_type_label,
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
        mode_detail_area,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            model.config.word_bank_label(),
            fg(&model.theme.sub),
        )),
        word_bank_area,
    );

    // Bottom-right: raw wpm | time
    let [br_raw_area, br_time_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(bottom_right);

    let [raw_label, raw_val_area, _] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(br_raw_area);

    let [time_label, time_val_area, _] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(br_time_area);

    frame.render_widget(
        Paragraph::new(Span::styled("raw", fg(&model.theme.sub))),
        raw_label,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:.0}", raw_val),
            fg(&model.theme.text).add_modifier(Modifier::BOLD),
        )),
        raw_val_area,
    );

    frame.render_widget(
        Paragraph::new(Span::styled("time", fg(&model.theme.sub))),
        time_label,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{}s", elapsed.as_secs()),
            fg(&model.theme.text).add_modifier(Modifier::BOLD),
        )),
        time_val_area,
    );

    // Footer
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[tab] change/restart", fg(&model.theme.sub)),
            Span::raw("   "),
            Span::styled("[esc] quit", fg(&model.theme.sub)),
        ]))
        .alignment(Alignment::Center),
        footer_area,
    );
}
