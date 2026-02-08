use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row};
use ratatui::Frame;

use crate::app::{App, BottomPanel, SortColumn};
use super::overlay::highlight_sql_inline;
use super::theme::Theme;
use super::util::{compute_match_indices, format_duration, highlight_matches, styled_table};

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
        Cell::from("Query"),
        Cell::from(format!("User{}", sort_indicator(SortColumn::User))),
        Cell::from("Database"),
        Cell::from(format!("Duration{}", sort_indicator(SortColumn::Duration))),
        Cell::from(format!("State{}", sort_indicator(SortColumn::State))),
        Cell::from("Wait"),
    ])
    .style(
        Style::default()
            .fg(Theme::fg())
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(0);

    // Calculate query column width: Fill(6) out of total Fill(16), minus borders/highlight
    let query_width = ((area.width.saturating_sub(4)) as usize * 6 / 16).max(20);

    // Check if filtering is active
    let is_filtering = app.bottom_panel == BottomPanel::Queries
        && !app.filter_text.is_empty()
        && (app.filter_active || app.view_mode == crate::app::ViewMode::Filter);
    let filter_text = &app.filter_text;

    let rows: Vec<Row> = match &app.snapshot {
        Some(snap) => indices
            .iter()
            .map(|&i| {
                let q = &snap.active_queries[i];
                let dur_color = Theme::duration_color(q.duration_secs);
                let state_color = Theme::state_color(q.state.as_deref());
                let query_text = q.query.as_deref().unwrap_or("");
                let usename = q.usename.clone().unwrap_or_else(|| "-".into());
                let datname = q.datname.clone().unwrap_or_else(|| "-".into());

                // Compute match indices if filtering
                let match_indices = if is_filtering {
                    compute_match_indices(query_text, filter_text)
                } else {
                    None
                };

                // Build query cell with optional highlighting
                let query_cell = if let Some(indices) = match_indices {
                    // Truncate query_text for display
                    let display_text = if query_text.len() > query_width {
                        format!("{}…", &query_text[..query_width.saturating_sub(1)])
                    } else {
                        query_text.to_string()
                    };

                    let spans = highlight_matches(
                        &display_text,
                        &indices,
                        Style::default().fg(Theme::fg()),
                    );
                    Cell::from(Line::from(spans))
                } else {
                    Cell::from(Line::from(highlight_sql_inline(query_text, query_width)))
                };

                Row::new(vec![
                    Cell::from(q.pid.to_string()),
                    query_cell,
                    Cell::from(usename),
                    Cell::from(datname).style(Style::default().fg(Theme::fg_dim())),
                    Cell::from(format_duration(q.duration_secs))
                        .style(Style::default().fg(dur_color)),
                    Cell::from(short_state(q.state.as_deref()))
                        .style(Style::default().fg(state_color)),
                    Cell::from(q.wait_event.clone().unwrap_or_else(|| "-".into()))
                        .style(Style::default().fg(if q.wait_event.is_some() {
                            Color::Yellow
                        } else {
                            Theme::fg_dim()
                        })),
                ])
            })
            .collect(),
        None => vec![],
    };

    let widths = [
        Constraint::Fill(1), // PID
        Constraint::Fill(6), // Query (gets most space)
        Constraint::Fill(2), // User
        Constraint::Fill(2), // Database
        Constraint::Fill(1), // Duration
        Constraint::Fill(2), // State
        Constraint::Fill(2), // Wait
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.query_table_state);
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
