use chrono::Utc;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use super::theme::Theme;
use super::util::format_bytes;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Server Stats ")
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_style(Theme::border_active()));

    let mut lines: Vec<Line> = Vec::new();

    let info = &app.server_info;

    // Line 1: PG version + uptime
    let short_version = extract_pg_version(&info.version);
    let uptime = format_uptime(info.start_time);
    lines.push(Line::from(vec![
        Span::styled(
            short_version,
            Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(Theme::border_dim())),
        Span::styled(
            format!("up {}", uptime),
            Style::default().fg(Theme::fg()),
        ),
    ]));

    if let Some(snap) = &app.snapshot {
        // Line 2: DB size + connections
        let db_size = format_bytes(snap.db_size);
        let total = snap.summary.total_backends;
        let max = info.max_connections;
        let conn_pct = if max > 0 {
            (total as f64 / max as f64 * 100.0) as i64
        } else {
            0
        };
        let conn_color = if conn_pct > 90 {
            Theme::border_danger()
        } else if conn_pct > 70 {
            Theme::border_warn()
        } else {
            Theme::fg()
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("DB: {}", db_size),
                Style::default().fg(Theme::fg()),
            ),
            Span::styled(" · ", Style::default().fg(Theme::border_dim())),
            Span::styled(
                format!("{}/{} conn ({}%)", total, max, conn_pct),
                Style::default().fg(conn_color),
            ),
        ]));

        // Line 3: Activity summary
        let active = snap.summary.active_query_count;
        let idle_txn = snap.summary.idle_in_transaction_count;
        let waiting = snap.summary.waiting_count;
        let idle_txn_color = if idle_txn > 0 {
            Theme::state_idle_txn()
        } else {
            Theme::fg()
        };
        let waiting_color = if waiting > 0 {
            Theme::border_warn()
        } else {
            Theme::fg()
        };
        lines.push(Line::from(vec![
            Span::styled("Active: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", active),
                Style::default()
                    .fg(Theme::state_active())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Idle/Txn: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", idle_txn),
                Style::default().fg(idle_txn_color),
            ),
            Span::styled("  Wait: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", waiting),
                Style::default().fg(waiting_color),
            ),
        ]));

        // Line 4: Locks + longest query
        let locks = snap.summary.lock_count;
        let lock_color = if locks > 0 {
            Theme::border_danger()
        } else {
            Theme::border_ok()
        };
        let longest = snap
            .active_queries
            .iter()
            .filter(|q| q.state.as_deref() == Some("active"))
            .map(|q| q.duration_secs)
            .fold(0.0_f64, f64::max);
        let longest_color = Theme::duration_color(longest);
        lines.push(Line::from(vec![
            Span::styled("Locks: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", locks),
                Style::default()
                    .fg(lock_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" · ", Style::default().fg(Theme::border_dim())),
            Span::styled("Longest: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_duration_short(longest),
                Style::default().fg(longest_color),
            ),
        ]));

        // Line 5: Cache hit ratio
        let cache_pct = snap.buffer_cache.hit_ratio * 100.0;
        let cache_color = Theme::hit_ratio_color(cache_pct);
        // Worst dead tuple ratio
        let worst_dead = snap
            .table_stats
            .iter()
            .max_by(|a, b| {
                a.dead_ratio
                    .partial_cmp(&b.dead_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let dead_spans = if let Some(t) = worst_dead {
            let dead_color = if t.dead_ratio > 20.0 {
                Theme::border_danger()
            } else if t.dead_ratio > 5.0 {
                Theme::border_warn()
            } else {
                Theme::border_ok()
            };
            vec![
                Span::styled("  Dead: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.1}%", t.dead_ratio),
                    Style::default().fg(dead_color),
                ),
            ]
        } else {
            vec![]
        };
        let mut cache_line = vec![
            Span::styled("Cache: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.1}%", cache_pct),
                Style::default()
                    .fg(cache_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        cache_line.extend(dead_spans);
        lines.push(Line::from(cache_line));

        // Line 6: Wraparound
        let worst_wrap = snap
            .wraparound
            .iter()
            .max_by(|a, b| {
                a.pct_towards_wraparound
                    .partial_cmp(&b.pct_towards_wraparound)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some(w) = worst_wrap {
            let wrap_color = if w.pct_towards_wraparound > 75.0 {
                Theme::border_danger()
            } else if w.pct_towards_wraparound > 50.0 {
                Theme::border_warn()
            } else {
                Theme::border_ok()
            };
            lines.push(Line::from(vec![
                Span::styled("XID: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.1}%", w.pct_towards_wraparound),
                    Style::default().fg(wrap_color),
                ),
                Span::styled(
                    format!(" ({})", w.datname),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        // Line 7: Replication lag
        let max_replay = snap
            .replication
            .iter()
            .filter_map(|r| r.replay_lag_secs)
            .fold(None, |acc: Option<f64>, v| {
                Some(acc.map_or(v, |a: f64| a.max(v)))
            });
        match max_replay {
            Some(lag) => {
                let lag_color = if lag > 10.0 {
                    Theme::border_danger()
                } else if lag > 1.0 {
                    Theme::border_warn()
                } else {
                    Theme::border_ok()
                };
                lines.push(Line::from(vec![
                    Span::styled("Repl lag: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{:.2}s", lag),
                        Style::default().fg(lag_color),
                    ),
                ]));
            }
            None => {
                lines.push(Line::from(Span::styled(
                    "No replicas",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        // Line 8: Checkpoint stats
        if let Some(ref chkpt) = snap.checkpoint_stats {
            lines.push(Line::from(vec![
                Span::styled("Chkpt: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} timed", chkpt.checkpoints_timed),
                    Style::default().fg(Theme::fg()),
                ),
                Span::styled(" + ", Style::default().fg(Theme::border_dim())),
                Span::styled(
                    format!("{} req", chkpt.checkpoints_req),
                    Style::default().fg(if chkpt.checkpoints_req > 0 {
                        Theme::border_warn()
                    } else {
                        Theme::fg()
                    }),
                ),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Waiting for data...",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Extensions line
    let ext = &app.server_info.extensions;
    let mut ext_spans: Vec<Span> = vec![Span::styled(
        "Ext: ",
        Style::default().fg(Color::DarkGray),
    )];
    let ext_list = [
        ("ss", ext.pg_stat_statements),
        ("kc", ext.pg_stat_kcache),
        ("ws", ext.pg_wait_sampling),
        ("bc", ext.pg_buffercache),
    ];
    let mut any_ext = false;
    for (tag, enabled) in ext_list {
        if enabled {
            if any_ext {
                ext_spans.push(Span::styled(" ", Style::default()));
            }
            ext_spans.push(Span::styled(
                format!("[{}]", tag),
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::BOLD),
            ));
            any_ext = true;
        }
    }
    if !any_ext {
        ext_spans.push(Span::styled("none", Style::default().fg(Color::DarkGray)));
    }
    lines.push(Line::from(ext_spans));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn extract_pg_version(full: &str) -> String {
    // "PostgreSQL 16.2 on ..." -> "PG 16.2"
    let parts: Vec<&str> = full.split_whitespace().collect();
    if parts.len() >= 2 && parts[0] == "PostgreSQL" {
        format!("PG {}", parts[1])
    } else {
        full.chars().take(20).collect()
    }
}

fn format_uptime(start: chrono::DateTime<Utc>) -> String {
    let dur = Utc::now().signed_duration_since(start);
    let total_secs = dur.num_seconds();
    if total_secs < 0 {
        return "0s".into();
    }
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;
    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn format_duration_short(secs: f64) -> String {
    if secs < 0.001 {
        "0s".into()
    } else if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.1}s", secs)
    } else if secs < 3600.0 {
        format!("{:.0}m{:.0}s", secs / 60.0, secs % 60.0)
    } else {
        format!("{:.0}h{:.0}m", secs / 3600.0, (secs % 3600.0) / 60.0)
    }
}
