use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, Wrap,
};
use ratatui::Frame;

use crate::app::{App, IndexSortColumn};
use crate::config::ConfigItem;
use super::theme::Theme;

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

    let block = overlay_block("Query Details  [Esc] close", Theme::border_active());

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

    let lines = vec![
        Line::from(vec![
            Span::styled("  PID:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(q.pid.to_string(), Style::default().fg(Theme::fg()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  User:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                q.usename.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Database:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                q.datname.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  State:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                q.state.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(state_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Duration:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_duration(q.duration_secs),
                Style::default().fg(duration_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Wait:      ", Style::default().fg(Color::DarkGray)),
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
            Span::styled("  Backend:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                q.backend_type.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Query:",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("  {}", q.query.clone().unwrap_or_else(|| "<no query>".into())),
            Style::default().fg(Theme::fg()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Actions:  C cancel query  K terminate backend",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
}

pub fn render_blocking(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(75, 60, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Blocking Chains  [Esc] close", Theme::border_danger());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(
            Paragraph::new("No data").block(block),
            popup,
        );
        return;
    };

    if snap.blocking_info.is_empty() {
        let msg = Paragraph::new("\n  No blocking detected")
            .style(
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(block);
        frame.render_widget(msg, popup);
        return;
    }

    let header = Row::new(vec!["Blocker", "", "Blocked", "Duration", "Blocker Query"])
        .style(Theme::title_style())
        .bottom_margin(0);

    let rows: Vec<Row> = snap
        .blocking_info
        .iter()
        .map(|b| {
            Row::new(vec![
                Cell::from(format!("{}", b.blocker_pid))
                    .style(Style::default().fg(Theme::border_danger())),
                Cell::from("→"),
                Cell::from(format!("{}", b.blocked_pid))
                    .style(Style::default().fg(Theme::border_warn())),
                Cell::from(format!("{:.1}s", b.blocked_duration_secs))
                    .style(Style::default().fg(Theme::duration_color(b.blocked_duration_secs))),
                Cell::from(
                    b.blocker_query.clone().unwrap_or_else(|| "-".into()),
                ),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(2),
        Constraint::Length(8),
        Constraint::Length(9),
        Constraint::Min(15),
    ];

    let table = Table::new(rows, widths).header(header).block(block);
    frame.render_widget(table, popup);
}

pub fn render_wait_events(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 55, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Wait Events  [Esc] close", Theme::border_warn());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(
            Paragraph::new("No data").block(block),
            popup,
        );
        return;
    };

    if snap.wait_events.is_empty() {
        let msg = Paragraph::new("\n  No active wait events")
            .style(
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(block);
        frame.render_widget(msg, popup);
        return;
    }

    let max_count = snap.wait_events.iter().map(|w| w.count).max().unwrap_or(1);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let bar_width = inner.width.saturating_sub(22) as i64;

    let lines: Vec<Line> = snap
        .wait_events
        .iter()
        .map(|w| {
            let color = Theme::wait_event_color(&w.wait_event_type);
            let label = format!("{:>12}", truncate(&w.wait_event_type, 12));
            let bar_len = if max_count > 0 {
                ((w.count as f64 / max_count as f64) * bar_width as f64) as usize
            } else {
                0
            };
            let bar: String = "█".repeat(bar_len);
            let count_str = format!(" {}", w.count);

            Line::from(vec![
                Span::styled(label, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(count_str, Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
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
            Style::default().fg(Color::DarkGray),
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
            Style::default().fg(Color::DarkGray),
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

pub fn render_table_stats(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(80, 70, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Table Stats  [Esc] close", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    if snap.table_stats.is_empty() {
        let msg = Paragraph::new("\n  No user tables found")
            .style(
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(block);
        frame.render_widget(msg, popup);
        return;
    }

    let header = Row::new(vec![
        "Table", "Size", "SeqScan", "IdxScan", "Dead", "Dead%", "Last Vacuum",
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    let rows: Vec<Row> = snap
        .table_stats
        .iter()
        .map(|t| {
            let dead_color = if t.dead_ratio > 20.0 {
                Theme::border_danger()
            } else if t.dead_ratio > 5.0 {
                Theme::border_warn()
            } else {
                Theme::fg()
            };
            Row::new(vec![
                Cell::from(format!("{}.{}", t.schemaname, truncate(&t.relname, 20))),
                Cell::from(format_bytes(t.total_size_bytes)),
                Cell::from(t.seq_scan.to_string()),
                Cell::from(t.idx_scan.to_string()),
                Cell::from(t.n_dead_tup.to_string()).style(Style::default().fg(dead_color)),
                Cell::from(format!("{:.1}%", t.dead_ratio)).style(Style::default().fg(dead_color)),
                Cell::from(
                    t.last_autovacuum
                        .map(|ts| ts.format("%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "never".into()),
                ),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(9),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(13),
    ];

    let table = Table::new(rows, widths).header(header).block(block);
    frame.render_widget(table, popup);
}

pub fn render_replication(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(75, 50, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Replication Lag  [Esc] close", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    if snap.replication.is_empty() {
        let msg = Paragraph::new("\n  No replicas connected")
            .style(
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(block);
        frame.render_widget(msg, popup);
        return;
    }

    let header = Row::new(vec![
        "App Name", "Client", "State", "Write Lag", "Flush Lag", "Replay Lag",
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    let rows: Vec<Row> = snap
        .replication
        .iter()
        .map(|r| {
            Row::new(vec![
                Cell::from(
                    r.application_name
                        .clone()
                        .unwrap_or_else(|| "-".into()),
                ),
                Cell::from(r.client_addr.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.state.clone().unwrap_or_else(|| "-".into())),
                Cell::from(format_lag(r.write_lag_secs)),
                Cell::from(format_lag(r.flush_lag_secs)),
                Cell::from(format_lag(r.replay_lag_secs)).style(Style::default().fg(
                    lag_color(r.replay_lag_secs),
                )),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(14),
        Constraint::Length(16),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(11),
    ];

    let table = Table::new(rows, widths).header(header).block(block);
    frame.render_widget(table, popup);
}

pub fn render_vacuum_progress(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(75, 50, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Vacuum Progress  [Esc] close", Theme::border_warn());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    if snap.vacuum_progress.is_empty() {
        let msg = Paragraph::new("\n  No vacuums running")
            .style(
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(block);
        frame.render_widget(msg, popup);
        return;
    }

    let header = Row::new(vec!["PID", "Table", "Phase", "Progress", "Dead Tuples"])
        .style(Theme::title_style())
        .bottom_margin(0);

    let rows: Vec<Row> = snap
        .vacuum_progress
        .iter()
        .map(|v| {
            Row::new(vec![
                Cell::from(v.pid.to_string()),
                Cell::from(truncate(&v.table_name, 30).to_string()),
                Cell::from(truncate(&v.phase, 20).to_string()),
                Cell::from(format!("{:.1}%", v.progress_pct)),
                Cell::from(v.num_dead_tuples.to_string()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths).header(header).block(block);
    frame.render_widget(table, popup);
}

pub fn render_wraparound(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 50, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Transaction Wraparound  [Esc] close", Theme::border_warn());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    if snap.wraparound.is_empty() {
        let msg = Paragraph::new("\n  No databases found")
            .style(
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(block);
        frame.render_widget(msg, popup);
        return;
    }

    let header = Row::new(vec!["Database", "XID Age", "Remaining", "% Used"])
        .style(Theme::title_style())
        .bottom_margin(0);

    let rows: Vec<Row> = snap
        .wraparound
        .iter()
        .map(|w| {
            let pct_color = if w.pct_towards_wraparound > 75.0 {
                Theme::border_danger()
            } else if w.pct_towards_wraparound > 50.0 {
                Theme::border_warn()
            } else {
                Theme::border_ok()
            };
            Row::new(vec![
                Cell::from(w.datname.clone()),
                Cell::from(format_number(w.xid_age as i64)),
                Cell::from(format_number(w.xids_remaining)),
                Cell::from(format!("{:.2}%", w.pct_towards_wraparound))
                    .style(Style::default().fg(pct_color)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(16),
        Constraint::Length(16),
        Constraint::Length(16),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths).header(header).block(block);
    frame.render_widget(table, popup);
}

pub fn render_indexes(frame: &mut Frame, app: &mut App, area: Rect) {
    let popup = centered_rect(85, 70, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(
        "Indexes  [s] sort  [Enter] inspect  [Esc] close",
        Theme::border_active(),
    );

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    if snap.indexes.is_empty() {
        let msg = Paragraph::new("\n  No user indexes found")
            .style(
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(block);
        frame.render_widget(msg, popup);
        return;
    }

    let sort_indicator = |col: IndexSortColumn| -> &str {
        if app.index_sort_column == col {
            if app.index_sort_ascending {
                " ↑"
            } else {
                " ↓"
            }
        } else {
            ""
        }
    };

    let header = Row::new(vec![
        Cell::from("Table"),
        Cell::from("Index"),
        Cell::from(format!("Size{}", sort_indicator(IndexSortColumn::Size))),
        Cell::from(format!("Scans{}", sort_indicator(IndexSortColumn::Scans))),
        Cell::from(format!("Tup Read{}", sort_indicator(IndexSortColumn::TupRead))),
        Cell::from(format!(
            "Tup Fetch{}",
            sort_indicator(IndexSortColumn::TupFetch)
        )),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    let indices = app.sorted_index_indices();
    let rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let idx = &snap.indexes[i];
            let scan_color = if idx.idx_scan == 0 {
                Theme::border_danger()
            } else {
                Theme::fg()
            };
            Row::new(vec![
                Cell::from(format!("{}.{}", idx.schemaname, idx.table_name)),
                Cell::from(idx.index_name.clone()),
                Cell::from(format_bytes(idx.index_size_bytes)),
                Cell::from(idx.idx_scan.to_string())
                    .style(Style::default().fg(scan_color)),
                Cell::from(idx.idx_tup_read.to_string()),
                Cell::from(idx.idx_tup_fetch.to_string()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(18),
        Constraint::Min(20),
        Constraint::Length(9),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Theme::highlight_bg())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    frame.render_stateful_widget(table, popup, &mut app.index_table_state);
}

pub fn render_index_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(75, 55, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Index Details  [Esc] back", Theme::border_active());

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

    let lines = vec![
        Line::from(vec![
            Span::styled("  Schema:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&idx.schemaname, Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Table:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(&idx.table_name, Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Index:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &idx.index_name,
                Style::default()
                    .fg(Theme::fg())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Size:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_bytes(idx.index_size_bytes),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Scans:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                idx.idx_scan.to_string(),
                Style::default()
                    .fg(scan_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tup Read:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                idx.idx_tup_read.to_string(),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tup Fetch:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                idx.idx_tup_fetch.to_string(),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Definition:",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("  {}", idx.index_definition),
            Style::default().fg(Theme::fg()),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
}

pub fn render_config(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(50, 45, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(
        "Config  [←→] change  [Esc] save & close",
        Theme::border_active(),
    );

    let mut lines = vec![Line::from("")];

    for (i, item) in ConfigItem::ALL.iter().enumerate() {
        let selected = i == app.config_selected;
        let indicator = if selected { "► " } else { "  " };

        let value_str = match item {
            ConfigItem::GraphMarker => app.config.graph_marker.label().to_string(),
            ConfigItem::ColorTheme => app.config.color_theme.label().to_string(),
            ConfigItem::RefreshInterval => format!("{}s", app.config.refresh_interval_secs),
            ConfigItem::WarnDuration => format!("{:.1}s", app.config.warn_duration_secs),
            ConfigItem::DangerDuration => format!("{:.1}s", app.config.danger_duration_secs),
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
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}{:<20}", indicator, item.label()),
                label_style,
            ),
            Span::styled(format!("◄ {} ►", value_str), value_style),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = 1024 * 1024;
    const GB: i64 = 1024 * 1024 * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_lag(secs: Option<f64>) -> String {
    match secs {
        Some(s) => format!("{:.3}s", s),
        None => "-".into(),
    }
}

fn lag_color(secs: Option<f64>) -> Color {
    match secs {
        Some(s) if s > 10.0 => Theme::border_danger(),
        Some(s) if s > 1.0 => Theme::border_warn(),
        _ => Theme::fg(),
    }
}

fn format_number(n: i64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
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

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
