//! UI rendering snapshot tests using insta
//!
//! These tests verify that UI rendering produces consistent output by comparing
//! rendered frames against stored snapshots. Changes to UI appearance will fail
//! tests until the snapshots are reviewed and updated.

use chrono::{Duration, TimeZone, Utc};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use std::path::PathBuf;

use crate::app::{App, BottomPanel, InspectTarget, SortColumn, ViewMode};
use crate::config::AppConfig;
use crate::db::models::*;
use crate::recorder::RecordingInfo;

// ─────────────────────────────────────────────────────────────────────────────
// Test Fixtures
// ─────────────────────────────────────────────────────────────────────────────

fn make_server_info() -> ServerInfo {
    ServerInfo {
        version: "PostgreSQL 15.4 on x86_64-pc-linux-gnu".to_string(),
        // Use relative time to get stable "756d 10h" uptime (9 chars = "XXXd XXh")
        start_time: Utc::now() - Duration::days(756) - Duration::hours(10),
        max_connections: 100,
        extensions: DetectedExtensions {
            pg_stat_statements: true,
            pg_stat_statements_version: Some("1.10".to_string()),
            pg_stat_kcache: false,
            pg_wait_sampling: false,
            pg_buffercache: true,
            pgstattuple: false,
            pgstattuple_version: None,
        },
        settings: vec![],
        extensions_list: vec![],
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
                n_live_tup: 100_000,
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
                bloat_source: None,
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
                bloat_source: None,
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
            heap_blks_total: 100_000,
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
                bloat_source: None,
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
                bloat_source: None,
            },
        ],
        stat_statements: vec![StatStatement {
            queryid: 123_456_789,
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
            pgstattuple: false,
            pgstattuple_version: None,
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
            // Use relative time to get consistent "10h 15m ago" output (11 chars = "XXh XXm ago")
            last_archived_time: Some(Utc::now() - Duration::hours(10) - Duration::minutes(15)),
            last_failed_wal: Some("00000001000000000000000E".to_string()),
            last_failed_time: Some(Utc::now() - Duration::hours(34) - Duration::minutes(25)),
        }),
        bgwriter_stats: Some(BgwriterStats {
            buffers_clean: 5000,
            maxwritten_clean: 10,
            buffers_alloc: 50000,
        }),
        db_stats: Some(DatabaseStats {
            xact_commit: 100_000,
            xact_rollback: 50,
            blks_read: 5000,
        }),
        table_schemas: vec![],
        foreign_keys: vec![],
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
        table_schemas: vec![],
        foreign_keys: vec![],
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
        app.metrics.connections.push((20 + i % 10) as u64);
        app.metrics.hit_ratio.push(900 + (i % 50) as u64);
        app.metrics.avg_query_time.push((100 + i * 10) as u64);
        app.metrics.active_queries.push((3 + i % 5) as u64);
        app.metrics.lock_count.push((i % 3) as u64);
        app.metrics.tps.push((1000 + i * 50) as u64);
        app.metrics.wal_rate.push((1024 * 1024 + i * 10000) as u64);
        app.metrics.blks_read.push((500 + i * 10) as u64);
    }
    app.metrics.current_tps = Some(1500.0);
    app.metrics.current_wal_rate = Some(1.5 * 1024.0 * 1024.0);
    app.metrics.current_blks_read_rate = Some(650.0);
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
    let result = redact_timestamps(&result);
    // Replace version numbers to avoid snapshot churn on version bumps
    let result = redact_version(&result);
    // Replace recordings directory path to avoid machine-specific paths
    redact_recordings_dir(&result)
}

/// Replace recordings directory path with placeholder for reproducible snapshots
fn redact_recordings_dir(s: &str) -> String {
    // The path may be truncated in the display, so we need to match the pattern
    // in the "Recordings Dir" line and replace it.
    // Pattern: "Recordings Dir      ◀  /path/to/..." or "[Enter]  /path/to/..."
    let home_dir = dirs::home_dir().unwrap_or_default();
    let home_str = home_dir.to_string_lossy();

    let mut result = String::with_capacity(s.len());
    for line in s.lines() {
        if line.contains("Recordings Dir") {
            // Replace any path starting from home directory in this line
            if let Some(idx) = line.find(&*home_str) {
                result.push_str(&line[..idx]);
                result.push_str("<RECORDINGS_DIR>");
                // Skip to end of path (find closing ▶ or end of meaningful content)
                let rest = &line[idx..];
                if let Some(end_idx) = rest.find(" ▶") {
                    // Find content after " ▶" and normalize whitespace before next │
                    let after_arrow = &rest[end_idx..];
                    if let Some(border_idx) = after_arrow.find('│') {
                        // Keep " ▶" then single space then │ and rest
                        result.push_str(" ▶ ");
                        result.push_str(&after_arrow[border_idx..]);
                    } else {
                        result.push_str(after_arrow);
                    }
                } else if let Some(end_idx) = rest.find("│") {
                    // Handle truncated paths that end at border
                    result.push_str(&rest[end_idx..]);
                }
            } else {
                result.push_str(line);
            }
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    // Remove trailing newline if original didn't have one
    if !s.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    result
}

/// Replace version numbers with placeholder for reproducible snapshots
fn redact_version(s: &str) -> String {
    // Replace "Version:    X.Y.Z" pattern with "Version:    X.X.X"
    let mut result = s.to_string();

    // Find "Version:" followed by spaces and a semver-like pattern
    if let Some(idx) = result.find("Version:") {
        let after_version = &result[idx + 8..];
        // Skip whitespace
        let trimmed = after_version.trim_start();
        let whitespace_len = after_version.len() - trimmed.len();

        // Find the version number (digits and dots)
        let mut version_end = 0;
        for (i, c) in trimmed.chars().enumerate() {
            if c.is_ascii_digit() || c == '.' {
                version_end = i + 1;
            } else {
                break;
            }
        }

        if version_end > 0 {
            let start = idx + 8 + whitespace_len;
            let end = start + version_end;
            result.replace_range(start..end, "X.X.X");
        }
    }

    result
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
        // Handle truncated timestamp DD:DD: at end of line (tiny terminals)
        else if i + 5 < chars.len()
            && chars[i].is_ascii_digit()
            && chars[i + 1].is_ascii_digit()
            && chars[i + 2] == ':'
            && chars[i + 3].is_ascii_digit()
            && chars[i + 4].is_ascii_digit()
            && chars[i + 5] == ':'
            && (i + 6 >= chars.len() || chars[i + 6] == '\n' || chars[i + 6] == ' ')
        {
            result.push_str("XX:XX:");
            i += 6;
        }
        // Handle severely truncated timestamp DD: at end of line (very tiny terminals)
        else if i + 2 < chars.len()
            && chars[i].is_ascii_digit()
            && chars[i + 1].is_ascii_digit()
            && chars[i + 2] == ':'
            && (i + 3 >= chars.len() || chars[i + 3] == '\n')
        {
            result.push_str("XX:");
            i += 3;
        }
        // Look for uptime pattern: "up XXXd YYh" (e.g., "up 756d 10h")
        else if is_uptime_start(&chars, i) {
            if let Some((replacement, skip)) = extract_uptime(&chars, i) {
                result.push_str(&replacement);
                i += skip;
            } else {
                result.push(chars[i]);
                i += 1;
            }
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

/// Check if position i starts an uptime pattern "up XXXd YYh"
fn is_uptime_start(chars: &[char], i: usize) -> bool {
    // Look for "up " followed by digits and "d"
    if i + 4 >= chars.len() {
        return false;
    }
    if chars[i] == 'u' && chars[i + 1] == 'p' && chars[i + 2] == ' ' {
        // Check for digits followed by 'd'
        let mut j = i + 3;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        if j > i + 3 && j < chars.len() && chars[j] == 'd' {
            return true;
        }
    }
    false
}

/// Extract uptime pattern and return (placeholder, chars_to_skip)
fn extract_uptime(chars: &[char], start: usize) -> Option<(String, usize)> {
    // Pattern: "up XXXd YYh" or "up XXXd"
    let mut end = start + 3; // Skip "up "

    // Skip digits for days
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    // Skip 'd'
    if end < chars.len() && chars[end] == 'd' {
        end += 1;
    } else {
        return None;
    }

    // Check for optional " XXh" part
    if end + 1 < chars.len() && chars[end] == ' ' {
        let hour_start = end + 1;
        let mut hour_end = hour_start;
        while hour_end < chars.len() && chars[hour_end].is_ascii_digit() {
            hour_end += 1;
        }
        if hour_end > hour_start && hour_end < chars.len() && chars[hour_end] == 'h' {
            end = hour_end + 1;
        }
    }

    Some(("up XXXd XXh".to_string(), end - start))
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
    app.feedback.last_error = Some("connection refused".to_string());

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
    app.connection.ssl_mode = Some("TLS 1.3".to_string());

    terminal.draw(|frame| {
        super::header::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn header_replay_mode() {
    use crate::app::ReplayState;
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.replay = Some(ReplayState {
        filename: "recording-2024-01-15.jsonl".to_string(),
        position: 42,
        total: 100,
        speed: 2.0,
        playing: true,
    });

    terminal.draw(|frame| {
        super::header::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn header_replay_paused() {
    use crate::app::ReplayState;
    let backend = TestBackend::new(100, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.replay = Some(ReplayState {
        filename: "recording-2024-01-15.jsonl".to_string(),
        position: 42,
        total: 100,
        speed: 0.5,
        playing: false,
    });

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
    app.filter.text = "SELECT".to_string();

    terminal.draw(|frame| {
        super::footer::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn footer_replay_mode() {
    use crate::app::ReplayState;
    let backend = TestBackend::new(120, 2);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.replay = Some(ReplayState::new("test.jsonl".to_string(), 10));

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
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(12345));
    app.panels.queries.state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 12345);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_no_selection() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(99999));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 99999);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_index_inspect() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Indexes;
    app.view_mode = ViewMode::Inspect(InspectTarget::Index("public.orders_pkey".to_string()));
    app.panels.indexes.state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_index_inspect(frame, &app, frame.area(), "public.orders_pkey");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_index_inspect_unused() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Indexes;
    app.view_mode = ViewMode::Inspect(InspectTarget::Index("public.orders_status_idx".to_string()));
    // Select the second index which has 0 scans (unused)
    app.panels.indexes.state.select(Some(1));

    terminal.draw(|frame| {
        super::overlay::render_index_inspect(frame, &app, frame.area(), "public.orders_status_idx");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_statement_inspect() {
    let backend = TestBackend::new(110, 50);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Statements;
    app.view_mode = ViewMode::Inspect(InspectTarget::Statement(123456789));
    app.panels.statements.state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_statement_inspect(frame, &app, frame.area(), 123456789);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_table_inspect() {
    let backend = TestBackend::new(110, 55);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::TableStats;
    app.view_mode = ViewMode::Inspect(InspectTarget::Table("public.orders".to_string()));
    app.panels.table_stats.state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_table_inspect(frame, &app, frame.area(), "public.orders");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_replication_inspect() {
    let backend = TestBackend::new(100, 45);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Replication;
    app.view_mode = ViewMode::Inspect(InspectTarget::Replication(23456));
    app.panels.replication.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_replication_inspect(frame, &app, frame.area(), 23456);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_blocking_inspect() {
    let backend = TestBackend::new(110, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Blocking;
    app.view_mode = ViewMode::Inspect(InspectTarget::Blocking(12347));
    app.panels.blocking.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_blocking_inspect(frame, &app, frame.area(), 12347);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_vacuum_inspect() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::VacuumProgress;
    app.view_mode = ViewMode::Inspect(InspectTarget::Vacuum(34567));
    app.panels.vacuum.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_vacuum_inspect(frame, &app, frame.area(), 34567);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_wraparound_inspect() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.bottom_panel = BottomPanel::Wraparound;
    app.view_mode = ViewMode::Inspect(InspectTarget::Wraparound("production".to_string()));
    app.panels.wraparound.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_wraparound_inspect(frame, &app, frame.area(), "production");
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
    app.view_mode = ViewMode::Inspect(InspectTarget::Wraparound("critical_db".to_string()));
    app.panels.wraparound.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_wraparound_inspect(frame, &app, frame.area(), "critical_db");
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
    app.filter.text = "SELECT".to_string();
    app.filter.active = true;
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
    app.panels.queries.sort_column = SortColumn::Duration;
    app.panels.queries.sort_ascending = false;

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
    app.panels.queries.sort_column = SortColumn::Duration;
    app.panels.queries.sort_ascending = true;

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
    use crate::app::ReplayState;
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));
    app.replay = Some(ReplayState {
        filename: "recording-2024-01-15.jsonl".to_string(),
        position: 42,
        total: 100,
        speed: 1.0,
        playing: true,
    });

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

// ─────────────────────────────────────────────────────────────────────────────
// Edge Cases - Terminal Sizes
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn full_layout_tiny_terminal() {
    // Classic 80x24 terminal - should not panic, may truncate
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_very_small_terminal() {
    // Extremely small - should not panic
    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_wide_terminal() {
    // Very wide terminal
    let backend = TestBackend::new(200, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn full_layout_tall_terminal() {
    // Very tall terminal
    let backend = TestBackend::new(120, 80);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge Cases - Extreme Data Values
// ─────────────────────────────────────────────────────────────────────────────

/// Create a snapshot with extreme/edge case values
fn make_extreme_snapshot() -> PgSnapshot {
    PgSnapshot {
        timestamp: Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 45).unwrap(),
        active_queries: vec![
            // Very long query
            ActiveQuery {
                pid: 99_999_999,
                usename: Some("a]very_long_username_that_exceeds_normal_limits_and_should_be_truncated".to_string()),
                datname: Some("extremely_long_database_name_that_is_way_too_long_for_display".to_string()),
                state: Some("active".to_string()),
                wait_event_type: Some("LWLock".to_string()),
                wait_event: Some("WALWriteLock".to_string()),
                query_start: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 40).unwrap()),
                duration_secs: 99999.999,
                query: Some("SELECT * FROM extremely_long_table_name_here WHERE column_one = 'value' AND column_two = 'another_value' AND column_three IN (SELECT id FROM other_table WHERE status = 'active' AND created_at > NOW() - INTERVAL '30 days' ORDER BY id DESC LIMIT 1000) AND column_four LIKE '%pattern%' ORDER BY column_five DESC NULLS LAST LIMIT 100 OFFSET 50".to_string()),
                backend_type: Some("client backend".to_string()),
            },
            // Query with all None optional fields
            ActiveQuery {
                pid: 1,
                usename: None,
                datname: None,
                state: None,
                wait_event_type: None,
                wait_event: None,
                query_start: None,
                duration_secs: 0.0,
                query: None,
                backend_type: None,
            },
            // Unicode in query
            ActiveQuery {
                pid: 12345,
                usename: Some("用户".to_string()),
                datname: Some("データベース".to_string()),
                state: Some("idle in transaction".to_string()),
                wait_event_type: None,
                wait_event: None,
                query_start: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 28, 0).unwrap()),
                duration_secs: 0.001,
                query: Some("SELECT * FROM users WHERE name = '日本語テスト' AND emoji = '🎉🚀💻'".to_string()),
                backend_type: Some("client backend".to_string()),
            },
        ],
        wait_events: vec![],
        blocking_info: vec![
            // Blocking with None fields
            BlockingInfo {
                blocked_pid: 1,
                blocked_user: None,
                blocked_query: None,
                blocked_duration_secs: 0.0,
                blocker_pid: 2,
                blocker_user: None,
                blocker_query: None,
                blocker_state: None,
            },
        ],
        buffer_cache: BufferCacheStats {
            blks_hit: i64::MAX,
            blks_read: 0,
            hit_ratio: 100.0,
        },
        summary: ActivitySummary {
            active_query_count: 999_999,
            idle_in_transaction_count: 888_888,
            total_backends: 777_777,
            lock_count: 666_666,
            waiting_count: 555_555,
            oldest_xact_secs: Some(99_999_999.9),
            autovacuum_count: 444_444,
        },
        table_stats: vec![
            // Table with extreme values
            TableStat {
                schemaname: "public".to_string(),
                relname: "テーブル_with_unicode_名前".to_string(),
                total_size_bytes: i64::MAX / 2,
                table_size_bytes: i64::MAX / 4,
                indexes_size_bytes: i64::MAX / 4,
                seq_scan: i64::MAX,
                seq_tup_read: i64::MAX,
                idx_scan: 0,
                idx_tup_fetch: 0,
                n_live_tup: i64::MAX,
                n_dead_tup: i64::MAX / 2,
                dead_ratio: 99.99,
                n_tup_ins: i64::MAX,
                n_tup_upd: i64::MAX,
                n_tup_del: i64::MAX,
                n_tup_hot_upd: 0,
                last_vacuum: None,
                last_autovacuum: None,
                last_analyze: None,
                last_autoanalyze: None,
                vacuum_count: 0,
                autovacuum_count: 0,
                bloat_bytes: Some(i64::MAX),
                bloat_pct: Some(99.9),
                bloat_source: None,
            },
        ],
        replication: vec![
            // Replication with minimal data
            ReplicationInfo {
                pid: 1,
                usesysid: None,
                usename: None,
                application_name: None,
                client_addr: None,
                client_hostname: None,
                client_port: None,
                backend_start: None,
                backend_xmin: None,
                state: None,
                sent_lsn: None,
                write_lsn: None,
                flush_lsn: None,
                replay_lsn: None,
                write_lag_secs: None,
                flush_lag_secs: None,
                replay_lag_secs: None,
                sync_priority: None,
                sync_state: None,
                reply_time: None,
            },
        ],
        replication_slots: vec![],
        subscriptions: vec![],
        vacuum_progress: vec![
            // Vacuum at 0%
            VacuumProgress {
                pid: 1,
                datname: None,
                table_name: "schema.table".to_string(),
                phase: "initializing".to_string(),
                heap_blks_total: i64::MAX,
                heap_blks_vacuumed: 0,
                progress_pct: 0.0,
                num_dead_tuples: i64::MAX,
            },
        ],
        wraparound: vec![
            // Critical wraparound
            WraparoundInfo {
                datname: "critical".to_string(),
                xid_age: 2_000_000_000,
                xids_remaining: 147_000_000,
                pct_towards_wraparound: 93.2,
            },
        ],
        indexes: vec![
            // Index with zero usage
            IndexInfo {
                schemaname: "public".to_string(),
                table_name: "t".to_string(),
                index_name: "unused_idx_with_very_long_name_that_should_be_truncated_in_display".to_string(),
                index_size_bytes: 0,
                idx_scan: 0,
                idx_tup_read: 0,
                idx_tup_fetch: 0,
                index_definition: "CREATE INDEX unused_idx_with_very_long_name_that_should_be_truncated_in_display ON public.t USING btree (col1, col2, col3, col4, col5)".to_string(),
                bloat_bytes: Some(0),
                bloat_pct: Some(0.0),
                bloat_source: None,
            },
        ],
        stat_statements: vec![
            // Statement with extreme values
            StatStatement {
                queryid: i64::MAX,
                query: "SELECT".to_string(),  // Very short
                calls: i64::MAX,
                total_exec_time: f64::MAX / 2.0,
                min_exec_time: 0.0,
                mean_exec_time: f64::MAX / 4.0,
                max_exec_time: f64::MAX / 2.0,
                stddev_exec_time: f64::MAX / 4.0,
                rows: i64::MAX,
                shared_blks_hit: 0,
                shared_blks_read: i64::MAX,
                shared_blks_dirtied: i64::MAX,
                shared_blks_written: i64::MAX,
                local_blks_hit: i64::MAX,
                local_blks_read: i64::MAX,
                local_blks_dirtied: i64::MAX,
                local_blks_written: i64::MAX,
                temp_blks_read: i64::MAX,
                temp_blks_written: i64::MAX,
                blk_read_time: f64::MAX / 2.0,
                blk_write_time: f64::MAX / 2.0,
                hit_ratio: 0.0,
            },
        ],
        stat_statements_error: Some("Error: permission denied for view pg_stat_statements".to_string()),
        extensions: DetectedExtensions::default(),
        db_size: i64::MAX,
        checkpoint_stats: Some(CheckpointStats {
            checkpoints_timed: i64::MAX,
            checkpoints_req: i64::MAX,
            checkpoint_write_time: f64::MAX / 2.0,
            checkpoint_sync_time: f64::MAX / 2.0,
            buffers_checkpoint: i64::MAX,
            buffers_backend: i64::MAX,
        }),
        wal_stats: None,
        archiver_stats: None,
        bgwriter_stats: None,
        db_stats: None,
        table_schemas: vec![],
        foreign_keys: vec![],
    }
}

#[test]
fn full_layout_extreme_values() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_queries_extreme_values() {
    let backend = TestBackend::new(140, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));

    terminal.draw(|frame| {
        super::active_queries::render(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_table_stats_extreme_values() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_table_stats(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_blocking_extreme_values() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_blocking(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_wraparound_critical() {
    let backend = TestBackend::new(100, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_wraparound(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_extreme_values() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(99_999_999));
    app.panels.queries.state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 99_999_999);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_all_none_fields() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(1));
    // Select the query with all None fields
    app.panels.queries.state.select(Some(1));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 1);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_unicode() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(12345));
    // Select the Unicode query
    app.panels.queries.state.select(Some(2));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 12345);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_replication_inspect_all_none() {
    let backend = TestBackend::new(100, 45);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));
    app.bottom_panel = BottomPanel::Replication;
    app.view_mode = ViewMode::Inspect(InspectTarget::Replication(1));
    app.panels.replication.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_replication_inspect(frame, &app, frame.area(), 1);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_blocking_inspect_all_none() {
    let backend = TestBackend::new(110, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));
    app.bottom_panel = BottomPanel::Blocking;
    app.view_mode = ViewMode::Inspect(InspectTarget::Blocking(1));
    app.panels.blocking.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_blocking_inspect(frame, &app, frame.area(), 1);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_wraparound_inspect_critical() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_extreme_snapshot()));
    app.bottom_panel = BottomPanel::Wraparound;
    app.view_mode = ViewMode::Inspect(InspectTarget::Wraparound("critical".to_string()));
    app.panels.wraparound.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_wraparound_inspect(frame, &app, frame.area(), "critical");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_settings_inspect() {
    use crate::db::models::PgSetting;

    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    // Add settings to server_info
    app.server_info.settings = vec![
        PgSetting {
            name: "shared_buffers".to_string(),
            setting: "128MB".to_string(),
            unit: Some("8kB".to_string()),
            category: "Resource Usage / Memory".to_string(),
            short_desc: "Sets the number of shared memory buffers used by the server.".to_string(),
            context: "postmaster".to_string(),
            source: "configuration file".to_string(),
            pending_restart: false,
        },
        PgSetting {
            name: "work_mem".to_string(),
            setting: "4MB".to_string(),
            unit: Some("kB".to_string()),
            category: "Resource Usage / Memory".to_string(),
            short_desc: "Sets the maximum memory to be used for query workspaces.".to_string(),
            context: "user".to_string(),
            source: "default".to_string(),
            pending_restart: false,
        },
    ];

    app.bottom_panel = BottomPanel::Settings;
    app.view_mode = ViewMode::Inspect(InspectTarget::Settings("shared_buffers".to_string()));
    app.panels.settings.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_settings_inspect(frame, &app, frame.area(), "shared_buffers");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_settings_inspect_pending_restart() {
    use crate::db::models::PgSetting;

    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    // Add settings with pending restart
    app.server_info.settings = vec![
        PgSetting {
            name: "max_connections".to_string(),
            setting: "200".to_string(),
            unit: None,
            category: "Connections and Authentication".to_string(),
            short_desc: "Sets the maximum number of concurrent connections.".to_string(),
            context: "postmaster".to_string(),
            source: "configuration file".to_string(),
            pending_restart: true,
        },
    ];

    app.bottom_panel = BottomPanel::Settings;
    app.view_mode = ViewMode::Inspect(InspectTarget::Settings("max_connections".to_string()));
    app.panels.settings.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_settings_inspect(frame, &app, frame.area(), "max_connections");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_settings_inspect_sighup() {
    use crate::db::models::PgSetting;

    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    // Add sighup setting (requires reload)
    app.server_info.settings = vec![
        PgSetting {
            name: "log_min_duration_statement".to_string(),
            setting: "1000".to_string(),
            unit: Some("ms".to_string()),
            category: "Reporting and Logging / When to Log".to_string(),
            short_desc: "Sets the minimum execution time above which statements will be logged.".to_string(),
            context: "sighup".to_string(),
            source: "configuration file".to_string(),
            pending_restart: false,
        },
    ];

    app.bottom_panel = BottomPanel::Settings;
    app.view_mode = ViewMode::Inspect(InspectTarget::Settings("log_min_duration_statement".to_string()));
    app.panels.settings.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_settings_inspect(frame, &app, frame.area(), "log_min_duration_statement");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_settings_inspect_user() {
    use crate::db::models::PgSetting;

    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    // Add user setting (can SET at runtime)
    app.server_info.settings = vec![
        PgSetting {
            name: "work_mem".to_string(),
            setting: "4096".to_string(),
            unit: Some("kB".to_string()),
            category: "Resource Usage / Memory".to_string(),
            short_desc: "Sets the maximum memory to be used for query workspaces.".to_string(),
            context: "user".to_string(),
            source: "default".to_string(),
            pending_restart: false,
        },
    ];

    app.bottom_panel = BottomPanel::Settings;
    app.view_mode = ViewMode::Inspect(InspectTarget::Settings("work_mem".to_string()));
    app.panels.settings.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_settings_inspect(frame, &app, frame.area(), "work_mem");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn stats_panel_extreme_values() {
    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_extreme_snapshot()));

    terminal.draw(|frame| {
        super::stats_panel::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge Cases - Special Characters in SQL
// ─────────────────────────────────────────────────────────────────────────────

fn make_special_chars_snapshot() -> PgSnapshot {
    let mut snapshot = make_empty_snapshot();
    snapshot.active_queries = vec![
        // SQL injection attempt (should be safely displayed)
        ActiveQuery {
            pid: 1,
            usename: Some("user'; DROP TABLE--".to_string()),
            datname: Some("db".to_string()),
            state: Some("active".to_string()),
            wait_event_type: None,
            wait_event: None,
            query_start: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 40).unwrap()),
            duration_secs: 1.0,
            query: Some("SELECT * FROM users WHERE name = ''; DROP TABLE users; --'".to_string()),
            backend_type: Some("client backend".to_string()),
        },
        // Newlines and tabs in query
        ActiveQuery {
            pid: 2,
            usename: Some("user".to_string()),
            datname: Some("db".to_string()),
            state: Some("active".to_string()),
            wait_event_type: None,
            wait_event: None,
            query_start: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 40).unwrap()),
            duration_secs: 1.0,
            query: Some("SELECT\n\t*\nFROM\n\tusers\nWHERE\n\tid = 1".to_string()),
            backend_type: Some("client backend".to_string()),
        },
        // ANSI escape sequences (should not affect terminal)
        ActiveQuery {
            pid: 3,
            usename: Some("user".to_string()),
            datname: Some("db".to_string()),
            state: Some("active".to_string()),
            wait_event_type: None,
            wait_event: None,
            query_start: Some(Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 40).unwrap()),
            duration_secs: 1.0,
            query: Some("SELECT '\x1b[31mRED\x1b[0m' AS color".to_string()),
            backend_type: Some("client backend".to_string()),
        },
        // Empty string query
        ActiveQuery {
            pid: 4,
            usename: Some(String::new()),
            datname: Some(String::new()),
            state: Some(String::new()),
            wait_event_type: Some(String::new()),
            wait_event: Some(String::new()),
            query_start: None,
            duration_secs: 0.0,
            query: Some(String::new()),
            backend_type: Some(String::new()),
        },
    ];
    snapshot
}

#[test]
fn panel_queries_special_characters() {
    let backend = TestBackend::new(140, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_special_chars_snapshot()));

    terminal.draw(|frame| {
        super::active_queries::render(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_sql_injection() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_special_chars_snapshot()));
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(1));
    app.panels.queries.state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 1);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_newlines_tabs() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_special_chars_snapshot()));
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(2));
    app.panels.queries.state.select(Some(1));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 2);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_ansi_escapes() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_special_chars_snapshot()));
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(3));
    app.panels.queries.state.select(Some(2));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 3);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_query_inspect_empty_strings() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_special_chars_snapshot()));
    app.view_mode = ViewMode::Inspect(InspectTarget::Query(4));
    app.panels.queries.state.select(Some(3));

    terminal.draw(|frame| {
        super::overlay::render_inspect(frame, &app, frame.area(), 4);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge Cases - Zero and Boundary Values
// ─────────────────────────────────────────────────────────────────────────────

fn make_zero_values_snapshot() -> PgSnapshot {
    PgSnapshot {
        timestamp: Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 45).unwrap(),
        active_queries: vec![],
        wait_events: vec![],
        blocking_info: vec![],
        buffer_cache: BufferCacheStats {
            blks_hit: 0,
            blks_read: 0,
            hit_ratio: 0.0,  // 0/0 case
        },
        summary: ActivitySummary {
            active_query_count: 0,
            idle_in_transaction_count: 0,
            total_backends: 0,
            lock_count: 0,
            waiting_count: 0,
            oldest_xact_secs: Some(0.0),
            autovacuum_count: 0,
        },
        table_stats: vec![
            TableStat {
                schemaname: "public".to_string(),
                relname: "empty_table".to_string(),
                total_size_bytes: 0,
                table_size_bytes: 0,
                indexes_size_bytes: 0,
                seq_scan: 0,
                seq_tup_read: 0,
                idx_scan: 0,
                idx_tup_fetch: 0,
                n_live_tup: 0,
                n_dead_tup: 0,
                dead_ratio: 0.0,
                n_tup_ins: 0,
                n_tup_upd: 0,
                n_tup_del: 0,
                n_tup_hot_upd: 0,
                last_vacuum: None,
                last_autovacuum: None,
                last_analyze: None,
                last_autoanalyze: None,
                vacuum_count: 0,
                autovacuum_count: 0,
                bloat_bytes: Some(0),
                bloat_pct: Some(0.0),
                bloat_source: None,
            },
        ],
        replication: vec![],
        replication_slots: vec![],
        subscriptions: vec![],
        vacuum_progress: vec![],
        wraparound: vec![
            WraparoundInfo {
                datname: "db".to_string(),
                xid_age: 0,
                xids_remaining: 2_147_483_647,
                pct_towards_wraparound: 0.0,
            },
        ],
        indexes: vec![],
        stat_statements: vec![],
        stat_statements_error: None,
        extensions: DetectedExtensions::default(),
        db_size: 0,
        checkpoint_stats: Some(CheckpointStats {
            checkpoints_timed: 0,
            checkpoints_req: 0,
            checkpoint_write_time: 0.0,
            checkpoint_sync_time: 0.0,
            buffers_checkpoint: 0,
            buffers_backend: 0,
        }),
        wal_stats: Some(WalStats {
            wal_records: 0,
            wal_fpi: 0,
            wal_bytes: 0,
            wal_buffers_full: 0,
            wal_write: 0,
            wal_sync: 0,
            wal_write_time: 0.0,
            wal_sync_time: 0.0,
        }),
        archiver_stats: Some(ArchiverStats {
            archived_count: 0,
            failed_count: 0,
            last_archived_wal: None,
            last_archived_time: None,
            last_failed_wal: None,
            last_failed_time: None,
        }),
        bgwriter_stats: Some(BgwriterStats {
            buffers_clean: 0,
            maxwritten_clean: 0,
            buffers_alloc: 0,
        }),
        db_stats: Some(DatabaseStats {
            xact_commit: 0,
            xact_rollback: 0,
            blks_read: 0,
        }),
        table_schemas: vec![],
        foreign_keys: vec![],
    }
}

#[test]
fn full_layout_zero_values() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_zero_values_snapshot()));

    terminal.draw(|frame| {
        super::render(frame, &mut app);
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_wal_io_zero_values() {
    let backend = TestBackend::new(100, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_zero_values_snapshot()));

    terminal.draw(|frame| {
        super::panels::render_wal_io(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn stats_panel_zero_values() {
    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = make_app(Some(make_zero_values_snapshot()));

    terminal.draw(|frame| {
        super::stats_panel::render(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_table_inspect_zero_values() {
    let backend = TestBackend::new(110, 55);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_zero_values_snapshot()));
    app.bottom_panel = BottomPanel::TableStats;
    app.view_mode = ViewMode::Inspect(InspectTarget::Table("public.empty_table".to_string()));
    app.panels.table_stats.state.select(Some(0));

    terminal.draw(|frame| {
        super::overlay::render_table_inspect(frame, &app, frame.area(), "public.empty_table");
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Settings
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_settings_with_data() {
    let backend = TestBackend::new(140, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    // Add settings data
    app.server_info.settings = vec![
        PgSetting {
            name: "max_connections".to_string(),
            setting: "100".to_string(),
            unit: None,
            category: "Connections and Authentication".to_string(),
            short_desc: "Sets the maximum number of concurrent connections.".to_string(),
            context: "postmaster".to_string(),
            source: "configuration file".to_string(),
            pending_restart: false,
        },
        PgSetting {
            name: "shared_buffers".to_string(),
            setting: "128MB".to_string(),
            unit: Some("8kB".to_string()),
            category: "Resource Usage / Memory".to_string(),
            short_desc: "Sets the number of shared memory buffers.".to_string(),
            context: "postmaster".to_string(),
            source: "configuration file".to_string(),
            pending_restart: true,
        },
        PgSetting {
            name: "work_mem".to_string(),
            setting: "4MB".to_string(),
            unit: Some("kB".to_string()),
            category: "Resource Usage / Memory".to_string(),
            short_desc: "Sets the memory for internal sort operations.".to_string(),
            context: "user".to_string(),
            source: "default".to_string(),
            pending_restart: false,
        },
        PgSetting {
            name: "maintenance_work_mem".to_string(),
            setting: "64MB".to_string(),
            unit: Some("kB".to_string()),
            category: "Resource Usage / Memory".to_string(),
            short_desc: "Sets the memory for maintenance operations.".to_string(),
            context: "user".to_string(),
            source: "session".to_string(),
            pending_restart: false,
        },
    ];
    app.bottom_panel = BottomPanel::Settings;

    terminal.draw(|frame| {
        super::panels::render_settings(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_settings_empty() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));
    app.server_info.settings = vec![];
    app.bottom_panel = BottomPanel::Settings;

    terminal.draw(|frame| {
        super::panels::render_settings(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_settings_with_filter() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    // Add settings data
    app.server_info.settings = vec![
        PgSetting {
            name: "max_connections".to_string(),
            setting: "100".to_string(),
            unit: None,
            category: "Connections".to_string(),
            short_desc: "Maximum connections".to_string(),
            context: "postmaster".to_string(),
            source: "configuration file".to_string(),
            pending_restart: false,
        },
        PgSetting {
            name: "max_wal_senders".to_string(),
            setting: "10".to_string(),
            unit: None,
            category: "Replication".to_string(),
            short_desc: "Maximum WAL senders".to_string(),
            context: "postmaster".to_string(),
            source: "default".to_string(),
            pending_restart: false,
        },
        PgSetting {
            name: "work_mem".to_string(),
            setting: "4MB".to_string(),
            unit: Some("kB".to_string()),
            category: "Memory".to_string(),
            short_desc: "Work memory".to_string(),
            context: "user".to_string(),
            source: "default".to_string(),
            pending_restart: false,
        },
    ];
    app.bottom_panel = BottomPanel::Settings;
    app.filter.text = "max".to_string();
    app.filter.active = true;

    terminal.draw(|frame| {
        super::panels::render_settings(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel Tests - Extensions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn panel_extensions_with_data() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    // Add extensions data
    app.server_info.extensions_list = vec![
        PgExtension {
            name: "pg_stat_statements".to_string(),
            version: "1.10".to_string(),
            schema: "public".to_string(),
            relocatable: false,
            description: Some("Track execution statistics of all SQL statements".to_string()),
        },
        PgExtension {
            name: "pgcrypto".to_string(),
            version: "1.3".to_string(),
            schema: "public".to_string(),
            relocatable: true,
            description: Some("Cryptographic functions".to_string()),
        },
        PgExtension {
            name: "uuid-ossp".to_string(),
            version: "1.1".to_string(),
            schema: "extensions".to_string(),
            relocatable: true,
            description: Some("Generate universally unique identifiers (UUIDs)".to_string()),
        },
        PgExtension {
            name: "postgis".to_string(),
            version: "3.3.2".to_string(),
            schema: "public".to_string(),
            relocatable: false,
            description: Some("PostGIS geometry and geography spatial types and functions".to_string()),
        },
    ];
    app.bottom_panel = BottomPanel::Extensions;

    terminal.draw(|frame| {
        super::panels::render_extensions(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_extensions_empty() {
    let backend = TestBackend::new(140, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_empty_snapshot()));
    app.server_info.extensions_list = vec![];
    app.bottom_panel = BottomPanel::Extensions;

    terminal.draw(|frame| {
        super::panels::render_extensions(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn panel_extensions_with_filter() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    // Add extensions data
    app.server_info.extensions_list = vec![
        PgExtension {
            name: "pg_stat_statements".to_string(),
            version: "1.10".to_string(),
            schema: "public".to_string(),
            relocatable: false,
            description: Some("Track SQL statistics".to_string()),
        },
        PgExtension {
            name: "pg_trgm".to_string(),
            version: "1.6".to_string(),
            schema: "public".to_string(),
            relocatable: true,
            description: Some("Trigram matching".to_string()),
        },
        PgExtension {
            name: "uuid-ossp".to_string(),
            version: "1.1".to_string(),
            schema: "public".to_string(),
            relocatable: true,
            description: Some("UUID generation".to_string()),
        },
    ];
    app.bottom_panel = BottomPanel::Extensions;
    app.filter.text = "pg_".to_string();
    app.filter.active = true;

    terminal.draw(|frame| {
        super::panels::render_extensions(frame, &mut app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

// ─────────────────────────────────────────────────────────────────────────────
// Overlay Tests - Recordings
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn overlay_recordings_with_data() {
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    app.view_mode = ViewMode::Recordings;
    app.recordings.list = vec![
        RecordingInfo {
            path: PathBuf::from("/tmp/recording1.jsonl"),
            host: "localhost".to_string(),
            port: 5432,
            dbname: "production".to_string(),
            recorded_at: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap(),
            pg_version: "PostgreSQL 15.4".to_string(),
            file_size: 1_500_000,
        },
        RecordingInfo {
            path: PathBuf::from("/tmp/recording2.jsonl"),
            host: "db.example.com".to_string(),
            port: 5433,
            dbname: "staging".to_string(),
            recorded_at: Utc.with_ymd_and_hms(2024, 1, 14, 14, 45, 30).unwrap(),
            pg_version: "PostgreSQL 14.10".to_string(),
            file_size: 256_000,
        },
        RecordingInfo {
            path: PathBuf::from("/tmp/recording3.jsonl"),
            host: "192.168.1.100".to_string(),
            port: 5432,
            dbname: "dev".to_string(),
            recorded_at: Utc.with_ymd_and_hms(2024, 1, 13, 9, 0, 0).unwrap(),
            pg_version: "PostgreSQL 16.1".to_string(),
            file_size: 50_000,
        },
    ];
    app.recordings.selected = 1; // Select the second item

    terminal.draw(|frame| {
        super::overlay::render_recordings(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_recordings_empty() {
    let backend = TestBackend::new(100, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = make_app(Some(make_snapshot()));

    app.view_mode = ViewMode::Recordings;
    app.recordings.list = vec![];

    terminal.draw(|frame| {
        super::overlay::render_recordings(frame, &app, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}

#[test]
fn overlay_recordings_delete_confirm() {
    let backend = TestBackend::new(80, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let _app = make_app(Some(make_snapshot()));

    let path = PathBuf::from("/home/user/.local/share/pg_glimpse/recordings/2024-01-15_103000_localhost_production.jsonl");

    terminal.draw(|frame| {
        super::overlay::render_confirm_delete_recording(frame, &path, frame.area());
    }).unwrap();

    insta::assert_snapshot!(buffer_to_string(&terminal));
}
