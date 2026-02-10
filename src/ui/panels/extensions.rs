use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Cell, Row};
use ratatui::Frame;

use crate::app::{App, BottomPanel, ViewMode};
use crate::ui::theme::Theme;
use crate::ui::util::{compute_match_indices, empty_state, highlight_matches, styled_table};

use super::panel_block;

pub fn render_extensions(frame: &mut Frame, app: &mut App, area: Rect) {
    let total_count = app.server_info.extensions_list.len();
    let indices = app.sorted_extensions_indices();
    let filtered_count = indices.len();

    let title = if app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Extensions)
    {
        format!(
            "Extensions [{}/{}] (filter: {})",
            filtered_count, total_count, app.filter.text
        )
    } else {
        format!("Extensions [{}]", total_count)
    };

    let block = panel_block(&title);

    if app.server_info.extensions_list.is_empty() {
        frame.render_widget(empty_state("No extensions installed", block), area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Version"),
        Cell::from("Schema"),
        Cell::from("Relocatable"),
        Cell::from("Description"),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    // Check if filtering is active
    let is_filtering = app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Extensions);
    let filter_text = &app.filter.text;

    let rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let ext = &app.server_info.extensions_list[i];

            let name_style = Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD);

            // Compute match indices if filtering
            let match_indices = if is_filtering {
                compute_match_indices(&ext.name, filter_text)
            } else {
                None
            };

            let name_cell = if let Some(ref indices) = match_indices {
                let spans = highlight_matches(&ext.name, indices, name_style);
                Cell::from(Line::from(spans))
            } else {
                Cell::from(ext.name.clone()).style(name_style)
            };

            let relocatable_display = if ext.relocatable { "Yes" } else { "No" };
            let relocatable_style = if ext.relocatable {
                Style::default().fg(Theme::border_ok())
            } else {
                Style::default().fg(Theme::fg_dim())
            };

            // Truncate description for display
            let desc_display = ext
                .description
                .as_ref()
                .map(|d| {
                    if d.len() > 50 {
                        format!("{}...", &d[..47])
                    } else {
                        d.clone()
                    }
                })
                .unwrap_or_else(|| "-".into());

            Row::new(vec![
                name_cell,
                Cell::from(ext.version.clone()),
                Cell::from(ext.schema.clone()),
                Cell::from(relocatable_display).style(relocatable_style),
                Cell::from(desc_display).style(Style::default().fg(Theme::fg_dim())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(20),      // Name
        Constraint::Length(10),   // Version
        Constraint::Length(12),   // Schema
        Constraint::Length(12),   // Relocatable
        Constraint::Min(30),      // Description
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.extensions_table_state);
}
