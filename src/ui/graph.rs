use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

fn dim(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(r / 3, g / 3, b / 3),
        other => other,
    }
}

fn make_block<'a>(title: &'a str, current_label: &'a str, color: Color, border_color: Color) -> Block<'a> {
    let title_line = Line::from(vec![
        Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(Color::Rgb(192, 202, 245))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("── {} ", current_label),
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    Block::default()
        .title(title_line)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
}

pub fn render_line_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    current_label: &str,
    data: &[u64],
    color: Color,
    border_color: Color,
) {
    let block = make_block(title, current_label, color, border_color);

    if data.is_empty() || area.width < 4 || area.height < 4 {
        frame.render_widget(block, area);
        return;
    }

    let max_val = data.iter().copied().max().unwrap_or(1).max(1) as f64;
    let y_ceil = nice_ceil(max_val);
    let n = data.len();
    let x_max = (n - 1).max(1) as f64;
    let fill_color = dim(color);

    let data_owned: Vec<u64> = data.to_vec();

    let canvas = Canvas::default()
        .block(block)
        .marker(Marker::Braille)
        .x_bounds([0.0, x_max])
        .y_bounds([0.0, y_ceil])
        .paint(move |ctx| {
            // Fill: vertical lines from baseline to value
            for (i, &val) in data_owned.iter().enumerate() {
                if val > 0 {
                    ctx.draw(&CanvasLine {
                        x1: i as f64,
                        y1: 0.0,
                        x2: i as f64,
                        y2: val as f64,
                        color: fill_color,
                    });
                }
            }
            // Top edge: connecting line between consecutive points
            for i in 0..data_owned.len().saturating_sub(1) {
                ctx.draw(&CanvasLine {
                    x1: i as f64,
                    y1: data_owned[i] as f64,
                    x2: (i + 1) as f64,
                    y2: data_owned[i + 1] as f64,
                    color,
                });
            }
        });

    frame.render_widget(canvas, area);
}

pub fn render_ratio_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    current_label: &str,
    data: &[u64],
    color: Color,
    border_color: Color,
) {
    let block = make_block(title, current_label, color, border_color);

    if data.is_empty() || area.width < 4 || area.height < 4 {
        frame.render_widget(block, area);
        return;
    }

    let n = data.len();
    let x_max = (n - 1).max(1) as f64;
    let fill_color = dim(color);

    let data_owned: Vec<u64> = data.to_vec();

    let canvas = Canvas::default()
        .block(block)
        .marker(Marker::Braille)
        .x_bounds([0.0, x_max])
        .y_bounds([0.0, 1000.0])
        .paint(move |ctx| {
            for (i, &val) in data_owned.iter().enumerate() {
                if val > 0 {
                    ctx.draw(&CanvasLine {
                        x1: i as f64,
                        y1: 0.0,
                        x2: i as f64,
                        y2: val as f64,
                        color: fill_color,
                    });
                }
            }
            for i in 0..data_owned.len().saturating_sub(1) {
                ctx.draw(&CanvasLine {
                    x1: i as f64,
                    y1: data_owned[i] as f64,
                    x2: (i + 1) as f64,
                    y2: data_owned[i + 1] as f64,
                    color,
                });
            }
        });

    frame.render_widget(canvas, area);
}

fn nice_ceil(val: f64) -> f64 {
    if val <= 0.0 {
        return 10.0;
    }
    let magnitude = 10.0_f64.powf(val.log10().floor());
    let normalized = val / magnitude;
    let nice = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * magnitude
}
