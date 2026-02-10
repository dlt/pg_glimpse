use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::{App, BottomPanel, TableStatSortColumn, ViewMode};
use crate::ui::theme::Theme;
use crate::ui::util::{compute_match_indices, empty_state, format_bytes, highlight_matches, styled_table};

use super::panel_block;

pub fn render_table_stats(frame: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.sorted_table_stat_indices();
    let total_count = app
        .snapshot
        .as_ref()
        .map_or(0, |s| s.table_stats.len());

    let title = format!("Table Stats [{total_count}]");
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
        if app.table_stats.sort_column == col {
            if app.table_stats.sort_ascending {
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
        Cell::from("Bloat[b]"),
        Cell::from("Last Vacuum"),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    // Check if filtering is active
    let is_filtering = app.bottom_panel == BottomPanel::TableStats
        && !app.filter.text.is_empty()
        && (app.filter.active || app.view_mode == ViewMode::Filter);
    let filter_text = &app.filter.text;

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

            let bloat_cell = t.bloat_pct.map_or_else(
                || Cell::from("-"),
                |pct| {
                    let color = Theme::bloat_color(pct);
                    Cell::from(format!("{pct:.1}%")).style(Style::default().fg(color))
                },
            );

            Row::new(vec![
                table_cell,
                Cell::from(format_bytes(t.total_size_bytes)),
                Cell::from(t.seq_scan.to_string()),
                Cell::from(t.idx_scan.to_string()),
                Cell::from(t.n_dead_tup.to_string()).style(Style::default().fg(dead_color)),
                Cell::from(format!("{:.1}%", t.dead_ratio))
                    .style(Style::default().fg(dead_color)),
                bloat_cell,
                Cell::from(
                    t.last_autovacuum.map_or_else(|| "never".into(), |ts| ts.format("%m-%d %H:%M").to_string()),
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
        Constraint::Length(8),
        Constraint::Length(13),
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.table_stats.state);
}
