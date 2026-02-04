mod active_queries;
mod footer;
mod graph;
mod header;
mod layout;
mod overlay;
pub mod theme;

use crate::app::{App, ViewMode};
use ratatui::Frame;
use theme::Theme;

pub fn render(frame: &mut Frame, app: &mut App) {
    let areas = layout::compute_layout(frame.area());

    header::render(frame, app, areas.header);

    // Top half: 2x2 graph grid
    let conn_data = app.connection_history.as_vec();
    let conn_current = app.connection_history.last().unwrap_or(0);
    graph::render_line_chart(
        frame,
        areas.graph_tl,
        "Connections",
        &conn_current.to_string(),
        &conn_data,
        Theme::GRAPH_CONNECTIONS,
        Theme::GRAPH_CONNECTIONS,
    );

    let avg_data = app.avg_query_time_history.as_vec();
    let avg_current = app.avg_query_time_history.last().unwrap_or(0);
    let avg_label = format_duration_ms(avg_current);
    graph::render_line_chart(
        frame,
        areas.graph_tr,
        "Avg Query Time",
        &avg_label,
        &avg_data,
        Theme::GRAPH_QUERIES,
        Theme::GRAPH_QUERIES,
    );

    let cache_data = app.hit_ratio_history.as_vec();
    let cache_current = app.hit_ratio_history.last().unwrap_or(0);
    let cache_pct = cache_current as f64 / 10.0;
    let cache_color = Theme::hit_ratio_color(cache_pct);
    graph::render_ratio_chart(
        frame,
        areas.graph_bl,
        "Cache Hit Ratio",
        &format!("{:.1}%", cache_pct),
        &cache_data,
        cache_color,
        Theme::GRAPH_CACHE,
    );

    let lock_data = app.lock_count_history.as_vec();
    let lock_current = app.lock_count_history.last().unwrap_or(0);
    let lock_color = if lock_current > 0 {
        Theme::GRAPH_LOCKS
    } else {
        Theme::BORDER_OK
    };
    graph::render_line_chart(
        frame,
        areas.graph_br,
        "Locks",
        &lock_current.to_string(),
        &lock_data,
        lock_color,
        Theme::GRAPH_LOCKS,
    );

    // Bottom half: full-width query table
    active_queries::render(frame, app, areas.queries);

    footer::render(frame, areas.footer);

    // Overlays
    match &app.view_mode {
        ViewMode::Inspect => overlay::render_inspect(frame, app, frame.area()),
        ViewMode::Blocking => overlay::render_blocking(frame, app, frame.area()),
        ViewMode::WaitEvents => overlay::render_wait_events(frame, app, frame.area()),
        ViewMode::ConfirmCancel(pid) => {
            overlay::render_confirm_cancel(frame, *pid, frame.area())
        }
        ViewMode::ConfirmKill(pid) => {
            overlay::render_confirm_kill(frame, *pid, frame.area())
        }
        ViewMode::TableStats => overlay::render_table_stats(frame, app, frame.area()),
        ViewMode::Replication => overlay::render_replication(frame, app, frame.area()),
        ViewMode::VacuumProgress => overlay::render_vacuum_progress(frame, app, frame.area()),
        ViewMode::Wraparound => overlay::render_wraparound(frame, app, frame.area()),
        ViewMode::Indexes => {
            let area = frame.area();
            overlay::render_indexes(frame, app, area);
        }
        ViewMode::IndexInspect => overlay::render_index_inspect(frame, app, frame.area()),
        ViewMode::Normal => {}
    }
}

fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{:.0}m{:.0}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}
