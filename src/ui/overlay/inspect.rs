use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;
use crate::ui::util::{format_bytes, format_compact, format_duration, format_lag, format_time_ms};

use super::sql_highlight::highlight_sql;
use super::{centered_rect, overlay_block, section_header};

pub fn render_inspect(frame: &mut Frame, app: &App, area: Rect, pid: i32) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);

    let emoji = if app.config.show_emojis { "🔍 " } else { "" };
    let title = format!("{emoji}Query Details  [j/k] scroll  [y] copy query  [C] cancel  [K] kill  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(
            Paragraph::new("No data").block(block),
            popup,
        );
        return;
    };

    let Some(q) = snap.active_queries.iter().find(|q| q.pid == pid) else {
        frame.render_widget(
            Paragraph::new("Query no longer exists").block(block),
            popup,
        );
        return;
    };

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

pub fn render_index_inspect(frame: &mut Frame, app: &App, area: Rect, key: &str) {
    let popup = centered_rect(75, 60, area);
    frame.render_widget(Clear, popup);

    let emoji = if app.config.show_emojis { "📑 " } else { "" };
    let title = format!("{emoji}Index Details  [j/k] scroll  [y] copy definition  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let Some(idx) = snap.indexes.iter().find(|i| {
        format!("{}.{}", i.schemaname, i.index_name) == key
    }) else {
        frame.render_widget(
            Paragraph::new("Index no longer exists").block(block),
            popup,
        );
        return;
    };

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
            Span::styled(format!("{:<10}", idx.idx_tup_read), Style::default().fg(Theme::fg())),
            Span::styled("Tup Fetch: ", Style::default().fg(Theme::fg_dim())),
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

pub fn render_replication_inspect(frame: &mut Frame, app: &App, area: Rect, pid: i32) {
    let popup = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup);

    let emoji = if app.config.show_emojis { "🔄 " } else { "" };
    let title = format!("{emoji}Replication Details  [j/k] scroll  [y] copy app  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let Some(r) = snap.replication.iter().find(|r| r.pid == pid) else {
        frame.render_widget(
            Paragraph::new("Replication slot no longer exists").block(block),
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
            Span::styled(
                format!("{:<10}", r.usename.as_deref().unwrap_or("-")),
                Style::default().fg(Theme::fg()),
            ),
            label("User SysID:    "),
            val(r.usesysid.map_or_else(|| "-".into(), |id| id.to_string())),
        ]),
        Line::from(vec![
            label("  Application:     "),
            val_opt(&r.application_name),
        ]),
        Line::from(vec![
            label("  Client Addr:     "),
            Span::styled(
                format!("{:<10}", r.client_addr.as_deref().unwrap_or("-")),
                Style::default().fg(Theme::fg()),
            ),
            label("Port:          "),
            val(r.client_port.map_or_else(|| "-".into(), |p| p.to_string())),
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
            Span::styled(
                format!("{:<10}", r.sync_state.as_deref().unwrap_or("-")),
                Style::default().fg(Theme::fg()),
            ),
            label("Sync Priority: "),
            val(r.sync_priority.map_or_else(|| "-".into(), |p| p.to_string())),
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

pub fn render_table_inspect(frame: &mut Frame, app: &App, area: Rect, key: &str) {
    let popup = centered_rect(75, 75, area);
    frame.render_widget(Clear, popup);

    let emoji = if app.config.show_emojis { "📋 " } else { "" };
    let title = format!("{emoji}Table Details  [j/k] scroll  [y] copy name  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let Some(tbl) = snap.table_stats.iter().find(|t| {
        format!("{}.{}", t.schemaname, t.relname) == key
    }) else {
        frame.render_widget(
            Paragraph::new("Table no longer exists").block(block),
            popup,
        );
        return;
    };

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
            Span::styled(format!("{:<10}", format_bytes(tbl.table_size_bytes)), Style::default().fg(Theme::fg())),
            Span::styled("Indexes: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_bytes(tbl.indexes_size_bytes), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        section_header("Row Stats"),
        Line::from(vec![
            Span::styled("  Live:          ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                format!("{:<10}", format_compact(tbl.n_live_tup)),
                Style::default().fg(Theme::fg()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Dead: ", Style::default().fg(Theme::fg_dim())),
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
                format!("{:<10}", format_compact(tbl.seq_scan)),
                Style::default().fg(seq_scan_color),
            ),
            Span::styled("Rows Read: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_compact(tbl.seq_tup_read), Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Idx Scans:     ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format!("{:<10}", format_compact(tbl.idx_scan)), Style::default().fg(Theme::fg())),
            Span::styled("Rows Fetch: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format_compact(tbl.idx_tup_fetch), Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        section_header("DML Activity"),
        Line::from(vec![
            Span::styled("  Inserts:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format!("{:<10}", format_compact(tbl.n_tup_ins)), Style::default().fg(Theme::fg())),
            Span::styled("Updates: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(format!("{:<10}", format_compact(tbl.n_tup_upd)), Style::default().fg(Theme::fg())),
            Span::styled("Deletes: ", Style::default().fg(Theme::fg_dim())),
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
            Span::styled(format!("{:<10}", tbl.vacuum_count), Style::default().fg(Theme::fg())),
            Span::styled("AutoVac: ", Style::default().fg(Theme::fg_dim())),
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

pub fn render_blocking_inspect(frame: &mut Frame, app: &App, area: Rect, blocked_pid: i32) {
    let popup = centered_rect(80, 70, area);
    frame.render_widget(Clear, popup);

    let emoji = if app.config.show_emojis { "🔒 " } else { "" };
    let title = format!("{emoji}Lock Details  [j/k] scroll  [y] copy query  [Esc] close");
    let block = overlay_block(&title, Theme::border_danger());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let Some(info) = snap.blocking_info.iter().find(|b| b.blocked_pid == blocked_pid) else {
        frame.render_widget(
            Paragraph::new("Lock info no longer exists").block(block),
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

pub fn render_vacuum_inspect(frame: &mut Frame, app: &App, area: Rect, pid: i32) {
    let popup = centered_rect(70, 60, area);
    frame.render_widget(Clear, popup);

    let emoji = if app.config.show_emojis { "🧹 " } else { "" };
    let title = format!("{emoji}Vacuum Progress  [j/k] scroll  [y] copy table  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let Some(vac) = snap.vacuum_progress.iter().find(|v| v.pid == pid) else {
        frame.render_widget(
            Paragraph::new("Vacuum no longer in progress").block(block),
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
                format!("{:<10}", vac.datname.as_deref().unwrap_or("-")),
                Style::default().fg(Theme::fg()),
            ),
            Span::styled("PID: ", Style::default().fg(Theme::fg_dim())),
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

pub fn render_wraparound_inspect(frame: &mut Frame, app: &App, area: Rect, datname: &str) {
    let popup = centered_rect(70, 65, area);
    frame.render_widget(Clear, popup);

    let emoji = if app.config.show_emojis { "⚠️ " } else { "" };
    let title = format!("{emoji}XID Details  [j/k] scroll  [y] copy db  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let Some(wrap) = snap.wraparound.iter().find(|w| w.datname == datname) else {
        frame.render_widget(
            Paragraph::new("Database no longer in wraparound list").block(block),
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
                format_compact(i64::from(wrap.xid_age)),
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

pub fn render_statement_inspect(frame: &mut Frame, app: &App, area: Rect, queryid: i64) {
    let popup = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup);

    let emoji = if app.config.show_emojis { "📝 " } else { "" };
    let title = format!("{emoji}Statement Details  [j/k] scroll  [y] copy query  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup);
        return;
    };

    let Some(stmt) = snap.stat_statements.iter().find(|s| s.queryid == queryid) else {
        frame.render_widget(
            Paragraph::new("Statement no longer exists").block(block),
            popup,
        );
        return;
    };

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
            val_bold(format!("{:<10}", stmt.calls)),
            label("Rows:          "),
            val(format!("{:<10}", stmt.rows)),
            label("Rows/Call:     "),
            val(rows_per_call),
        ]),
        Line::from(vec![
            label("  Total Time:      "),
            val_bold(format_time_ms(stmt.total_exec_time)),
        ]),
        Line::from(vec![
            label("  Mean Time:       "),
            val(format!("{:<10}", format_time_ms(stmt.mean_exec_time))),
            label("Min Time:      "),
            val(format_time_ms(stmt.min_exec_time)),
        ]),
        Line::from(vec![
            label("  Max Time:        "),
            val(format!("{:<10}", format_time_ms(stmt.max_exec_time))),
            label("Stddev:        "),
            val(format_time_ms(stmt.stddev_exec_time)),
        ]),
        Line::from(""),
        section("  Shared Buffers"),
        Line::from(vec![
            label("  Hit:             "),
            val(format!("{:<10}", stmt.shared_blks_hit)),
            label("Read:          "),
            val(stmt.shared_blks_read.to_string()),
        ]),
        Line::from(vec![
            label("  Dirtied:         "),
            val(format!("{:<10}", stmt.shared_blks_dirtied)),
            label("Written:       "),
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
            val(format!("{:<10}", stmt.local_blks_hit)),
            label("Read:          "),
            val(stmt.local_blks_read.to_string()),
        ]),
        Line::from(vec![
            label("  Dirtied:         "),
            val(format!("{:<10}", stmt.local_blks_dirtied)),
            label("Written:       "),
            val(stmt.local_blks_written.to_string()),
        ]),
        Line::from(""),
        section("  Temp & I/O"),
        Line::from(vec![
            label("  Temp Read:       "),
            Span::styled(
                format!("{:<10}", stmt.temp_blks_read),
                Style::default().fg(temp_color),
            ),
            label("Temp Written:  "),
            Span::styled(
                stmt.temp_blks_written.to_string(),
                Style::default().fg(temp_color),
            ),
        ]),
        Line::from(vec![
            label("  Blk Read Time:   "),
            Span::styled(
                format!("{:<10}", format_time_ms(stmt.blk_read_time)),
                Style::default().fg(io_time_color),
            ),
            label("Blk Write Time:"),
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

pub fn render_settings_inspect(frame: &mut Frame, app: &App, area: Rect, name: &str) {
    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup_area);

    let emoji = if app.config.show_emojis { "⚙️ " } else { "" };
    let title = format!("{emoji}Setting Details");
    let block = overlay_block(&title, Theme::border_active());

    let Some(s) = app.server_info.settings.iter().find(|s| s.name == name) else {
        frame.render_widget(
            Paragraph::new("Setting not found").block(block),
            popup_area,
        );
        return;
    };

    let mut lines = vec![
        // Setting section
        Line::from(""),
        section_header("Setting"),
        Line::from(vec![
            Span::styled("  Name:        ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&s.name, Style::default().fg(Theme::fg()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Value:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&s.setting, Style::default().fg(Theme::border_active())),
        ]),
    ];
    if let Some(unit) = &s.unit {
        lines.push(Line::from(vec![
            Span::styled("  Unit:        ", Style::default().fg(Theme::fg_dim())),
            Span::styled(unit, Style::default().fg(Theme::fg())),
        ]));
    }
    lines.push(Line::from(""));

    // Category section
    lines.push(section_header("Category"));
    lines.push(Line::from(vec![
        Span::styled("  Category:    ", Style::default().fg(Theme::fg_dim())),
        Span::styled(&s.category, Style::default().fg(Theme::fg())),
    ]));
    lines.push(Line::from(""));

    // Description section
    lines.push(section_header("Description"));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(&s.short_desc, Style::default().fg(Theme::fg())),
    ]));
    lines.push(Line::from(""));

    // Configuration section
    lines.push(section_header("Configuration"));
    lines.push(Line::from(vec![
        Span::styled("  Context:     ", Style::default().fg(Theme::fg_dim())),
        Span::styled(&s.context, Style::default().fg(settings_context_color(&s.context))),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Source:      ", Style::default().fg(Theme::fg_dim())),
        Span::styled(&s.source, Style::default().fg(Theme::fg())),
    ]));

    // Add "To Apply" description based on context
    let (apply_action, apply_color) = settings_context_description(&s.context);
    lines.push(Line::from(vec![
        Span::styled("  To Apply:    ", Style::default().fg(Theme::fg_dim())),
        Span::styled(apply_action, Style::default().fg(apply_color)),
    ]));

    if s.pending_restart {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ⚠ ", Style::default().fg(Theme::border_danger())),
            Span::styled(
                "Pending restart - value changed but not yet active",
                Style::default().fg(Theme::border_danger()),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));

    frame.render_widget(paragraph, popup_area);
}

fn settings_context_color(context: &str) -> Color {
    match context {
        "postmaster" => Theme::border_danger(),  // Requires restart
        "sighup" => Theme::border_warn(),        // Requires reload
        "superuser" | "user" => Theme::border_active(), // Can change at runtime
        _ => Theme::fg(),
    }
}

fn settings_context_description(context: &str) -> (&'static str, Color) {
    match context {
        "postmaster" => ("Server restart required", Theme::border_danger()),
        "sighup" => ("Config reload (pg_reload_conf())", Theme::border_warn()),
        "superuser" => ("SET command (superuser only)", Theme::border_active()),
        "user" => ("SET command (any user)", Theme::border_ok()),
        "superuser-backend" => ("Connection start (superuser)", Theme::fg()),
        "backend" => ("Connection start only", Theme::fg()),
        "internal" => ("Cannot be changed", Theme::fg_dim()),
        _ => ("Unknown", Theme::fg()),
    }
}

pub fn render_extensions_inspect(frame: &mut Frame, app: &App, area: Rect, name: &str) {
    let popup_area = centered_rect(60, 55, area);
    frame.render_widget(Clear, popup_area);

    let emoji = if app.config.show_emojis { "🧩 " } else { "" };
    let title = format!("{emoji}Extension Details  [j/k] scroll  [y] copy name  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(ext) = app.server_info.extensions_list.iter().find(|e| e.name == name) else {
        frame.render_widget(
            Paragraph::new("Extension not found").block(block),
            popup_area,
        );
        return;
    };

    let relocatable_color = if ext.relocatable {
        Theme::border_ok()
    } else {
        Theme::fg_dim()
    };

    let mut lines = vec![
        Line::from(""),
        section_header("Extension"),
        Line::from(vec![
            Span::styled("  Name:        ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &ext.name,
                Style::default()
                    .fg(Theme::border_active())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Version:     ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&ext.version, Style::default().fg(Theme::fg())),
        ]),
        Line::from(""),
        section_header("Location"),
        Line::from(vec![
            Span::styled("  Schema:      ", Style::default().fg(Theme::fg_dim())),
            Span::styled(&ext.schema, Style::default().fg(Theme::fg())),
        ]),
        Line::from(vec![
            Span::styled("  Relocatable: ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                if ext.relocatable { "Yes" } else { "No" },
                Style::default().fg(relocatable_color),
            ),
        ]),
        Line::from(""),
        section_header("Description"),
    ];

    // Add description lines, wrapping if needed
    let description = ext
        .description
        .as_deref()
        .unwrap_or("No description available");

    // Simple word wrapping for description
    let max_width = 50;
    let words: Vec<&str> = description.split_whitespace().collect();
    let mut current_line = String::from("  ");

    for word in words {
        if current_line.len() + word.len() + 1 > max_width + 2 {
            lines.push(Line::from(Span::styled(
                current_line.clone(),
                Style::default().fg(Theme::fg()),
            )));
            current_line = format!("  {word}");
        } else {
            if current_line.len() > 2 {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }
    if current_line.len() > 2 {
        lines.push(Line::from(Span::styled(
            current_line,
            Style::default().fg(Theme::fg()),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));

    frame.render_widget(paragraph, popup_area);
}

pub fn render_schema_erd_inspect(frame: &mut Frame, app: &App, area: Rect, key: &str) {
    let popup_area = centered_rect(70, 70, area);
    frame.render_widget(Clear, popup_area);

    let emoji = if app.config.show_emojis { "📊 " } else { "" };
    let title = format!("{emoji}Table Schema  [j/k] scroll  [y] copy  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let Some(ref snap) = app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), popup_area);
        return;
    };

    // Parse schema.table from key
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() != 2 {
        frame.render_widget(
            Paragraph::new("Invalid table key").block(block),
            popup_area,
        );
        return;
    }
    let (schema, table) = (parts[0], parts[1]);

    let Some(table_schema) = snap.table_schemas.iter().find(|t| {
        t.schema_name == schema && t.table_name == table
    }) else {
        frame.render_widget(
            Paragraph::new("Table not found").block(block),
            popup_area,
        );
        return;
    };

    let mut lines = vec![
        Line::from(""),
        section_header("Table"),
        Line::from(vec![
            Span::styled("  Schema:      ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &table_schema.schema_name,
                Style::default().fg(Theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Table:       ", Style::default().fg(Theme::fg_dim())),
            Span::styled(
                &table_schema.table_name,
                Style::default()
                    .fg(Theme::border_active())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    // Columns section
    if !table_schema.columns.is_empty() {
        lines.push(section_header(&format!("Columns ({})", table_schema.columns.len())));
        for col in &table_schema.columns {
            let mut badges = Vec::new();
            if col.is_primary_key {
                badges.push(Span::styled(" PK", Style::default().fg(Theme::border_ok())));
            }
            if col.is_foreign_key {
                badges.push(Span::styled(" FK", Style::default().fg(Theme::border_warn())));
            }
            if !col.is_nullable {
                badges.push(Span::styled(" NOT NULL", Style::default().fg(Theme::fg_dim())));
            }

            let mut spans = vec![
                Span::styled("  • ", Style::default().fg(Theme::border_active())),
                Span::styled(&col.column_name, Style::default().fg(Theme::fg())),
                Span::styled(
                    format!(" {}", col.data_type),
                    Style::default().fg(Theme::fg_dim()),
                ),
            ];
            spans.extend(badges);
            lines.push(Line::from(spans));
        }
        lines.push(Line::from(""));
    }

    // Primary keys section
    if !table_schema.primary_keys.is_empty() {
        lines.push(section_header(&format!(
            "Primary Keys ({})",
            table_schema.primary_keys.len()
        )));
        for pk in &table_schema.primary_keys {
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(Theme::border_ok())),
                Span::styled(pk, Style::default().fg(Theme::fg())),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Foreign keys OUT
    if !table_schema.foreign_keys_out.is_empty() {
        lines.push(section_header(&format!(
            "References ({})",
            table_schema.foreign_keys_out.len()
        )));
        for fk in &table_schema.foreign_keys_out {
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(Theme::border_warn())),
                Span::styled(&fk.column_name, Style::default().fg(Theme::fg())),
                Span::styled(" → ", Style::default().fg(Theme::fg_dim())),
                Span::styled(
                    format!("{}.{}", fk.foreign_table_schema, fk.foreign_table_name),
                    Style::default().fg(Theme::border_active()),
                ),
                Span::styled(
                    format!("({})", fk.foreign_column_name),
                    Style::default().fg(Theme::fg_dim())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("    ON DELETE ", Style::default().fg(Theme::fg_dim())),
                Span::styled(&fk.delete_rule, Style::default().fg(Theme::fg())),
                Span::styled(", ON UPDATE ", Style::default().fg(Theme::fg_dim())),
                Span::styled(&fk.update_rule, Style::default().fg(Theme::fg())),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Foreign keys IN
    if !table_schema.foreign_keys_in.is_empty() {
        lines.push(section_header(&format!(
            "Referenced By ({})",
            table_schema.foreign_keys_in.len()
        )));
        for fk in &table_schema.foreign_keys_in {
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(Theme::border_ok())),
                Span::styled(
                    format!("{}.{}", fk.table_schema, fk.table_name),
                    Style::default().fg(Theme::border_active()),
                ),
                Span::styled(
                    format!("({})", fk.column_name),
                    Style::default().fg(Theme::fg_dim())),
                Span::styled(" → ", Style::default().fg(Theme::fg_dim())),
                Span::styled(&fk.foreign_column_name, Style::default().fg(Theme::fg())),
            ]));
        }
        lines.push(Line::from(""));
    }

    if table_schema.primary_keys.is_empty()
        && table_schema.foreign_keys_out.is_empty()
        && table_schema.foreign_keys_in.is_empty()
    {
        lines.push(Line::from(vec![Span::styled(
            "  No primary keys or foreign key relationships",
            Style::default().fg(Theme::fg_dim()),
        )]));
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.overlay_scroll, 0));

    frame.render_widget(paragraph, popup_area);
}
