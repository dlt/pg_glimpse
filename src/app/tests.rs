//! Tests for App state and key handling.

use super::*;
use crate::db::models::{
    ActiveQuery, ActivitySummary, BufferCacheStats, DetectedExtensions, PgExtension,
    PgSnapshot, ServerInfo,
};
use chrono::Utc;

fn make_server_info() -> ServerInfo {
    ServerInfo {
        version: "PostgreSQL 14.5".into(),
        start_time: Utc::now(),
        max_connections: 100,
        extensions: DetectedExtensions::default(),
        settings: vec![],
        extensions_list: vec![],
    }
}

fn make_snapshot() -> PgSnapshot {
    PgSnapshot {
        timestamp: Utc::now(),
        active_queries: vec![ActiveQuery {
            pid: 12345,
            usename: Some("postgres".into()),
            datname: Some("testdb".into()),
            state: Some("active".into()),
            query: Some("SELECT 1".into()),
            duration_secs: 1.5,
            wait_event_type: None,
            wait_event: None,
            query_start: None,
            backend_type: None,
        }],
        wait_events: vec![],
        blocking_info: vec![],
        buffer_cache: BufferCacheStats {
            blks_hit: 9900,
            blks_read: 100,
            hit_ratio: 0.99,
        },
        summary: ActivitySummary {
            total_backends: 10,
            active_query_count: 1,
            idle_in_transaction_count: 0,
            waiting_count: 0,
            lock_count: 0,
            oldest_xact_secs: None,
            autovacuum_count: 0,
        },
        table_stats: vec![],
        replication: vec![],
        replication_slots: vec![],
        subscriptions: vec![],
        vacuum_progress: vec![],
        wraparound: vec![],
        indexes: vec![],
        stat_statements: vec![],
        stat_statements_error: None,
        extensions: DetectedExtensions::default(),
        db_size: 1_000_000,
        checkpoint_stats: None,
        wal_stats: None,
        archiver_stats: None,
        bgwriter_stats: None,
        db_stats: None,
    }
}

fn make_app() -> App {
    App::new(
        "localhost".into(),
        5432,
        "postgres".into(),
        "postgres".into(),
        2,
        120,
        AppConfig::default(),
        make_server_info(),
    )
}

fn make_replay_app() -> App {
    App::new_replay(
        "localhost".into(),
        5432,
        "postgres".into(),
        "postgres".into(),
        120,
        AppConfig::default(),
        make_server_info(),
        "test.jsonl".into(),
        10,
    )
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn key_ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

// ─────────────────────────────────────────────────────────────────────────────
// Global keys
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn quit_from_queries_panel() {
    let mut app = make_app();
    assert!(app.running);
    app.handle_key(key(KeyCode::Char('q')));
    assert!(!app.running);
}

#[test]
fn quit_from_other_panel_returns_to_queries() {
    let mut app = make_app();
    app.bottom_panel = BottomPanel::Indexes;
    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.running);
    assert_eq!(app.bottom_panel, BottomPanel::Queries);
}

#[test]
fn ctrl_c_quits() {
    let mut app = make_app();
    app.handle_key(key_ctrl(KeyCode::Char('c')));
    assert!(!app.running);
}

#[test]
fn esc_from_queries_quits() {
    let mut app = make_app();
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.running);
}

#[test]
fn esc_from_other_panel_returns_to_queries() {
    let mut app = make_app();
    app.bottom_panel = BottomPanel::Replication;
    app.handle_key(key(KeyCode::Esc));
    assert!(app.running);
    assert_eq!(app.bottom_panel, BottomPanel::Queries);
}

#[test]
fn pause_toggle() {
    let mut app = make_app();
    assert!(!app.paused);
    app.handle_key(key(KeyCode::Char('p')));
    assert!(app.paused);
    app.handle_key(key(KeyCode::Char('p')));
    assert!(!app.paused);
}

#[test]
fn pause_disabled_in_replay_mode() {
    let mut app = make_replay_app();
    assert!(!app.paused);
    app.handle_key(key(KeyCode::Char('p')));
    assert!(!app.paused); // Should remain unchanged
}

#[test]
fn force_refresh() {
    let mut app = make_app();
    app.handle_key(key(KeyCode::Char('r')));
    assert!(matches!(app.pending_action, Some(AppAction::ForceRefresh)));
}

#[test]
fn force_refresh_disabled_in_replay_mode() {
    let mut app = make_replay_app();
    app.handle_key(key(KeyCode::Char('r')));
    assert!(app.pending_action.is_none());
}

#[test]
fn bloat_refresh_disabled_in_replay_mode_tables() {
    let mut app = make_replay_app();
    app.bottom_panel = BottomPanel::TableStats;
    app.handle_key(key(KeyCode::Char('b')));
    assert!(app.pending_action.is_none());
    assert!(!app.bloat_loading);
}

#[test]
fn bloat_refresh_disabled_in_replay_mode_indexes() {
    let mut app = make_replay_app();
    app.bottom_panel = BottomPanel::Indexes;
    app.handle_key(key(KeyCode::Char('b')));
    assert!(app.pending_action.is_none());
    assert!(!app.bloat_loading);
}

#[test]
fn recordings_browser_disabled_in_replay_mode() {
    let mut app = make_replay_app();
    app.handle_key(key(KeyCode::Char('L')));
    // Should not open recordings browser since we're already in replay
    assert_eq!(app.view_mode, ViewMode::Normal);
}

#[test]
fn help_opens() {
    let mut app = make_app();
    app.handle_key(key(KeyCode::Char('?')));
    assert_eq!(app.view_mode, ViewMode::Help);
}

#[test]
fn config_opens() {
    let mut app = make_app();
    app.handle_key(key(KeyCode::Char(',')));
    assert_eq!(app.view_mode, ViewMode::Config);
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel switching
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn tab_switches_to_blocking() {
    let mut app = make_app();
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.bottom_panel, BottomPanel::Blocking);
}

#[test]
fn tab_toggles_back_to_queries() {
    let mut app = make_app();
    app.bottom_panel = BottomPanel::Blocking;
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.bottom_panel, BottomPanel::Queries);
}

#[test]
fn panel_switch_keys() {
    let cases = [
        ('w', BottomPanel::WaitEvents),
        ('t', BottomPanel::TableStats),
        ('R', BottomPanel::Replication),
        ('v', BottomPanel::VacuumProgress),
        ('x', BottomPanel::Wraparound),
        ('I', BottomPanel::Indexes),
        ('S', BottomPanel::Statements),
        ('A', BottomPanel::WalIo),
        ('P', BottomPanel::Settings),
        ('Q', BottomPanel::Queries),
    ];
    for (ch, expected) in cases {
        let mut app = make_app();
        app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(app.bottom_panel, expected, "Key '{ch}' should switch to {expected:?}");
    }
}

#[test]
fn panel_switch_clears_filter() {
    let mut app = make_app();
    app.filter.text = "test".into();
    app.filter.active = true;
    app.handle_key(key(KeyCode::Char('I')));
    assert!(app.filter.text.is_empty());
    assert!(!app.filter.active);
}

// ─────────────────────────────────────────────────────────────────────────────
// Filter mode
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn filter_opens_on_supported_panels() {
    for panel in [
        BottomPanel::Queries,
        BottomPanel::Indexes,
        BottomPanel::Statements,
        BottomPanel::TableStats,
        BottomPanel::Settings,
    ] {
        let mut app = make_app();
        app.bottom_panel = panel;
        app.handle_key(key(KeyCode::Char('/')));
        assert_eq!(app.view_mode, ViewMode::Filter, "Filter should open on {panel:?}");
    }
}

#[test]
fn filter_does_not_open_on_unsupported_panels() {
    for panel in [
        BottomPanel::Blocking,
        BottomPanel::WaitEvents,
        BottomPanel::Replication,
        BottomPanel::VacuumProgress,
        BottomPanel::Wraparound,
        BottomPanel::WalIo,
    ] {
        let mut app = make_app();
        app.bottom_panel = panel;
        app.handle_key(key(KeyCode::Char('/')));
        assert_eq!(app.view_mode, ViewMode::Normal, "Filter should not open on {panel:?}");
    }
}

#[test]
fn filter_typing() {
    let mut app = make_app();
    app.view_mode = ViewMode::Filter;
    app.handle_key(key(KeyCode::Char('t')));
    app.handle_key(key(KeyCode::Char('e')));
    app.handle_key(key(KeyCode::Char('s')));
    app.handle_key(key(KeyCode::Char('t')));
    assert_eq!(app.filter.text, "test");
}

#[test]
fn filter_backspace() {
    let mut app = make_app();
    app.view_mode = ViewMode::Filter;
    app.filter.text = "test".into();
    app.handle_key(key(KeyCode::Backspace));
    assert_eq!(app.filter.text, "tes");
}

#[test]
fn filter_enter_activates() {
    let mut app = make_app();
    app.view_mode = ViewMode::Filter;
    app.filter.text = "query".into();
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(app.filter.active);
}

#[test]
fn filter_enter_with_empty_text_does_not_activate() {
    let mut app = make_app();
    app.view_mode = ViewMode::Filter;
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(!app.filter.active);
}

#[test]
fn filter_esc_clears_and_exits() {
    let mut app = make_app();
    app.view_mode = ViewMode::Filter;
    app.filter.text = "test".into();
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(app.filter.text.is_empty());
    assert!(!app.filter.active);
}

// ─────────────────────────────────────────────────────────────────────────────
// Config mode
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn config_navigation() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = 0;

    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.config_selected, 1);

    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.config_selected, 2);

    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.config_selected, 1);

    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.config_selected, 0);
}

#[test]
fn config_esc_saves() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(matches!(app.pending_action, Some(AppAction::SaveConfig)));
}

#[test]
fn config_adjust_refresh_interval() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::RefreshInterval)
        .unwrap();
    let initial = app.config.refresh_interval_secs;
    app.handle_key(key(KeyCode::Right));
    assert_eq!(app.config.refresh_interval_secs, initial + 1);
    app.handle_key(key(KeyCode::Left));
    assert_eq!(app.config.refresh_interval_secs, initial);
}

// ─────────────────────────────────────────────────────────────────────────────
// Help mode
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn help_scroll() {
    let mut app = make_app();
    app.view_mode = ViewMode::Help;
    app.overlay_scroll = 5;

    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.overlay_scroll, 6);

    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.overlay_scroll, 5);

    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.overlay_scroll, 0);

    app.handle_key(key(KeyCode::Char('G')));
    assert_eq!(app.overlay_scroll, u16::MAX);
}

#[test]
fn help_scroll_page() {
    let mut app = make_app();
    app.view_mode = ViewMode::Help;
    app.overlay_scroll = 20;

    // Ctrl+D scrolls down by page (10 lines)
    app.handle_key(key_ctrl(KeyCode::Char('d')));
    assert_eq!(app.overlay_scroll, 30);

    // Ctrl+U scrolls up by page
    app.handle_key(key_ctrl(KeyCode::Char('u')));
    assert_eq!(app.overlay_scroll, 20);

    // PageDown
    app.handle_key(key(KeyCode::PageDown));
    assert_eq!(app.overlay_scroll, 30);

    // PageUp
    app.handle_key(key(KeyCode::PageUp));
    assert_eq!(app.overlay_scroll, 20);

    // Ctrl+U at top doesn't underflow
    app.overlay_scroll = 5;
    app.handle_key(key_ctrl(KeyCode::Char('u')));
    assert_eq!(app.overlay_scroll, 0);
}

#[test]
fn help_exit_keys() {
    for code in [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Enter] {
        let mut app = make_app();
        app.view_mode = ViewMode::Help;
        app.overlay_scroll = 10;
        app.handle_key(key(code));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert_eq!(app.overlay_scroll, 0);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Confirm dialogs
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn confirm_cancel_yes() {
    let mut app = make_app();
    app.view_mode = ViewMode::ConfirmCancel(12345);
    app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(matches!(app.pending_action, Some(AppAction::CancelQuery(12345))));
}

#[test]
fn confirm_cancel_any_other_key_aborts() {
    let mut app = make_app();
    app.view_mode = ViewMode::ConfirmCancel(12345);
    app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(app.pending_action.is_none());
    assert!(app.status_message.as_ref().unwrap().contains("aborted"));
}

#[test]
fn confirm_kill_yes() {
    let mut app = make_app();
    app.view_mode = ViewMode::ConfirmKill(12345);
    app.handle_key(key(KeyCode::Char('Y')));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(matches!(app.pending_action, Some(AppAction::TerminateBackend(12345))));
}

#[test]
fn confirm_cancel_choice_one() {
    let mut app = make_app();
    app.view_mode = ViewMode::ConfirmCancelChoice {
        selected_pid: 100,
        all_pids: vec![100, 200, 300],
    };
    app.handle_key(key(KeyCode::Char('1')));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(matches!(app.pending_action, Some(AppAction::CancelQuery(100))));
}

#[test]
fn confirm_cancel_choice_all() {
    let mut app = make_app();
    app.view_mode = ViewMode::ConfirmCancelChoice {
        selected_pid: 100,
        all_pids: vec![100, 200, 300],
    };
    app.handle_key(key(KeyCode::Char('a')));
    assert!(matches!(app.view_mode, ViewMode::ConfirmCancelBatch(_)));
}

#[test]
fn confirm_cancel_batch_yes() {
    let mut app = make_app();
    app.view_mode = ViewMode::ConfirmCancelBatch(vec![100, 200, 300]);
    app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(app.view_mode, ViewMode::Normal);
    match &app.pending_action {
        Some(AppAction::CancelQueries(pids)) => assert_eq!(pids, &vec![100, 200, 300]),
        _ => panic!("Expected CancelQueries action"),
    }
}

#[test]
fn confirm_kill_choice_esc() {
    let mut app = make_app();
    app.view_mode = ViewMode::ConfirmKillChoice {
        selected_pid: 100,
        all_pids: vec![100, 200],
    };
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert!(app.pending_action.is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// Inspect modes
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn inspect_scroll_and_exit() {
    let modes = [
        ViewMode::Inspect,
        ViewMode::IndexInspect,
        ViewMode::StatementInspect,
        ViewMode::ReplicationInspect,
        ViewMode::TableInspect,
        ViewMode::BlockingInspect,
        ViewMode::VacuumInspect,
        ViewMode::WraparoundInspect,
    ];
    for mode in modes {
        let mut app = make_app();
        app.view_mode = mode.clone();
        app.overlay_scroll = 5;

        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.overlay_scroll, 6, "Down should scroll in {mode:?}");

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.overlay_scroll, 5, "k should scroll up in {mode:?}");

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.view_mode, ViewMode::Normal, "Esc should exit {mode:?}");
        assert_eq!(app.overlay_scroll, 0, "Overlay scroll should reset after {mode:?}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel-specific navigation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn queries_panel_navigation() {
    let mut app = make_app();
    app.queries.state.select(Some(5));
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.queries.selected(), Some(4));

    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.queries.selected(), Some(3));
}

#[test]
fn queries_panel_sort_cycle() {
    let mut app = make_app();
    assert_eq!(app.queries.sort_column, SortColumn::Duration);

    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(app.queries.sort_column, SortColumn::Pid);

    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(app.queries.sort_column, SortColumn::User);
}

#[test]
fn indexes_panel_bloat_refresh() {
    let mut app = make_app();
    app.bottom_panel = BottomPanel::Indexes;
    app.handle_key(key(KeyCode::Char('b')));
    assert!(matches!(app.pending_action, Some(AppAction::RefreshBloat)));
    assert!(app.bloat_loading);
}

#[test]
fn indexes_panel_bloat_disabled_in_replay() {
    let mut app = make_replay_app();
    app.bottom_panel = BottomPanel::Indexes;
    app.handle_key(key(KeyCode::Char('b')));
    assert!(app.pending_action.is_none());
    assert!(!app.bloat_loading);
}

#[test]
fn table_stats_panel_bloat_refresh() {
    let mut app = make_app();
    app.bottom_panel = BottomPanel::TableStats;
    app.handle_key(key(KeyCode::Char('b')));
    assert!(matches!(app.pending_action, Some(AppAction::RefreshBloat)));
}

// ─────────────────────────────────────────────────────────────────────────────
// Cancel/Kill in replay mode
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cancel_disabled_in_replay_mode() {
    let mut app = make_replay_app();
    // Set up a query so there's something to cancel
    app.snapshot = Some(make_snapshot());
    app.queries.state.select(Some(0));
    app.handle_key(key(KeyCode::Char('C')));
    // Should not enter any confirm mode
    assert_eq!(app.view_mode, ViewMode::Normal);
}

#[test]
fn kill_disabled_in_replay_mode() {
    let mut app = make_replay_app();
    app.snapshot = Some(make_snapshot());
    app.queries.state.select(Some(0));
    app.handle_key(key(KeyCode::Char('K')));
    assert_eq!(app.view_mode, ViewMode::Normal);
}

// ─────────────────────────────────────────────────────────────────────────────
// Modal consumes all input
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn modal_blocks_global_keys() {
    let mut app = make_app();
    app.view_mode = ViewMode::Help;

    // 'q' should not quit when in help mode, just close help
    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.running); // Should still be running
    assert_eq!(app.view_mode, ViewMode::Normal);
}

#[test]
fn confirm_modal_blocks_panel_switch() {
    let mut app = make_app();
    app.view_mode = ViewMode::ConfirmCancel(123);

    // Tab should not switch panels
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.bottom_panel, BottomPanel::Queries);
    // Instead it should abort the confirm
    assert_eq!(app.view_mode, ViewMode::Normal);
}

// ─────────────────────────────────────────────────────────────────────────────
// Sort column cycling
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sort_column_cycles() {
    assert_eq!(SortColumn::Duration.next(), SortColumn::Pid);
    assert_eq!(SortColumn::Pid.next(), SortColumn::User);
    assert_eq!(SortColumn::User.next(), SortColumn::State);
    assert_eq!(SortColumn::State.next(), SortColumn::Duration);
}

#[test]
fn index_sort_column_cycles() {
    assert_eq!(IndexSortColumn::Scans.next(), IndexSortColumn::Size);
    assert_eq!(IndexSortColumn::Size.next(), IndexSortColumn::Name);
    assert_eq!(IndexSortColumn::Name.next(), IndexSortColumn::TupRead);
    assert_eq!(IndexSortColumn::TupRead.next(), IndexSortColumn::TupFetch);
    assert_eq!(IndexSortColumn::TupFetch.next(), IndexSortColumn::Scans);
}

#[test]
fn statement_sort_column_cycles() {
    assert_eq!(StatementSortColumn::TotalTime.next(), StatementSortColumn::MeanTime);
    assert_eq!(StatementSortColumn::Temp.next(), StatementSortColumn::TotalTime);
}

#[test]
fn table_stat_sort_column_cycles() {
    assert_eq!(TableStatSortColumn::DeadTuples.next(), TableStatSortColumn::Size);
    assert_eq!(TableStatSortColumn::DeadRatio.next(), TableStatSortColumn::DeadTuples);
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel supports_filter
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_supports_filter() {
    assert!(BottomPanel::Queries.supports_filter());
    assert!(BottomPanel::Indexes.supports_filter());
    assert!(BottomPanel::Statements.supports_filter());
    assert!(BottomPanel::TableStats.supports_filter());
    assert!(BottomPanel::Settings.supports_filter());

    assert!(!BottomPanel::Blocking.supports_filter());
    assert!(!BottomPanel::WaitEvents.supports_filter());
    assert!(!BottomPanel::Replication.supports_filter());
    assert!(!BottomPanel::VacuumProgress.supports_filter());
    assert!(!BottomPanel::Wraparound.supports_filter());
    assert!(!BottomPanel::WalIo.supports_filter());
}

// ─────────────────────────────────────────────────────────────────────────────
// App::update edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn update_with_empty_snapshot() {
    let mut app = make_app();
    let mut snap = make_snapshot();
    snap.active_queries.clear();
    snap.summary.total_backends = 0;
    snap.summary.active_query_count = 0;

    app.update(snap);

    assert!(app.snapshot.is_some());
    assert_eq!(app.snapshot.as_ref().unwrap().active_queries.len(), 0);
    // History should still be updated
    assert!(!app.metrics.connections.as_vec().is_empty());
}

#[test]
fn update_clears_last_error() {
    let mut app = make_app();
    app.last_error = Some("Previous error".to_string());

    app.update(make_snapshot());

    assert!(app.last_error.is_none());
}

#[test]
fn update_populates_histories() {
    let mut app = make_app();
    assert!(app.metrics.connections.as_vec().is_empty());
    assert!(app.metrics.hit_ratio.as_vec().is_empty());

    let mut snap = make_snapshot();
    snap.summary.total_backends = 25;
    snap.buffer_cache.hit_ratio = 0.95;
    app.update(snap);

    assert_eq!(app.metrics.connections.as_vec().len(), 1);
    assert_eq!(app.metrics.hit_ratio.as_vec().len(), 1);
}

#[test]
fn update_calculates_avg_query_time() {
    let mut app = make_app();
    let mut snap = make_snapshot();
    snap.active_queries = vec![
        ActiveQuery {
            pid: 1,
            usename: None,
            datname: None,
            state: Some("active".into()),
            query: None,
            duration_secs: 2.0,
            wait_event_type: None,
            wait_event: None,
            query_start: None,
            backend_type: None,
        },
        ActiveQuery {
            pid: 2,
            usename: None,
            datname: None,
            state: Some("active".into()),
            query: None,
            duration_secs: 4.0,
            wait_event_type: None,
            wait_event: None,
            query_start: None,
            backend_type: None,
        },
    ];

    app.update(snap);

    // Average of 2.0 and 4.0 = 3.0 seconds = 3000ms
    let last_avg = app.metrics.avg_query_time.last().unwrap();
    assert_eq!(last_avg, 3000);
}

#[test]
fn update_handles_no_active_queries() {
    let mut app = make_app();
    let mut snap = make_snapshot();
    // Only idle queries (not "active" or "idle in transaction")
    snap.active_queries = vec![ActiveQuery {
        pid: 1,
        usename: None,
        datname: None,
        state: Some("idle".into()),
        query: None,
        duration_secs: 100.0,
        wait_event_type: None,
        wait_event: None,
        query_start: None,
        backend_type: None,
    }];

    app.update(snap);

    // No active queries, so avg should be 0
    let last_avg = app.metrics.avg_query_time.last().unwrap();
    assert_eq!(last_avg, 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// sorted_*_indices edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sorted_query_indices_no_snapshot() {
    let app = make_app();
    assert!(app.snapshot.is_none());
    assert!(app.sorted_query_indices().is_empty());
}

#[test]
fn sorted_query_indices_empty_queries() {
    let mut app = make_app();
    let mut snap = make_snapshot();
    snap.active_queries.clear();
    app.update(snap);

    assert!(app.sorted_query_indices().is_empty());
}

#[test]
fn sorted_index_indices_no_snapshot() {
    let app = make_app();
    assert!(app.sorted_index_indices().is_empty());
}

#[test]
fn sorted_stmt_indices_no_snapshot() {
    let app = make_app();
    assert!(app.sorted_stmt_indices().is_empty());
}

#[test]
fn sorted_table_stat_indices_no_snapshot() {
    let app = make_app();
    assert!(app.sorted_table_stat_indices().is_empty());
}

#[test]
fn sorted_settings_indices_no_snapshot() {
    let app = make_app();
    assert!(app.sorted_settings_indices().is_empty());
}

#[test]
fn selected_query_pid_no_snapshot() {
    let app = make_app();
    assert!(app.selected_query_pid().is_none());
}

#[test]
fn selected_query_pid_no_selection() {
    let mut app = make_app();
    app.update(make_snapshot());
    app.queries.state.select(None);

    assert!(app.selected_query_pid().is_none());
}

#[test]
fn get_filtered_pids_no_snapshot() {
    let app = make_app();
    assert!(app.get_filtered_pids().is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Filter edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn filter_with_no_matches() {
    let mut app = make_app();
    app.update(make_snapshot());
    app.bottom_panel = BottomPanel::Queries;
    app.filter.text = "xyznonexistent123".to_string();
    app.filter.active = true;

    let indices = app.sorted_query_indices();
    assert!(indices.is_empty());
}

#[test]
fn filter_with_special_characters() {
    let mut app = make_app();
    let mut snap = make_snapshot();
    snap.active_queries[0].query = Some("SELECT * FROM \"table-with-dashes\"".into());
    app.update(snap);

    app.bottom_panel = BottomPanel::Queries;
    app.filter.text = "table-with".to_string();
    app.filter.active = true;

    let indices = app.sorted_query_indices();
    assert!(!indices.is_empty());
}

#[test]
fn filter_inactive_ignores_filter_text() {
    let mut app = make_app();
    app.update(make_snapshot());
    app.bottom_panel = BottomPanel::Queries;
    app.filter.text = "xyznonexistent123".to_string();
    app.filter.active = false;
    app.view_mode = ViewMode::Normal;

    // When filter is not active, all queries should be returned
    let indices = app.sorted_query_indices();
    assert!(!indices.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// config_adjust bounds checking
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn config_adjust_refresh_interval_lower_bound() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::RefreshInterval)
        .unwrap();

    app.config.refresh_interval_secs = 1; // At minimum
    app.config_adjust(-1); // Try to go below

    assert_eq!(app.config.refresh_interval_secs, 1); // Should stay at 1
}

#[test]
fn config_adjust_refresh_interval_upper_bound() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::RefreshInterval)
        .unwrap();

    app.config.refresh_interval_secs = 60; // At maximum
    app.config_adjust(1); // Try to go above

    assert_eq!(app.config.refresh_interval_secs, 60); // Should stay at 60
}

#[test]
fn config_adjust_warn_duration_lower_bound() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::WarnDuration)
        .unwrap();

    app.config.warn_duration_secs = 0.1;
    app.config_adjust(-1); // Try to go below 0.1

    assert!(app.config.warn_duration_secs >= 0.1);
}

#[test]
fn config_adjust_warn_duration_clamped_to_danger() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::WarnDuration)
        .unwrap();

    app.config.danger_duration_secs = 5.0;
    app.config.warn_duration_secs = 5.0;
    app.config_adjust(1); // Try to go above danger

    assert!(app.config.warn_duration_secs <= app.config.danger_duration_secs);
}

#[test]
fn config_adjust_danger_duration_clamped_to_warn() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::DangerDuration)
        .unwrap();

    app.config.warn_duration_secs = 5.0;
    app.config.danger_duration_secs = 5.0;
    app.config_adjust(-1); // Try to go below warn

    assert!(app.config.danger_duration_secs >= app.config.warn_duration_secs);
}

#[test]
fn config_adjust_danger_duration_upper_bound() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::DangerDuration)
        .unwrap();

    app.config.danger_duration_secs = 300.0; // At maximum
    app.config_adjust(1); // Try to go above

    assert_eq!(app.config.danger_duration_secs, 300.0);
}

#[test]
fn config_adjust_recording_retention_lower_bound() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::RecordingRetention)
        .unwrap();

    app.config.recording_retention_secs = 600; // At minimum (10 min)
    app.config_adjust(-1);

    assert_eq!(app.config.recording_retention_secs, 600);
}

#[test]
fn config_adjust_recording_retention_upper_bound() {
    let mut app = make_app();
    app.view_mode = ViewMode::Config;
    app.config_selected = ConfigItem::ALL
        .iter()
        .position(|&i| i == ConfigItem::RecordingRetention)
        .unwrap();

    app.config.recording_retention_secs = 86400; // At maximum (24 hours)
    app.config_adjust(1);

    assert_eq!(app.config.recording_retention_secs, 86400);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rate calculation edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn rate_calculation_with_counter_reset() {
    use crate::db::models::DatabaseStats;

    let mut app = make_app();

    // First snapshot with high counter values
    let mut snap1 = make_snapshot();
    snap1.db_stats = Some(DatabaseStats {
        xact_commit: 1_000_000,
        xact_rollback: 100,
        blks_read: 50000,
    });
    app.update(snap1);

    // Second snapshot with lower values (simulating server restart)
    let mut snap2 = make_snapshot();
    snap2.timestamp = chrono::Utc::now() + chrono::Duration::seconds(2);
    snap2.db_stats = Some(DatabaseStats {
        xact_commit: 100, // Lower than before - counter reset
        xact_rollback: 0,
        blks_read: 100,
    });
    app.update(snap2);

    // TPS should not be calculated when counters go backwards
    // (the rate calculation guards against negative values)
}

// ─────────────────────────────────────────────────────────────────────────────
// Bloat preservation during update
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn update_preserves_bloat_data() {
    use crate::db::models::TableStat;

    let mut app = make_app();

    // First update with bloat data
    let mut snap1 = make_snapshot();
    snap1.table_stats = vec![TableStat {
        schemaname: "public".into(),
        relname: "users".into(),
        total_size_bytes: 1_000_000,
        table_size_bytes: 800_000,
        indexes_size_bytes: 200_000,
        seq_scan: 100,
        seq_tup_read: 5000,
        idx_scan: 500,
        idx_tup_fetch: 4500,
        n_live_tup: 10000,
        n_dead_tup: 500,
        dead_ratio: 5.0,
        n_tup_ins: 100,
        n_tup_upd: 50,
        n_tup_del: 10,
        n_tup_hot_upd: 20,
        last_vacuum: None,
        last_autovacuum: None,
        last_analyze: None,
        last_autoanalyze: None,
        vacuum_count: 0,
        autovacuum_count: 0,
        bloat_bytes: Some(100_000),
        bloat_pct: Some(12.5),
        bloat_source: Some(crate::db::models::BloatSource::Statistical),
    }];
    app.update(snap1);

    // Second update without bloat data
    let mut snap2 = make_snapshot();
    snap2.table_stats = vec![TableStat {
        schemaname: "public".into(),
        relname: "users".into(),
        total_size_bytes: 1_000_100, // Slightly different
        table_size_bytes: 800_100,
        indexes_size_bytes: 200_000,
        seq_scan: 110,
        seq_tup_read: 5500,
        idx_scan: 510,
        idx_tup_fetch: 4600,
        n_live_tup: 10050,
        n_dead_tup: 480,
        dead_ratio: 4.8,
        n_tup_ins: 150,
        n_tup_upd: 60,
        n_tup_del: 15,
        n_tup_hot_upd: 25,
        last_vacuum: None,
        last_autovacuum: None,
        last_analyze: None,
        last_autoanalyze: None,
        vacuum_count: 0,
        autovacuum_count: 0,
        bloat_bytes: None, // No bloat in new snapshot
        bloat_pct: None,
        bloat_source: None,
    }];
    app.update(snap2);

    // Bloat data should be preserved from previous snapshot
    let snap = app.snapshot.as_ref().unwrap();
    assert_eq!(snap.table_stats[0].bloat_bytes, Some(100_000));
    assert_eq!(snap.table_stats[0].bloat_pct, Some(12.5));
}

// ─────────────────────────────────────────────────────────────────────────────
// Navigation with empty data
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn navigate_queries_with_empty_list() {
    let mut app = make_app();
    let mut snap = make_snapshot();
    snap.active_queries.clear();
    app.update(snap);

    // Navigating should not panic
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Up));
    app.handle_key(key(KeyCode::Enter)); // Inspect on empty list
}

#[test]
fn navigate_indexes_with_empty_list() {
    let mut app = make_app();
    app.update(make_snapshot());
    app.bottom_panel = BottomPanel::Indexes;

    // Navigating should not panic
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Up));
    app.handle_key(key(KeyCode::Enter)); // Inspect on empty list
}

// ─────────────────────────────────────────────────────────────────────────────
// Replay mode specific
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn replay_mode_state() {
    let app = make_replay_app();
    assert!(app.is_replay_mode());
    let replay = app.replay.as_ref().unwrap();
    assert_eq!(replay.filename, "test.jsonl");
    assert_eq!(replay.total, 10);
}

#[test]
fn replay_mode_disables_cancel_kill() {
    let mut app = make_replay_app();
    app.update(make_snapshot());
    app.queries.state.select(Some(0));

    // C and K should be ignored in replay mode
    app.handle_key(key(KeyCode::Char('C')));
    assert_eq!(app.view_mode, ViewMode::Normal);

    app.handle_key(key(KeyCode::Char('K')));
    assert_eq!(app.view_mode, ViewMode::Normal);
}

// ─────────────────────────────────────────────────────────────────────────────
// BottomPanel::label
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bottom_panel_labels() {
    assert_eq!(BottomPanel::Queries.label(), "Queries");
    assert_eq!(BottomPanel::Blocking.label(), "Blocking");
    assert_eq!(BottomPanel::WaitEvents.label(), "Wait Events");
    assert_eq!(BottomPanel::TableStats.label(), "Table Stats");
    assert_eq!(BottomPanel::Replication.label(), "Replication");
    assert_eq!(BottomPanel::VacuumProgress.label(), "Vacuum Progress");
    assert_eq!(BottomPanel::Wraparound.label(), "Wraparound");
    assert_eq!(BottomPanel::Indexes.label(), "Indexes");
    assert_eq!(BottomPanel::Statements.label(), "Statements");
    assert_eq!(BottomPanel::WalIo.label(), "WAL & I/O");
    assert_eq!(BottomPanel::Settings.label(), "Settings");
}

// ─────────────────────────────────────────────────────────────────────────────
// Rate calculation tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn rate_calculation_first_snapshot_no_rate() {
    use crate::db::models::DatabaseStats;

    let mut app = make_app();

    // First snapshot - should not calculate rates (no previous)
    let mut snap = make_snapshot();
    snap.db_stats = Some(DatabaseStats {
        xact_commit: 1000,
        xact_rollback: 10,
        blks_read: 500,
    });
    app.update(snap);

    // No previous snapshot, so no rate should be calculated
    assert!(app.metrics.current_tps.is_none());
    assert!(app.metrics.current_blks_read_rate.is_none());
}

#[test]
fn rate_calculation_tps_normal() {
    use crate::db::models::DatabaseStats;

    let mut app = make_app();
    let base_time = chrono::Utc::now();

    // First snapshot
    let mut snap1 = make_snapshot();
    snap1.timestamp = base_time;
    snap1.db_stats = Some(DatabaseStats {
        xact_commit: 1000,
        xact_rollback: 10,
        blks_read: 500,
    });
    app.update(snap1);

    // Second snapshot 2 seconds later with 200 more transactions
    let mut snap2 = make_snapshot();
    snap2.timestamp = base_time + chrono::Duration::seconds(2);
    snap2.db_stats = Some(DatabaseStats {
        xact_commit: 1190, // +190 commits
        xact_rollback: 20, // +10 rollbacks
        blks_read: 600,    // +100 reads
    });
    app.update(snap2);

    // TPS should be (190 + 10) / 2 = 100 TPS
    assert!(app.metrics.current_tps.is_some());
    let tps = app.metrics.current_tps.unwrap();
    assert!((tps - 100.0).abs() < 0.1, "Expected ~100 TPS, got {tps}");

    // Blks/sec should be 100 / 2 = 50
    assert!(app.metrics.current_blks_read_rate.is_some());
    let blks = app.metrics.current_blks_read_rate.unwrap();
    assert!((blks - 50.0).abs() < 0.1, "Expected ~50 blks/s, got {blks}");

    // History should have one entry
    assert_eq!(app.metrics.tps.as_vec().len(), 1);
    assert_eq!(app.metrics.blks_read.as_vec().len(), 1);
}

#[test]
fn rate_calculation_wal_rate() {
    use crate::db::models::{DatabaseStats, WalStats};

    let mut app = make_app();
    let base_time = chrono::Utc::now();

    // First snapshot with WAL stats
    let mut snap1 = make_snapshot();
    snap1.timestamp = base_time;
    snap1.db_stats = Some(DatabaseStats {
        xact_commit: 1000,
        xact_rollback: 10,
        blks_read: 500,
    });
    snap1.wal_stats = Some(WalStats {
        wal_records: 10000,
        wal_fpi: 100,
        wal_bytes: 1_000_000, // 1 MB
        wal_buffers_full: 0,
        wal_write: 1000,
        wal_sync: 1000,
        wal_write_time: 100.0,
        wal_sync_time: 50.0,
    });
    app.update(snap1);

    // Second snapshot 2 seconds later with 2MB more WAL
    let mut snap2 = make_snapshot();
    snap2.timestamp = base_time + chrono::Duration::seconds(2);
    snap2.db_stats = Some(DatabaseStats {
        xact_commit: 1100,
        xact_rollback: 10,
        blks_read: 600,
    });
    snap2.wal_stats = Some(WalStats {
        wal_records: 12000,
        wal_fpi: 120,
        wal_bytes: 3_000_000, // 3 MB (+2 MB)
        wal_buffers_full: 0,
        wal_write: 1200,
        wal_sync: 1200,
        wal_write_time: 120.0,
        wal_sync_time: 60.0,
    });
    app.update(snap2);

    // WAL rate should be 2MB / 2s = 1MB/s
    assert!(app.metrics.current_wal_rate.is_some());
    let wal_rate = app.metrics.current_wal_rate.unwrap();
    let expected = 1_000_000.0; // 1 MB/s
    assert!(
        (wal_rate - expected).abs() < 1000.0,
        "Expected ~1MB/s WAL rate, got {wal_rate}"
    );

    // History should have one entry (stored as KB/s)
    assert_eq!(app.metrics.wal_rate.as_vec().len(), 1);
}

#[test]
fn rate_calculation_missing_db_stats() {
    let mut app = make_app();
    let base_time = chrono::Utc::now();

    // First snapshot without db_stats
    let mut snap1 = make_snapshot();
    snap1.timestamp = base_time;
    snap1.db_stats = None;
    app.update(snap1);

    // Second snapshot also without db_stats
    let mut snap2 = make_snapshot();
    snap2.timestamp = base_time + chrono::Duration::seconds(2);
    snap2.db_stats = None;
    app.update(snap2);

    // No rates should be calculated
    assert!(app.metrics.current_tps.is_none());
    assert!(app.metrics.current_blks_read_rate.is_none());
}

#[test]
fn rate_calculation_missing_wal_stats() {
    use crate::db::models::DatabaseStats;

    let mut app = make_app();
    let base_time = chrono::Utc::now();

    // First snapshot without wal_stats
    let mut snap1 = make_snapshot();
    snap1.timestamp = base_time;
    snap1.db_stats = Some(DatabaseStats {
        xact_commit: 1000,
        xact_rollback: 10,
        blks_read: 500,
    });
    snap1.wal_stats = None;
    app.update(snap1);

    // Second snapshot
    let mut snap2 = make_snapshot();
    snap2.timestamp = base_time + chrono::Duration::seconds(2);
    snap2.db_stats = Some(DatabaseStats {
        xact_commit: 1100,
        xact_rollback: 10,
        blks_read: 600,
    });
    snap2.wal_stats = None;
    app.update(snap2);

    // TPS should be calculated, but WAL rate should not
    assert!(app.metrics.current_tps.is_some());
    assert!(app.metrics.current_wal_rate.is_none());
}

#[test]
fn rate_calculation_zero_time_difference() {
    use crate::db::models::DatabaseStats;

    let mut app = make_app();
    let same_time = chrono::Utc::now();

    // Two snapshots with same timestamp
    let mut snap1 = make_snapshot();
    snap1.timestamp = same_time;
    snap1.db_stats = Some(DatabaseStats {
        xact_commit: 1000,
        xact_rollback: 10,
        blks_read: 500,
    });
    app.update(snap1);

    let mut snap2 = make_snapshot();
    snap2.timestamp = same_time; // Same timestamp
    snap2.db_stats = Some(DatabaseStats {
        xact_commit: 1100,
        xact_rollback: 20,
        blks_read: 600,
    });
    app.update(snap2);

    // No rate should be calculated with zero time difference
    // (would cause division by zero)
    assert!(app.metrics.current_tps.is_none());
}

#[test]
fn rate_calculation_very_small_interval() {
    use crate::db::models::DatabaseStats;

    let mut app = make_app();
    let base_time = chrono::Utc::now();

    // First snapshot
    let mut snap1 = make_snapshot();
    snap1.timestamp = base_time;
    snap1.db_stats = Some(DatabaseStats {
        xact_commit: 1000,
        xact_rollback: 10,
        blks_read: 500,
    });
    app.update(snap1);

    // Second snapshot 100ms later
    let mut snap2 = make_snapshot();
    snap2.timestamp = base_time + chrono::Duration::milliseconds(100);
    snap2.db_stats = Some(DatabaseStats {
        xact_commit: 1010, // +10 in 100ms
        xact_rollback: 10,
        blks_read: 505,
    });
    app.update(snap2);

    // TPS should be 10 / 0.1 = 100 TPS
    assert!(app.metrics.current_tps.is_some());
    let tps = app.metrics.current_tps.unwrap();
    assert!((tps - 100.0).abs() < 1.0, "Expected ~100 TPS, got {tps}");
}

#[test]
fn rate_calculation_history_accumulates() {
    use crate::db::models::DatabaseStats;

    let mut app = make_app();
    let base_time = chrono::Utc::now();

    // Initial snapshot
    let mut snap = make_snapshot();
    snap.timestamp = base_time;
    snap.db_stats = Some(DatabaseStats {
        xact_commit: 1000,
        xact_rollback: 0,
        blks_read: 100,
    });
    app.update(snap);

    // Add 5 more snapshots
    for i in 1..=5 {
        let mut snap = make_snapshot();
        snap.timestamp = base_time + chrono::Duration::seconds(i * 2);
        snap.db_stats = Some(DatabaseStats {
            xact_commit: 1000 + (i * 100), // +100 per 2 sec = 50 TPS
            xact_rollback: 0,
            blks_read: 100 + (i * 10),
        });
        app.update(snap);
    }

    // Should have 5 history entries
    assert_eq!(app.metrics.tps.as_vec().len(), 5);
    assert_eq!(app.metrics.blks_read.as_vec().len(), 5);
}

#[test]
fn rate_calculation_counter_reset_blks() {
    use crate::db::models::DatabaseStats;

    let mut app = make_app();
    let base_time = chrono::Utc::now();

    // First snapshot with high blks_read
    let mut snap1 = make_snapshot();
    snap1.timestamp = base_time;
    snap1.db_stats = Some(DatabaseStats {
        xact_commit: 1000,
        xact_rollback: 10,
        blks_read: 1_000_000,
    });
    app.update(snap1);

    // Second snapshot with lower blks_read (counter reset)
    let mut snap2 = make_snapshot();
    snap2.timestamp = base_time + chrono::Duration::seconds(2);
    snap2.db_stats = Some(DatabaseStats {
        xact_commit: 1100, // Normal increase
        xact_rollback: 20,
        blks_read: 100, // Counter reset
    });
    app.update(snap2);

    // TPS should be calculated (commits/rollbacks increased normally)
    assert!(app.metrics.current_tps.is_some());
    // But blks rate should not be in history (counter went backwards)
    // Note: current_blks_read_rate may still have old value
}

// ─────────────────────────────────────────────────────────────────────────────
// Error handling tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn update_error_sets_last_error() {
    let mut app = make_app();

    assert!(app.last_error.is_none());

    app.update_error("Connection refused".to_string());

    assert!(app.last_error.is_some());
    assert_eq!(app.last_error.as_ref().unwrap(), "Connection refused");
}

#[test]
fn update_error_overwrites_previous() {
    let mut app = make_app();

    app.update_error("First error".to_string());
    assert_eq!(app.last_error.as_ref().unwrap(), "First error");

    app.update_error("Second error".to_string());
    assert_eq!(app.last_error.as_ref().unwrap(), "Second error");
}

#[test]
fn update_clears_error() {
    let mut app = make_app();

    app.update_error("Some error".to_string());
    assert!(app.last_error.is_some());

    // Successful update should clear error
    app.update(make_snapshot());
    assert!(app.last_error.is_none());
}

#[test]
fn stat_statements_error_displayed() {
    let mut app = make_app();

    let mut snap = make_snapshot();
    snap.stat_statements_error = Some("permission denied for view pg_stat_statements".to_string());
    app.update(snap);

    // Error should be preserved in snapshot
    assert!(app.snapshot.is_some());
    let snapshot = app.snapshot.as_ref().unwrap();
    assert!(snapshot.stat_statements_error.is_some());
    assert!(snapshot.stat_statements_error.as_ref().unwrap().contains("permission denied"));
}

#[test]
fn empty_active_queries_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();

    let mut snap = make_snapshot();
    snap.active_queries = vec![];
    app.update(snap);

    // Navigation should not panic with empty queries
    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(key_j);
    app.handle_key(key_k);

    // Selection should stay at None or 0
    let selected = app.queries.selected();
    assert!(selected.is_none() || selected == Some(0));
}

#[test]
fn empty_tables_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    app.bottom_panel = BottomPanel::TableStats;

    let mut snap = make_snapshot();
    snap.table_stats = vec![];
    app.update(snap);

    // Navigation should not panic
    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(key_j);
    app.handle_key(key_k);
}

#[test]
fn empty_indexes_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    app.bottom_panel = BottomPanel::Indexes;

    let mut snap = make_snapshot();
    snap.indexes = vec![];
    app.update(snap);

    // Navigation should not panic
    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(key_j);
    app.handle_key(key_k);
}

#[test]
fn empty_blocking_info_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    app.bottom_panel = BottomPanel::Blocking;

    let mut snap = make_snapshot();
    snap.blocking_info = vec![];
    app.update(snap);

    // Navigation should not panic
    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(key_j);
    app.handle_key(key_k);
}

#[test]
fn empty_replication_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    app.bottom_panel = BottomPanel::Replication;

    let mut snap = make_snapshot();
    snap.replication = vec![];
    app.update(snap);

    // Navigation should not panic
    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(key_j);
    app.handle_key(key_k);
}

#[test]
fn empty_vacuum_progress_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    app.bottom_panel = BottomPanel::VacuumProgress;

    let mut snap = make_snapshot();
    snap.vacuum_progress = vec![];
    app.update(snap);

    // Navigation should not panic
    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(key_j);
    app.handle_key(key_k);
}

#[test]
fn empty_wraparound_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    app.bottom_panel = BottomPanel::Wraparound;

    let mut snap = make_snapshot();
    snap.wraparound = vec![];
    app.update(snap);

    // Navigation should not panic
    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(key_j);
    app.handle_key(key_k);
}

#[test]
fn empty_statements_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    app.bottom_panel = BottomPanel::Statements;

    let mut snap = make_snapshot();
    snap.stat_statements = vec![];
    app.update(snap);

    // Navigation should not panic
    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    app.handle_key(key_j);
    app.handle_key(key_k);
}

#[test]
fn no_snapshot_navigation_no_panic() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = make_app();
    app.snapshot = None;

    let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);

    // All navigation should be safe with no snapshot
    app.handle_key(key_j);
    app.handle_key(key_k);

    // Panel switching should work
    for panel in [
        BottomPanel::Queries,
        BottomPanel::Blocking,
        BottomPanel::WaitEvents,
        BottomPanel::TableStats,
        BottomPanel::Replication,
        BottomPanel::VacuumProgress,
        BottomPanel::Wraparound,
        BottomPanel::Indexes,
        BottomPanel::Statements,
        BottomPanel::WalIo,
    ] {
        app.bottom_panel = panel;
        app.handle_key(key_j);
        app.handle_key(key_k);
    }
}

#[test]
fn inspect_with_no_selection_no_panic() {
    let mut app = make_app();
    app.update(make_snapshot());

    // Clear selection
    app.queries.state.select(None);

    // Trying to enter inspect mode should be safe
    app.view_mode = ViewMode::Inspect;

    // App should handle this state gracefully
    assert!(app.snapshot.is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// Sort column labels
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sort_column_labels() {
    assert_eq!(SortColumn::Pid.label(), "PID");
    assert_eq!(SortColumn::Duration.label(), "Duration");
    assert_eq!(SortColumn::State.label(), "State");
    assert_eq!(SortColumn::User.label(), "User");
}

#[test]
fn table_stat_sort_column_labels() {
    assert_eq!(TableStatSortColumn::DeadTuples.label(), "Dead Tuples");
    assert_eq!(TableStatSortColumn::Size.label(), "Size");
    assert_eq!(TableStatSortColumn::Name.label(), "Name");
    assert_eq!(TableStatSortColumn::SeqScan.label(), "Seq Scan");
    assert_eq!(TableStatSortColumn::IdxScan.label(), "Idx Scan");
    assert_eq!(TableStatSortColumn::DeadRatio.label(), "Dead %");
}

#[test]
fn statement_sort_column_labels() {
    assert_eq!(StatementSortColumn::TotalTime.label(), "Total Time");
    assert_eq!(StatementSortColumn::MeanTime.label(), "Mean Time");
    assert_eq!(StatementSortColumn::MaxTime.label(), "Max Time");
    assert_eq!(StatementSortColumn::Stddev.label(), "Stddev");
    assert_eq!(StatementSortColumn::Calls.label(), "Calls");
    assert_eq!(StatementSortColumn::Rows.label(), "Rows");
    assert_eq!(StatementSortColumn::HitRatio.label(), "Hit %");
    assert_eq!(StatementSortColumn::SharedReads.label(), "Reads");
    assert_eq!(StatementSortColumn::IoTime.label(), "I/O Time");
    assert_eq!(StatementSortColumn::Temp.label(), "Temp");
}

// ─────────────────────────────────────────────────────────────────────────────
// Extensions panel tests
// ─────────────────────────────────────────────────────────────────────────────

fn make_extensions() -> Vec<PgExtension> {
    vec![
        PgExtension {
            name: "pg_stat_statements".into(),
            version: "1.10".into(),
            schema: "public".into(),
            relocatable: true,
            description: Some("track execution statistics of all SQL statements executed".into()),
        },
        PgExtension {
            name: "plpgsql".into(),
            version: "1.0".into(),
            schema: "pg_catalog".into(),
            relocatable: false,
            description: Some("PL/pgSQL procedural language".into()),
        },
        PgExtension {
            name: "uuid-ossp".into(),
            version: "1.1".into(),
            schema: "public".into(),
            relocatable: true,
            description: Some("generate universally unique identifiers (UUIDs)".into()),
        },
    ]
}

fn make_app_with_extensions() -> App {
    let mut server_info = make_server_info();
    server_info.extensions_list = make_extensions();
    App::new(
        "localhost".into(),
        5432,
        "postgres".into(),
        "postgres".into(),
        2,
        120,
        AppConfig::default(),
        server_info,
    )
}

#[test]
fn extensions_panel_switch() {
    let mut app = make_app();
    app.handle_key(key(KeyCode::Char('E')));
    assert_eq!(app.bottom_panel, BottomPanel::Extensions);
}

#[test]
fn extensions_panel_toggle_back_to_queries() {
    let mut app = make_app();
    app.bottom_panel = BottomPanel::Extensions;
    app.handle_key(key(KeyCode::Char('E')));
    assert_eq!(app.bottom_panel, BottomPanel::Queries);
}

#[test]
fn extensions_panel_supports_filter() {
    assert!(BottomPanel::Extensions.supports_filter());
}

#[test]
fn extensions_panel_label() {
    assert_eq!(BottomPanel::Extensions.label(), "Extensions");
}

#[test]
fn extensions_filter_opens() {
    let mut app = make_app();
    app.bottom_panel = BottomPanel::Extensions;
    app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(app.view_mode, ViewMode::Filter);
}

#[test]
fn sorted_extensions_indices_empty() {
    let app = make_app();
    assert!(app.sorted_extensions_indices().is_empty());
}

#[test]
fn sorted_extensions_indices_with_data() {
    let app = make_app_with_extensions();
    let indices = app.sorted_extensions_indices();
    assert_eq!(indices.len(), 3);
}

#[test]
fn extensions_filter_matches_name() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.filter.text = "plpgsql".into();
    app.filter.active = true;

    let indices = app.sorted_extensions_indices();
    assert_eq!(indices.len(), 1);
    assert_eq!(app.server_info.extensions_list[indices[0]].name, "plpgsql");
}

#[test]
fn extensions_filter_matches_schema() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    // pg_catalog is unique to plpgsql in our test data
    app.filter.text = "pg_catalog".into();
    app.filter.active = true;

    let indices = app.sorted_extensions_indices();
    assert_eq!(indices.len(), 1);
    assert_eq!(app.server_info.extensions_list[indices[0]].name, "plpgsql");
}

#[test]
fn extensions_filter_no_matches() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.filter.text = "nonexistent123".into();
    app.filter.active = true;

    let indices = app.sorted_extensions_indices();
    assert!(indices.is_empty());
}

#[test]
fn extensions_filter_inactive_shows_all() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.filter.text = "stat".into();
    app.filter.active = false;
    app.view_mode = ViewMode::Normal;

    // When filter is inactive, all extensions should be returned
    let indices = app.sorted_extensions_indices();
    assert_eq!(indices.len(), 3);
}

#[test]
fn extensions_navigation_down() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.extensions_table_state.select(Some(0));

    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.extensions_table_state.selected(), Some(1));

    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.extensions_table_state.selected(), Some(2));
}

#[test]
fn extensions_navigation_up() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.extensions_table_state.select(Some(2));

    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.extensions_table_state.selected(), Some(1));

    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.extensions_table_state.selected(), Some(0));
}

#[test]
fn extensions_navigation_up_at_top() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.extensions_table_state.select(Some(0));

    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.extensions_table_state.selected(), Some(0));
}

#[test]
fn extensions_navigation_down_at_bottom() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.extensions_table_state.select(Some(2));

    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.extensions_table_state.selected(), Some(2));
}

#[test]
fn extensions_inspect_opens() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.extensions_table_state.select(Some(0));

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.view_mode, ViewMode::ExtensionsInspect);
    assert_eq!(app.overlay_scroll, 0);
}

#[test]
fn extensions_inspect_does_not_open_when_empty() {
    let mut app = make_app(); // No extensions
    app.bottom_panel = BottomPanel::Extensions;

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.view_mode, ViewMode::Normal);
}

#[test]
fn extensions_inspect_closes_with_esc() {
    let mut app = make_app_with_extensions();
    app.view_mode = ViewMode::ExtensionsInspect;
    app.overlay_scroll = 5;

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.view_mode, ViewMode::Normal);
    assert_eq!(app.overlay_scroll, 0);
}

#[test]
fn extensions_inspect_closes_with_q() {
    let mut app = make_app_with_extensions();
    app.view_mode = ViewMode::ExtensionsInspect;

    app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(app.view_mode, ViewMode::Normal);
}

#[test]
fn extensions_inspect_scroll_down() {
    let mut app = make_app_with_extensions();
    app.view_mode = ViewMode::ExtensionsInspect;
    app.overlay_scroll = 0;

    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.overlay_scroll, 1);

    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.overlay_scroll, 2);
}

#[test]
fn extensions_inspect_scroll_up() {
    let mut app = make_app_with_extensions();
    app.view_mode = ViewMode::ExtensionsInspect;
    app.overlay_scroll = 5;

    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.overlay_scroll, 4);

    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.overlay_scroll, 3);
}

#[test]
fn extensions_reset_selection_on_filter_confirm() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.extensions_table_state.select(Some(2));
    app.view_mode = ViewMode::Filter;
    app.filter.text = "test".into();

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.extensions_table_state.selected(), Some(0));
}

#[test]
fn extensions_panel_switch_clears_filter() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;
    app.filter.text = "test".into();
    app.filter.active = true;

    app.handle_key(key(KeyCode::Char('I'))); // Switch to Indexes
    assert!(app.filter.text.is_empty());
    assert!(!app.filter.active);
}

#[test]
fn extensions_esc_returns_to_queries() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.bottom_panel, BottomPanel::Queries);
}

#[test]
fn extensions_q_returns_to_queries() {
    let mut app = make_app_with_extensions();
    app.bottom_panel = BottomPanel::Extensions;

    app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(app.bottom_panel, BottomPanel::Queries);
}

#[test]
fn empty_extensions_no_panic() {
    let mut app = make_app();
    app.bottom_panel = BottomPanel::Extensions;

    // Navigation should not panic with empty list
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Up));
    app.handle_key(key(KeyCode::Enter));

    // Filter should work on empty list
    app.view_mode = ViewMode::Filter;
    app.handle_key(key(KeyCode::Char('t')));
    app.handle_key(key(KeyCode::Enter));

    assert!(app.sorted_extensions_indices().is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Recordings browser
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn recordings_opens_with_l_key() {
    let mut app = make_app();
    app.handle_key(key(KeyCode::Char('L')));
    assert_eq!(app.view_mode, ViewMode::Recordings);
}

#[test]
fn recordings_l_key_disabled_in_replay_mode() {
    let mut app = make_replay_app();
    app.handle_key(key(KeyCode::Char('L')));
    assert_eq!(app.view_mode, ViewMode::Normal);
}

#[test]
fn recordings_closes_with_esc() {
    let mut app = make_app();
    app.view_mode = ViewMode::Recordings;
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.view_mode, ViewMode::Normal);
}

#[test]
fn recordings_closes_with_q() {
    let mut app = make_app();
    app.view_mode = ViewMode::Recordings;
    app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(app.view_mode, ViewMode::Normal);
}

#[test]
fn recordings_navigation_with_empty_list() {
    let mut app = make_app();
    app.view_mode = ViewMode::Recordings;
    // Should not panic
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Up));
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Char('d')));
}

#[test]
fn recordings_navigation_down() {
    use crate::recorder::RecordingInfo;
    use std::path::PathBuf;

    let mut app = make_app();
    app.view_mode = ViewMode::Recordings;
    app.recordings_list = vec![
        RecordingInfo {
            path: PathBuf::from("/tmp/test1.jsonl"),
            host: "host1".into(),
            port: 5432,
            dbname: "db1".into(),
            recorded_at: Utc::now(),
            pg_version: "PostgreSQL 15.0".into(),
            file_size: 1000,
        },
        RecordingInfo {
            path: PathBuf::from("/tmp/test2.jsonl"),
            host: "host2".into(),
            port: 5432,
            dbname: "db2".into(),
            recorded_at: Utc::now(),
            pg_version: "PostgreSQL 14.0".into(),
            file_size: 2000,
        },
    ];

    assert_eq!(app.recordings_selected, 0);
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.recordings_selected, 1);
    // At bottom, should not go further
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.recordings_selected, 1);
}

#[test]
fn recordings_navigation_up() {
    use crate::recorder::RecordingInfo;
    use std::path::PathBuf;

    let mut app = make_app();
    app.view_mode = ViewMode::Recordings;
    app.recordings_list = vec![
        RecordingInfo {
            path: PathBuf::from("/tmp/test1.jsonl"),
            host: "host1".into(),
            port: 5432,
            dbname: "db1".into(),
            recorded_at: Utc::now(),
            pg_version: "PostgreSQL 15.0".into(),
            file_size: 1000,
        },
        RecordingInfo {
            path: PathBuf::from("/tmp/test2.jsonl"),
            host: "host2".into(),
            port: 5432,
            dbname: "db2".into(),
            recorded_at: Utc::now(),
            pg_version: "PostgreSQL 14.0".into(),
            file_size: 2000,
        },
    ];
    app.recordings_selected = 1;

    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.recordings_selected, 0);
    // At top, should not go further
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.recordings_selected, 0);
}

#[test]
fn recordings_enter_sets_pending_replay_path() {
    use crate::recorder::RecordingInfo;
    use std::path::PathBuf;

    let mut app = make_app();
    app.view_mode = ViewMode::Recordings;
    let expected_path = PathBuf::from("/tmp/test.jsonl");
    app.recordings_list = vec![RecordingInfo {
        path: expected_path.clone(),
        host: "host".into(),
        port: 5432,
        dbname: "db".into(),
        recorded_at: Utc::now(),
        pg_version: "PostgreSQL 15.0".into(),
        file_size: 1000,
    }];

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.pending_replay_path, Some(expected_path));
    assert!(!app.running);
}

#[test]
fn recordings_d_key_opens_delete_confirm() {
    use crate::recorder::RecordingInfo;
    use std::path::PathBuf;

    let mut app = make_app();
    app.view_mode = ViewMode::Recordings;
    let test_path = PathBuf::from("/tmp/test.jsonl");
    app.recordings_list = vec![RecordingInfo {
        path: test_path.clone(),
        host: "host".into(),
        port: 5432,
        dbname: "db".into(),
        recorded_at: Utc::now(),
        pg_version: "PostgreSQL 15.0".into(),
        file_size: 1000,
    }];

    app.handle_key(key(KeyCode::Char('d')));
    assert_eq!(app.view_mode, ViewMode::ConfirmDeleteRecording(test_path));
}

#[test]
fn recordings_delete_confirm_cancel() {
    use std::path::PathBuf;

    let mut app = make_app();
    let test_path = PathBuf::from("/tmp/test.jsonl");
    app.view_mode = ViewMode::ConfirmDeleteRecording(test_path);

    // Any key except y should cancel
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.view_mode, ViewMode::Recordings);
}
