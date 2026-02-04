use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::{App, BottomPanel, SortColumn};
use super::theme::Theme;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let total_count = app
        .snapshot
        .as_ref()
        .map_or(0, |s| s.active_queries.len());

    let indices = app.sorted_query_indices();
    let filtered_count = indices.len();

    let sort_indicator = |col: SortColumn| -> &str {
        if app.sort_column == col {
            if app.sort_ascending {
                " ↑"
            } else {
                " ↓"
            }
        } else {
            ""
        }
    };

    let title = if app.bottom_panel == BottomPanel::Queries && (app.filter_active || (!app.filter_text.is_empty() && app.view_mode == crate::app::ViewMode::Filter)) {
        format!(
            " Queries [{}/{}] (filter: {}) ",
            filtered_count, total_count, app.filter_text
        )
    } else {
        format!(" Queries [{}] ", total_count)
    };
    let block = Block::default()
        .title(title)
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_style(Theme::border_active()));

    let header = Row::new(vec![
        Cell::from(format!("PID{}", sort_indicator(SortColumn::Pid))),
        Cell::from(format!("User{}", sort_indicator(SortColumn::User))),
        Cell::from("Database"),
        Cell::from(format!("Duration{}", sort_indicator(SortColumn::Duration))),
        Cell::from(format!("State{}", sort_indicator(SortColumn::State))),
        Cell::from("Wait"),
        Cell::from("Query"),
    ])
    .style(
        Style::default()
            .fg(Theme::fg())
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(0);

    let rows: Vec<Row> = match &app.snapshot {
        Some(snap) => indices
            .iter()
            .map(|&i| {
                let q = &snap.active_queries[i];
                let dur_color = Theme::duration_color(q.duration_secs);
                let state_color = Theme::state_color(q.state.as_deref());

                Row::new(vec![
                    Cell::from(q.pid.to_string()),
                    Cell::from(q.usename.clone().unwrap_or_else(|| "-".into())),
                    Cell::from(q.datname.clone().unwrap_or_else(|| "-".into()))
                        .style(Style::default().fg(Color::DarkGray)),
                    Cell::from(format_duration(q.duration_secs))
                        .style(Style::default().fg(dur_color)),
                    Cell::from(short_state(q.state.as_deref()))
                        .style(Style::default().fg(state_color)),
                    Cell::from(q.wait_event.clone().unwrap_or_else(|| "-".into()))
                        .style(Style::default().fg(if q.wait_event.is_some() {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        })),
                    Cell::from(q.query.clone().unwrap_or_default()),
                ])
            })
            .collect(),
        None => vec![],
    };

    let widths = [
        Constraint::Fill(1), // PID
        Constraint::Fill(2), // User
        Constraint::Fill(2), // Database
        Constraint::Fill(1), // Duration
        Constraint::Fill(2), // State
        Constraint::Fill(2), // Wait
        Constraint::Fill(6), // Query (gets most space)
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Theme::highlight_bg())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    frame.render_stateful_widget(table, area, &mut app.query_table_state);
}

fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else if secs < 3600.0 {
        format!("{:.0}m{:.0}s", secs / 60.0, secs % 60.0)
    } else {
        format!("{:.0}h{:.0}m", secs / 3600.0, (secs % 3600.0) / 60.0)
    }
}

fn short_state(state: Option<&str>) -> String {
    match state {
        Some("active") => "active".into(),
        Some("idle in transaction") => "idle-txn".into(),
        Some("idle in transaction (aborted)") => "idle-abort".into(),
        Some("idle") => "idle".into(),
        Some(s) => s.to_string(),
        None => "-".into(),
    }
}
