use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect) {
    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let sep_style = Style::default().fg(Theme::border_dim());
    let desc_style = Style::default().fg(Theme::fg());

    let spans = vec![
        Span::styled(" q", key_style),
        Span::styled(" quit", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("p", key_style),
        Span::styled(" pause", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("↑↓", key_style),
        Span::styled(" select", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("enter", key_style),
        Span::styled(" inspect", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("C", key_style),
        Span::styled(" cancel", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("K", key_style),
        Span::styled(" kill", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("s", key_style),
        Span::styled(" sort", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("tab", key_style),
        Span::styled(" locks", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("w", key_style),
        Span::styled(" waits", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("t", key_style),
        Span::styled(" tables", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("R", key_style),
        Span::styled(" repl", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("v", key_style),
        Span::styled(" vacuum", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("x", key_style),
        Span::styled(" xid", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("I", key_style),
        Span::styled(" idx", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("S", key_style),
        Span::styled(" stmts", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("?", key_style),
        Span::styled(" help", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled(",", key_style),
        Span::styled(" config", desc_style),
    ];

    let paragraph =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Theme::header_bg()));

    frame.render_widget(paragraph, area);
}
