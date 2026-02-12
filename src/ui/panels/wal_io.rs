use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::db::models::{ArchiverStats, BgwriterStats, CheckpointStats, WalStats};
use crate::ui::theme::Theme;
use crate::ui::util::{format_byte_rate, format_bytes, format_compact, format_time_ms};

use super::panel_block;

pub fn render_wal_io(frame: &mut Frame, app: &App, area: Rect) {
    let emoji = if app.config.show_emojis { "💿 " } else { "" };
    let title = format!("{emoji}WAL & I/O");
    let block = panel_block(&title);

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into top section (3 columns) and bottom section (buffer I/O)
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(inner);

    // Top section: 3 columns - WAL Generation, Checkpoints, Archiver
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(sections[0]);

    // Render WAL Generation (PG14+ only)
    render_wal_column(frame, snap.wal_stats.as_ref(), app.metrics.current_wal_rate, columns[0]);

    // Render Checkpoints
    render_checkpoint_column(frame, snap.checkpoint_stats.as_ref(), columns[1]);

    // Render Archiver
    render_archiver_column(frame, snap.archiver_stats.as_ref(), columns[2]);

    // Render Buffer I/O at bottom
    render_buffer_io_row(
        frame,
        snap.checkpoint_stats.as_ref(),
        snap.bgwriter_stats.as_ref(),
        sections[1],
    );
}

fn render_wal_column(frame: &mut Frame, wal: Option<&WalStats>, wal_rate: Option<f64>, area: Rect) {
    let title_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Theme::fg_dim());
    let value_style = Style::default().fg(Theme::fg());

    let mut lines = vec![
        Line::from(Span::styled("WAL Generation", title_style)),
        Line::from(""),
    ];

    if let Some(w) = wal {
        // Show rate first (most important metric)
        let rate_display = wal_rate.map_or_else(|| "\u{2014}".into(), format_byte_rate);
        lines.push(Line::from(vec![
            Span::styled("Rate:         ", label_style),
            Span::styled(
                rate_display,
                Style::default()
                    .fg(Theme::border_active())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Records:      ", label_style),
            Span::styled(format_compact(w.wal_records), value_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Total Size:   ", label_style),
            Span::styled(format_bytes(w.wal_bytes), value_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Buffers Full: ", label_style),
            Span::styled(
                format_compact(w.wal_buffers_full),
                if w.wal_buffers_full > 0 {
                    Style::default().fg(Theme::border_warn())
                } else {
                    value_style
                },
            ),
        ]));
        if w.wal_write_time > 0.0 || w.wal_sync_time > 0.0 {
            lines.push(Line::from(vec![
                Span::styled("Write Time:   ", label_style),
                Span::styled(format_time_ms(w.wal_write_time), value_style),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Sync Time:    ", label_style),
                Span::styled(format_time_ms(w.wal_sync_time), value_style),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "N/A (PG14+)",
            Style::default().fg(Theme::fg_dim()),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_checkpoint_column(frame: &mut Frame, chkpt: Option<&CheckpointStats>, area: Rect) {
    let title_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Theme::fg_dim());
    let value_style = Style::default().fg(Theme::fg());

    let mut lines = vec![
        Line::from(Span::styled("Checkpoints", title_style)),
        Line::from(""),
    ];

    if let Some(c) = chkpt {
        let total = c.checkpoints_timed + c.checkpoints_req;
        let forced_pct = if total > 0 {
            (c.checkpoints_req as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let timed_pct = if total > 0 {
            (c.checkpoints_timed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        // Color for forced checkpoints
        let forced_color = if forced_pct > 20.0 {
            Theme::border_danger()
        } else if forced_pct > 5.0 {
            Theme::border_warn()
        } else {
            Theme::border_ok()
        };

        lines.push(Line::from(vec![
            Span::styled("Total:        ", label_style),
            Span::styled(format_compact(total), value_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Timed:        ", label_style),
            Span::styled(
                format!("{} ({:.0}%)", format_compact(c.checkpoints_timed), timed_pct),
                value_style,
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Forced:       ", label_style),
            Span::styled(
                format!("{} ({:.0}%)", format_compact(c.checkpoints_req), forced_pct),
                Style::default().fg(forced_color),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Write Time:   ", label_style),
            Span::styled(format_time_ms(c.checkpoint_write_time), value_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Sync Time:    ", label_style),
            Span::styled(format_time_ms(c.checkpoint_sync_time), value_style),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "No data",
            Style::default().fg(Theme::fg_dim()),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_archiver_column(frame: &mut Frame, archiver: Option<&ArchiverStats>, area: Rect) {
    let title_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Theme::fg_dim());
    let value_style = Style::default().fg(Theme::fg());

    let mut lines = vec![
        Line::from(Span::styled("Archiver", title_style)),
        Line::from(""),
    ];

    if let Some(a) = archiver {
        // Failed count color
        let failed_color = if a.failed_count > 0 {
            Theme::border_danger()
        } else {
            Theme::border_ok()
        };

        lines.push(Line::from(vec![
            Span::styled("Archived:     ", label_style),
            Span::styled(format_compact(a.archived_count), value_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Failed:       ", label_style),
            Span::styled(
                format_compact(a.failed_count),
                Style::default().fg(failed_color),
            ),
        ]));

        // Calculate archive lag if we have a last archived time
        if let Some(last_time) = a.last_archived_time {
            let lag = chrono::Utc::now() - last_time;
            let lag_secs = lag.num_seconds();
            let lag_str = if lag_secs < 60 {
                format!("{lag_secs}s ago")
            } else if lag_secs < 3600 {
                format!("{}m {}s ago", lag_secs / 60, lag_secs % 60)
            } else {
                format!("{}h {}m ago", lag_secs / 3600, (lag_secs % 3600) / 60)
            };

            // Color based on lag
            let lag_color = if lag_secs > 1800 {
                Theme::border_danger()
            } else if lag_secs > 300 {
                Theme::border_warn()
            } else {
                Theme::fg()
            };

            lines.push(Line::from(vec![
                Span::styled("Last Archive: ", label_style),
                Span::styled(lag_str, Style::default().fg(lag_color)),
            ]));
        }

        if let Some(ref last_wal) = a.last_archived_wal {
            // Show last 12 chars of WAL name (timeline + segment)
            let wal_display = if last_wal.len() > 12 {
                &last_wal[last_wal.len() - 12..]
            } else {
                last_wal
            };
            lines.push(Line::from(vec![
                Span::styled("Last WAL:     ", label_style),
                Span::styled(wal_display.to_string(), value_style),
            ]));
        }

        if a.failed_count > 0 {
            if let Some(ref failed_wal) = a.last_failed_wal {
                let wal_display = if failed_wal.len() > 12 {
                    &failed_wal[failed_wal.len() - 12..]
                } else {
                    failed_wal
                };
                lines.push(Line::from(vec![
                    Span::styled("Last Failed:  ", label_style),
                    Span::styled(
                        wal_display.to_string(),
                        Style::default().fg(Theme::border_danger()),
                    ),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Archiving disabled",
            Style::default().fg(Theme::fg_dim()),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_buffer_io_row(
    frame: &mut Frame,
    chkpt: Option<&CheckpointStats>,
    bgwriter: Option<&BgwriterStats>,
    area: Rect,
) {
    let title_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Theme::fg_dim());
    let value_style = Style::default().fg(Theme::fg());

    let mut spans: Vec<Span> = vec![
        Span::styled("Buffer I/O: ", title_style),
    ];

    if let Some(c) = chkpt {
        spans.push(Span::styled("Checkpoint: ", label_style));
        spans.push(Span::styled(format_compact(c.buffers_checkpoint), value_style));
        spans.push(Span::raw("   "));

        // Backend writes are bad - they bypass the bgwriter
        let backend_pct = if c.buffers_checkpoint > 0 {
            (c.buffers_backend as f64 / c.buffers_checkpoint as f64) * 100.0
        } else {
            0.0
        };
        let backend_color = if backend_pct > 5.0 {
            Theme::border_danger()
        } else if backend_pct > 1.0 {
            Theme::border_warn()
        } else {
            Theme::border_ok()
        };

        spans.push(Span::styled("Backend: ", label_style));
        spans.push(Span::styled(
            format!("{} ({:.1}%)", format_compact(c.buffers_backend), backend_pct),
            Style::default().fg(backend_color),
        ));
        spans.push(Span::raw("   "));
    }

    if let Some(b) = bgwriter {
        spans.push(Span::styled("Clean: ", label_style));
        spans.push(Span::styled(format_compact(b.buffers_clean), value_style));
        spans.push(Span::raw("   "));

        // maxwritten_clean > 0 means bgwriter is being throttled
        if b.maxwritten_clean > 0 {
            spans.push(Span::styled("Throttled: ", label_style));
            spans.push(Span::styled(
                format_compact(b.maxwritten_clean),
                Style::default().fg(Theme::border_warn()),
            ));
            spans.push(Span::raw("   "));
        }

        spans.push(Span::styled("Alloc: ", label_style));
        spans.push(Span::styled(format_compact(b.buffers_alloc), value_style));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(vec![Line::from(""), line]), area);
}
