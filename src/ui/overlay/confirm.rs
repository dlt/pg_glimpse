use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::ui::theme::Theme;

use super::{centered_rect, overlay_block, separator_line};

pub fn render_confirm_cancel(frame: &mut Frame, pid: i32, area: Rect) {
    let popup = centered_rect(50, 25, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Cancel Query ", Theme::border_warn());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Cancel query on PID ", Style::default().fg(Theme::fg())),
            Span::styled(
                format!("{pid}"),
                Style::default().fg(Theme::border_warn()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  The current query will be interrupted.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(""),
        separator_line(),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " y ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_warn()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" confirm    ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                " Esc ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_dim()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" abort", Style::default().fg(Theme::fg_dim())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}

pub fn render_confirm_kill(frame: &mut Frame, pid: i32, area: Rect) {
    let popup = centered_rect(50, 25, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Terminate Backend ", Theme::border_danger());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Terminate backend PID ", Style::default().fg(Theme::fg())),
            Span::styled(
                format!("{pid}"),
                Style::default().fg(Theme::border_danger()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ This will kill the connection entirely.",
            Style::default().fg(Theme::border_danger()),
        )),
        Line::from(""),
        separator_line(),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " y ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_danger()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" confirm    ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                " Esc ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_dim()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" abort", Style::default().fg(Theme::fg_dim())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}

pub fn render_cancel_choice(
    frame: &mut Frame,
    selected_pid: i32,
    all_pids: &[i32],
    filter: &str,
    area: Rect,
) {
    let popup = centered_rect(55, 35, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Cancel Query ", Theme::border_warn());

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
            Span::styled(
                " 1 ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_active()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" Cancel this query (PID {selected_pid})"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " a ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_warn()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" Cancel ALL {count} matching queries"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        separator_line(),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " Esc ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_dim()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" abort", Style::default().fg(Theme::fg_dim())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}

pub fn render_kill_choice(
    frame: &mut Frame,
    selected_pid: i32,
    all_pids: &[i32],
    filter: &str,
    area: Rect,
) {
    let popup = centered_rect(55, 35, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Terminate Backend ", Theme::border_danger());

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
            Span::styled(
                " 1 ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_active()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" Kill this backend (PID {selected_pid})"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " a ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_danger()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" Kill ALL {count} matching backends"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ This will terminate connections entirely.",
            Style::default().fg(Theme::border_danger()),
        )),
        Line::from(""),
        separator_line(),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " Esc ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_dim()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" abort", Style::default().fg(Theme::fg_dim())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}

pub fn render_confirm_cancel_batch(frame: &mut Frame, pids: &[i32], area: Rect) {
    let popup = centered_rect(55, 35, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Cancel Queries ", Theme::border_warn());

    let count = pids.len();
    let pids_str = if count <= 8 {
        pids.iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let first_six: Vec<_> = pids.iter().take(6).map(std::string::ToString::to_string).collect();
        format!("{}, ... (+{} more)", first_six.join(", "), count - 6)
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  Cancel {count} queries?"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  PIDs: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(pids_str, Style::default().fg(Theme::border_warn())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  The current queries will be interrupted.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(""),
        separator_line(),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " y ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_warn()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" confirm    ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                " Esc ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_dim()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" abort", Style::default().fg(Theme::fg_dim())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}

pub fn render_confirm_kill_batch(frame: &mut Frame, pids: &[i32], area: Rect) {
    let popup = centered_rect(55, 40, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Terminate Backends ", Theme::border_danger());

    let count = pids.len();
    let pids_str = if count <= 8 {
        pids.iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let first_six: Vec<_> = pids.iter().take(6).map(std::string::ToString::to_string).collect();
        format!("{}, ... (+{} more)", first_six.join(", "), count - 6)
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  Terminate {count} backends?"), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  PIDs: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(pids_str, Style::default().fg(Theme::border_danger())),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ⚠ This will kill the connections entirely.",
            Style::default().fg(Theme::border_danger()),
        )),
        Line::from(""),
        separator_line(),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " y ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_danger()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" confirm    ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                " Esc ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_dim()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" abort", Style::default().fg(Theme::fg_dim())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}
