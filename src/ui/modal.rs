use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::domain::model::Overlay;
use crate::domain::update::parse_custom_time;
use crate::ui::widgets::fg;

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

pub(crate) fn render_modal(model: &crate::domain::model::Model, frame: &mut Frame) {
    let (is_time, input) = match &model.overlay {
        Overlay::CustomTime { input } => (true, input.as_str()),
        Overlay::CustomWords { input } => (false, input.as_str()),
        Overlay::None => return,
    };
    let area = frame.area();

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

    let title = if is_time {
        "Test Duration"
    } else {
        "Custom word amount"
    };

    let preview = if input.is_empty() {
        String::new()
    } else if is_time {
        let secs = parse_custom_time(input);
        if secs == 0 {
            "infinite".to_string()
        } else {
            format_duration(secs)
        }
    } else {
        match input.parse::<usize>() {
            Ok(0) => "infinite".to_string(),
            Ok(n) => format!("{} words", n),
            Err(_) => String::new(),
        }
    };
    let input_display = format!("{}_", input);

    // Time mode shows an extra hint row above the confirm line; words mode omits it.
    let (title_area, preview_area, input_area, hint_area, confirm_area) = if is_time {
        let [title_a, preview_a, input_a, hint_a, confirm_a] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner_area);
        (title_a, preview_a, input_a, Some(hint_a), confirm_a)
    } else {
        let [title_a, preview_a, input_a, confirm_a] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(inner_area);
        (title_a, preview_a, input_a, None, confirm_a)
    };

    frame.render_widget(
        Paragraph::new(Span::styled(title, fg(&model.theme.sub))).alignment(Alignment::Center),
        title_area,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(preview, fg(&model.theme.sub))).alignment(Alignment::Center),
        preview_area,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(input_display, fg(&model.theme.text))).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(fg(&model.theme.main)),
        ),
        input_area,
    );
    if let Some(hint_area) = hint_area {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "use h for hours, m for minutes (e.g. 1h30m). 0 = infinite.",
                fg(&model.theme.sub),
            ))
            .alignment(Alignment::Center),
            hint_area,
        );
    }
    frame.render_widget(
        Paragraph::new(Span::styled(
            "[enter] apply   [esc] cancel",
            fg(&model.theme.sub),
        ))
        .alignment(Alignment::Center),
        confirm_area,
    );
}
