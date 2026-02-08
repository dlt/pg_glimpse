use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::config::ConfigItem;
use super::theme::Theme;
use super::util::{format_bytes, format_lag, format_time_ms, lag_color};

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

fn overlay_block(title: &str, color: Color) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(Theme::overlay_bg()))
}

pub fn render_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Query Details  [j/k] scroll  [y] copy  [Esc] close", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(
            Paragraph::new("No data").block(block),
            popup,
        );
        return;
    };

    let idx = app.query_table_state.selected().unwrap_or(0);
    let indices = app.sorted_query_indices();
    let Some(&real_idx) = indices.get(idx) else {
        frame.render_widget(
            Paragraph::new("No query selected").block(block),
            popup,
        );
        return;
    };
    let q = &snap.active_queries[real_idx];

    let duration_color = Theme::duration_color(q.duration_secs);
    let state_color = Theme::state_color(q.state.as_deref());

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  PID:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(q.pid.to_string(), Style::default().fg(Theme::fg()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  User:      ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                q.usename.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Database:  ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                q.datname.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  State:     ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                q.state.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(state_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Duration:  ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_duration(q.duration_secs),
                Style::default().fg(duration_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Wait:      ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!(
                    "{} / {}",
                    q.wait_event_type.as_deref().unwrap_or("-"),
                    q.wait_event.as_deref().unwrap_or("-")
                ),
                Style::default().fg(if q.wait_event_type.is_some() {
                    Color::Yellow
                } else {
                    Theme::fg()
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Backend:   ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                q.backend_type.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Query:",
            Style::default()
                .fg(Theme::fg_dim())
                .add_modifier(Modifier::BOLD),
        )),
    ];
    lines.extend(highlight_sql(
        q.query.as_deref().unwrap_or("<no query>"),
        "  ",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Actions:  C cancel query  K terminate backend",
        Style::default().fg(Theme::fg_dim()),
    )));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_index_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(75, 55, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Index Details  [j/k] scroll  [y] copy  [Esc] back", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let sel = app.index_table_state.selected().unwrap_or(0);
    let indices = app.sorted_index_indices();
    let Some(&real_idx) = indices.get(sel) else {
        frame.render_widget(
            Paragraph::new("No index selected").block(block),
            popup,
        );
        return;
    };
    let idx = &snap.indexes[real_idx];

    let scan_color = if idx.idx_scan == 0 {
        Theme::border_danger()
    } else {
        Theme::border_ok()
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Schema:      ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&idx.schemaname, Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Table:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&idx.table_name, Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Index:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &idx.index_name,
                Style::default()
                    .fg(Theme::fg())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Size:        ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_bytes(idx.index_size_bytes),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Scans:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                idx.idx_scan.to_string(),
                Style::default()
                    .fg(scan_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tup Read:    ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                idx.idx_tup_read.to_string(),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tup Fetch:   ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                idx.idx_tup_fetch.to_string(),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Definition:",
            Style::default()
                .fg(Theme::fg_dim())
                .add_modifier(Modifier::BOLD),
        )),
    ];
    lines.extend(highlight_sql(&idx.index_definition, "  "));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_confirm_cancel(frame: &mut Frame, pid: i32, area: Rect) {
    let popup = centered_rect(45, 20, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Confirm Cancel", Theme::border_warn());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Cancel query on PID {}?", pid),
            Style::default()
                .fg(Theme::border_warn())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  The current query will be interrupted.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(Theme::border_warn()).add_modifier(Modifier::BOLD)),
            Span::styled(" confirm  ", Style::default().fg(Theme::fg())),
            Span::styled("any key", Style::default().fg(Theme::border_ok()).add_modifier(Modifier::BOLD)),
            Span::styled(" abort", Style::default().fg(Theme::fg())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}

pub fn render_confirm_kill(frame: &mut Frame, pid: i32, area: Rect) {
    let popup = centered_rect(45, 20, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Confirm Kill", Theme::border_danger());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Terminate backend PID {}?", pid),
            Style::default()
                .fg(Theme::border_danger())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  This will kill the connection entirely.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(Theme::border_danger()).add_modifier(Modifier::BOLD)),
            Span::styled(" confirm  ", Style::default().fg(Theme::fg())),
            Span::styled("any key", Style::default().fg(Theme::border_ok()).add_modifier(Modifier::BOLD)),
            Span::styled(" abort", Style::default().fg(Theme::fg())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, popup);
}

pub fn render_replication_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Replication Details  [j/k] scroll  [Esc] back", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let sel = app.replication_table_state.selected().unwrap_or(0);
    let Some(r) = snap.replication.get(sel) else {
        frame.render_widget(
            Paragraph::new("No replication slot selected").block(block),
            popup,
        );
        return;
    };

    let label = |s: &'static str| Span::styled(s, Style::default().fg(Theme::fg_dim()));
    let val = |s: String| Span::styled(s, Style::default().fg(Theme::fg()));
    let val_opt = |o: &Option<String>| {
        Span::styled(
            o.clone().unwrap_or_else(|| "-".into()),
            Style::default().fg(Theme::fg()),
        )
    };
    let section = |s: &'static str| {
        Line::from(Span::styled(
            s,
            Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD),
        ))
    };

    let format_timestamp = |ts: &Option<chrono::DateTime<chrono::Utc>>| -> String {
        ts.map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "-".into())
    };

    let state_color = match r.state.as_deref() {
        Some("streaming") => Theme::border_ok(),
        Some("catchup") => Theme::border_warn(),
        _ => Theme::fg(),
    };

    let lines = vec![
        Line::from(""),
        section("  Connection"),
        Line::from(vec![
            label("  PID:             "),
            val(r.pid.to_string()),
        ]),
        Line::from(vec![
            label("  User:            "),
            val_opt(&r.usename),
            label("      User SysID:    "),
            val(r.usesysid.map(|id| id.to_string()).unwrap_or_else(|| "-".into())),
        ]),
        Line::from(vec![
            label("  Application:     "),
            val_opt(&r.application_name),
        ]),
        Line::from(vec![
            label("  Client Addr:     "),
            val_opt(&r.client_addr),
            label("      Port:          "),
            val(r.client_port.map(|p| p.to_string()).unwrap_or_else(|| "-".into())),
        ]),
        Line::from(vec![
            label("  Client Hostname: "),
            val_opt(&r.client_hostname),
        ]),
        Line::from(vec![
            label("  Backend Start:   "),
            val(format_timestamp(&r.backend_start)),
        ]),
        Line::from(""),
        section("  Replication State"),
        Line::from(vec![
            label("  State:           "),
            Span::styled(
                r.state.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(state_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            label("  Sync State:      "),
            val_opt(&r.sync_state),
            label("      Sync Priority: "),
            val(r.sync_priority.map(|p| p.to_string()).unwrap_or_else(|| "-".into())),
        ]),
        Line::from(vec![
            label("  Backend Xmin:    "),
            val_opt(&r.backend_xmin),
        ]),
        Line::from(""),
        section("  WAL Positions"),
        Line::from(vec![
            label("  Sent LSN:        "),
            val_opt(&r.sent_lsn),
        ]),
        Line::from(vec![
            label("  Write LSN:       "),
            val_opt(&r.write_lsn),
        ]),
        Line::from(vec![
            label("  Flush LSN:       "),
            val_opt(&r.flush_lsn),
        ]),
        Line::from(vec![
            label("  Replay LSN:      "),
            val_opt(&r.replay_lsn),
        ]),
        Line::from(""),
        section("  Replication Lag"),
        Line::from(vec![
            label("  Write Lag:       "),
            val(format_lag(r.write_lag_secs)),
        ]),
        Line::from(vec![
            label("  Flush Lag:       "),
            val(format_lag(r.flush_lag_secs)),
        ]),
        Line::from(vec![
            label("  Replay Lag:      "),
            Span::styled(
                format_lag(r.replay_lag_secs),
                Style::default().fg(lag_color(r.replay_lag_secs)).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            label("  Reply Time:      "),
            val(format_timestamp(&r.reply_time)),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_statement_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Statement Details  [j/k] scroll  [y] copy  [Esc] back", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let sel = app.stmt_table_state.selected().unwrap_or(0);
    let indices = app.sorted_stmt_indices();
    let Some(&real_idx) = indices.get(sel) else {
        frame.render_widget(
            Paragraph::new("No statement selected").block(block),
            popup,
        );
        return;
    };
    let stmt = &snap.stat_statements[real_idx];

    let hit_color = if stmt.hit_ratio >= 0.99 {
        Theme::border_ok()
    } else if stmt.hit_ratio >= 0.90 {
        Theme::border_warn()
    } else {
        Theme::border_danger()
    };

    let label = |s: &'static str| Span::styled(s, Style::default().fg(Theme::fg_dim()));
    let val = |s: String| Span::styled(s, Style::default().fg(Theme::fg()));
    let val_bold =
        |s: String| Span::styled(s, Style::default().fg(Theme::fg()).add_modifier(Modifier::BOLD));
    let section = |s: &'static str| {
        Line::from(Span::styled(
            s,
            Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD),
        ))
    };

    let rows_per_call = if stmt.calls > 0 {
        format!("{:.1}", stmt.rows as f64 / stmt.calls as f64)
    } else {
        "-".into()
    };

    let temp_color = if stmt.temp_blks_read + stmt.temp_blks_written > 0 {
        Theme::border_warn()
    } else {
        Theme::fg()
    };

    let io_time_color = if stmt.blk_read_time + stmt.blk_write_time > 0.0 {
        Theme::border_warn()
    } else {
        Theme::fg()
    };

    let mut lines = vec![
        Line::from(vec![
            label("  Query ID:        "),
            val(stmt.queryid.to_string()),
        ]),
        Line::from(""),
        section("  Query"),
    ];
    lines.extend(highlight_sql(&stmt.query, "  "));
    lines.extend(vec![
        Line::from(""),
        section("  Execution"),
        Line::from(vec![
            label("  Calls:           "),
            val_bold(stmt.calls.to_string()),
            label("      Rows:          "),
            val(stmt.rows.to_string()),
            label("      Rows/Call:     "),
            val(rows_per_call),
        ]),
        Line::from(vec![
            label("  Total Time:      "),
            val_bold(format_time_ms(stmt.total_exec_time)),
        ]),
        Line::from(vec![
            label("  Mean Time:       "),
            val(format_time_ms(stmt.mean_exec_time)),
            label("      Min Time:      "),
            val(format_time_ms(stmt.min_exec_time)),
        ]),
        Line::from(vec![
            label("  Max Time:        "),
            val(format_time_ms(stmt.max_exec_time)),
            label("      Stddev:        "),
            val(format_time_ms(stmt.stddev_exec_time)),
        ]),
        Line::from(""),
        section("  Shared Buffers"),
        Line::from(vec![
            label("  Hit:             "),
            val(stmt.shared_blks_hit.to_string()),
            label("      Read:          "),
            val(stmt.shared_blks_read.to_string()),
        ]),
        Line::from(vec![
            label("  Dirtied:         "),
            val(stmt.shared_blks_dirtied.to_string()),
            label("      Written:       "),
            val(stmt.shared_blks_written.to_string()),
        ]),
        Line::from(vec![
            label("  Hit Ratio:       "),
            Span::styled(
                format!("{:.2}%", stmt.hit_ratio * 100.0),
                Style::default()
                    .fg(hit_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        section("  Local Buffers"),
        Line::from(vec![
            label("  Hit:             "),
            val(stmt.local_blks_hit.to_string()),
            label("      Read:          "),
            val(stmt.local_blks_read.to_string()),
        ]),
        Line::from(vec![
            label("  Dirtied:         "),
            val(stmt.local_blks_dirtied.to_string()),
            label("      Written:       "),
            val(stmt.local_blks_written.to_string()),
        ]),
        Line::from(""),
        section("  Temp & I/O"),
        Line::from(vec![
            label("  Temp Read:       "),
            Span::styled(
                stmt.temp_blks_read.to_string(),
                Style::default().fg(temp_color),
            ),
            label("      Temp Written:  "),
            Span::styled(
                stmt.temp_blks_written.to_string(),
                Style::default().fg(temp_color),
            ),
        ]),
        Line::from(vec![
            label("  Blk Read Time:   "),
            Span::styled(
                format_time_ms(stmt.blk_read_time),
                Style::default().fg(io_time_color),
            ),
            label("      Blk Write Time:"),
            Span::styled(
                format!(" {}", format_time_ms(stmt.blk_write_time)),
                Style::default().fg(io_time_color),
            ),
        ]),
    ]);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_config(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 75, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(
        "Config  [\u{2190}\u{2192}] change  [Esc] save & close",
        Theme::border_active(),
    );

    let logo_style = Style::default().fg(Theme::border_active());
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(" ██████╗  ██████╗     ██████╗ ██╗     ██╗███╗   ███╗██████╗ ███████╗███████╗", logo_style)),
        Line::from(Span::styled(" ██╔══██╗██╔════╝    ██╔════╝ ██║     ██║████╗ ████║██╔══██╗██╔════╝██╔════╝", logo_style)),
        Line::from(Span::styled(" ██████╔╝██║  ███╗   ██║  ███╗██║     ██║██╔████╔██║██████╔╝███████╗█████╗  ", logo_style)),
        Line::from(Span::styled(" ██╔═══╝ ██║   ██║   ██║   ██║██║     ██║██║╚██╔╝██║██╔═══╝ ╚════██║██╔══╝  ", logo_style)),
        Line::from(Span::styled(" ██║     ╚██████╔╝   ╚██████╔╝███████╗██║██║ ╚═╝ ██║██║     ███████║███████╗", logo_style)),
        Line::from(Span::styled(" ╚═╝      ╚═════╝     ╚═════╝ ╚══════╝╚═╝╚═╝     ╚═╝╚═╝     ╚══════╝╚══════╝", logo_style)),
        Line::from(""),
        Line::from(""),
    ];

    for (i, item) in ConfigItem::ALL.iter().enumerate() {
        let selected = i == app.config_selected;
        let indicator = if selected { "\u{25ba} " } else { "  " };

        let value_str = match item {
            ConfigItem::GraphMarker => app.config.graph_marker.label().to_string(),
            ConfigItem::ColorTheme => app.config.color_theme.label().to_string(),
            ConfigItem::RefreshInterval => format!("{}s", app.config.refresh_interval_secs),
            ConfigItem::WarnDuration => format!("{:.1}s", app.config.warn_duration_secs),
            ConfigItem::DangerDuration => format!("{:.1}s", app.config.danger_duration_secs),
            ConfigItem::RecordingRetention => {
                let secs = app.config.recording_retention_secs;
                if secs >= 3600 {
                    format!("{}h", secs / 3600)
                } else {
                    format!("{}m", secs / 60)
                }
            }
        };

        let label_style = if selected {
            Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::fg())
        };

        let value_style = if selected {
            Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::fg_dim())
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}{:<20}", indicator, item.label()),
                label_style,
            ),
            Span::styled(format!("\u{25c4} {} \u{25ba}", value_str), value_style),
        ]));
    }

    // About section
    let label_style = Style::default().fg(Theme::fg_dim());
    let value_style = Style::default().fg(Theme::fg());
    let link_style = Style::default().fg(Theme::border_active());
    let section_style = Style::default().fg(Theme::border_warn()).add_modifier(Modifier::BOLD);

    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  About", section_style)));
    lines.push(Line::from(vec![
        Span::styled("  Version:    ", label_style),
        Span::styled(env!("CARGO_PKG_VERSION"), value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  License:    ", label_style),
        Span::styled("MIT", value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Built with: ", label_style),
        Span::styled("Rust + ratatui + tokio-postgres", value_style),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  GitHub:     ", label_style),
        Span::styled("github.com/dlt/pg_glimpse", link_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Issues:     ", label_style),
        Span::styled("github.com/dlt/pg_glimpse/issues", link_style),
    ]));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

pub fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 80, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Keybindings  [j/k] scroll  [Esc] close", Theme::border_active());

    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Theme::fg());
    let section_style = Style::default()
        .fg(Theme::border_warn())
        .add_modifier(Modifier::BOLD);

    let entry = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<14}", key), key_style),
            Span::styled(desc.to_string(), desc_style),
        ])
    };

    let section = |title: &str| -> Line<'static> {
        Line::from(Span::styled(format!("  {}", title), section_style))
    };

    let lines = vec![
        Line::from(""),
        section("Navigation"),
        entry("q", "Quit application"),
        entry("Ctrl+C", "Force quit"),
        entry("p", "Pause / resume refresh"),
        entry("r", "Force refresh now"),
        entry("?", "This help screen"),
        entry(",", "Configuration"),
        Line::from(""),
        section("Panels"),
        entry("Tab", "Blocking chains"),
        entry("w", "Wait events"),
        entry("t", "Table stats"),
        entry("R", "Replication lag"),
        entry("v", "Vacuum progress"),
        entry("x", "Transaction wraparound"),
        entry("I", "Index stats"),
        entry("S", "pg_stat_statements"),
        Line::from(""),
        section("Panel Controls"),
        entry("Esc", "Back to queries (or quit from queries)"),
        entry("\u{2191} / k", "Select previous row"),
        entry("\u{2193} / j", "Select next row"),
        entry("s", "Cycle sort column"),
        entry("/", "Fuzzy filter (queries, indexes, stmts)"),
        entry("Enter", "Inspect selected row"),
        Line::from(""),
        section("Query Actions"),
        entry("C", "Cancel query (pg_cancel_backend)"),
        entry("K", "Kill backend (pg_terminate_backend)"),
        entry("y", "Yank (copy to clipboard)"),
        Line::from(""),
        section("Filter"),
        entry("/", "Open fuzzy filter"),
        entry("Enter", "Confirm filter"),
        entry("Esc", "Clear filter and close"),
        entry("Backspace", "Delete character"),
        Line::from(""),
        section("Overlay Controls"),
        entry("Esc / q", "Close overlay"),
        entry("\u{2191}/\u{2193} or j/k", "Scroll"),
        entry("g / G", "Top / bottom"),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else if secs < 3600.0 {
        format!("{:.0}m {:.0}s", secs / 60.0, secs % 60.0)
    } else {
        format!("{:.0}h {:.0}m", secs / 3600.0, (secs % 3600.0) / 60.0)
    }
}

/// SQL keywords to highlight
const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "IS", "NULL", "AS",
    "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "FULL", "CROSS", "ON", "USING",
    "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "TRUNCATE",
    "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW", "SCHEMA", "DATABASE",
    "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "UNIQUE", "CHECK", "DEFAULT",
    "CONSTRAINT", "CASCADE", "RESTRICT", "GRANT", "REVOKE", "COMMIT", "ROLLBACK",
    "BEGIN", "END", "TRANSACTION", "SAVEPOINT", "RELEASE",
    "ORDER", "BY", "ASC", "DESC", "NULLS", "FIRST", "LAST",
    "GROUP", "HAVING", "LIMIT", "OFFSET", "FETCH", "NEXT", "ROWS", "ONLY",
    "UNION", "INTERSECT", "EXCEPT", "ALL", "DISTINCT", "EXISTS",
    "CASE", "WHEN", "THEN", "ELSE", "COALESCE", "NULLIF", "CAST",
    "TRUE", "FALSE", "LIKE", "ILIKE", "SIMILAR", "BETWEEN", "ANY", "SOME",
    "WITH", "RECURSIVE", "RETURNING", "CONFLICT", "DO", "NOTHING",
    "OVER", "PARTITION", "WINDOW", "FILTER", "WITHIN", "LATERAL",
    "FOR", "SHARE", "NOWAIT", "SKIP", "LOCKED",
    "EXPLAIN", "ANALYZE", "VERBOSE", "COSTS", "BUFFERS", "TIMING", "FORMAT",
    "VACUUM", "REINDEX", "CLUSTER", "REFRESH", "MATERIALIZED",
    "TRIGGER", "FUNCTION", "PROCEDURE", "RETURNS", "LANGUAGE", "SECURITY", "DEFINER",
    "IF", "THEN", "ELSIF", "LOOP", "WHILE", "EXIT", "CONTINUE", "RETURN",
    "DECLARE", "VARIABLE", "CONSTANT", "CURSOR", "EXCEPTION", "RAISE", "PERFORM",
    "EXECUTE", "PREPARE", "DEALLOCATE",
];

/// Highlight SQL syntax for inline display (single line, for table cells)
/// Collapses whitespace and truncates to max_len
pub fn highlight_sql_inline(text: &str, max_len: usize) -> Vec<Span<'static>> {
    let keyword_style = Style::default().fg(Theme::sql_keyword());
    let string_style = Style::default().fg(Theme::sql_string());
    let number_style = Style::default().fg(Theme::sql_number());
    let default_style = Style::default().fg(Theme::fg());

    // Collapse whitespace and truncate
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let display = if collapsed.len() > max_len {
        &collapsed[..max_len]
    } else {
        &collapsed[..]
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = display.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Check for string literal
        if c == '\'' {
            let start = i;
            i += 1;
            while i < len {
                if chars[i] == '\'' {
                    if i + 1 < len && chars[i + 1] == '\'' {
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::styled(s, string_style));
            continue;
        }

        // Check for positional parameter $N
        if c == '$' && i + 1 < len && chars[i + 1].is_ascii_digit() {
            let start = i;
            i += 1;
            while i < len && chars[i].is_ascii_digit() {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::styled(s, default_style));
            continue;
        }

        // Check for number
        if c.is_ascii_digit() || (c == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num: String = chars[start..i].iter().collect();
            spans.push(Span::styled(num, number_style));
            continue;
        }

        // Check for identifier/keyword
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_uppercase();
            let style = if SQL_KEYWORDS.contains(&upper.as_str()) {
                keyword_style
            } else {
                default_style
            };
            spans.push(Span::styled(word, style));
            continue;
        }

        // Any other characters (whitespace, operators, punctuation)
        let start = i;
        while i < len {
            let ch = chars[i];
            if ch.is_alphabetic()
                || ch == '_'
                || ch.is_ascii_digit()
                || ch == '\''
                || ch == '$'
            {
                break;
            }
            i += 1;
        }
        if i == start {
            i += 1;
        }
        let other: String = chars[start..i].iter().collect();
        spans.push(Span::styled(other, default_style));
    }

    spans
}

/// Highlight SQL syntax in the given text, returning styled spans
fn highlight_sql(text: &str, indent: &str) -> Vec<Line<'static>> {
    let keyword_style = Style::default().fg(Theme::sql_keyword());
    let string_style = Style::default().fg(Theme::sql_string());
    let number_style = Style::default().fg(Theme::sql_number());
    let comment_style = Style::default().fg(Theme::sql_comment());
    let default_style = Style::default().fg(Theme::fg());

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = vec![Span::styled(indent.to_string(), default_style)];

    // Helper to push a styled string, splitting on newlines
    let push_styled = |s: String, style: Style, spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>, indent: &str, default_style: Style| {
        let parts: Vec<&str> = s.split('\n').collect();
        for (idx, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                spans.push(Span::styled(part.to_string(), style));
            }
            if idx < parts.len() - 1 {
                // There's a newline after this part
                lines.push(Line::from(std::mem::take(spans)));
                spans.push(Span::styled(indent.to_string(), default_style));
            }
        }
    };

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Check for single-line comment --
        if c == '-' && i + 1 < len && chars[i + 1] == '-' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            let comment: String = chars[start..i].iter().collect();
            push_styled(comment, comment_style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Check for multi-line comment /* */
        if c == '/' && i + 1 < len && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip */
            }
            let comment: String = chars[start..i].iter().collect();
            push_styled(comment, comment_style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Check for string literal
        if c == '\'' {
            let start = i;
            i += 1;
            while i < len {
                if chars[i] == '\'' {
                    if i + 1 < len && chars[i + 1] == '\'' {
                        i += 2; // escaped quote
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            push_styled(s, string_style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Check for dollar-quoted string $tag$...$tag$
        if c == '$' {
            let tag_start = i;
            i += 1;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            if i < len && chars[i] == '$' {
                i += 1;
                let tag: String = chars[tag_start..i].iter().collect();
                // Find closing tag
                while i < len {
                    if chars[i] == '$' {
                        let mut matches = true;
                        for (j, tc) in tag.chars().enumerate() {
                            if i + j >= len || chars[i + j] != tc {
                                matches = false;
                                break;
                            }
                        }
                        if matches {
                            i += tag.len();
                            break;
                        }
                    }
                    i += 1;
                }
                let s: String = chars[tag_start..i].iter().collect();
                push_styled(s, string_style, &mut current_spans, &mut lines, indent, default_style);
                continue;
            } else {
                // Check if it's a positional parameter like $1, $23
                let scanned: String = chars[tag_start..i].iter().collect();
                if scanned.len() > 1 && scanned[1..].chars().all(|ch| ch.is_ascii_digit()) {
                    // It's a positional parameter - treat as default style
                    push_styled(scanned, default_style, &mut current_spans, &mut lines, indent, default_style);
                    continue;
                }
                // Not a dollar-quoted string or parameter, just a $
                i = tag_start;
            }
        }

        // Check for number
        if c.is_ascii_digit() || (c == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'e' || chars[i] == 'E' || chars[i] == '+' || chars[i] == '-') {
                // Handle scientific notation carefully
                if (chars[i] == '+' || chars[i] == '-') && i > start {
                    let prev = chars[i - 1];
                    if prev != 'e' && prev != 'E' {
                        break;
                    }
                }
                i += 1;
            }
            let num: String = chars[start..i].iter().collect();
            push_styled(num, number_style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Check for identifier/keyword
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_uppercase();
            let style = if SQL_KEYWORDS.contains(&upper.as_str()) {
                keyword_style
            } else {
                default_style
            };
            push_styled(word, style, &mut current_spans, &mut lines, indent, default_style);
            continue;
        }

        // Any other characters (whitespace, operators, punctuation)
        let start = i;
        while i < len {
            let ch = chars[i];
            // Stop if we hit something that needs special handling (including newline)
            if ch == '\n'
                || ch.is_alphabetic()
                || ch == '_'
                || ch.is_ascii_digit()
                || ch == '\''
                || (ch == '-' && i + 1 < len && chars[i + 1] == '-')
                || (ch == '/' && i + 1 < len && chars[i + 1] == '*')
                || (ch == '.' && i + 1 < len && chars[i + 1].is_ascii_digit())
            {
                break;
            }
            i += 1;
        }
        // Always make progress - if nothing matched, take at least one char
        // (handles edge cases like standalone $ that isn't a dollar-quote)
        if i == start {
            i += 1;
        }
        let other: String = chars[start..i].iter().collect();
        push_styled(other, default_style, &mut current_spans, &mut lines, indent, default_style);
    }

    // Push any remaining spans as the final line
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(indent.to_string(), default_style)));
    }

    lines
}
