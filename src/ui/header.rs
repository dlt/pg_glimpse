use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use super::theme::Theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();

    let conns = app
        .snapshot
        .as_ref()
        .map_or(0, |s| s.summary.total_backends);

    let mut spans = vec![
        Span::styled(
            " pg_glimpse ",
            Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            format!("{}:{}/{}", app.host, app.port, app.dbname),
            Style::default().fg(Theme::fg()),
        ),
        Span::styled(" │ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            &app.user,
            Style::default().fg(Theme::fg()),
        ),
        Span::styled(" │ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            format!("conns: {}", conns),
            Style::default().fg(Theme::fg()),
        ),
        Span::styled(" │ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            format!("{}s", app.refresh_interval_secs),
            Style::default().fg(Theme::fg()),
        ),
    ];

    if app.paused {
        spans.push(Span::styled(
            " │ PAUSED",
            Style::default()
                .fg(Theme::border_warn())
                .add_modifier(Modifier::BOLD),
        ));
    }

    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            format!(" │ {}", msg),
            Style::default().fg(Theme::border_active()),
        ));
    }

    if let Some(ref err) = app.last_error {
        spans.push(Span::styled(
            format!(" │ ERR: {}", truncate(err, 40)),
            Style::default()
                .fg(Theme::border_danger())
                .add_modifier(Modifier::BOLD),
        ));
    }

    spans.push(Span::styled(
        format!(" │ {}", now),
        Style::default().fg(Theme::border_dim()),
    ));

    let paragraph =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Theme::header_bg()));

    frame.render_widget(paragraph, area);
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
