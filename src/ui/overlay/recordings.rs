use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;
use std::path::Path;

use crate::app::App;
use crate::ui::theme::Theme;

use super::{centered_rect, overlay_block, section_header};

pub fn render_recordings(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(80, 70, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(
        " Recordings  [j/k] nav  [Enter] open  [d] delete  [Esc] close ",
        Theme::border_active(),
    );

    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let header_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(Theme::fg_dim());
    let selected_style = Style::default()
        .fg(Theme::overlay_bg())
        .bg(Theme::border_active())
        .add_modifier(Modifier::BOLD);

    let mut lines = vec![
        Line::from(""),
        section_header("Available Recordings"),
        Line::from(""),
    ];

    if app.recordings.list.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("No recordings found.", dim_style),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                "Recordings are automatically created when running in live mode.",
                dim_style,
            ),
        ]));
    } else {
        // Header row
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(format!("{:<32}", "Connection"), header_style),
            Span::styled(format!("{:<22}", "Recorded At"), header_style),
            Span::styled(format!("{:<8}", "Version"), header_style),
            Span::styled("Size", header_style),
        ]));

        // Separator
        lines.push(Line::from(vec![Span::styled(
            format!("    {}", "─".repeat(70)),
            Style::default().fg(Theme::border_dim()),
        )]));

        // Data rows
        for (i, recording) in app.recordings.list.iter().enumerate() {
            let is_selected = i == app.recordings.selected;
            let indicator = if is_selected { "  > " } else { "    " };

            let connection = recording.connection_display();
            let connection = if connection.len() > 30 {
                format!("{}...", &connection[..27])
            } else {
                connection
            };

            let date = recording.recorded_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let version = recording.pg_version_short();
            let size = recording.size_display();

            let row_style = if is_selected {
                selected_style
            } else {
                dim_style
            };

            lines.push(Line::from(vec![
                Span::styled(indicator, key_style),
                Span::styled(format!("{connection:<30}  "), row_style),
                Span::styled(format!("{date:<20}  "), row_style),
                Span::styled(format!("{version:<6}  "), row_style),
                Span::styled(format!("{size:>6}"), row_style),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    Press ", dim_style),
        Span::styled("Enter", key_style),
        Span::styled(" to start replay, ", dim_style),
        Span::styled("d", key_style),
        Span::styled(" to delete", dim_style),
    ]));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

pub fn render_confirm_delete_recording(frame: &mut Frame, path: &Path, area: Rect) {
    let popup = centered_rect(50, 25, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Delete Recording ", Theme::border_danger());

    let key_style = Style::default()
        .fg(Theme::border_danger())
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(Theme::fg_dim());
    let filename_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);

    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown");

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Delete this recording?", dim_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(filename, filename_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", dim_style),
            Span::styled("y", key_style),
            Span::styled(" to confirm, any other key to cancel", dim_style),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}
