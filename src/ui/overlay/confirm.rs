use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::ui::theme::Theme;

use super::{centered_rect, overlay_block, separator_line};

// Helper to create a styled button: " key " with background color
fn button(key: &str, bg: Color) -> Span<'static> {
    Span::styled(
        format!(" {key} "),
        Style::default()
            .fg(Theme::overlay_bg())
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    )
}

// Standard confirm/abort buttons row
fn confirm_abort_buttons(confirm_color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled("  ", Style::default()),
        button("y", confirm_color),
        Span::styled(" confirm    ", Style::default().fg(Theme::fg_dim())),
        button("Esc", Theme::border_dim()),
        Span::styled(" abort", Style::default().fg(Theme::fg_dim())),
    ])
}

// Abort-only button row
fn abort_button() -> Line<'static> {
    Line::from(vec![
        Span::styled("  ", Style::default()),
        button("Esc", Theme::border_dim()),
        Span::styled(" abort", Style::default().fg(Theme::fg_dim())),
    ])
}

// Format PIDs list with truncation for large lists
fn format_pids(pids: &[i32]) -> String {
    let count = pids.len();
    if count <= 8 {
        pids.iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let first_six: Vec<_> = pids.iter().take(6).map(std::string::ToString::to_string).collect();
        format!("{}, ... (+{} more)", first_six.join(", "), count - 6)
    }
}

// Render a confirmation dialog with standard layout
fn render_dialog(frame: &mut Frame, area: Rect, width: u16, height: u16, title: &str, border_color: Color, lines: Vec<Line<'static>>) {
    let popup = centered_rect(width, height, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(title, border_color);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}

pub fn render_confirm_cancel(frame: &mut Frame, pid: i32, area: Rect) {
    let color = Theme::border_warn();
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Cancel query on PID ", Style::default().fg(Theme::fg())),
            Span::styled(format!("{pid}"), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled("?", Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  The current query will be interrupted.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(""),
        separator_line(),
        confirm_abort_buttons(color),
    ];
    render_dialog(frame, area, 50, 25, " Cancel Query ", color, lines);
}

pub fn render_confirm_kill(frame: &mut Frame, pid: i32, area: Rect) {
    let color = Theme::border_danger();
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Terminate backend PID ", Style::default().fg(Theme::fg())),
            Span::styled(format!("{pid}"), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled("?", Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ This will kill the connection entirely.",
            Style::default().fg(color),
        )),
        Line::from(""),
        separator_line(),
        confirm_abort_buttons(color),
    ];
    render_dialog(frame, area, 50, 25, " Terminate Backend ", color, lines);
}

pub fn render_cancel_choice(
    frame: &mut Frame,
    selected_pid: i32,
    all_pids: &[i32],
    filter: &str,
    area: Rect,
) {
    let count = all_pids.len();
    let filter_display = if filter.is_empty() { "active filter" } else { filter };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Filter '", Style::default().fg(Theme::fg())),
            Span::styled(
                filter_display.to_string(),
                Style::default().fg(Theme::border_active()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("' matches {count} queries"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            button("1", Theme::border_active()),
            Span::styled(format!(" Cancel this query (PID {selected_pid})"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            button("a", Theme::border_warn()),
            Span::styled(format!(" Cancel ALL {count} matching queries"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        separator_line(),
        abort_button(),
    ];
    render_dialog(frame, area, 55, 35, " Cancel Query ", Theme::border_warn(), lines);
}

pub fn render_kill_choice(
    frame: &mut Frame,
    selected_pid: i32,
    all_pids: &[i32],
    filter: &str,
    area: Rect,
) {
    let color = Theme::border_danger();
    let count = all_pids.len();
    let filter_display = if filter.is_empty() { "active filter" } else { filter };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Filter '", Style::default().fg(Theme::fg())),
            Span::styled(
                filter_display.to_string(),
                Style::default().fg(Theme::border_active()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("' matches {count} queries"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            button("1", Theme::border_active()),
            Span::styled(format!(" Kill this backend (PID {selected_pid})"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            button("a", color),
            Span::styled(format!(" Kill ALL {count} matching backends"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ This will terminate connections entirely.",
            Style::default().fg(color),
        )),
        Line::from(""),
        separator_line(),
        abort_button(),
    ];
    render_dialog(frame, area, 55, 35, " Terminate Backend ", color, lines);
}

pub fn render_confirm_cancel_batch(frame: &mut Frame, pids: &[i32], area: Rect) {
    let color = Theme::border_warn();
    let count = pids.len();
    let pids_str = format_pids(pids);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  Cancel {count} queries?"), Style::default().fg(Theme::fg()))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  PIDs: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(pids_str, Style::default().fg(color)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  The current queries will be interrupted.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(""),
        separator_line(),
        confirm_abort_buttons(color),
    ];
    render_dialog(frame, area, 55, 35, " Cancel Queries ", color, lines);
}

pub fn render_confirm_kill_batch(frame: &mut Frame, pids: &[i32], area: Rect) {
    let color = Theme::border_danger();
    let count = pids.len();
    let pids_str = format_pids(pids);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  Terminate {count} backends?"), Style::default().fg(Theme::fg()))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  PIDs: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(pids_str, Style::default().fg(color)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ This will kill the connections entirely.",
            Style::default().fg(color),
        )),
        Line::from(""),
        separator_line(),
        confirm_abort_buttons(color),
    ];
    render_dialog(frame, area, 55, 40, " Terminate Backends ", color, lines);
}

pub fn render_confirm_reset_statements(frame: &mut Frame, area: Rect) {
    let color = Theme::border_danger();
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Reset pg_stat_statements?", Style::default().fg(Theme::fg()))),
        Line::from(""),
        Line::from(Span::styled(
            "  This will clear ALL statement statistics.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(Span::styled(
            "  Accumulated timing and execution data will be lost.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ This action cannot be undone.",
            Style::default().fg(color),
        )),
        Line::from(""),
        separator_line(),
        confirm_abort_buttons(color),
    ];
    render_dialog(frame, area, 55, 30, " Reset Statistics ", color, lines);
}
