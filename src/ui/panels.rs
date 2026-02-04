use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::{App, BottomPanel, IndexSortColumn, StatementSortColumn, ViewMode};
use super::theme::Theme;
use super::util::{
    collapse_whitespace, format_bytes, format_lag, format_number, format_time_ms, lag_color,
    truncate,
};

fn panel_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_style(Theme::border_active()))
}

pub fn render_blocking(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Blocking Chains");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
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
        frame.render_widget(msg, area);
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
                Cell::from("\u{2192}"),
                Cell::from(format!("{}", b.blocked_pid))
                    .style(Style::default().fg(Theme::border_warn())),
                Cell::from(format!("{:.1}s", b.blocked_duration_secs))
                    .style(Style::default().fg(Theme::duration_color(b.blocked_duration_secs))),
                Cell::from(b.blocker_query.clone().unwrap_or_else(|| "-".into())),
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
    frame.render_widget(table, area);
}

pub fn render_wait_events(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Wait Events");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
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
        frame.render_widget(msg, area);
        return;
    }

    let max_count = snap.wait_events.iter().map(|w| w.count).max().unwrap_or(1);
    let inner = block.inner(area);
    frame.render_widget(block, area);

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
            let bar: String = "\u{2588}".repeat(bar_len);
            let count_str = format!(" {}", w.count);

            Line::from(vec![
                Span::styled(label, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(
                    count_str,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

pub fn render_table_stats(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Table Stats");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
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
        frame.render_widget(msg, area);
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
                Cell::from(format!("{:.1}%", t.dead_ratio))
                    .style(Style::default().fg(dead_color)),
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
    frame.render_widget(table, area);
}

pub fn render_replication(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Replication Lag");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
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
        frame.render_widget(msg, area);
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
                Cell::from(r.application_name.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.client_addr.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.state.clone().unwrap_or_else(|| "-".into())),
                Cell::from(format_lag(r.write_lag_secs)),
                Cell::from(format_lag(r.flush_lag_secs)),
                Cell::from(format_lag(r.replay_lag_secs))
                    .style(Style::default().fg(lag_color(r.replay_lag_secs))),
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
    frame.render_widget(table, area);
}

pub fn render_vacuum_progress(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Vacuum Progress");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
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
        frame.render_widget(msg, area);
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
    frame.render_widget(table, area);
}

pub fn render_wraparound(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Transaction Wraparound");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
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
        frame.render_widget(msg, area);
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
    frame.render_widget(table, area);
}

pub fn render_indexes(frame: &mut Frame, app: &mut App, area: Rect) {
    let total_count = app
        .snapshot
        .as_ref()
        .map_or(0, |s| s.indexes.len());
    let indices = app.sorted_index_indices();
    let filtered_count = indices.len();

    let title = if app.filter_active
        || (!app.filter_text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Indexes)
    {
        format!(
            "Indexes [{}/{}] (filter: {})",
            filtered_count, total_count, app.filter_text
        )
    } else {
        format!("Indexes [{}]", total_count)
    };

    let block = panel_block(&title);

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
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
        frame.render_widget(msg, area);
        return;
    }

    let sort_indicator = |col: IndexSortColumn| -> &str {
        if app.index_sort_column == col {
            if app.index_sort_ascending {
                " \u{2191}"
            } else {
                " \u{2193}"
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
        Cell::from(format!(
            "Tup Read{}",
            sort_indicator(IndexSortColumn::TupRead)
        )),
        Cell::from(format!(
            "Tup Fetch{}",
            sort_indicator(IndexSortColumn::TupFetch)
        )),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

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
        .highlight_symbol("\u{25ba} ");

    frame.render_stateful_widget(table, area, &mut app.index_table_state);
}

pub fn render_statements(frame: &mut Frame, app: &mut App, area: Rect) {
    let total_count = app
        .snapshot
        .as_ref()
        .map_or(0, |s| s.stat_statements.len());
    let indices = app.sorted_stmt_indices();
    let filtered_count = indices.len();

    let title = if app.filter_active
        || (!app.filter_text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Statements)
    {
        format!(
            "pg_stat_statements [{}/{}] (filter: {})",
            filtered_count, total_count, app.filter_text
        )
    } else {
        format!("pg_stat_statements [{}]", total_count)
    };

    let block = panel_block(&title);

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if !snap.pg_stat_statements_available {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  pg_stat_statements extension is not available",
                Style::default()
                    .fg(Theme::border_warn())
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  To enable it:",
                Style::default().fg(Theme::fg()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  1. Add to postgresql.conf:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "     shared_preload_libraries = 'pg_stat_statements'",
                Style::default().fg(Theme::fg()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  2. Restart PostgreSQL",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  3. Create the extension:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "     CREATE EXTENSION pg_stat_statements;",
                Style::default().fg(Theme::fg()),
            )),
        ];
        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    if snap.stat_statements.is_empty() {
        let msg = Paragraph::new("\n  No statement data collected yet")
            .style(
                Style::default()
                    .fg(Theme::border_ok())
                    .add_modifier(Modifier::ITALIC),
            )
            .block(block);
        frame.render_widget(msg, area);
        return;
    }

    let sort_indicator = |col: StatementSortColumn| -> &str {
        if app.stmt_sort_column == col {
            if app.stmt_sort_ascending {
                " \u{2191}"
            } else {
                " \u{2193}"
            }
        } else {
            ""
        }
    };

    let header = Row::new(vec![
        Cell::from("Query"),
        Cell::from(format!(
            "Calls{}",
            sort_indicator(StatementSortColumn::Calls)
        )),
        Cell::from(format!(
            "Total{}",
            sort_indicator(StatementSortColumn::TotalTime)
        )),
        Cell::from(format!(
            "Mean{}",
            sort_indicator(StatementSortColumn::MeanTime)
        )),
        Cell::from(format!(
            "Max{}",
            sort_indicator(StatementSortColumn::MaxTime)
        )),
        Cell::from(format!(
            "Rows{}",
            sort_indicator(StatementSortColumn::Rows)
        )),
        Cell::from(format!(
            "Rows/Call{}",
            sort_indicator(StatementSortColumn::RowsPerCall)
        )),
        Cell::from(format!(
            "Buffers{}",
            sort_indicator(StatementSortColumn::Buffers)
        )),
        Cell::from("Hit %"),
        Cell::from("Stddev"),
        Cell::from("Temp Blks"),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    let rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let stmt = &snap.stat_statements[i];
            let query_display = collapse_whitespace(&stmt.query);
            let hit_color = if stmt.hit_ratio >= 0.99 {
                Theme::border_ok()
            } else if stmt.hit_ratio >= 0.90 {
                Theme::border_warn()
            } else {
                Theme::border_danger()
            };
            let temp_color = if stmt.temp_blks_written > 0 {
                Theme::border_warn()
            } else {
                Theme::fg()
            };
            let rows_per_call = if stmt.calls > 0 {
                format!("{:.1}", stmt.rows as f64 / stmt.calls as f64)
            } else {
                "-".into()
            };
            let total_bufs = stmt.shared_blks_hit + stmt.shared_blks_read;
            Row::new(vec![
                Cell::from(truncate(&query_display, 50).to_string()),
                Cell::from(stmt.calls.to_string()),
                Cell::from(format_time_ms(stmt.total_exec_time)),
                Cell::from(format_time_ms(stmt.mean_exec_time)),
                Cell::from(format_time_ms(stmt.max_exec_time)),
                Cell::from(stmt.rows.to_string()),
                Cell::from(rows_per_call),
                Cell::from(format_number(total_bufs)),
                Cell::from(format!("{:.1}%", stmt.hit_ratio * 100.0))
                    .style(Style::default().fg(hit_color)),
                Cell::from(format_time_ms(stmt.stddev_exec_time)),
                Cell::from(stmt.temp_blks_written.to_string())
                    .style(Style::default().fg(temp_color)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(9),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(9),
        Constraint::Length(10),
        Constraint::Length(9),
        Constraint::Length(7),
        Constraint::Length(11),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Theme::highlight_bg())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25ba} ");

    frame.render_stateful_widget(table, area, &mut app.stmt_table_state);
}
