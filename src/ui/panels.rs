use ratatui::layout::{Constraint, Layout, Direction, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::{App, BottomPanel, IndexSortColumn, StatementSortColumn, TableStatSortColumn, ViewMode};
use crate::db::models::{ArchiverStats, BgwriterStats, CheckpointStats, WalStats};
use super::overlay::highlight_sql_inline;
use super::theme::Theme;
use super::util::{
    compute_match_indices, empty_state, format_bytes, format_compact, format_lag, format_time_ms,
    highlight_matches, styled_table, truncate,
};

fn panel_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_style(Theme::border_active()))
}


pub fn render_blocking(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Blocking Chains");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.blocking_info.is_empty() {
        frame.render_widget(empty_state("No blocking detected", block), area);
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

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.blocking_table_state);
}

pub fn render_wait_events(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Wait Events");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.wait_events.is_empty() {
        frame.render_widget(empty_state("No active wait events", block), area);
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
                Span::styled(label, Style::default().fg(Theme::fg_dim())),
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

pub fn render_table_stats(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.sorted_table_stat_indices();
    let total_count = app
        .snapshot
        .as_ref()
        .map_or(0, |s| s.table_stats.len());

    let title = format!("Table Stats [{}]", total_count);
    let block = panel_block(&title);

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.table_stats.is_empty() {
        frame.render_widget(empty_state("No user tables found", block), area);
        return;
    }

    let sort_indicator = |col: TableStatSortColumn| -> &str {
        if app.table_stat_sort_column == col {
            if app.table_stat_sort_ascending {
                " \u{2191}"
            } else {
                " \u{2193}"
            }
        } else {
            ""
        }
    };

    let header = Row::new(vec![
        Cell::from(format!("Table{}", sort_indicator(TableStatSortColumn::Name))),
        Cell::from(format!("Size{}", sort_indicator(TableStatSortColumn::Size))),
        Cell::from(format!("SeqScan{}", sort_indicator(TableStatSortColumn::SeqScan))),
        Cell::from(format!("IdxScan{}", sort_indicator(TableStatSortColumn::IdxScan))),
        Cell::from(format!("Dead{}", sort_indicator(TableStatSortColumn::DeadTuples))),
        Cell::from(format!("Dead%{}", sort_indicator(TableStatSortColumn::DeadRatio))),
        Cell::from("Last Vacuum"),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    // Check if filtering is active
    let is_filtering = app.bottom_panel == BottomPanel::TableStats
        && !app.filter_text.is_empty()
        && (app.filter_active || app.view_mode == ViewMode::Filter);
    let filter_text = &app.filter_text;

    let rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let t = &snap.table_stats[i];
            let dead_color = Theme::dead_ratio_color(t.dead_ratio);
            let table_name = format!("{}.{}", t.schemaname, &t.relname);

            // Compute match indices if filtering
            let match_indices = if is_filtering {
                compute_match_indices(&table_name, filter_text)
            } else {
                None
            };

            let table_cell = if let Some(indices) = match_indices {
                let spans = highlight_matches(
                    &table_name,
                    &indices,
                    Style::default().fg(Theme::fg()),
                );
                Cell::from(Line::from(spans))
            } else {
                Cell::from(table_name)
            };

            Row::new(vec![
                table_cell,
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
        Constraint::Fill(1),
        Constraint::Length(9),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(9),
        Constraint::Length(13),
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.table_stat_table_state);
}

pub fn render_replication(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Replication Lag");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.replication.is_empty() {
        frame.render_widget(empty_state("No replicas connected", block), area);
        return;
    }

    let header = Row::new(vec![
        "PID", "User", "App Name", "Client", "State", "Sent LSN", "Replay LSN",
        "Write Lag", "Flush Lag", "Replay Lag", "Sync",
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    let rows: Vec<Row> = snap
        .replication
        .iter()
        .map(|r| {
            Row::new(vec![
                Cell::from(r.pid.to_string()),
                Cell::from(r.usename.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.application_name.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.client_addr.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.state.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.sent_lsn.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.replay_lsn.clone().unwrap_or_else(|| "-".into())),
                Cell::from(format_lag(r.write_lag_secs)),
                Cell::from(format_lag(r.flush_lag_secs)),
                Cell::from(format_lag(r.replay_lag_secs))
                    .style(Style::default().fg(Theme::lag_color(r.replay_lag_secs))),
                Cell::from(r.sync_state.clone().unwrap_or_else(|| "-".into())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(7),   // PID
        Constraint::Length(12),  // User
        Constraint::Length(14),  // App Name
        Constraint::Length(16),  // Client
        Constraint::Length(10),  // State
        Constraint::Length(12),  // Sent LSN
        Constraint::Length(12),  // Replay LSN
        Constraint::Length(10),  // Write Lag
        Constraint::Length(10),  // Flush Lag
        Constraint::Length(10),  // Replay Lag
        Constraint::Length(8),   // Sync
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.replication_table_state);
}

pub fn render_vacuum_progress(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Vacuum Progress");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.vacuum_progress.is_empty() {
        frame.render_widget(empty_state("No vacuums running", block), area);
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
                Cell::from(truncate(&v.table_name, 30)),
                Cell::from(truncate(&v.phase, 20)),
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

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.vacuum_table_state);
}

pub fn render_wraparound(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = panel_block("Transaction Wraparound");

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    if snap.wraparound.is_empty() {
        frame.render_widget(empty_state("No databases found", block), area);
        return;
    }

    let header = Row::new(vec!["Database", "XID Age", "Remaining", "% Used"])
        .style(Theme::title_style())
        .bottom_margin(0);

    let rows: Vec<Row> = snap
        .wraparound
        .iter()
        .map(|w| {
            let pct_color = Theme::wraparound_color(w.pct_towards_wraparound);
            Row::new(vec![
                Cell::from(w.datname.clone()),
                Cell::from(format_compact(w.xid_age as i64)),
                Cell::from(format_compact(w.xids_remaining)),
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

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.wraparound_table_state);
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
        frame.render_widget(empty_state("No user indexes found", block), area);
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

    // Check if filtering is active
    let is_filtering = app.filter_active
        || (!app.filter_text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Indexes);
    let filter_text = &app.filter_text;

    let rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let idx = &snap.indexes[i];
            let scan_color = Theme::index_usage_color(idx.idx_scan);
            let table_name = format!("{}.{}", idx.schemaname, idx.table_name);

            // Compute match indices if filtering - match against index name
            let match_indices = if is_filtering {
                compute_match_indices(&idx.index_name, filter_text)
            } else {
                None
            };

            let index_cell = if let Some(indices) = match_indices {
                let spans = highlight_matches(
                    &idx.index_name,
                    &indices,
                    Style::default().fg(Theme::fg()),
                );
                Cell::from(Line::from(spans))
            } else {
                Cell::from(idx.index_name.clone())
            };

            Row::new(vec![
                Cell::from(table_name),
                index_cell,
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

    let table = styled_table(rows, widths, header, block);
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

    if !snap.extensions.pg_stat_statements {
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
                Style::default().fg(Theme::fg_dim()),
            )),
            Line::from(Span::styled(
                "     shared_preload_libraries = 'pg_stat_statements'",
                Style::default().fg(Theme::fg()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  2. Restart PostgreSQL",
                Style::default().fg(Theme::fg_dim()),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  3. Create the extension:",
                Style::default().fg(Theme::fg_dim()),
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
        frame.render_widget(empty_state("No statement data collected yet", block), area);
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
            "Stddev{}",
            sort_indicator(StatementSortColumn::Stddev)
        )),
        Cell::from(format!(
            "Rows{}",
            sort_indicator(StatementSortColumn::Rows)
        )),
        Cell::from(format!(
            "Hit%{}",
            sort_indicator(StatementSortColumn::HitRatio)
        )),
        Cell::from(format!(
            "Reads{}",
            sort_indicator(StatementSortColumn::SharedReads)
        )),
        Cell::from(format!(
            "I/O{}",
            sort_indicator(StatementSortColumn::IoTime)
        )),
        Cell::from(format!(
            "Temp{}",
            sort_indicator(StatementSortColumn::Temp)
        )),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    // Calculate query column width: area width - borders - highlight symbol - fixed columns
    // Fixed columns: 7+9+9+9+8+7+5+7+9+7 = 77
    let query_width = (area.width as usize).saturating_sub(2 + 2 + 77).max(20);

    // Check if filtering is active
    let is_filtering = app.filter_active
        || (!app.filter_text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Statements);
    let filter_text = &app.filter_text;

    let rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let stmt = &snap.stat_statements[i];
            let hit_color = Theme::hit_ratio_color(stmt.hit_ratio);
            // Max time: orange if >2x mean (indicates spiky query)
            let max_color = if stmt.max_exec_time > stmt.mean_exec_time * 2.0 {
                Theme::border_warn()
            } else {
                Theme::fg()
            };
            let reads_color = if stmt.shared_blks_read > 1000 {
                Theme::border_warn()
            } else {
                Theme::fg()
            };
            let io_time = stmt.blk_read_time + stmt.blk_write_time;
            let io_color = if io_time > 1000.0 {
                Theme::border_warn()
            } else {
                Theme::fg()
            };
            let temp_total = stmt.temp_blks_read + stmt.temp_blks_written;
            let temp_color = if temp_total > 0 {
                Theme::border_warn()
            } else {
                Theme::fg()
            };

            // Compute match indices if filtering
            let match_indices = if is_filtering {
                compute_match_indices(&stmt.query, filter_text)
            } else {
                None
            };

            // For statements, filter string is just the query
            let query_cell = if let Some(indices) = match_indices {
                // Truncate query for display
                let display_text = if stmt.query.len() > query_width {
                    format!("{}…", &stmt.query[..query_width.saturating_sub(1)])
                } else {
                    stmt.query.clone()
                };

                let spans = highlight_matches(
                    &display_text,
                    &indices,
                    Style::default().fg(Theme::fg()),
                );
                Cell::from(Line::from(spans))
            } else {
                Cell::from(Line::from(highlight_sql_inline(&stmt.query, query_width)))
            };

            Row::new(vec![
                query_cell,
                Cell::from(format_compact(stmt.calls)),
                Cell::from(format_time_ms(stmt.total_exec_time)),
                Cell::from(format_time_ms(stmt.mean_exec_time)),
                Cell::from(format_time_ms(stmt.max_exec_time))
                    .style(Style::default().fg(max_color)),
                Cell::from(format_time_ms(stmt.stddev_exec_time)),
                Cell::from(format_compact(stmt.rows)),
                Cell::from(format!("{:.0}%", stmt.hit_ratio * 100.0))
                    .style(Style::default().fg(hit_color)),
                Cell::from(format_compact(stmt.shared_blks_read))
                    .style(Style::default().fg(reads_color)),
                Cell::from(format_time_ms(io_time))
                    .style(Style::default().fg(io_color)),
                Cell::from(format_compact(temp_total))
                    .style(Style::default().fg(temp_color)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Fill(1),
        Constraint::Length(7),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(5),
        Constraint::Length(7),
        Constraint::Length(9),
        Constraint::Length(7),
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.stmt_table_state);
}

pub fn render_wal_io(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("WAL & I/O");

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
    render_wal_column(frame, snap.wal_stats.as_ref(), columns[0]);

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

fn render_wal_column(frame: &mut Frame, wal: Option<&WalStats>, area: Rect) {
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
        lines.push(Line::from(vec![
            Span::styled("Records:      ", label_style),
            Span::styled(format_compact(w.wal_records), value_style),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Full Pages:   ", label_style),
            Span::styled(format_compact(w.wal_fpi), value_style),
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
                format!("{}s ago", lag_secs)
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
