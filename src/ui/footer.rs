use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, BottomPanel, ViewMode};
use super::theme::Theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    if app.view_mode == ViewMode::Filter {
        render_filter(frame, app, area);
        return;
    }

    if app.replay_mode {
        render_replay(frame, app, area);
    } else {
        render_live(frame, app, area);
    }
}

fn render_filter(frame: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Theme::fg_dim());
    let input_style = Style::default().fg(Theme::fg());
    let label_style = Style::default()
        .fg(Theme::header_bg())
        .bg(Theme::border_active())
        .add_modifier(Modifier::BOLD);

    let line1 = vec![
        Span::styled(" Filter ", label_style),
        Span::styled("  ", Style::default()),
        Span::styled(&app.filter_text, input_style),
        Span::styled("▌", Style::default().fg(Theme::border_active())),
    ];

    let line2 = vec![
        Span::styled(" ", Style::default()),
        Span::styled("⏎", key_style),
        Span::styled(" confirm", desc_style),
        Span::styled(" · ", Style::default().fg(Theme::border_dim())),
        Span::styled("Esc", key_style),
        Span::styled(" cancel", desc_style),
    ];

    let paragraph = Paragraph::new(vec![Line::from(line1), Line::from(line2)])
        .style(Style::default().bg(Theme::header_bg()));
    frame.render_widget(paragraph, area);
}

fn render_live(frame: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let sep_style = Style::default().fg(Theme::border_dim());
    let desc_style = Style::default().fg(Theme::fg_dim());
    let section_style = Style::default()
        .fg(Theme::header_bg())
        .bg(Theme::border_active())
        .add_modifier(Modifier::BOLD);

    let sep = || Span::styled("  ", sep_style);
    let dot = || Span::styled(" · ", sep_style);
    let key = |k: &str| Span::styled(k.to_string(), key_style);
    let desc = |d: &str| Span::styled(d.to_string(), desc_style);

    // Panel name for context
    let panel_name = match app.bottom_panel {
        BottomPanel::Queries => "Queries",
        BottomPanel::Blocking => "Locks",
        BottomPanel::WaitEvents => "Waits",
        BottomPanel::TableStats => "Tables",
        BottomPanel::Replication => "Replication",
        BottomPanel::VacuumProgress => "Vacuum",
        BottomPanel::Wraparound => "XID",
        BottomPanel::Indexes => "Indexes",
        BottomPanel::Statements => "Statements",
    };

    // Line 1: Panel name + contextual actions
    let mut line1: Vec<Span> = vec![
        Span::styled(format!(" {} ", panel_name), section_style),
        Span::styled(" ", sep_style),
    ];
    render_panel_keys(&mut line1, app, &sep, &dot, &key, &desc);

    // Line 2: Panel switches + global keys
    let mut line2: Vec<Span> = vec![Span::styled(" ", sep_style)];
    render_panel_switch_keys(&mut line2, &dot, &key, &desc);
    line2.push(sep());
    line2.push(Span::styled("│", sep_style));
    line2.push(sep());
    line2.push(key("?"));
    line2.push(desc(" help"));
    line2.push(dot());
    line2.push(key(","));
    line2.push(desc(" config"));
    line2.push(dot());
    line2.push(key("q"));
    line2.push(desc(" quit"));

    let paragraph = Paragraph::new(vec![Line::from(line1), Line::from(line2)])
        .style(Style::default().bg(Theme::header_bg()));

    frame.render_widget(paragraph, area);
}

fn render_replay(frame: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default()
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);
    let sep_style = Style::default().fg(Theme::border_dim());
    let desc_style = Style::default().fg(Theme::fg_dim());
    let section_style = Style::default()
        .fg(Theme::header_bg())
        .bg(Theme::border_warn())
        .add_modifier(Modifier::BOLD);

    let sep = || Span::styled("  ", sep_style);
    let dot = || Span::styled(" · ", sep_style);
    let key = |k: &str| Span::styled(k.to_string(), key_style);
    let desc = |d: &str| Span::styled(d.to_string(), desc_style);

    // Line 1: Replay controls + panel-specific actions
    let mut line1: Vec<Span> = vec![
        Span::styled(" Replay ", section_style),
        Span::styled(" ", sep_style),
        key("Space"),
        desc(" play/pause"),
        dot(),
        key("←→"),
        desc(" step"),
        dot(),
        key("<>"),
        desc(" speed"),
        dot(),
        key("g"),
        desc("/"),
        key("G"),
        desc(" jump"),
    ];
    render_panel_keys(&mut line1, app, &sep, &dot, &key, &desc);

    // Line 2: Panel switches + quit
    let mut line2: Vec<Span> = vec![Span::styled(" ", sep_style)];
    render_panel_switch_keys(&mut line2, &dot, &key, &desc);
    line2.push(sep());
    line2.push(Span::styled("│", sep_style));
    line2.push(sep());
    line2.push(key("q"));
    line2.push(desc(" quit"));

    let paragraph = Paragraph::new(vec![Line::from(line1), Line::from(line2)])
        .style(Style::default().bg(Theme::header_bg()));

    frame.render_widget(paragraph, area);
}

fn render_panel_keys<'a>(
    spans: &mut Vec<Span<'a>>,
    app: &App,
    sep: &dyn Fn() -> Span<'a>,
    dot: &dyn Fn() -> Span<'a>,
    key: &dyn Fn(&str) -> Span<'a>,
    desc: &dyn Fn(&str) -> Span<'a>,
) {
    match app.bottom_panel {
        BottomPanel::Queries => {
            spans.push(sep());
            spans.push(key("↑↓"));
            spans.push(desc(" nav"));
            spans.push(dot());
            spans.push(key("⏎"));
            spans.push(desc(" inspect"));
            spans.push(dot());
            spans.push(key("s"));
            spans.push(desc(" sort"));
            spans.push(dot());
            spans.push(key("/"));
            spans.push(desc(" filter"));
            if !app.replay_mode {
                spans.push(dot());
                spans.push(key("C"));
                spans.push(desc("/"));
                spans.push(key("K"));
                spans.push(desc(" cancel/kill"));
            }
        }
        BottomPanel::TableStats => {
            spans.push(sep());
            spans.push(key("↑↓"));
            spans.push(desc(" nav"));
            spans.push(dot());
            spans.push(key("⏎"));
            spans.push(desc(" inspect"));
            spans.push(dot());
            spans.push(key("s"));
            spans.push(desc(" sort"));
            spans.push(dot());
            spans.push(key("/"));
            spans.push(desc(" filter"));
            spans.push(dot());
            spans.push(key("Esc"));
            spans.push(desc(" back"));
        }
        BottomPanel::Replication => {
            spans.push(sep());
            spans.push(key("↑↓"));
            spans.push(desc(" nav"));
            spans.push(dot());
            spans.push(key("⏎"));
            spans.push(desc(" inspect"));
            spans.push(dot());
            spans.push(key("Esc"));
            spans.push(desc(" back"));
        }
        BottomPanel::Indexes | BottomPanel::Statements => {
            spans.push(sep());
            spans.push(key("↑↓"));
            spans.push(desc(" nav"));
            spans.push(dot());
            spans.push(key("⏎"));
            spans.push(desc(" inspect"));
            spans.push(dot());
            spans.push(key("s"));
            spans.push(desc(" sort"));
            spans.push(dot());
            spans.push(key("/"));
            spans.push(desc(" filter"));
            spans.push(dot());
            spans.push(key("Esc"));
            spans.push(desc(" back"));
        }
        BottomPanel::Blocking => {
            spans.push(sep());
            spans.push(key("↑↓"));
            spans.push(desc(" nav"));
            spans.push(dot());
            spans.push(key("⏎"));
            spans.push(desc(" inspect"));
            spans.push(dot());
            spans.push(key("Esc"));
            spans.push(desc(" back"));
        }
        BottomPanel::VacuumProgress => {
            spans.push(sep());
            spans.push(key("↑↓"));
            spans.push(desc(" nav"));
            spans.push(dot());
            spans.push(key("⏎"));
            spans.push(desc(" inspect"));
            spans.push(dot());
            spans.push(key("Esc"));
            spans.push(desc(" back"));
        }
        BottomPanel::Wraparound => {
            spans.push(sep());
            spans.push(key("↑↓"));
            spans.push(desc(" nav"));
            spans.push(dot());
            spans.push(key("⏎"));
            spans.push(desc(" inspect"));
            spans.push(dot());
            spans.push(key("Esc"));
            spans.push(desc(" back"));
        }
        BottomPanel::WaitEvents => {
            spans.push(sep());
            spans.push(key("Esc"));
            spans.push(desc(" back"));
        }
    }
}

fn render_panel_switch_keys<'a>(
    spans: &mut Vec<Span<'a>>,
    dot: &dyn Fn() -> Span<'a>,
    key: &dyn Fn(&str) -> Span<'a>,
    desc: &dyn Fn(&str) -> Span<'a>,
) {
    spans.push(key("⇥"));
    spans.push(desc(" locks"));
    spans.push(dot());
    spans.push(key("w"));
    spans.push(desc(" waits"));
    spans.push(dot());
    spans.push(key("t"));
    spans.push(desc(" tables"));
    spans.push(dot());
    spans.push(key("R"));
    spans.push(desc(" repl"));
    spans.push(dot());
    spans.push(key("v"));
    spans.push(desc(" vacuum"));
    spans.push(dot());
    spans.push(key("x"));
    spans.push(desc(" xid"));
    spans.push(dot());
    spans.push(key("I"));
    spans.push(desc(" idx"));
    spans.push(dot());
    spans.push(key("S"));
    spans.push(desc(" stmts"));
}
