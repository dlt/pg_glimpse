use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, BottomPanel};
use crate::config::ConfigItem;
use super::theme::Theme;
use super::util::{format_bytes, format_compact, format_duration, format_lag, format_time_ms};

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
                .fg(Theme::overlay_bg())
                .bg(color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(Theme::overlay_bg()))
}

/// Create a section header line with visual styling
fn section_header(title: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {} ", title),
            Style::default()
                .fg(Theme::border_warn())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "─".repeat(40),
            Style::default().fg(Theme::border_dim()),
        ),
    ])
}

/// Create a separator line
fn separator_line() -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", "─".repeat(50)),
        Style::default().fg(Theme::border_dim()),
    ))
}

pub fn render_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Query Details  [j/k] scroll  [y] copy query  [C] cancel  [K] kill  [Esc] close ", Theme::border_active());

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
        Line::from(""),
        section_header("Connection"),
        Line::from(vec![
            Span::styled("  PID:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(q.pid.to_string(), Style::default().fg(Theme::fg()).add_modifier(Modifier::BOLD)),
            Span::styled("     User: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                q.usename.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
            Span::styled("     DB: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                q.datname.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::border_active()),
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
        section_header("Status"),
        Line::from(vec![
            Span::styled("  State:     ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!(" {} ", q.state.clone().unwrap_or_else(|| "-".into())),
                Style::default().fg(Theme::overlay_bg()).bg(state_color),
            ),
            Span::styled("     Duration: ", Style::default().fg(Theme::fg_dim())),
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
        Line::from(""),
        section_header("Query"),
    ];
    lines.extend(highlight_sql(
        q.query.as_deref().unwrap_or("<no query>"),
        "  ",
    ));
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_index_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(75, 60, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Index Details  [j/k] scroll  [y] copy definition  [Esc] close ", Theme::border_active());

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

    let scan_color = Theme::index_usage_color(idx.idx_scan);

    let mut lines = vec![
        Line::from(""),
        section_header("Index Info"),
        Line::from(vec![
            Span::styled("  Schema:      ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&idx.schemaname, Style::default().fg(Theme::fg())),
            Span::styled("     Table: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&idx.table_name, Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Index:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &idx.index_name,
                Style::default()
                    .fg(Theme::border_active())
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
        section_header("Usage Stats"),
        Line::from(vec![
            Span::styled("  Scans:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!(" {} ", idx.idx_scan),
                Style::default()
                    .fg(Theme::overlay_bg())
                    .bg(scan_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if idx.idx_scan == 0 { "  ← unused index" } else { "" },
                Style::default().fg(Theme::border_danger()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tup Read:    ", Style::default().fg(Theme::fg_dim())),
            Span::styled(idx.idx_tup_read.to_string(), Style::default().fg(Theme::fg())),
            Span::styled("     Tup Fetch: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(idx.idx_tup_fetch.to_string(), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        section_header("Definition"),
    ];
    lines.extend(highlight_sql(&idx.index_definition, "  "));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_confirm_cancel(frame: &mut Frame, pid: i32, area: Rect) {
    let popup = centered_rect(50, 25, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Cancel Query ", Theme::border_warn());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Cancel query on PID ", Style::default().fg(Theme::fg())),
            Span::styled(
                format!("{}", pid),
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
                format!("{}", pid),
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
            Span::styled(format!("' matches {} queries", count), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " 1 ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_active()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" Cancel this query (PID {})", selected_pid), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " a ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_warn()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" Cancel ALL {} matching queries", count), Style::default().fg(Theme::fg())),
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
            Span::styled(format!("' matches {} queries", count), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " 1 ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_active()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" Kill this backend (PID {})", selected_pid), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " a ",
                Style::default().fg(Theme::overlay_bg()).bg(Theme::border_danger()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" Kill ALL {} matching backends", count), Style::default().fg(Theme::fg())),
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
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let first_six: Vec<_> = pids.iter().take(6).map(|p| p.to_string()).collect();
        format!("{}, ... (+{} more)", first_six.join(", "), count - 6)
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  Cancel {} queries?", count), Style::default().fg(Theme::fg())),
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
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let first_six: Vec<_> = pids.iter().take(6).map(|p| p.to_string()).collect();
        format!("{}, ... (+{} more)", first_six.join(", "), count - 6)
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  Terminate {} backends?", count), Style::default().fg(Theme::fg())),
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

pub fn render_replication_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Replication Details  [j/k] scroll  [y] copy app  [Esc] close ", Theme::border_active());

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
                Style::default().fg(Theme::lag_color(r.replay_lag_secs)).add_modifier(Modifier::BOLD),
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

pub fn render_table_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(75, 75, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Table Details  [j/k] scroll  [y] copy name  [Esc] close ", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let sel = app.table_stat_table_state.selected().unwrap_or(0);
    let indices = app.sorted_table_stat_indices();
    let Some(&real_idx) = indices.get(sel) else {
        frame.render_widget(
            Paragraph::new("No table selected").block(block),
            popup,
        );
        return;
    };
    let tbl = &snap.table_stats[real_idx];

    let dead_color = Theme::dead_ratio_color(tbl.dead_ratio);

    let seq_scan_color = if tbl.seq_scan > tbl.idx_scan && tbl.n_live_tup > 1000 {
        Theme::border_warn()
    } else {
        Theme::fg()
    };

    let format_timestamp = |ts: &Option<chrono::DateTime<chrono::Utc>>| -> String {
        ts.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".into())
    };

    let hot_pct = if tbl.n_tup_upd > 0 {
        tbl.n_tup_hot_upd as f64 / tbl.n_tup_upd as f64 * 100.0
    } else {
        0.0
    };
    let hot_color = if hot_pct > 80.0 {
        Theme::border_ok()
    } else if hot_pct > 50.0 {
        Theme::border_warn()
    } else if tbl.n_tup_upd > 0 {
        Theme::border_danger()
    } else {
        Theme::fg()
    };

    // Find related indexes
    let related_indexes: Vec<_> = snap.indexes.iter()
        .filter(|idx| idx.schemaname == tbl.schemaname && idx.table_name == tbl.relname)
        .collect();

    let mut lines = vec![
        Line::from(""),
        section_header("Table Info"),
        Line::from(vec![
            Span::styled("  Schema:        ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&tbl.schemaname, Style::default().fg(Theme::fg())),
            Span::styled("     Table: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &tbl.relname,
                Style::default()
                    .fg(Theme::border_active())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        section_header("Size"),
        Line::from(vec![
            Span::styled("  Total:         ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_bytes(tbl.total_size_bytes),
                Style::default().fg(Theme::fg()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Table:         ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_bytes(tbl.table_size_bytes), Style::default().fg(Theme::fg())),
            Span::styled("     Indexes: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_bytes(tbl.indexes_size_bytes), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        section_header("Row Stats"),
        Line::from(vec![
            Span::styled("  Live:          ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_compact(tbl.n_live_tup),
                Style::default().fg(Theme::fg()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("     Dead: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{} ({:.1}%)", format_compact(tbl.n_dead_tup), tbl.dead_ratio),
                Style::default().fg(dead_color),
            ),
        ]),
        Line::from(""),
        section_header("Scan Activity"),
        Line::from(vec![
            Span::styled("  Seq Scans:     ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_compact(tbl.seq_scan),
                Style::default().fg(seq_scan_color),
            ),
            Span::styled("     Rows Read: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_compact(tbl.seq_tup_read), Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Idx Scans:     ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_compact(tbl.idx_scan), Style::default().fg(Theme::fg())),
            Span::styled("     Rows Fetch: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_compact(tbl.idx_tup_fetch), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        section_header("DML Activity"),
        Line::from(vec![
            Span::styled("  Inserts:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_compact(tbl.n_tup_ins), Style::default().fg(Theme::fg())),
            Span::styled("     Updates: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_compact(tbl.n_tup_upd), Style::default().fg(Theme::fg())),
            Span::styled("     Deletes: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_compact(tbl.n_tup_del), Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  HOT Updates:   ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{} ({:.0}%)", format_compact(tbl.n_tup_hot_upd), hot_pct),
                Style::default().fg(hot_color),
            ),
        ]),
        Line::from(""),
        section_header("Maintenance"),
        Line::from(vec![
            Span::styled("  Last Vacuum:   ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_timestamp(&tbl.last_vacuum), Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Last AutoVac:  ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_timestamp(&tbl.last_autovacuum), Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Last Analyze:  ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_timestamp(&tbl.last_analyze), Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Last AutoAnly: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_timestamp(&tbl.last_autoanalyze), Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Vacuum Count:  ", Style::default().fg(Theme::fg_dim())),
            Span::styled(tbl.vacuum_count.to_string(), Style::default().fg(Theme::fg())),
            Span::styled("     AutoVac: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(tbl.autovacuum_count.to_string(), Style::default().fg(Theme::fg())),
        ]),
    ];

    // Add indexes section if any
    if !related_indexes.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header(&format!("Indexes ({})", related_indexes.len())));
        for idx in &related_indexes {
            let scan_color = Theme::index_usage_color(idx.idx_scan);
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{:<30}", &idx.index_name),
                    Style::default().fg(Theme::fg()),
                ),
                Span::styled(
                    format!(" {} ", format_bytes(idx.index_size_bytes)),
                    Style::default().fg(Theme::fg_dim()),
                ),
                Span::styled(
                    format!("{} scans", idx.idx_scan),
                    Style::default().fg(scan_color),
                ),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_blocking_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(80, 70, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Lock Details  [j/k] scroll  [y] copy query  [Esc] close ", Theme::border_danger());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let sel = app.blocking_table_state.selected().unwrap_or(0);
    let Some(info) = snap.blocking_info.get(sel) else {
        frame.render_widget(
            Paragraph::new("No blocking info selected").block(block),
            popup,
        );
        return;
    };

    let duration_color = Theme::duration_color(info.blocked_duration_secs);

    let mut lines = vec![
        Line::from(""),
        section_header("Blocked Process"),
        Line::from(vec![
            Span::styled("  PID:           ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                info.blocked_pid.to_string(),
                Style::default().fg(Theme::border_danger()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("     User: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                info.blocked_user.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Waiting:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{:.1}s", info.blocked_duration_secs),
                Style::default().fg(duration_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Query:", Style::default().fg(Theme::fg_dim()))),
    ];
    lines.extend(highlight_sql(
        info.blocked_query.as_deref().unwrap_or("<no query>"),
        "  ",
    ));

    lines.push(Line::from(""));
    lines.push(section_header("Blocking Process"));
    lines.push(Line::from(vec![
        Span::styled("  PID:           ", Style::default().fg(Theme::fg_dim())),
        Span::styled(
            info.blocker_pid.to_string(),
            Style::default().fg(Theme::border_warn()).add_modifier(Modifier::BOLD),
        ),
        Span::styled("     User: ", Style::default().fg(Theme::fg_dim())),
        Span::styled(
            info.blocker_user.clone().unwrap_or_else(|| "-".into()),
            Style::default().fg(Theme::fg()),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  State:         ", Style::default().fg(Theme::fg_dim())),
        Span::styled(
            info.blocker_state.clone().unwrap_or_else(|| "-".into()),
            Style::default().fg(Theme::state_color(info.blocker_state.as_deref())),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Query:", Style::default().fg(Theme::fg_dim()))));
    lines.extend(highlight_sql(
        info.blocker_query.as_deref().unwrap_or("<no query>"),
        "  ",
    ));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_vacuum_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 60, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Vacuum Progress  [j/k] scroll  [y] copy table  [Esc] close ", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let sel = app.vacuum_table_state.selected().unwrap_or(0);
    let Some(vac) = snap.vacuum_progress.get(sel) else {
        frame.render_widget(
            Paragraph::new("No vacuum in progress").block(block),
            popup,
        );
        return;
    };

    let progress_color = if vac.progress_pct > 80.0 {
        Theme::border_ok()
    } else if vac.progress_pct > 50.0 {
        Theme::border_warn()
    } else {
        Theme::border_active()
    };

    // Create a simple progress bar
    let bar_width = 40;
    let filled = (vac.progress_pct / 100.0 * bar_width as f64) as usize;
    let empty = bar_width - filled;
    let progress_bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

    let phase_description = match vac.phase.as_str() {
        "initializing" => "Setting up vacuum operation",
        "scanning heap" => "Reading table pages to find dead tuples",
        "vacuuming indexes" => "Removing dead index entries",
        "vacuuming heap" => "Removing dead tuples from table",
        "cleaning up indexes" => "Finalizing index cleanup",
        "truncating heap" => "Shrinking table file if possible",
        "performing final cleanup" => "Finishing vacuum operation",
        _ => "",
    };

    let lines = vec![
        Line::from(""),
        section_header("Vacuum Target"),
        Line::from(vec![
            Span::styled("  Table:         ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &vac.table_name,
                Style::default().fg(Theme::border_active()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Database:      ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                vac.datname.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Theme::fg()),
            ),
            Span::styled("     PID: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(vac.pid.to_string(), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        section_header("Progress"),
        Line::from(vec![
            Span::styled("  Phase:         ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &vac.phase,
                Style::default().fg(Theme::border_warn()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("                 ", Style::default()),
            Span::styled(phase_description, Style::default().fg(Theme::fg_dim())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(progress_bar, Style::default().fg(progress_color)),
            Span::styled(
                format!(" {:.1}%", vac.progress_pct),
                Style::default().fg(progress_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Heap Blocks:   ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{} / {}", format_compact(vac.heap_blks_vacuumed), format_compact(vac.heap_blks_total)),
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Dead Tuples:   ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_compact(vac.num_dead_tuples),
                Style::default().fg(if vac.num_dead_tuples > 0 { Theme::border_warn() } else { Theme::fg() }),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}

pub fn render_wraparound_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 65, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" XID Details  [j/k] scroll  [y] copy db  [Esc] close ", Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let sel = app.wraparound_table_state.selected().unwrap_or(0);
    let Some(wrap) = snap.wraparound.get(sel) else {
        frame.render_widget(
            Paragraph::new("No wraparound data").block(block),
            popup,
        );
        return;
    };

    let pct_color = Theme::wraparound_color(wrap.pct_towards_wraparound);

    let urgency = if wrap.pct_towards_wraparound > 75.0 {
        ("CRITICAL", Theme::border_danger())
    } else if wrap.pct_towards_wraparound > 50.0 {
        ("WARNING", Theme::border_warn())
    } else {
        ("OK", Theme::border_ok())
    };

    // Progress bar for wraparound
    let bar_width = 40;
    let filled = (wrap.pct_towards_wraparound / 100.0 * bar_width as f64) as usize;
    let empty = bar_width - filled;
    let progress_bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

    let lines = vec![
        Line::from(""),
        section_header("Database"),
        Line::from(vec![
            Span::styled("  Name:          ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &wrap.datname,
                Style::default().fg(Theme::border_active()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("     Status: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!(" {} ", urgency.0),
                Style::default().fg(Theme::overlay_bg()).bg(urgency.1).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        section_header("Transaction ID Age"),
        Line::from(vec![
            Span::styled("  XID Age:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_compact(wrap.xid_age as i64),
                Style::default().fg(pct_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" transactions", Style::default().fg(Theme::fg_dim())),
        ]),
        Line::from(vec![
            Span::styled("  Remaining:     ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format_compact(wrap.xids_remaining),
                Style::default().fg(Theme::fg()),
            ),
            Span::styled(" until wraparound", Style::default().fg(Theme::fg_dim())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(progress_bar, Style::default().fg(pct_color)),
            Span::styled(
                format!(" {:.1}%", wrap.pct_towards_wraparound),
                Style::default().fg(pct_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        section_header("What This Means"),
        Line::from(Span::styled(
            "  PostgreSQL uses 32-bit transaction IDs that wrap around after",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(Span::styled(
            "  ~2 billion transactions. VACUUM must run to freeze old rows",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(Span::styled(
            "  before wraparound occurs, or the database will shut down.",
            Style::default().fg(Theme::fg_dim()),
        )),
        Line::from(""),
        if wrap.pct_towards_wraparound > 50.0 {
            Line::from(vec![
                Span::styled("  ⚠ ", Style::default().fg(Theme::border_warn())),
                Span::styled(
                    "Consider running VACUUM FREEZE on large tables",
                    Style::default().fg(Theme::border_warn()),
                ),
            ])
        } else {
            Line::from(Span::styled(
                "  ✓ Transaction ID age is healthy",
                Style::default().fg(Theme::border_ok()),
            ))
        },
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

    let block = overlay_block(" Statement Details  [j/k] scroll  [y] copy query  [Esc] close ", Theme::border_active());

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

    let hit_color = Theme::hit_ratio_color(stmt.hit_ratio);

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

    let block = overlay_block(" Configuration  [←→] change  [Esc] save & close ", Theme::border_active());

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
        section_header("Settings"),
    ];

    for (i, item) in ConfigItem::ALL.iter().enumerate() {
        let selected = i == app.config_selected;
        let indicator = if selected { "▸ " } else { "  " };

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
                .fg(Theme::overlay_bg())
                .bg(Theme::border_active())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::fg_dim())
        };

        let arrow_style = if selected {
            Style::default().fg(Theme::border_active())
        } else {
            Style::default().fg(Theme::border_dim())
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {}{:<20}", indicator, item.label()), label_style),
            Span::styled("◀ ", arrow_style),
            Span::styled(format!(" {} ", value_str), value_style),
            Span::styled(" ▶", arrow_style),
        ]));
    }

    // About section
    let label_style = Style::default().fg(Theme::fg_dim());
    let value_style = Style::default().fg(Theme::fg());
    let link_style = Style::default().fg(Theme::border_active());

    lines.push(Line::from(""));
    lines.push(section_header("About"));
    lines.push(Line::from(vec![
        Span::styled("    Version:    ", label_style),
        Span::styled(env!("CARGO_PKG_VERSION"), value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    License:    ", label_style),
        Span::styled("MIT", value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    Built with: ", label_style),
        Span::styled("Rust + ratatui + tokio-postgres", value_style),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    GitHub:     ", label_style),
        Span::styled("github.com/dlt/pg_glimpse", link_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    Issues:     ", label_style),
        Span::styled("github.com/dlt/pg_glimpse/issues", link_style),
    ]));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

pub fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 80, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Keybindings  [j/k] scroll  [Esc] close ", Theme::border_active());

    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Theme::fg());

    let entry = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("    {:<12}", key), key_style),
            Span::styled(desc.to_string(), desc_style),
        ])
    };

    let panel = app.bottom_panel;

    let mut lines = vec![
        Line::from(""),
        section_header("Navigation"),
        entry("q", "Quit application"),
        entry("Ctrl+C", "Force quit"),
        entry("p", "Pause / resume refresh"),
        entry("r", "Force refresh now"),
        entry("?", "This help screen"),
        entry(",", "Configuration"),
        Line::from(""),
        section_header("Panels"),
        entry("Tab", "Blocking chains"),
        entry("w", "Wait events"),
        entry("t", "Table stats"),
        entry("R", "Replication (lag, slots, subs)"),
        entry("v", "Vacuum progress"),
        entry("x", "Transaction wraparound"),
        entry("I", "Index stats"),
        entry("S", "pg_stat_statements"),
        entry("A", "WAL & I/O stats"),
        entry("P", "PostgreSQL settings"),
        Line::from(""),
        section_header("Panel Controls"),
        entry("Esc", "Back to queries (or quit)"),
        entry("↑ / k", "Select previous row"),
        entry("↓ / j", "Select next row"),
        entry("s", "Cycle sort column"),
    ];

    // Filter - only for panels that support it
    if panel.supports_filter() {
        lines.push(entry("/", "Fuzzy filter"));
    }

    lines.push(entry("Enter", "Inspect selected row"));

    // Bloat refresh - only for Tables and Indexes
    if matches!(panel, BottomPanel::TableStats | BottomPanel::Indexes) {
        lines.push(entry("b", "Refresh bloat estimates"));
    }

    // Query actions - only for Queries panel
    if panel == BottomPanel::Queries {
        lines.push(Line::from(""));
        lines.push(section_header("Query Actions"));
        lines.push(entry("C", "Cancel query (batch if filtered)"));
        lines.push(entry("K", "Terminate backend (batch if filtered)"));
        lines.push(entry("y", "Copy query to clipboard"));
    }

    lines.extend([
        Line::from(""),
        section_header("Overlay"),
        entry("Esc / q", "Close"),
        entry("j / k", "Scroll"),
        entry("g / G", "Top / bottom"),
    ]);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
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
