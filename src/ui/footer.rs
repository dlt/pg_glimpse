use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, BottomPanel, ViewMode};
use super::theme::Theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    if app.view_mode == ViewMode::Filter {
        let key_style = Style::default()
            .fg(Theme::border_active())
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Theme::fg());

        let spans = vec![
            Span::styled(" / ", key_style),
            Span::styled(&app.filter_text, desc_style),
            Span::styled("_", Style::default().fg(Theme::border_active())),
            Span::styled("  (", Style::default().fg(Theme::border_dim())),
            Span::styled("Enter", key_style),
            Span::styled(" confirm, ", desc_style),
            Span::styled("Esc", key_style),
            Span::styled(" cancel)", desc_style),
        ];

        let paragraph =
            Paragraph::new(Line::from(spans)).style(Style::default().bg(Theme::header_bg()));
        frame.render_widget(paragraph, area);
        return;
    }

    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let sep_style = Style::default().fg(Theme::border_dim());
    let desc_style = Style::default().fg(Theme::fg());

    let sep = || Span::styled(" \u{2502} ", sep_style);
    let key = |k: &str| Span::styled(format!("{}", k), key_style);
    let desc = |d: &str| Span::styled(format!(" {}", d), desc_style);

    let mut spans: Vec<Span> = Vec::new();

    // Common prefix
    spans.push(Span::styled(" q", key_style));
    spans.push(desc("quit"));
    spans.push(sep());
    spans.push(key("p"));
    spans.push(desc("pause"));

    match app.bottom_panel {
        BottomPanel::Queries => {
            // Full queries footer
            spans.push(sep());
            spans.push(key("\u{2191}\u{2193}"));
            spans.push(desc("select"));
            spans.push(sep());
            spans.push(key("enter"));
            spans.push(desc("inspect"));
            spans.push(sep());
            spans.push(key("C"));
            spans.push(desc("cancel"));
            spans.push(sep());
            spans.push(key("K"));
            spans.push(desc("kill"));
            spans.push(sep());
            spans.push(key("s"));
            spans.push(desc("sort"));
            spans.push(sep());
            spans.push(key("/"));
            spans.push(desc("filter"));
        }
        BottomPanel::TableStats => {
            spans.push(sep());
            spans.push(key("\u{2191}\u{2193}"));
            spans.push(desc("select"));
            spans.push(sep());
            spans.push(key("s"));
            spans.push(desc("sort"));
            spans.push(sep());
            spans.push(key("Esc"));
            spans.push(desc("back"));
        }
        BottomPanel::Indexes | BottomPanel::Statements => {
            // Interactive panels with sort/filter
            spans.push(sep());
            spans.push(key("\u{2191}\u{2193}"));
            spans.push(desc("select"));
            spans.push(sep());
            spans.push(key("enter"));
            spans.push(desc("inspect"));
            spans.push(sep());
            spans.push(key("s"));
            spans.push(desc("sort"));
            spans.push(sep());
            spans.push(key("/"));
            spans.push(desc("filter"));
            spans.push(sep());
            spans.push(key("Esc"));
            spans.push(desc("back"));
        }
        _ => {
            // Static panels
            spans.push(sep());
            spans.push(key("Esc"));
            spans.push(desc("back"));
        }
    }

    // Panel switch keys (always shown)
    spans.push(sep());
    spans.push(key("tab"));
    spans.push(desc("locks"));
    spans.push(sep());
    spans.push(key("w"));
    spans.push(desc("waits"));
    spans.push(sep());
    spans.push(key("t"));
    spans.push(desc("tables"));
    spans.push(sep());
    spans.push(key("R"));
    spans.push(desc("repl"));
    spans.push(sep());
    spans.push(key("v"));
    spans.push(desc("vacuum"));
    spans.push(sep());
    spans.push(key("x"));
    spans.push(desc("xid"));
    spans.push(sep());
    spans.push(key("I"));
    spans.push(desc("idx"));
    spans.push(sep());
    spans.push(key("S"));
    spans.push(desc("stmts"));
    spans.push(sep());
    spans.push(key("?"));
    spans.push(desc("help"));
    spans.push(sep());
    spans.push(key(","));
    spans.push(desc("config"));

    let paragraph =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Theme::header_bg()));

    frame.render_widget(paragraph, area);
}
