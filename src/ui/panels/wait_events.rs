use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;
use crate::ui::util::{empty_state, truncate};

use super::panel_block;

pub fn render_wait_events(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Wait Events");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.wait_events.is_empty() {
        frame.render_widget(empty_state("No active wait events", block), area);
        return;
    }

    let max_count = snap.wait_events.iter().map(|w| w.count).max().unwrap_or(1);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let bar_width = inner.width.saturating_sub(22) as i64;

    let lines: Vec<Line> = snap
        .wait_events
        .iter()
        .map(|w| {
            let color = Theme::wait_event_color(&w.wait_event_type);
            let label = format!("{:>12}", truncate(&w.wait_event_type, 12));
            let bar_len = if max_count > 0 {
                ((w.count as f64 / max_count as f64) * bar_width as f64) as usize
            } else {
                0
            };
            let bar: String = "\u{2588}".repeat(bar_len);
            let count_str = format!(" {}", w.count);

            Line::from(vec![
                Span::styled(label, Style::default().fg(Theme::fg_dim())),
                Span::raw(" "),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(
                    count_str,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
