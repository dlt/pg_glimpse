use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use super::theme::Theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    if app.replay_mode {
        render_replay(frame, app, area);
    } else {
        render_live(frame, app, area);
    }
}

fn render_live(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_replay(frame: &mut Frame, app: &App, area: Rect) {
    let filename = app
        .replay_filename
        .as_deref()
        .unwrap_or("unknown");

    let snap_ts = app
        .snapshot
        .as_ref()
        .map(|s| s.timestamp.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "--:--:--".to_string());

    let play_state = if app.replay_playing {
        "PLAYING"
    } else {
        "PAUSED"
    };

    let speed_label = format_speed(app.replay_speed);

    let mut spans = vec![
        Span::styled(
            " REPLAY ",
            Style::default()
                .fg(Theme::border_warn())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            truncate(filename, 40),
            Style::default().fg(Theme::fg()),
        ),
        Span::styled(" │ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            format!("[{}/{}]", app.replay_position, app.replay_total),
            Style::default().fg(Theme::border_active()),
        ),
        Span::styled(" │ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            speed_label,
            Style::default().fg(Theme::fg()),
        ),
        Span::styled(" │ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            play_state,
            Style::default()
                .fg(if app.replay_playing {
                    Theme::border_ok()
                } else {
                    Theme::border_warn()
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            snap_ts,
            Style::default().fg(Theme::border_dim()),
        ),
    ];

    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            format!(" │ {}", msg),
            Style::default().fg(Theme::border_active()),
        ));
    }

    let paragraph =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Theme::header_bg()));

    frame.render_widget(paragraph, area);
}

fn format_speed(speed: f64) -> String {
    if speed == (speed as u32) as f64 {
        format!("{}x", speed as u32)
    } else {
        format!("{:.2}x", speed)
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
