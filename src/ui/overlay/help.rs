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

    let emoji = if app.config.show_emojis { "❓ " } else { "" };
    let title = format!("{emoji}Keybindings  [j/k] scroll  [Esc] close");
    let block = overlay_block(&title, Theme::border_active());

    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Theme::fg());

    let entry = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("    {key:<12}"), key_style),
            Span::styled(desc.to_string(), desc_style),
        ])
    };

    let panel = app.bottom_panel;

    let mut lines = vec![
        Line::from(""),
        section_header("Navigation"),
        entry("q", "Back to queries / quit"),
        entry("Ctrl+C", "Force quit"),
    ];

    // Live mode only keys
    if !app.is_replay_mode() {
        lines.push(entry("p", "Pause / resume refresh"));
        lines.push(entry("r", "Force refresh now"));
    }

    lines.push(entry("?", "This help screen"));
    lines.push(entry(",", "Configuration"));
    lines.push(entry("z", "Toggle zen mode (collapse graphs)"));

    if !app.is_replay_mode() {
        lines.push(entry("L", "Load recording (replay mode)"));
    }

    lines.extend([
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
    ]);

    // Filter - only for panels that support it
    if panel.supports_filter() {
        lines.push(entry("/", "Fuzzy filter"));
    }

    lines.push(entry("Enter", "Inspect selected row"));

    // Bloat refresh - only for Tables and Indexes
    if matches!(panel, BottomPanel::TableStats | BottomPanel::Indexes) {
        lines.push(entry("b", "Refresh bloat estimates"));
    }

    // Query actions - only for Queries panel in live mode
    if panel == BottomPanel::Queries && !app.is_replay_mode() {
        lines.push(Line::from(""));
        lines.push(section_header("Query Actions"));
        lines.push(entry("C", "Cancel query (batch if filtered)"));
        lines.push(entry("K", "Terminate backend (batch if filtered)"));
        lines.push(entry("y", "Copy query to clipboard"));
    }

    // Replay controls - only in replay mode
    if app.is_replay_mode() {
        lines.push(Line::from(""));
        lines.push(section_header("Playback"));
        lines.push(entry("Space", "Play / pause"));
        lines.push(entry("← / h", "Step backward"));
        lines.push(entry("→ / l", "Step forward"));
        lines.push(entry("< / >", "Decrease / increase speed"));
        lines.push(entry("g / G", "Jump to start / end"));
    }

    lines.extend([
        Line::from(""),
        section_header("Overlay"),
        entry("Esc / q", "Close"),
        entry("j / k", "Scroll line"),
        entry("Ctrl+d/u", "Scroll page"),
        entry("PgDn/PgUp", "Scroll page"),
        entry("g / G", "Top / bottom"),
    ]);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.overlay_scroll, 0));
    frame.render_widget(paragraph, popup);
}
