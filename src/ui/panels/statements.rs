use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::{App, BottomPanel, StatementSortColumn, ViewMode};
use crate::ui::overlay::highlight_sql_inline;
use crate::ui::theme::Theme;
use crate::ui::util::{compute_match_indices, empty_state, format_compact, format_time_ms, highlight_matches, styled_table};

use super::panel_block;

pub fn render_statements(frame: &mut Frame, app: &mut App, area: Rect) {
    let total_count = app
        .snapshot
        .as_ref()
        .map_or(0, |s| s.stat_statements.len());
    let indices = app.sorted_stmt_indices();
    let filtered_count = indices.len();

    let title = if app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Statements)
    {
        format!(
            "pg_stat_statements [{}/{}] (filter: {})",
            filtered_count, total_count, app.filter.text
        )
    } else {
        format!("pg_stat_statements [{total_count}]")
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
        if let Some(ref err) = snap.stat_statements_error {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Error reading pg_stat_statements:",
                    Style::default()
                        .fg(Theme::border_danger())
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {err}"),
                    Style::default().fg(Theme::fg()),
                )),
            ];
            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, area);
        } else {
            frame.render_widget(empty_state("No statement data collected yet", block), area);
        }
        return;
    }

    let sort_indicator = |col: StatementSortColumn| -> &str {
        if app.statements.sort_column == col {
            if app.statements.sort_ascending {
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
    let is_filtering = app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Statements);
    let filter_text = &app.filter.text;

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
    frame.render_stateful_widget(table, area, &mut app.statements.state);
}
