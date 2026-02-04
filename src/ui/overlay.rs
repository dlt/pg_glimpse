use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::config::ConfigItem;
use super::theme::Theme;
use super::util::{format_bytes, format_time_ms};

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

    let block = overlay_block("Query Details  [y] copy  [Esc] close", Theme::border_active());

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

pub fn render_index_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(75, 55, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Index Details  [y] copy  [Esc] back", Theme::border_active());

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

pub fn render_statement_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Statement Details  [y] copy  [Esc] back", Theme::border_active());

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

    let label = |s: &'static str| Span::styled(s, Style::default().fg(Color::DarkGray));
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

    let lines = vec![
        Line::from(vec![
            label("  Query ID:        "),
            val(stmt.queryid.to_string()),
        ]),
        Line::from(""),
        section("  Query"),
        Line::from(Span::styled(
            format!("  {}", stmt.query),
            Style::default().fg(Theme::fg()),
        )),
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
        "Config  [\u{2190}\u{2192}] change  [Esc] save & close",
        Theme::border_active(),
    );

    let mut lines = vec![Line::from("")];

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
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}{:<20}", indicator, item.label()),
                label_style,
            ),
            Span::styled(format!("\u{25c4} {} \u{25ba}", value_str), value_style),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}

pub fn render_help(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(70, 80, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("Keybindings  [Esc] close", Theme::border_active());

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
    ];

    let paragraph = Paragraph::new(lines).block(block);
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
