use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Cell, Paragraph, Row};
use ratatui::Frame;

use crate::app::{App, BottomPanel, IndexSortColumn, ViewMode};
use crate::db::models::BloatSource;
use crate::ui::theme::Theme;
use crate::ui::util::{compute_match_indices, empty_state, format_bytes, highlight_matches, styled_table};

use super::panel_block;

pub fn render_indexes(frame: &mut Frame, app: &mut App, area: Rect) {
    let total_count = app
        .snapshot
        .as_ref()
        .map_or(0, |s| s.indexes.len());
    let indices = app.sorted_index_indices();
    let filtered_count = indices.len();

    let title = if app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Indexes)
    {
        format!(
            "Indexes [{}/{}] (filter: {})",
            filtered_count, total_count, app.filter.text
        )
    } else {
        format!("Indexes [{total_count}]")
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
        if app.panels.indexes.sort_column == col {
            if app.panels.indexes.sort_ascending {
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
        Cell::from("Bloat[b]"),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    // Check if filtering is active
    let is_filtering = app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Indexes);
    let filter_text = &app.filter.text;

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

            let index_cell = match_indices.map_or_else(
                || Cell::from(idx.index_name.clone()),
                |indices| {
                    let spans = highlight_matches(
                        &idx.index_name,
                        &indices,
                        Style::default().fg(Theme::fg()),
                    );
                    Cell::from(Line::from(spans))
                },
            );

            let bloat_cell = idx.bloat_pct.map_or_else(
                || Cell::from("-"),
                |pct| {
                    let color = Theme::bloat_color(pct);
                    // Show ~ prefix for estimated values (non-pgstattuple)
                    let prefix = match idx.bloat_source {
                        Some(BloatSource::Pgstattuple) => "",
                        _ => "~",
                    };
                    Cell::from(format!("{prefix}{pct:.1}%")).style(Style::default().fg(color))
                },
            );

            Row::new(vec![
                Cell::from(table_name),
                index_cell,
                Cell::from(format_bytes(idx.index_size_bytes)),
                Cell::from(idx.idx_scan.to_string())
                    .style(Style::default().fg(scan_color)),
                Cell::from(idx.idx_tup_read.to_string()),
                Cell::from(idx.idx_tup_fetch.to_string()),
                bloat_cell,
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
        Constraint::Length(8),
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.panels.indexes.state);
}
