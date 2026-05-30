use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Paragraph,
        canvas::{Canvas, Line as CanvasLine},
    },
};

use crate::domain::model::Model;
use crate::ui::widgets::fg;

pub(crate) fn render_chart(model: &Model, frame: &mut Frame, area: Rect) {
    let wpm_history = &model.session.wpm_history;

    if wpm_history.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("no data", fg(&model.theme.sub)))
                .alignment(ratatui::layout::Alignment::Center),
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
    let [y_labels_area, chart_right] =
        Layout::horizontal([Constraint::Length(y_label_width), Constraint::Fill(1)]).areas(area);

    let [canvas_area, x_labels_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(chart_right);

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
    frame.render_widget(Paragraph::new(y_lines), y_labels_area);

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
