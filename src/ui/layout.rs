use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct LayoutAreas {
    pub header: Rect,
    pub graph_tl: Rect,
    pub graph_tr: Rect,
    pub graph_bl: Rect,
    pub graph_br: Rect,
    pub queries: Rect,
    pub footer: Rect,
}

pub fn compute_layout(area: Rect) -> LayoutAreas {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(40),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    let graph_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    let graph_top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(graph_rows[0]);

    let graph_bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(graph_rows[1]);

    LayoutAreas {
        header: outer[0],
        graph_tl: graph_top[0],
        graph_tr: graph_top[1],
        graph_bl: graph_bot[0],
        graph_br: graph_bot[1],
        queries: outer[2],
        footer: outer[3],
    }
}
