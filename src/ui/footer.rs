use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, BottomPanel, ViewMode};
use super::theme::Theme;

struct FooterStyles {
    key_style: Style,
    sep_style: Style,
    desc_style: Style,
    section_style: Style,
}

impl FooterStyles {
    fn new(section_bg: Color) -> Self {
        Self {
            key_style: Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD),
            sep_style: Style::default().fg(Theme::border_dim()),
            desc_style: Style::default().fg(Theme::fg_dim()),
            section_style: Style::default()
                .fg(Theme::header_bg())
                .bg(section_bg)
                .add_modifier(Modifier::BOLD),
        }
    }

    fn live() -> Self {
        Self::new(Theme::border_active())
    }

    fn replay() -> Self {
        Self::new(Theme::border_warn())
    }

    fn sep(&self) -> Span<'static> {
        Span::styled("  ", self.sep_style)
    }

    fn dot(&self) -> Span<'static> {
        Span::styled(" · ", self.sep_style)
    }

    fn key(&self, k: &str) -> Span<'static> {
        Span::styled(k.to_string(), self.key_style)
    }

    fn desc(&self, d: &str) -> Span<'static> {
        Span::styled(d.to_string(), self.desc_style)
    }

    fn pipe(&self) -> Span<'static> {
        Span::styled("│", self.sep_style)
    }

    fn space(&self) -> Span<'static> {
        Span::styled(" ", self.sep_style)
    }
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    if app.view_mode == ViewMode::Filter {
        render_filter(frame, app, area);
        return;
    }

    if app.is_replay_mode() {
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
        Span::styled(&app.filter.text, input_style),
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
    let styles = FooterStyles::live();

    let panel_name = panel_name(app.bottom_panel);

    // Line 1: Panel name + contextual actions
    let mut line1: Vec<Span> = vec![
        Span::styled(format!(" {panel_name} "), styles.section_style),
        styles.space(),
    ];
    render_panel_keys(&mut line1, app, &styles);

    // Line 2: Panel switches + global keys
    let mut line2: Vec<Span> = vec![styles.space()];
    render_panel_switch_keys(&mut line2, &styles);
    render_global_keys(&mut line2, app, &styles, true);

    let paragraph = Paragraph::new(vec![Line::from(line1), Line::from(line2)])
        .style(Style::default().bg(Theme::header_bg()));

    frame.render_widget(paragraph, area);
}

fn render_replay(frame: &mut Frame, app: &App, area: Rect) {
    let styles = FooterStyles::replay();

    // Line 1: Replay controls + panel-specific actions
    let mut line1: Vec<Span> = vec![
        Span::styled(" Replay ", styles.section_style),
        styles.space(),
        styles.key("Space"),
        styles.desc(" play/pause"),
        styles.dot(),
        styles.key("←→"),
        styles.desc(" step"),
        styles.dot(),
        styles.key("<>"),
        styles.desc(" speed"),
        styles.dot(),
        styles.key("g"),
        styles.desc("/"),
        styles.key("G"),
        styles.desc(" jump"),
    ];
    render_panel_keys(&mut line1, app, &styles);

    // Line 2: Panel switches + quit
    let mut line2: Vec<Span> = vec![styles.space()];
    render_panel_switch_keys(&mut line2, &styles);
    render_global_keys(&mut line2, app, &styles, false);

    let paragraph = Paragraph::new(vec![Line::from(line1), Line::from(line2)])
        .style(Style::default().bg(Theme::header_bg()));

    frame.render_widget(paragraph, area);
}

fn panel_name(panel: BottomPanel) -> &'static str {
    match panel {
        BottomPanel::Queries => "Queries",
        BottomPanel::Blocking => "Locks",
        BottomPanel::WaitEvents => "Waits",
        BottomPanel::TableStats => "Tables",
        BottomPanel::Replication => "Replication",
        BottomPanel::VacuumProgress => "Vacuum",
        BottomPanel::Wraparound => "XID",
        BottomPanel::Indexes => "Indexes",
        BottomPanel::Statements => "Statements",
        BottomPanel::WalIo => "WAL",
        BottomPanel::Settings => "Settings",
        BottomPanel::Extensions => "Extensions",
        BottomPanel::SchemaERD => "ERD",
    }
}

fn render_global_keys(spans: &mut Vec<Span<'static>>, app: &App, styles: &FooterStyles, is_live: bool) {
    spans.push(styles.sep());
    spans.push(styles.pipe());
    spans.push(styles.sep());
    spans.push(styles.key("z"));
    if app.graphs_collapsed {
        spans.push(styles.desc(" expand"));
    } else {
        spans.push(styles.desc(" zen"));
    }
    spans.push(styles.dot());
    if is_live {
        spans.push(styles.key("L"));
        spans.push(styles.desc(" replay"));
        spans.push(styles.dot());
        spans.push(styles.key("?"));
        spans.push(styles.desc(" help"));
        spans.push(styles.dot());
        spans.push(styles.key(","));
        spans.push(styles.desc(" config"));
        spans.push(styles.dot());
    }
    spans.push(styles.key("q"));
    spans.push(styles.desc(" quit"));
}

fn render_panel_keys(spans: &mut Vec<Span<'static>>, app: &App, styles: &FooterStyles) {
    match app.bottom_panel {
        BottomPanel::Queries => {
            spans.push(styles.sep());
            spans.push(styles.key("↑↓"));
            spans.push(styles.desc(" nav"));
            spans.push(styles.dot());
            spans.push(styles.key("⏎"));
            spans.push(styles.desc(" inspect"));
            spans.push(styles.dot());
            spans.push(styles.key("s"));
            spans.push(styles.desc(" sort"));
            spans.push(styles.dot());
            spans.push(styles.key("/"));
            spans.push(styles.desc(" filter"));
            if !app.is_replay_mode() {
                spans.push(styles.dot());
                spans.push(styles.key("C"));
                spans.push(styles.desc("/"));
                spans.push(styles.key("K"));
                spans.push(styles.desc(" cancel/kill"));
            }
        }
        BottomPanel::TableStats | BottomPanel::Indexes => {
            spans.push(styles.sep());
            spans.push(styles.key("↑↓"));
            spans.push(styles.desc(" nav"));
            spans.push(styles.dot());
            spans.push(styles.key("⏎"));
            spans.push(styles.desc(" inspect"));
            spans.push(styles.dot());
            spans.push(styles.key("s"));
            spans.push(styles.desc(" sort"));
            spans.push(styles.dot());
            spans.push(styles.key("/"));
            spans.push(styles.desc(" filter"));
            spans.push(styles.dot());
            spans.push(styles.key("Esc"));
            spans.push(styles.desc(" back"));
        }
        BottomPanel::Statements => {
            spans.push(styles.sep());
            spans.push(styles.key("↑↓"));
            spans.push(styles.desc(" nav"));
            spans.push(styles.dot());
            spans.push(styles.key("⏎"));
            spans.push(styles.desc(" inspect"));
            spans.push(styles.dot());
            spans.push(styles.key("s"));
            spans.push(styles.desc(" sort"));
            spans.push(styles.dot());
            spans.push(styles.key("/"));
            spans.push(styles.desc(" filter"));
            if !app.is_replay_mode() {
                spans.push(styles.dot());
                spans.push(styles.key("X"));
                spans.push(styles.desc(" reset"));
            }
            spans.push(styles.dot());
            spans.push(styles.key("Esc"));
            spans.push(styles.desc(" back"));
        }
        BottomPanel::Blocking | BottomPanel::VacuumProgress | BottomPanel::Wraparound | BottomPanel::Replication => {
            spans.push(styles.sep());
            spans.push(styles.key("↑↓"));
            spans.push(styles.desc(" nav"));
            spans.push(styles.dot());
            spans.push(styles.key("⏎"));
            spans.push(styles.desc(" inspect"));
            spans.push(styles.dot());
            spans.push(styles.key("Esc"));
            spans.push(styles.desc(" back"));
        }
        BottomPanel::WaitEvents | BottomPanel::WalIo => {
            spans.push(styles.sep());
            spans.push(styles.key("Esc"));
            spans.push(styles.desc(" back"));
        }
        BottomPanel::Settings | BottomPanel::Extensions => {
            spans.push(styles.sep());
            spans.push(styles.key("↑↓"));
            spans.push(styles.desc(" nav"));
            spans.push(styles.dot());
            spans.push(styles.key("⏎"));
            spans.push(styles.desc(" inspect"));
            spans.push(styles.dot());
            spans.push(styles.key("/"));
            spans.push(styles.desc(" filter"));
            spans.push(styles.dot());
            spans.push(styles.key("Esc"));
            spans.push(styles.desc(" back"));
        }
        BottomPanel::SchemaERD => {
            spans.push(styles.sep());
            spans.push(styles.key("↑↓"));
            spans.push(styles.desc(" nav"));
            spans.push(styles.dot());
            spans.push(styles.key("⏎"));
            spans.push(styles.desc(" inspect"));
            spans.push(styles.dot());
            spans.push(styles.key("/"));
            spans.push(styles.desc(" filter"));
            spans.push(styles.dot());
            spans.push(styles.key("Esc"));
            spans.push(styles.desc(" back"));
        }
    }
}

fn render_panel_switch_keys(spans: &mut Vec<Span<'static>>, styles: &FooterStyles) {
    spans.push(styles.key("⇥"));
    spans.push(styles.desc(" locks"));
    spans.push(styles.dot());
    spans.push(styles.key("w"));
    spans.push(styles.desc(" waits"));
    spans.push(styles.dot());
    spans.push(styles.key("t"));
    spans.push(styles.desc(" tables"));
    spans.push(styles.dot());
    spans.push(styles.key("R"));
    spans.push(styles.desc(" repl"));
    spans.push(styles.dot());
    spans.push(styles.key("v"));
    spans.push(styles.desc(" vacuum"));
    spans.push(styles.dot());
    spans.push(styles.key("x"));
    spans.push(styles.desc(" xid"));
    spans.push(styles.dot());
    spans.push(styles.key("I"));
    spans.push(styles.desc(" idx"));
    spans.push(styles.dot());
    spans.push(styles.key("S"));
    spans.push(styles.desc(" stmts"));
    spans.push(styles.dot());
    spans.push(styles.key("A"));
    spans.push(styles.desc(" wal"));
    spans.push(styles.dot());
    spans.push(styles.key("P"));
    spans.push(styles.desc(" cfg"));
    spans.push(styles.dot());
    spans.push(styles.key("E"));
    spans.push(styles.desc(" ext"));
}
