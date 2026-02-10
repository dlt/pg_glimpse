use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Cell, Row};
use ratatui::Frame;

use crate::app::{App, BottomPanel, ViewMode};
use crate::ui::theme::Theme;
use crate::ui::util::{compute_match_indices, empty_state, highlight_matches, styled_table};

use super::panel_block;

pub fn render_settings(frame: &mut Frame, app: &mut App, area: Rect) {
    let total_count = app.server_info.settings.len();
    let indices = app.sorted_settings_indices();
    let filtered_count = indices.len();

    let title = if app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Settings)
    {
        format!(
            "PostgreSQL Settings [{}/{}] (filter: {})",
            filtered_count, total_count, app.filter.text
        )
    } else {
        format!("PostgreSQL Settings [{}]", total_count)
    };

    let block = panel_block(&title);

    if app.server_info.settings.is_empty() {
        frame.render_widget(empty_state("No settings loaded", block), area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Value"),
        Cell::from("Unit"),
        Cell::from("Source"),
        Cell::from("Context"),
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    // Check if filtering is active
    let is_filtering = app.filter.active
        || (!app.filter.text.is_empty()
            && app.view_mode == ViewMode::Filter
            && app.bottom_panel == BottomPanel::Settings);
    let filter_text = &app.filter.text;

    let rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let s = &app.server_info.settings[i];

            // Highlight non-default values
            let is_non_default = s.source != "default";
            let needs_restart = s.context == "postmaster" && is_non_default;
            let pending = s.pending_restart;

            let name_style = if is_non_default {
                Style::default().fg(Theme::border_active()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::fg())
            };

            // Compute match indices if filtering
            let match_indices = if is_filtering {
                compute_match_indices(&s.name, filter_text)
            } else {
                None
            };

            let name_cell = if let Some(ref indices) = match_indices {
                let spans = highlight_matches(
                    &s.name,
                    indices,
                    name_style,
                );
                Cell::from(Line::from(spans))
            } else {
                Cell::from(s.name.clone()).style(name_style)
            };

            // Truncate value for display
            let value_display = if s.setting.len() > 40 {
                format!("{}…", &s.setting[..39])
            } else {
                s.setting.clone()
            };

            let value_style = if is_non_default {
                Style::default().fg(Theme::border_warn())
            } else {
                Style::default().fg(Theme::fg())
            };

            let unit_display = s.unit.clone().unwrap_or_else(|| "-".into());

            // Source display - highlight if not default
            let source_style = if is_non_default {
                Style::default().fg(Theme::border_active())
            } else {
                Style::default().fg(Theme::fg_dim())
            };

            // Context with restart indicator
            let context_display = if pending {
                format!("{} [restart!]", s.context)
            } else if needs_restart {
                format!("{} [restart]", s.context)
            } else {
                s.context.clone()
            };

            let context_style = if pending {
                Style::default().fg(Theme::border_danger())
            } else if needs_restart {
                Style::default().fg(Theme::border_warn())
            } else {
                Style::default().fg(Theme::fg_dim())
            };

            Row::new(vec![
                name_cell,
                Cell::from(value_display).style(value_style),
                Cell::from(unit_display),
                Cell::from(s.source.clone()).style(source_style),
                Cell::from(context_display).style(context_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(30),      // Name
        Constraint::Min(25),      // Value
        Constraint::Length(8),    // Unit
        Constraint::Length(18),   // Source
        Constraint::Length(18),   // Context
    ];

    let table = styled_table(rows, widths, header, block);
    frame.render_stateful_widget(table, area, &mut app.settings_table_state);
}
