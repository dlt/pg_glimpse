mod config;
mod confirm;
mod help;
mod inspect;
mod sql_highlight;

pub use config::render_config;
pub use confirm::{
    render_cancel_choice, render_confirm_cancel, render_confirm_cancel_batch,
    render_confirm_kill, render_confirm_kill_batch, render_kill_choice,
};
pub use help::render_help;
pub use inspect::{
    render_blocking_inspect, render_extensions_inspect, render_index_inspect, render_inspect,
    render_replication_inspect, render_settings_inspect, render_statement_inspect,
    render_table_inspect, render_vacuum_inspect, render_wraparound_inspect,
};
pub use sql_highlight::highlight_sql_inline;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders};

use super::theme::Theme;

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1])[1]
}

pub(crate) fn overlay_block(title: &str, color: Color) -> Block<'_> {
    Block::default()
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(Theme::overlay_bg())
                .bg(color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(Theme::overlay_bg()))
}

/// Create a section header line with visual styling
pub(crate) fn section_header(title: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {title} "),
            Style::default()
                .fg(Theme::border_warn())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "─".repeat(40),
            Style::default().fg(Theme::border_dim()),
        ),
    ])
}

/// Create a separator line
pub(crate) fn separator_line() -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", "─".repeat(50)),
        Style::default().fg(Theme::border_dim()),
    ))
}
