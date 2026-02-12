use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;
use crate::ui::util::{empty_state, styled_table, truncate};

use super::panel_block;

pub fn render_vacuum_progress(frame: &mut Frame, app: &mut App, area: Rect) {
    let emoji = if app.config.show_emojis { "🧹 " } else { "" };
    let title = format!("{emoji}Vacuum");
    let block = panel_block(&title);

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.vacuum_progress.is_empty() {
        frame.render_widget(empty_state("No vacuums running", block), area);
        return;
    }

    let header = Row::new(vec!["PID", "Table", "Phase", "Progress", "Dead Tuples"])
        .style(Theme::title_style())
        .bottom_margin(0);

    let rows: Vec<Row> = snap
        .vacuum_progress
        .iter()
        .map(|v| {
            Row::new(vec![
                Cell::from(v.pid.to_string()),
                Cell::from(truncate(&v.table_name, 30)),
                Cell::from(truncate(&v.phase, 20)),
                Cell::from(format!("{:.1}%", v.progress_pct)),
                Cell::from(v.num_dead_tuples.to_string()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(12),
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.panels.vacuum);
}
