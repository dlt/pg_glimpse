//! UI rendering snapshot tests using insta
//!
//! These tests verify that UI rendering produces consistent output by comparing
//! rendered frames against stored snapshots. Changes to UI appearance will fail
//! tests until the snapshots are reviewed and updated.

use chrono::{TimeZone, Utc};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use crate::app::{App, BottomPanel, SortColumn, ViewMode};
use crate::config::AppConfig;
use crate::db::models::*;

// ─────────────────────────────────────────────────────────────────────────────
// Test Fixtures
// ─────────────────────────────────────────────────────────────────────────────

fn make_server_info() -> ServerInfo {
    ServerInfo {
        version: "PostgreSQL 15.4 on x86_64-pc-linux-gnu".to_string(),
        start_time: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap(),
        max_connections: 100,
        extensions: DetectedExtensions {
            pg_stat_statements: true,
            pg_stat_statements_version: Some("1.10".to_string()),
            pg_stat_kcache: false,
            pg_wait_sampling: false,
            pg_buffercache: true,
        },
        settings: vec![],
    }
}

fn make_snapshot() -> PgSnapshot {
    PgSnapshot {
        timestamp: Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 45).unwrap(),
        active_queries: vec![
            ActiveQuery {
                pid: 12345,
                usename: Some("app_user".to_string()),
                datname: Some("production".to_string()),
                state: Some("active".to_string()),
                wait_event_type: Some("IO".to_string()),
                wait_event: Some("DataFileRead".to_string()),
                query_start: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 40).unwrap()),
                duration_secs: 5.5,
                query: Some("SELECT * FROM users WHERE id = $1".to_string()),
                backend_type: Some("client backend".to_string()),
            },
            ActiveQuery {
                pid: 12346,
                usename: Some("admin".to_string()),
                datname: Some("production".to_string()),
                state: Some("idle in transaction".to_string()),
                wait_event_type: Some("Client".to_string()),
                wait_event: Some("ClientRead".to_string()),
                query_start: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 28, 0).unwrap()),
                duration_secs: 165.0,
                query: Some("UPDATE orders SET status = 'shipped'".to_string()),
                backend_type: Some("client backend".to_string()),
            },
        ],
        wait_events: vec![
            WaitEventCount {
                wait_event_type: "IO".to_string(),
                wait_event: "DataFileRead".to_string(),
                count: 5,
            },
            WaitEventCount {
                wait_event_type: "Lock".to_string(),
                wait_event: "relation".to_string(),
                count: 3,
            },
            WaitEventCount {
                wait_event_type: "Client".to_string(),
                wait_event: "ClientRead".to_string(),
                count: 12,
            },
        ],
        blocking_info: vec![BlockingInfo {
            blocked_pid: 12347,
            blocked_user: Some("app_user".to_string()),
            blocked_query: Some("DELETE FROM orders WHERE id = 100".to_string()),
            blocked_duration_secs: 8.5,
            blocker_pid: 12346,
            blocker_user: Some("admin".to_string()),
            blocker_query: Some("UPDATE orders SET status = 'shipped'".to_string()),
            blocker_state: Some("idle in transaction".to_string()),
        }],
        buffer_cache: BufferCacheStats {
            blks_hit: 95000,
            blks_read: 5000,
            hit_ratio: 95.0,
        },
        summary: ActivitySummary {
            active_query_count: 5,
            idle_in_transaction_count: 2,
            total_backends: 25,
            lock_count: 3,
            waiting_count: 1,
            oldest_xact_secs: Some(165.0),
            autovacuum_count: 1,
        },
        table_stats: vec![
            TableStat {
                schemaname: "public".to_string(),
                relname: "orders".to_string(),
                total_size_bytes: 1_073_741_824,
                table_size_bytes: 858_993_459,
                indexes_size_bytes: 214_748_365,
                seq_scan: 150,
                seq_tup_read: 50000,
                idx_scan: 25000,
                idx_tup_fetch: 24500,
                n_live_tup: 100000,
                n_dead_tup: 5000,
                dead_ratio: 5.0,
                n_tup_ins: 1000,
                n_tup_upd: 500,
                n_tup_del: 100,
                n_tup_hot_upd: 200,
                last_vacuum: None,
                last_autovacuum: Some(Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap()),
                last_analyze: None,
                last_autoanalyze: Some(Utc.with_ymd_and_hms(2024, 1, 15, 11, 30, 0).unwrap()),
                vacuum_count: 5,
                autovacuum_count: 20,
                bloat_bytes: Some(52_428_800),
                bloat_pct: Some(6.1),
            },
            TableStat {
                schemaname: "public".to_string(),
                relname: "users".to_string(),
                total_size_bytes: 104_857_600,
                table_size_bytes: 83_886_080,
                indexes_size_bytes: 20_971_520,
                seq_scan: 5,
                seq_tup_read: 500,
                idx_scan: 50000,
                idx_tup_fetch: 49000,
                n_live_tup: 10000,
                n_dead_tup: 100,
                dead_ratio: 1.0,
                n_tup_ins: 50,
                n_tup_upd: 200,
                n_tup_del: 10,
                n_tup_hot_upd: 100,
                last_vacuum: Some(Utc.with_ymd_and_hms(2024, 1, 15, 8, 0, 0).unwrap()),
                last_autovacuum: None,
                last_analyze: Some(Utc.with_ymd_and_hms(2024, 1, 15, 8, 30, 0).unwrap()),
                last_autoanalyze: None,
                vacuum_count: 10,
                autovacuum_count: 5,
                bloat_bytes: None,
                bloat_pct: None,
            },
        ],
        replication: vec![ReplicationInfo {
            pid: 23456,
            usesysid: Some(16384),
            usename: Some("replicator".to_string()),
            application_name: Some("replica1".to_string()),
            client_addr: Some("10.0.1.50".to_string()),
            client_hostname: None,
            client_port: Some(54321),
            backend_start: Some(Utc.with_ymd_and_hms(2024, 1, 14, 0, 0, 0).unwrap()),
            backend_xmin: None,
            state: Some("streaming".to_string()),
            sent_lsn: Some("0/5000000".to_string()),
            write_lsn: Some("0/4FFFFFF".to_string()),
            flush_lsn: Some("0/4FFFFFE".to_string()),
            replay_lsn: Some("0/4FFFFFD".to_string()),
            write_lag_secs: Some(0.001),
            flush_lag_secs: Some(0.002),
            replay_lag_secs: Some(0.005),
            sync_priority: Some(1),
            sync_state: Some("async".to_string()),
            reply_time: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 44).unwrap()),
        }],
        replication_slots: vec![ReplicationSlot {
            slot_name: "replica1_slot".to_string(),
            slot_type: "physical".to_string(),
            database: None,
            active: true,
            restart_lsn: Some("0/4000000".to_string()),
            confirmed_flush_lsn: None,
            wal_retained_bytes: Some(16_777_216),
            temporary: false,
            spill_txns: None,
            spill_count: None,
            spill_bytes: None,
        }],
        subscriptions: vec![],
        vacuum_progress: vec![VacuumProgress {
            pid: 34567,
            datname: Some("production".to_string()),
            table_name: "public.large_table".to_string(),
            phase: "scanning heap".to_string(),
            heap_blks_total: 100000,
            heap_blks_vacuumed: 45000,
            progress_pct: 45.0,
            num_dead_tuples: 12500,
        }],
        wraparound: vec![
            WraparoundInfo {
                datname: "production".to_string(),
                xid_age: 150_000_000,
                xids_remaining: 1_997_000_000,
                pct_towards_wraparound: 7.0,
            },
            WraparoundInfo {
                datname: "template1".to_string(),
                xid_age: 50_000_000,
                xids_remaining: 2_097_000_000,
                pct_towards_wraparound: 2.3,
            },
        ],
        indexes: vec![
            IndexInfo {
                schemaname: "public".to_string(),
                table_name: "orders".to_string(),
                index_name: "orders_pkey".to_string(),
                index_size_bytes: 52_428_800,
                idx_scan: 50000,
                idx_tup_read: 50000,
                idx_tup_fetch: 49500,
                index_definition: "CREATE UNIQUE INDEX orders_pkey ON public.orders USING btree (id)".to_string(),
                bloat_bytes: None,
                bloat_pct: None,
            },
            IndexInfo {
                schemaname: "public".to_string(),
                table_name: "orders".to_string(),
                index_name: "orders_user_id_idx".to_string(),
                index_size_bytes: 26_214_400,
                idx_scan: 0,
                idx_tup_read: 0,
                idx_tup_fetch: 0,
                index_definition: "CREATE INDEX orders_user_id_idx ON public.orders USING btree (user_id)".to_string(),
                bloat_bytes: Some(5_242_880),
                bloat_pct: Some(20.0),
            },
        ],
        stat_statements: vec![StatStatement {
            queryid: 123456789,
            query: "SELECT * FROM users WHERE email = $1".to_string(),
            calls: 10000,
            total_exec_time: 5000.0,
            min_exec_time: 0.1,
            mean_exec_time: 0.5,
            max_exec_time: 25.0,
            stddev_exec_time: 2.5,
            rows: 10000,
            shared_blks_hit: 45000,
            shared_blks_read: 500,
            shared_blks_dirtied: 0,
            shared_blks_written: 0,
            local_blks_hit: 0,
            local_blks_read: 0,
            local_blks_dirtied: 0,
            local_blks_written: 0,
            temp_blks_read: 0,
            temp_blks_written: 0,
            blk_read_time: 50.0,
            blk_write_time: 0.0,
            hit_ratio: 98.9,
        }],
        stat_statements_error: None,
        extensions: DetectedExtensions {
            pg_stat_statements: true,
            pg_stat_statements_version: Some("1.10".to_string()),
            pg_stat_kcache: false,
            pg_wait_sampling: false,
            pg_buffercache: true,
        },
        db_size: 10_737_418_240,
        checkpoint_stats: Some(CheckpointStats {
            checkpoints_timed: 100,
            checkpoints_req: 5,
            checkpoint_write_time: 50000.0,
            checkpoint_sync_time: 1000.0,
            buffers_checkpoint: 10000,
            buffers_backend: 500,
        }),
        wal_stats: Some(WalStats {
            wal_records: 1_000_000,
            wal_fpi: 5000,
            wal_bytes: 536_870_912,
            wal_buffers_full: 100,
            wal_write: 50000,
            wal_sync: 45000,
            wal_write_time: 2500.0,
            wal_sync_time: 500.0,
        }),
        archiver_stats: Some(ArchiverStats {
            archived_count: 500,
            failed_count: 2,
            last_archived_wal: Some("00000001000000000000000F".to_string()),
            last_archived_time: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 25, 0).unwrap()),
            last_failed_wal: Some("00000001000000000000000E".to_string()),
            last_failed_time: Some(Utc.with_ymd_and_hms(2024, 1, 14, 10, 0, 0).unwrap()),
        }),
        bgwriter_stats: Some(BgwriterStats {
            buffers_clean: 5000,
            maxwritten_clean: 10,
            buffers_alloc: 50000,
        }),
        db_stats: Some(DatabaseStats {
            xact_commit: 100000,
            xact_rollback: 50,
            blks_read: 5000,
        }),
    }
}

fn make_empty_snapshot() -> PgSnapshot {
    PgSnapshot {
        timestamp: Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 45).unwrap(),
        active_queries: vec![],
        wait_events: vec![],
        blocking_info: vec![],
        buffer_cache: BufferCacheStats {
            blks_hit: 0,
            blks_read: 0,
            hit_ratio: 0.0,
        },
        summary: ActivitySummary {
            active_query_count: 0,
            idle_in_transaction_count: 0,
            total_backends: 0,
            lock_count: 0,
            waiting_count: 0,
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
        db_size: 0,
        checkpoint_stats: None,
        wal_stats: None,
        archiver_stats: None,
        bgwriter_stats: None,
        db_stats: None,
    }
}

fn make_app(snapshot: Option<PgSnapshot>) -> App {
    let mut app = App::new(
        "localhost".to_string(),
        5432,
        "production".to_string(),
        "postgres".to_string(),
        1,
        60,
        AppConfig::default(),
        make_server_info(),
    );
    app.snapshot = snapshot;
    // Populate history buffers for graphs
    for i in 0..30 {
        app.connection_history.push((20 + i % 10) as u64);
        app.hit_ratio_history.push(900 + (i % 50) as u64);
        app.avg_query_time_history.push((100 + i * 10) as u64);
        app.active_query_history.push((3 + i % 5) as u64);
        app.lock_count_history.push((i % 3) as u64);
        app.tps_history.push((1000 + i * 50) as u64);
        app.wal_rate_history.push((1024 * 1024 + i * 10000) as u64);
        app.blks_read_history.push((500 + i * 10) as u64);
    }
    app.current_tps = Some(1500.0);
    app.current_wal_rate = Some(1.5 * 1024.0 * 1024.0);
    app.current_blks_read_rate = Some(650.0);
    app
}

/// Convert a terminal buffer to a string representation for snapshot testing
/// Replaces dynamic timestamps with placeholders for reproducible snapshots
fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut result = String::new();

    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            result.push_str(cell.symbol());
        }
        result.push('\n');
    }

    // Replace timestamps (HH:MM:SS format) with placeholder for reproducibility
    // This simple approach replaces any sequence that looks like a time
    redact_timestamps(&result)
}

/// Replace timestamps and relative times with placeholders for reproducible snapshots
fn redact_timestamps(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Look for pattern: DD:DD:DD where D is digit (clock time)
        if i + 7 < chars.len()
            && chars[i].is_ascii_digit()
            && chars[i + 1].is_ascii_digit()
            && chars[i + 2] == ':'
            && chars[i + 3].is_ascii_digit()
            && chars[i + 4].is_ascii_digit()
            && chars[i + 5] == ':'
            && chars[i + 6].is_ascii_digit()
            && chars[i + 7].is_ascii_digit()
        {
            result.push_str("XX:XX:XX");
            i += 8;
        }
        // Look for relative time patterns like "18152h 24m ago" or "5m ago"
        else if is_relative_time_start(&chars, i) {
            // Find the end of the relative time expression (ends with " ago")
            if let Some((replacement, skip)) = extract_relative_time(&chars, i) {
                result.push_str(&replacement);
                i += skip;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Check if position i starts a relative time pattern (digits followed by h/m/s)
fn is_relative_time_start(chars: &[char], i: usize) -> bool {
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return false;
    }
    // Look ahead for digits followed by time unit
    let mut j = i;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    if j < chars.len() && (chars[j] == 'h' || chars[j] == 'm' || chars[j] == 's' || chars[j] == 'd') {
        // Check if this eventually has " ago"
        let remaining: String = chars[j..].iter().take(30).collect();
        return remaining.contains(" ago");
    }
    false
}

/// Extract a relative time expression and return (placeholder, chars_to_skip)
fn extract_relative_time(chars: &[char], start: usize) -> Option<(String, usize)> {
    // Find " ago" in the next 30 characters
    let mut end = start;
    let mut found_ago = false;

    while end < chars.len() && end - start < 30 {
        if end + 3 < chars.len()
            && chars[end] == ' '
            && chars[end + 1] == 'a'
            && chars[end + 2] == 'g'
            && chars[end + 3] == 'o'
        {
            found_ago = true;
            end += 4; // Include " ago"
            break;
        }
        end += 1;
    }

    if found_ago {
        Some(("XXh XXm ago".to_string(), end - start))
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Header Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn header_live_mode() {
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::header::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn header_live_paused() {
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.paused = true;

    terminal.draw(|frame| {
        super::header::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn header_live_with_error() {
    let backend = TestBackend::new(120, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.last_error = Some("connection refused".to_string());

    terminal.draw(|frame| {
        super::header::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn header_live_with_ssl() {
    let backend = TestBackend::new(110, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.ssl_mode_label = Some("TLS 1.3".to_string());

    terminal.draw(|frame| {
        super::header::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn header_replay_mode() {
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.replay_mode = true;
    app.replay_filename = Some("recording-2024-01-15.jsonl".to_string());
    app.replay_position = 42;
    app.replay_total = 100;
    app.replay_speed = 2.0;
    app.replay_playing = true;

    terminal.draw(|frame| {
        super::header::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn header_replay_paused() {
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.replay_mode = true;
    app.replay_filename = Some("recording-2024-01-15.jsonl".to_string());
    app.replay_position = 42;
    app.replay_total = 100;
    app.replay_speed = 0.5;
    app.replay_playing = false;

    terminal.draw(|frame| {
        super::header::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Footer Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn footer_live_queries_panel() {
    let backend = TestBackend::new(120, 2);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::footer::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn footer_live_blocking_panel() {
    let backend = TestBackend::new(120, 2);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Blocking;

    terminal.draw(|frame| {
        super::footer::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn footer_live_table_stats_panel() {
    let backend = TestBackend::new(120, 2);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::TableStats;

    terminal.draw(|frame| {
        super::footer::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn footer_filter_mode() {
    let backend = TestBackend::new(100, 2);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.view_mode = ViewMode::Filter;
    app.filter_text = "SELECT".to_string();

    terminal.draw(|frame| {
        super::footer::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn footer_replay_mode() {
    let backend = TestBackend::new(120, 2);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.replay_mode = true;

    terminal.draw(|frame| {
        super::footer::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Blocking
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_blocking_with_data() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_blocking(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_blocking_empty() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_blocking(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_blocking_no_snapshot() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(None);

    terminal.draw(|frame| {
        super::panels::render_blocking(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Wait Events
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_wait_events_with_data() {
    let backend = TestBackend::new(80, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_wait_events(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_wait_events_empty() {
    let backend = TestBackend::new(80, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_wait_events(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Table Stats
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_table_stats_with_data() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_table_stats(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_table_stats_empty() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_table_stats(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Replication
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_replication_with_data() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_replication(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_replication_empty() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_replication(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Vacuum Progress
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_vacuum_progress_with_data() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_vacuum_progress(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_vacuum_progress_empty() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_vacuum_progress(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Wraparound
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_wraparound_with_data() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_wraparound(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_wraparound_empty() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_wraparound(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Indexes
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_indexes_with_data() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_indexes(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_indexes_empty() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_indexes(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Statements (pg_stat_statements)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_statements_with_data() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_statements(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_statements_empty() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_statements(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - WAL I/O
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_wal_io_with_data() {
    let backend = TestBackend::new(100, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_wal_io(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_wal_io_empty() {
    let backend = TestBackend::new(100, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_wal_io(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Overlay Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn overlay_help() {
    let backend = TestBackend::new(90, 35);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.view_mode = ViewMode::Help;

    terminal.draw(|frame| {
        super::overlay::render_help(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_config() {
    let backend = TestBackend::new(70, 25);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.view_mode = ViewMode::Config;

    terminal.draw(|frame| {
        super::overlay::render_config(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_confirm_cancel() {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        super::overlay::render_confirm_cancel(frame, 12345, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_confirm_kill() {
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        super::overlay::render_confirm_kill(frame, 12345, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Stats Panel Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn stats_panel_with_data() {
    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::stats_panel::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn stats_panel_empty() {
    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::stats_panel::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn stats_panel_no_snapshot() {
    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(None);

    terminal.draw(|frame| {
        super::stats_panel::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Overlay Inspect Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn overlay_query_inspect() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.view_mode = ViewMode::Inspect;
    app.query_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_no_selection() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));
    app.view_mode = ViewMode::Inspect;

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_index_inspect() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Indexes;
    app.view_mode = ViewMode::Inspect;
    app.index_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_index_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_index_inspect_unused() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Indexes;
    app.view_mode = ViewMode::Inspect;
    // Select the second index which has 0 scans (unused)
    app.index_table_state.select(Some(1));

    terminal.draw(|frame| {
        super::overlay::render_index_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_statement_inspect() {
    let backend = TestBackend::new(110, 50);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Statements;
    app.view_mode = ViewMode::Inspect;
    app.stmt_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_statement_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_table_inspect() {
    let backend = TestBackend::new(110, 55);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::TableStats;
    app.view_mode = ViewMode::Inspect;
    app.table_stat_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_table_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_replication_inspect() {
    let backend = TestBackend::new(100, 45);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Replication;
    app.view_mode = ViewMode::Inspect;
    app.replication_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_replication_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_blocking_inspect() {
    let backend = TestBackend::new(110, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Blocking;
    app.view_mode = ViewMode::Inspect;
    app.blocking_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_blocking_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_vacuum_inspect() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::VacuumProgress;
    app.view_mode = ViewMode::Inspect;
    app.vacuum_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_vacuum_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_wraparound_inspect() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Wraparound;
    app.view_mode = ViewMode::Inspect;
    app.wraparound_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_wraparound_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_wraparound_inspect_warning() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    // Create snapshot with high wraparound percentage
    let mut snapshot = make_snapshot();
    snapshot.wraparound = vec![WraparoundInfo {
        datname: "critical_db".to_string(),
        xid_age: 1_500_000_000,
        xids_remaining: 647_000_000,
        pct_towards_wraparound: 70.0,
    }];

    let mut app = make_app(Some(snapshot));
    app.bottom_panel = BottomPanel::Wraparound;
    app.view_mode = ViewMode::Inspect;
    app.wraparound_table_state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_wraparound_inspect(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Overlay Choice/Batch Dialog Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn overlay_cancel_choice() {
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        super::overlay::render_cancel_choice(frame, 12345, &[12345, 12346, 12347], "SELECT", frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_kill_choice() {
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        super::overlay::render_kill_choice(frame, 12345, &[12345, 12346, 12347], "SELECT", frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_confirm_cancel_batch() {
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        super::overlay::render_confirm_cancel_batch(frame, &[12345, 12346, 12347], frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_confirm_cancel_batch_many() {
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let many_pids: Vec<i32> = (1..=15).map(|i| 12340 + i).collect();

    terminal.draw(|frame| {
        super::overlay::render_confirm_cancel_batch(frame, &many_pids, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_confirm_kill_batch() {
    let backend = TestBackend::new(80, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        super::overlay::render_confirm_kill_batch(frame, &[12345, 12346, 12347], frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Active Queries Panel Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_active_queries_with_data() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::active_queries::render(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_active_queries_empty() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::active_queries::render(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_active_queries_no_snapshot() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(None);

    terminal.draw(|frame| {
        super::active_queries::render(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_active_queries_with_filter() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.filter_text = "SELECT".to_string();
    app.filter_active = true;
    app.bottom_panel = BottomPanel::Queries;

    terminal.draw(|frame| {
        super::active_queries::render(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_active_queries_sorted_by_duration() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.sort_column = SortColumn::Duration;
    app.sort_ascending = false;

    terminal.draw(|frame| {
        super::active_queries::render(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_active_queries_sorted_ascending() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.sort_column = SortColumn::Duration;
    app.sort_ascending = true;

    terminal.draw(|frame| {
        super::active_queries::render(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Full Layout Tests (integration of header + panels + footer)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn full_layout_queries_panel() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_blocking_panel() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Blocking;

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_table_stats_panel() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::TableStats;

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_with_help_overlay() {
    let backend = TestBackend::new(140, 50);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.view_mode = ViewMode::Help;

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_with_config_overlay() {
    let backend = TestBackend::new(140, 50);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.view_mode = ViewMode::Config;

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_replay_mode() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.replay_mode = true;
    app.replay_filename = Some("recording-2024-01-15.jsonl".to_string());
    app.replay_position = 42;
    app.replay_total = 100;
    app.replay_speed = 1.0;
    app.replay_playing = true;

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_empty_data() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_no_snapshot() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(None);

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}
