use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Cell, Row};
use ratatui::Frame;

use crate::app::{App, BottomPanel, ViewMode};
use crate::ui::theme::Theme;
use crate::ui::util::{compute_match_indices, empty_state, highlight_matches, styled_table};

use super::panel_block;

pub fn render_schema_erd(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(ref snap) = app.snapshot else {
        frame.render_widget(empty_state("No data", panel_block("Schema ERD")), area);
        return;
    };

    let total_count = snap.table_schemas.len();
    let indices = app.sorted_schema_erd_indices();
    let filtered_count = indices.len();

    let emoji = if app.config.show_emojis { "📊 " } else { "" };
    let title = if app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::SchemaERD)
    {
        format!(
            "{emoji}Schema ERD [{}/{}] (filter: {})",
            filtered_count, total_count, app.filter.text
        )
    } else {
        format!("{emoji}Schema ERD [{total_count}]")
    };

    let block = panel_block(&title);

    if snap.table_schemas.is_empty() {
        frame.render_widget(empty_state("No tables found", block), area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Schema"),
        Cell::from("Table"),
        Cell::from("Columns"),
        Cell::from("PKs"),
        Cell::from("FKs Out"),
        Cell::from("FKs In"),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    // Check if filtering is active
    let is_filtering = app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::SchemaERD);
    let filter_text = &app.filter.text;

    let rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let table = &snap.table_schemas[i];

            let name_style = Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD);

            // Compute match indices if filtering
            let match_indices = if is_filtering {
                compute_match_indices(&table.table_name, filter_text)
            } else {
                None
            };

            let table_cell = match_indices.as_ref().map_or_else(
                || Cell::from(table.table_name.clone()).style(name_style),
                |indices| {
                    let spans = highlight_matches(&table.table_name, indices, name_style);
                    Cell::from(Line::from(spans))
                },
            );

            let fks_out_count = table.foreign_keys_out.len();
            let fks_in_count = table.foreign_keys_in.len();

            let fks_out_style = if fks_out_count > 0 {
                Style::default().fg(Theme::border_active())
            } else {
                Style::default().fg(Theme::fg_dim())
            };

            let fks_in_style = if fks_in_count > 0 {
                Style::default().fg(Theme::border_ok())
            } else {
                Style::default().fg(Theme::fg_dim())
            };

            Row::new(vec![
                Cell::from(table.schema_name.clone())
                    .style(Style::default().fg(Theme::fg_dim())),
                table_cell,
                Cell::from(table.columns.len().to_string()),
                Cell::from(table.primary_keys.len().to_string()),
                Cell::from(fks_out_count.to_string()).style(fks_out_style),
                Cell::from(fks_in_count.to_string()).style(fks_in_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(15), // Schema
        Constraint::Min(20),    // Table
        Constraint::Length(8),  // Columns
        Constraint::Length(5),  // PKs
        Constraint::Length(8),  // FKs Out
        Constraint::Length(8),  // FKs In
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.panels.schema_erd);
}
