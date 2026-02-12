use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;
use crate::ui::util::{empty_state, styled_table};

use super::panel_block;

pub fn render_blocking(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("🔒 Blocking");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.blocking_info.is_empty() {
        frame.render_widget(empty_state("No blocking detected", block), area);
        return;
    }

    let header = Row::new(vec!["Blocker", "", "Blocked", "Duration", "Blocker Query"])
        .style(Theme::title_style())
        .bottom_margin(0);

    let rows: Vec<Row> = snap
        .blocking_info
        .iter()
        .map(|b| {
            Row::new(vec![
                Cell::from(format!("{}", b.blocker_pid))
                    .style(Style::default().fg(Theme::border_danger())),
                Cell::from("\u{2192}"),
                Cell::from(format!("{}", b.blocked_pid))
                    .style(Style::default().fg(Theme::border_warn())),
                Cell::from(format!("{:.1}s", b.blocked_duration_secs))
                    .style(Style::default().fg(Theme::duration_color(b.blocked_duration_secs))),
                Cell::from(b.blocker_query.clone().unwrap_or_else(|| "-".into())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(2),
        Constraint::Length(8),
        Constraint::Length(9),
        Constraint::Min(15),
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.panels.blocking);
}
