mod active_queries;
mod footer;
mod graph;
mod header;
mod layout;
mod overlay;
mod panels;
mod sparkline;
mod stats_panel;
pub mod theme;
mod util;

use crate::app::{App, BottomPanel, ViewMode};
use ratatui::Frame;
use theme::Theme;
use util::format_duration;

pub fn render(frame: &mut Frame, app: &mut App) {
    let areas = layout::compute_layout(frame.area());
    let marker = app.config.graph_marker.to_marker();

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
        Theme::graph_connections(),
        Theme::graph_connections(),
        marker,
    );

    stats_panel::render(frame, app, areas.graph_tr);

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
        Theme::graph_cache(),
        marker,
    );

    let avg_data = app.avg_query_time_history.as_vec();
    let avg_current = app.avg_query_time_history.last().unwrap_or(0);
    let avg_label = format_duration(avg_current as f64 / 1000.0);
    graph::render_line_chart(
        frame,
        areas.graph_br,
        "Avg Duration",
        &avg_label,
        &avg_data,
        Theme::graph_latency(),
        Theme::graph_latency(),
        marker,
    );

    // Bottom half: dispatch based on active panel
    let panel = app.bottom_panel;
    match panel {
        BottomPanel::Queries => active_queries::render(frame, app, areas.queries),
        BottomPanel::Blocking => panels::render_blocking(frame, app, areas.queries),
        BottomPanel::WaitEvents => panels::render_wait_events(frame, app, areas.queries),
        BottomPanel::TableStats => panels::render_table_stats(frame, app, areas.queries),
        BottomPanel::Replication => panels::render_replication(frame, app, areas.queries),
        BottomPanel::VacuumProgress => panels::render_vacuum_progress(frame, app, areas.queries),
        BottomPanel::Wraparound => panels::render_wraparound(frame, app, areas.queries),
        BottomPanel::Indexes => panels::render_indexes(frame, app, areas.queries),
        BottomPanel::Statements => panels::render_statements(frame, app, areas.queries),
        BottomPanel::WalIo => panels::render_wal_io(frame, app, areas.queries),
    }

    footer::render(frame, app, areas.footer);

    // Overlays (popup-only)
    match &app.view_mode {
        ViewMode::Inspect => overlay::render_inspect(frame, app, frame.area()),
        ViewMode::IndexInspect => overlay::render_index_inspect(frame, app, frame.area()),
        ViewMode::StatementInspect => {
            overlay::render_statement_inspect(frame, app, frame.area())
        }
        ViewMode::ReplicationInspect => {
            overlay::render_replication_inspect(frame, app, frame.area())
        }
        ViewMode::TableInspect => {
            overlay::render_table_inspect(frame, app, frame.area())
        }
        ViewMode::BlockingInspect => {
            overlay::render_blocking_inspect(frame, app, frame.area())
        }
        ViewMode::VacuumInspect => {
            overlay::render_vacuum_inspect(frame, app, frame.area())
        }
        ViewMode::WraparoundInspect => {
            overlay::render_wraparound_inspect(frame, app, frame.area())
        }
        ViewMode::ConfirmCancel(pid) => {
            overlay::render_confirm_cancel(frame, *pid, frame.area())
        }
        ViewMode::ConfirmKill(pid) => {
            overlay::render_confirm_kill(frame, *pid, frame.area())
        }
        ViewMode::ConfirmCancelChoice { selected_pid, all_pids } => {
            overlay::render_cancel_choice(frame, *selected_pid, all_pids, &app.filter_text, frame.area())
        }
        ViewMode::ConfirmKillChoice { selected_pid, all_pids } => {
            overlay::render_kill_choice(frame, *selected_pid, all_pids, &app.filter_text, frame.area())
        }
        ViewMode::ConfirmCancelBatch(pids) => {
            overlay::render_confirm_cancel_batch(frame, pids, frame.area())
        }
        ViewMode::ConfirmKillBatch(pids) => {
            overlay::render_confirm_kill_batch(frame, pids, frame.area())
        }
        ViewMode::Config => overlay::render_config(frame, app, frame.area()),
        ViewMode::Help => overlay::render_help(frame, app, frame.area()),
        ViewMode::Normal | ViewMode::Filter => {}
    }
}


