use chrono::Utc;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use super::sparkline::render_sparkline;
use super::theme::Theme;
use super::util::{format_bytes, format_compact, format_duration};

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
        let inner_width = block.inner(area).width as usize;
        let sparkline_width = inner_width.saturating_sub(20).min(10);
        let sep_line = Line::from(Span::styled(
            "─".repeat(inner_width.saturating_sub(2)),
            Style::default().fg(Theme::border_dim()),
        ));

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

        // Separator before activity section
        lines.push(sep_line.clone());

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
        let active_spark = render_sparkline(&app.active_query_history.as_vec(), sparkline_width);
        lines.push(Line::from(vec![
            Span::styled("Active: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{}", active),
                Style::default()
                    .fg(Theme::state_active())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", active_spark),
                Style::default().fg(Theme::state_active()),
            ),
            Span::styled("  Idle/Txn: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{}", idle_txn),
                Style::default().fg(idle_txn_color),
            ),
            Span::styled("  Wait: ", Style::default().fg(Theme::fg_dim())),
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
        let lock_spark = render_sparkline(&app.lock_count_history.as_vec(), sparkline_width);
        lines.push(Line::from(vec![
            Span::styled("Locks: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{}", locks),
                Style::default()
                    .fg(lock_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", lock_spark),
                Style::default().fg(lock_color),
            ),
            Span::styled(" · ", Style::default().fg(Theme::border_dim())),
            Span::styled("Longest: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_duration(longest),
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
            let dead_color = Theme::dead_ratio_color(t.dead_ratio);
            vec![
                Span::styled("  Dead: ", Style::default().fg(Theme::fg_dim())),
                Span::styled(
                    format!("{:.1}%", t.dead_ratio),
                    Style::default().fg(dead_color),
                ),
            ]
        } else {
            vec![]
        };
        let cache_spark = render_sparkline(&app.hit_ratio_history.as_vec(), sparkline_width);
        let mut cache_line = vec![
            Span::styled("Cache: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{:.1}%", cache_pct),
                Style::default()
                    .fg(cache_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", cache_spark),
                Style::default().fg(cache_color),
            ),
        ];
        cache_line.extend(dead_spans);
        lines.push(Line::from(cache_line));

        // Separator before health section
        lines.push(sep_line.clone());

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
            let wrap_color = Theme::wraparound_color(w.pct_towards_wraparound);
            lines.push(Line::from(vec![
                Span::styled("XID: ", Style::default().fg(Theme::fg_dim())),
                Span::styled(
                    format!("{:.1}%", w.pct_towards_wraparound),
                    Style::default().fg(wrap_color),
                ),
                Span::styled(
                    format!(" ({})", w.datname),
                    Style::default().fg(Theme::fg_dim()),
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
                let lag_color = Theme::lag_color(Some(lag));
                lines.push(Line::from(vec![
                    Span::styled("Repl lag: ", Style::default().fg(Theme::fg_dim())),
                    Span::styled(
                        format!("{:.2}s", lag),
                        Style::default().fg(lag_color),
                    ),
                ]));
            }
            None => {
                lines.push(Line::from(Span::styled(
                    "No replicas",
                    Style::default().fg(Theme::fg_dim()),
                )));
            }
        }

        // Line 8: Checkpoint stats (counts with forced percentage)
        if let Some(ref chkpt) = snap.checkpoint_stats {
            let total = chkpt.checkpoints_timed + chkpt.checkpoints_req;
            let forced_pct = if total > 0 {
                chkpt.checkpoints_req as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            let forced_color = if forced_pct > 20.0 {
                Theme::border_danger()
            } else if forced_pct > 5.0 {
                Theme::border_warn()
            } else {
                Theme::border_ok()
            };

            lines.push(Line::from(vec![
                Span::styled("Chkpt: ", Style::default().fg(Theme::fg_dim())),
                Span::styled(format!("{}", total), Style::default().fg(Theme::fg())),
                Span::styled(" (", Style::default().fg(Theme::border_dim())),
                Span::styled(
                    format!("{:.1}% forced", forced_pct),
                    Style::default().fg(forced_color),
                ),
                Span::styled(")", Style::default().fg(Theme::border_dim())),
            ]));

            // Line 9: Buffer writes
            let backend_pct = if chkpt.buffers_checkpoint > 0 {
                chkpt.buffers_backend as f64 / chkpt.buffers_checkpoint as f64 * 100.0
            } else if chkpt.buffers_backend > 0 {
                100.0
            } else {
                0.0
            };
            let backend_color = if backend_pct > 10.0 {
                Theme::border_danger()
            } else if backend_pct > 5.0 {
                Theme::border_warn()
            } else {
                Theme::border_ok()
            };

            lines.push(Line::from(vec![
                Span::styled("BufW: ", Style::default().fg(Theme::fg_dim())),
                Span::styled(
                    format_compact(chkpt.buffers_checkpoint),
                    Style::default().fg(Theme::fg()),
                ),
                Span::styled(" ckpt / ", Style::default().fg(Theme::border_dim())),
                Span::styled(
                    format_compact(chkpt.buffers_backend),
                    Style::default().fg(backend_color),
                ),
                Span::styled(" backend", Style::default().fg(backend_color)),
            ]));
        }
        // Separator before extensions
        lines.push(sep_line.clone());
    } else {
        lines.push(Line::from(Span::styled(
            "Waiting for data...",
            Style::default().fg(Theme::fg_dim()),
        )));
    }

    // Extensions line
    let ext = &app.server_info.extensions;
    let mut ext_spans: Vec<Span> = vec![Span::styled(
        "Ext: ",
        Style::default().fg(Theme::fg_dim()),
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
        ext_spans.push(Span::styled("none", Style::default().fg(Theme::fg_dim())));
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


