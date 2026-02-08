use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use super::theme::Theme;
use super::util::truncate;

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
    let max_conns = app.server_info.max_connections;

    let brand_style = Style::default()
        .fg(Theme::header_bg())
        .bg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(Theme::border_dim());
    let normal_style = Style::default().fg(Theme::fg());
    let label_style = Style::default().fg(Theme::fg_dim());

    let mut spans = vec![
        Span::styled(" pg_glimpse ", brand_style),
        Span::styled("  ", dim_style),
        Span::styled("◆ ", Style::default().fg(Theme::border_ok())),
        Span::styled(
            format!("{}:{}", app.host, app.port),
            normal_style,
        ),
        Span::styled("/", dim_style),
        Span::styled(
            &app.dbname,
            Style::default().fg(Theme::border_active()),
        ),
        Span::styled("  ", dim_style),
        Span::styled("as ", label_style),
        Span::styled(&app.user, normal_style),
        Span::styled("  ", dim_style),
        Span::styled(
            format!("{}/{}", conns, max_conns),
            normal_style,
        ),
        Span::styled(" conns", label_style),
        Span::styled("  ", dim_style),
        Span::styled("⟳ ", label_style),
        Span::styled(
            format!("{}s", app.refresh_interval_secs),
            normal_style,
        ),
    ];

    if app.paused {
        spans.push(Span::styled("  ", dim_style));
        spans.push(Span::styled(
            " ⏸ PAUSED ",
            Style::default()
                .fg(Theme::header_bg())
                .bg(Theme::border_warn())
                .add_modifier(Modifier::BOLD),
        ));
    }

    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled("  ", dim_style));
        spans.push(Span::styled(
            format!("● {}", msg),
            Style::default().fg(Theme::border_active()),
        ));
    }

    if let Some(ref err) = app.last_error {
        spans.push(Span::styled("  ", dim_style));
        spans.push(Span::styled(
            format!("⚠ {}", truncate(err, 40)),
            Style::default()
                .fg(Theme::border_danger())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Right-align the time by adding padding
    let used_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let padding = (area.width as usize).saturating_sub(used_width + now.len() + 2);
    if padding > 0 {
        spans.push(Span::styled(" ".repeat(padding), dim_style));
    }
    spans.push(Span::styled(now, dim_style));
    spans.push(Span::styled(" ", dim_style));

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

    let speed_label = format_speed(app.replay_speed);

    let brand_style = Style::default()
        .fg(Theme::header_bg())
        .bg(Theme::border_warn())
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(Theme::border_dim());
    let normal_style = Style::default().fg(Theme::fg());
    let label_style = Style::default().fg(Theme::fg_dim());

    let mut spans = vec![
        Span::styled(" ▶ REPLAY ", brand_style),
        Span::styled("  ", dim_style),
        Span::styled("◆ ", Style::default().fg(Theme::border_warn())),
        Span::styled(
            truncate(filename, 35),
            normal_style,
        ),
        Span::styled("  ", dim_style),
        Span::styled(
            format!("{}", app.replay_position),
            Style::default().fg(Theme::border_active()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("/{}", app.replay_total),
            label_style,
        ),
        Span::styled("  ", dim_style),
        Span::styled("⟳ ", label_style),
        Span::styled(speed_label, normal_style),
        Span::styled("  ", dim_style),
    ];

    if app.replay_playing {
        spans.push(Span::styled(
            " ▶ PLAYING ",
            Style::default()
                .fg(Theme::header_bg())
                .bg(Theme::border_ok())
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::styled(
            " ⏸ PAUSED ",
            Style::default()
                .fg(Theme::header_bg())
                .bg(Theme::border_dim())
                .add_modifier(Modifier::BOLD),
        ));
    }

    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled("  ", dim_style));
        spans.push(Span::styled(
            format!("● {}", msg),
            Style::default().fg(Theme::border_active()),
        ));
    }

    // Right-align the timestamp
    let used_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let padding = (area.width as usize).saturating_sub(used_width + snap_ts.len() + 2);
    if padding > 0 {
        spans.push(Span::styled(" ".repeat(padding), dim_style));
    }
    spans.push(Span::styled(snap_ts, dim_style));
    spans.push(Span::styled(" ", dim_style));

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

