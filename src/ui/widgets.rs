use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use crate::domain::test_config::{DURATION_OPTIONS, TestMode, WORD_COUNT_OPTIONS};

pub(crate) fn fg(color: &crate::config::theme::HexColor) -> Style {
    Style::new().fg(color.to_ratatui_color())
}

pub(crate) fn toggle_span(
    symbol: &'static str,
    label: &'static str,
    active: bool,
    theme: &crate::config::theme::Theme,
) -> Span<'static> {
    if active {
        Span::styled(
            format!("[{} {}]", symbol, label),
            fg(&theme.text).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(format!("{} {}", symbol, label), fg(&theme.sub))
    }
}

fn options_strip_spans(
    labels: Vec<String>,
    selected_idx: usize,
    theme: &crate::config::theme::Theme,
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

pub(crate) fn duration_strip_spans(
    selected_idx: usize,
    custom_time_secs: Option<u64>,
    theme: &crate::config::theme::Theme,
) -> Vec<Span<'static>> {
    let mut labels: Vec<String> = DURATION_OPTIONS.iter().map(|s| s.to_string()).collect();
    labels.push(match custom_time_secs {
        None => "custom".to_string(),
        Some(0) => "\u{221e}".to_string(),
        Some(n) => n.to_string(),
    });
    options_strip_spans(labels, selected_idx, theme)
}

pub(crate) fn word_count_strip_spans(
    selected_idx: usize,
    custom_word_count: Option<usize>,
    theme: &crate::config::theme::Theme,
) -> Vec<Span<'static>> {
    let mut labels: Vec<String> = WORD_COUNT_OPTIONS.iter().map(|s| s.to_string()).collect();
    labels.push(match custom_word_count {
        None => "custom".to_string(),
        Some(0) => "\u{221e}".to_string(),
        Some(n) => n.to_string(),
    });
    options_strip_spans(labels, selected_idx, theme)
}

pub(crate) fn mode_selector_spans(
    mode: &TestMode,
    theme: &crate::config::theme::Theme,
) -> Vec<Span<'static>> {
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
