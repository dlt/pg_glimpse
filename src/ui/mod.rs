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

use crate::app::{App, BottomPanel, ConfirmAction, InspectTarget, ViewMode};
use ratatui::Frame;
use theme::Theme;
use util::format_duration;

#[cfg(test)]
mod snapshot_tests;

pub fn render(frame: &mut Frame, app: &mut App) {
    let areas = layout::compute_layout(frame.area());
    let marker = app.config.graph_marker.to_marker();

    header::render(frame, app, areas.header);

    // Top half: 2x2 graph grid
    let conn_data = app.metrics.connections.as_vec();
    let conn_current = app.metrics.connections.last().unwrap_or(0);
    graph::render_line_chart(
        frame,
        areas.graph_tl,
        "🔌 Connections",
        &conn_current.to_string(),
        &conn_data,
        Theme::graph_connections(),
        Theme::graph_connections(),
        marker,
    );

    stats_panel::render(frame, app, areas.graph_tr);

    let cache_data = app.metrics.hit_ratio.as_vec();
    let cache_current = app.metrics.hit_ratio.last().unwrap_or(0);
    let cache_pct = cache_current as f64 / 10.0;
    let cache_color = Theme::hit_ratio_color(cache_pct);
    graph::render_ratio_chart(
        frame,
        areas.graph_bl,
        "💾 Cache Hit",
        &format!("{cache_pct:.1}%"),
        &cache_data,
        cache_color,
        Theme::graph_cache(),
        marker,
    );

    let avg_data = app.metrics.avg_query_time.as_vec();
    let avg_current = app.metrics.avg_query_time.last().unwrap_or(0);
    let avg_label = format_duration(avg_current as f64 / 1000.0);
    graph::render_line_chart(
        frame,
        areas.graph_br,
        "⏱️ Avg Duration",
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
        BottomPanel::Settings => panels::render_settings(frame, app, areas.queries),
        BottomPanel::Extensions => panels::render_extensions(frame, app, areas.queries),
    }

    footer::render(frame, app, areas.footer);

    // Overlays (popup-only)
    match &app.view_mode {
        ViewMode::Inspect(target) => {
            let area = frame.area();
            match target {
                InspectTarget::Query(pid) => overlay::render_inspect(frame, app, area, *pid),
                InspectTarget::Index(key) => overlay::render_index_inspect(frame, app, area, key),
                InspectTarget::Statement(queryid) => overlay::render_statement_inspect(frame, app, area, *queryid),
                InspectTarget::Replication(pid) => overlay::render_replication_inspect(frame, app, area, *pid),
                InspectTarget::Table(key) => overlay::render_table_inspect(frame, app, area, key),
                InspectTarget::Blocking(pid) => overlay::render_blocking_inspect(frame, app, area, *pid),
                InspectTarget::Vacuum(pid) => overlay::render_vacuum_inspect(frame, app, area, *pid),
                InspectTarget::Wraparound(datname) => overlay::render_wraparound_inspect(frame, app, area, datname),
                InspectTarget::Settings(name) => overlay::render_settings_inspect(frame, app, area, name),
                InspectTarget::Extensions(name) => overlay::render_extensions_inspect(frame, app, area, name),
            }
        }
        ViewMode::Confirm(action) => {
            let area = frame.area();
            match action {
                ConfirmAction::Cancel(pid) => overlay::render_confirm_cancel(frame, *pid, area),
                ConfirmAction::Kill(pid) => overlay::render_confirm_kill(frame, *pid, area),
                ConfirmAction::CancelChoice { selected_pid, all_pids } => {
                    overlay::render_cancel_choice(frame, *selected_pid, all_pids, &app.filter.text, area);
                }
                ConfirmAction::KillChoice { selected_pid, all_pids } => {
                    overlay::render_kill_choice(frame, *selected_pid, all_pids, &app.filter.text, area);
                }
                ConfirmAction::CancelBatch(pids) => {
                    overlay::render_confirm_cancel_batch(frame, pids, area);
                }
                ConfirmAction::KillBatch(pids) => {
                    overlay::render_confirm_kill_batch(frame, pids, area);
                }
                ConfirmAction::DeleteRecording(ref path) => {
                    overlay::render_confirm_delete_recording(frame, path, area);
                }
            }
        }
        ViewMode::Config | ViewMode::ConfigEditRecordingsDir => {
            overlay::render_config(frame, app, frame.area());
        }
        ViewMode::Help => overlay::render_help(frame, app, frame.area()),
        ViewMode::Recordings => overlay::render_recordings(frame, app, frame.area()),
        ViewMode::Normal | ViewMode::Filter => {}
    }
}


