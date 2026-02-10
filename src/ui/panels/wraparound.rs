use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;
use crate::ui::util::{empty_state, format_compact, styled_table};

use super::panel_block;

pub fn render_wraparound(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Transaction Wraparound");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.wraparound.is_empty() {
        frame.render_widget(empty_state("No databases found", block), area);
        return;
    }

    let header = Row::new(vec!["Database", "XID Age", "Remaining", "% Used"])
        .style(Theme::title_style())
        .bottom_margin(0);

    let rows: Vec<Row> = snap
        .wraparound
        .iter()
        .map(|w| {
            let pct_color = Theme::wraparound_color(w.pct_towards_wraparound);
            Row::new(vec![
                Cell::from(w.datname.clone()),
                Cell::from(format_compact(i64::from(w.xid_age))),
                Cell::from(format_compact(w.xids_remaining)),
                Cell::from(format!("{:.2}%", w.pct_towards_wraparound))
                    .style(Style::default().fg(pct_color)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(16),
        Constraint::Length(16),
        Constraint::Length(16),
        Constraint::Length(10),
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.wraparound_table_state);
}
