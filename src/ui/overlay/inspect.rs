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

    let idx = app.queries.selected().unwrap_or(0);
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

    let sel = app.indexes.selected().unwrap_or(0);
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

    let sel = app.table_stats.selected().unwrap_or(0);
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

    let sel = app.statements.selected().unwrap_or(0);
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

pub fn render_settings_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup_area);

    let block = overlay_block("Setting Details", Theme::border_active());

    let indices = app.sorted_settings_indices();
    let selected = app.settings_table_state.selected().unwrap_or(0);
    let Some(&idx) = indices.get(selected) else {
        frame.render_widget(block, popup_area);
        return;
    };

    let s = &app.server_info.settings[idx];

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

pub fn render_extensions_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 55, area);
    frame.render_widget(Clear, popup_area);

    let block = overlay_block(" Extension Details  [j/k] scroll  [y] copy name  [Esc] close ", Theme::border_active());

    let indices = app.sorted_extensions_indices();
    let selected = app.extensions_table_state.selected().unwrap_or(0);
    let Some(&idx) = indices.get(selected) else {
        frame.render_widget(
            Paragraph::new("No extension selected").block(block),
            popup_area,
        );
        return;
    };

    let ext = &app.server_info.extensions_list[idx];

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
