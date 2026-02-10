use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, BottomPanel};
use crate::ui::theme::Theme;

use super::{centered_rect, overlay_block, section_header};

pub fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 80, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block(" Keybindings  [j/k] scroll  [Esc] close ", Theme::border_active());

    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Theme::fg());

    let entry = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("    {:<12}", key), key_style),
            Span::styled(desc.to_string(), desc_style),
        ])
    };

    let panel = app.bottom_panel;

    let mut lines = vec![
        Line::from(""),
        section_header("Navigation"),
        entry("q", "Back to queries / quit"),
        entry("Ctrl+C", "Force quit"),
        entry("p", "Pause / resume refresh"),
        entry("r", "Force refresh now"),
        entry("?", "This help screen"),
        entry(",", "Configuration"),
        Line::from(""),
        section_header("Panels"),
        entry("Q", "Queries (active)"),
        entry("Tab", "Blocking chains"),
        entry("w", "Wait events"),
        entry("t", "Table stats"),
        entry("R", "Replication (lag, slots, subs)"),
        entry("v", "Vacuum progress"),
        entry("x", "Transaction wraparound"),
        entry("I", "Index stats"),
        entry("S", "pg_stat_statements"),
        entry("A", "WAL & I/O stats"),
        entry("P", "PostgreSQL settings"),
        entry("E", "Extensions"),
        Line::from(""),
        section_header("Panel Controls"),
        entry("Esc", "Back to queries (or quit)"),
        entry("↑ / k", "Select previous row"),
        entry("↓ / j", "Select next row"),
        entry("s", "Cycle sort column"),
    ];

    // Filter - only for panels that support it
    if panel.supports_filter() {
        lines.push(entry("/", "Fuzzy filter"));
    }

    lines.push(entry("Enter", "Inspect selected row"));

    // Bloat refresh - only for Tables and Indexes
    if matches!(panel, BottomPanel::TableStats | BottomPanel::Indexes) {
        lines.push(entry("b", "Refresh bloat estimates"));
    }

    // Query actions - only for Queries panel
    if panel == BottomPanel::Queries {
        lines.push(Line::from(""));
        lines.push(section_header("Query Actions"));
        lines.push(entry("C", "Cancel query (batch if filtered)"));
        lines.push(entry("K", "Terminate backend (batch if filtered)"));
        lines.push(entry("y", "Copy query to clipboard"));
    }

    lines.extend([
        Line::from(""),
        section_header("Overlay"),
        entry("Esc / q", "Close"),
        entry("j / k", "Scroll"),
        entry("g / G", "Top / bottom"),
    ]);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}
