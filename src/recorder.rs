use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::db::models::{PgSnapshot, ServerInfo};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
enum RecordLine {
    #[serde(rename = "header")]
    Header {
        host: String,
        port: u16,
        dbname: String,
        user: String,
        server_info: ServerInfo,
        recorded_at: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "snapshot")]
    Snapshot { data: PgSnapshot },
}

pub struct Recorder {
    writer: BufWriter<File>,
}

impl Recorder {
    pub fn new(
        host: &str,
        port: u16,
        dbname: &str,
        user: &str,
        server_info: &ServerInfo,
    ) -> Result<Self> {
        let dir = Self::recordings_dir();
        fs::create_dir_all(&dir)?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}_{}.jsonl", host, port, timestamp);
        // Sanitize filename: replace any path-unfriendly chars
        let filename = filename.replace(['/', '\\'], "_");
        let path = dir.join(filename);

        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        let header = RecordLine::Header {
            host: host.to_string(),
            port,
            dbname: dbname.to_string(),
            user: user.to_string(),
            server_info: server_info.clone(),
            recorded_at: chrono::Utc::now(),
        };
        serde_json::to_writer(&mut writer, &header)?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        Ok(Self { writer })
    }

    pub fn record(&mut self, snapshot: &PgSnapshot) -> Result<()> {
        let line = RecordLine::Snapshot {
            data: snapshot.clone(),
        };
        serde_json::to_writer(&mut self.writer, &line)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn recordings_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pg_glimpse")
            .join("recordings")
    }

    pub fn cleanup_old(max_age_secs: u64) {
        let dir = Self::recordings_dir();
        let Ok(entries) = fs::read_dir(&dir) else {
            return;
        };
        let now = SystemTime::now();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Ok(meta) = path.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() > max_age_secs {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    pub fn new_with_path(
        path: PathBuf,
        host: &str,
        port: u16,
        dbname: &str,
        user: &str,
        server_info: &ServerInfo,
    ) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        let header = RecordLine::Header {
            host: host.to_string(),
            port,
            dbname: dbname.to_string(),
            user: user.to_string(),
            server_info: server_info.clone(),
            recorded_at: chrono::Utc::now(),
        };
        serde_json::to_writer(&mut writer, &header)?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        Ok(Self { writer })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{ActivitySummary, BufferCacheStats, DetectedExtensions};
    use std::io::{BufRead, BufReader};
    use tempfile::TempDir;

    fn make_server_info() -> ServerInfo {
        ServerInfo {
            version: "PostgreSQL 14.5".into(),
            start_time: chrono::Utc::now(),
            max_connections: 100,
            extensions: DetectedExtensions::default(),
            settings: vec![],
            extensions_list: vec![],
        }
    }

    fn make_snapshot() -> PgSnapshot {
        PgSnapshot {
            timestamp: chrono::Utc::now(),
            active_queries: vec![],
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
            db_size: 1000000,
            checkpoint_stats: None,
            wal_stats: None,
            archiver_stats: None,
            bgwriter_stats: None,
            db_stats: None,
        }
    }

    #[test]
    fn new_creates_file_with_header() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.jsonl");

        let _recorder = Recorder::new_with_path(
            path.clone(),
            "localhost",
            5432,
            "testdb",
            "testuser",
            &make_server_info(),
        )
        .unwrap();

        // File should exist
        assert!(path.exists());

        // Read and verify header
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let first_line = reader.lines().next().unwrap().unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&first_line).unwrap();
        assert_eq!(parsed["type"], "header");
        assert_eq!(parsed["host"], "localhost");
        assert_eq!(parsed["port"], 5432);
        assert_eq!(parsed["dbname"], "testdb");
        assert_eq!(parsed["user"], "testuser");
    }

    #[test]
    fn record_writes_snapshot() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.jsonl");

        let mut recorder = Recorder::new_with_path(
            path.clone(),
            "localhost",
            5432,
            "testdb",
            "testuser",
            &make_server_info(),
        )
        .unwrap();

        let snapshot = make_snapshot();
        recorder.record(&snapshot).unwrap();

        // Read file and check second line
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<_> = reader.lines().collect();

        assert_eq!(lines.len(), 2);

        let snapshot_line: serde_json::Value =
            serde_json::from_str(&lines[1].as_ref().unwrap()).unwrap();
        assert_eq!(snapshot_line["type"], "snapshot");
        assert!(snapshot_line["data"].is_object());
    }

    #[test]
    fn record_multiple_snapshots() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.jsonl");

        let mut recorder = Recorder::new_with_path(
            path.clone(),
            "localhost",
            5432,
            "testdb",
            "testuser",
            &make_server_info(),
        )
        .unwrap();

        // Record 5 snapshots
        for _ in 0..5 {
            recorder.record(&make_snapshot()).unwrap();
        }

        // Should have 1 header + 5 snapshots = 6 lines
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<_> = reader.lines().collect();
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn recorded_data_can_be_deserialized() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.jsonl");

        let server_info = make_server_info();
        let mut recorder = Recorder::new_with_path(
            path.clone(),
            "myhost",
            5433,
            "mydb",
            "myuser",
            &server_info,
        )
        .unwrap();

        let snapshot = make_snapshot();
        recorder.record(&snapshot).unwrap();

        // Read and deserialize each line
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);

        for (i, line) in reader.lines().enumerate() {
            let line = line.unwrap();
            let record: RecordLine = serde_json::from_str(&line).unwrap();
            match (i, record) {
                (0, RecordLine::Header { host, port, dbname, user, .. }) => {
                    assert_eq!(host, "myhost");
                    assert_eq!(port, 5433);
                    assert_eq!(dbname, "mydb");
                    assert_eq!(user, "myuser");
                }
                (1, RecordLine::Snapshot { data }) => {
                    assert_eq!(data.summary.total_backends, 10);
                }
                _ => panic!("Unexpected line index or type"),
            }
        }
    }

    #[test]
    fn cleanup_removes_old_files() {
        let tmp = TempDir::new().unwrap();
        let old_file = tmp.path().join("old.jsonl");
        let new_file = tmp.path().join("new.jsonl");

        // Create files
        File::create(&old_file).unwrap();
        File::create(&new_file).unwrap();

        // Set old file's mtime to 2 hours ago
        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
        filetime::set_file_mtime(&old_file, filetime::FileTime::from_system_time(old_time))
            .unwrap();

        // Run cleanup with 1 hour max age
        // We need to temporarily override the recordings_dir, so we'll test the logic directly
        let now = SystemTime::now();
        for entry in fs::read_dir(tmp.path()).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Ok(meta) = path.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() > 3600 {
                            fs::remove_file(&path).unwrap();
                        }
                    }
                }
            }
        }

        // Old file should be gone, new file should remain
        assert!(!old_file.exists());
        assert!(new_file.exists());
    }

    #[test]
    fn cleanup_ignores_non_jsonl_files() {
        let tmp = TempDir::new().unwrap();
        let txt_file = tmp.path().join("old.txt");
        let jsonl_file = tmp.path().join("old.jsonl");

        File::create(&txt_file).unwrap();
        File::create(&jsonl_file).unwrap();

        // Set both files to be old
        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
        filetime::set_file_mtime(&txt_file, filetime::FileTime::from_system_time(old_time))
            .unwrap();
        filetime::set_file_mtime(&jsonl_file, filetime::FileTime::from_system_time(old_time))
            .unwrap();

        // Simulate cleanup logic
        let now = SystemTime::now();
        for entry in fs::read_dir(tmp.path()).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Ok(meta) = path.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() > 3600 {
                            fs::remove_file(&path).unwrap();
                        }
                    }
                }
            }
        }

        // .txt file should remain (not cleaned up), .jsonl should be gone
        assert!(txt_file.exists());
        assert!(!jsonl_file.exists());
    }

    #[test]
    fn recordings_dir_returns_path() {
        let dir = Recorder::recordings_dir();
        assert!(dir.to_string_lossy().contains("pg_glimpse"));
        assert!(dir.to_string_lossy().contains("recordings"));
    }

    #[test]
    fn filename_sanitization() {
        // Test that slashes in hostname are replaced
        let filename = format!("{}_{}", "host/with/slashes", 5432);
        let sanitized = filename.replace(['/', '\\'], "_");
        assert_eq!(sanitized, "host_with_slashes_5432");
        assert!(!sanitized.contains('/'));
        assert!(!sanitized.contains('\\'));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Recording/Replay roundtrip test
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn roundtrip_record_and_replay() {
        use crate::db::models::{
            ActiveQuery, BlockingInfo, CheckpointStats, DatabaseStats, IndexInfo, ReplicationInfo,
            ReplicationSlot, StatStatement, Subscription, TableStat, VacuumProgress,
            WaitEventCount, WraparoundInfo,
        };
        use crate::replay::ReplaySession;

        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("roundtrip.jsonl");

        let server_info = ServerInfo {
            version: "PostgreSQL 15.3 on x86_64-pc-linux-gnu".to_string(),
            start_time: chrono::Utc::now(),
            max_connections: 200,
            extensions: DetectedExtensions {
                pg_stat_statements: true,
                pg_stat_statements_version: Some("1.10".to_string()),
                pg_stat_kcache: false,
                pg_wait_sampling: true,
                pg_buffercache: true,
            },
            settings: vec![],
            extensions_list: vec![],
        };

        // Create a complex snapshot with data in all fields
        let snapshot = PgSnapshot {
            timestamp: chrono::Utc::now(),
            active_queries: vec![
                ActiveQuery {
                    pid: 12345,
                    usename: Some("testuser".to_string()),
                    datname: Some("testdb".to_string()),
                    state: Some("active".to_string()),
                    wait_event_type: Some("IO".to_string()),
                    wait_event: Some("DataFileRead".to_string()),
                    query_start: Some(chrono::Utc::now()),
                    duration_secs: 5.5,
                    query: Some("SELECT * FROM large_table".to_string()),
                    backend_type: Some("client backend".to_string()),
                },
                ActiveQuery {
                    pid: 12346,
                    usename: Some("admin".to_string()),
                    datname: Some("postgres".to_string()),
                    state: Some("idle in transaction".to_string()),
                    wait_event_type: None,
                    wait_event: None,
                    query_start: Some(chrono::Utc::now()),
                    duration_secs: 120.0,
                    query: Some("BEGIN; UPDATE users SET x = 1".to_string()),
                    backend_type: Some("client backend".to_string()),
                },
            ],
            wait_events: vec![WaitEventCount {
                wait_event_type: "Lock".to_string(),
                wait_event: "relation".to_string(),
                count: 5,
            }],
            blocking_info: vec![BlockingInfo {
                blocked_pid: 100,
                blocked_user: Some("user1".to_string()),
                blocked_query: Some("UPDATE t SET x = 1".to_string()),
                blocked_duration_secs: 10.5,
                blocker_pid: 200,
                blocker_user: Some("user2".to_string()),
                blocker_query: Some("SELECT * FROM t FOR UPDATE".to_string()),
                blocker_state: Some("idle in transaction".to_string()),
            }],
            buffer_cache: BufferCacheStats {
                blks_hit: 99000,
                blks_read: 1000,
                hit_ratio: 0.99,
            },
            summary: ActivitySummary {
                total_backends: 25,
                active_query_count: 5,
                idle_in_transaction_count: 2,
                waiting_count: 1,
                lock_count: 10,
                oldest_xact_secs: Some(300.5),
                autovacuum_count: 1,
            },
            table_stats: vec![TableStat {
                schemaname: "public".to_string(),
                relname: "users".to_string(),
                total_size_bytes: 10000000,
                table_size_bytes: 8000000,
                indexes_size_bytes: 2000000,
                seq_scan: 100,
                seq_tup_read: 50000,
                idx_scan: 5000,
                idx_tup_fetch: 45000,
                n_live_tup: 10000,
                n_dead_tup: 500,
                dead_ratio: 5.0,
                n_tup_ins: 1000,
                n_tup_upd: 500,
                n_tup_del: 100,
                n_tup_hot_upd: 200,
                last_vacuum: None,
                last_autovacuum: Some(chrono::Utc::now()),
                last_analyze: None,
                last_autoanalyze: Some(chrono::Utc::now()),
                vacuum_count: 5,
                autovacuum_count: 10,
                bloat_bytes: Some(500000),
                bloat_pct: Some(6.25),
            }],
            replication: vec![ReplicationInfo {
                pid: 9999,
                usesysid: Some(10),
                usename: Some("replicator".to_string()),
                application_name: Some("replica1".to_string()),
                client_addr: Some("192.168.1.100".to_string()),
                client_hostname: None,
                client_port: Some(54321),
                backend_start: Some(chrono::Utc::now()),
                backend_xmin: None,
                state: Some("streaming".to_string()),
                sent_lsn: Some("0/1234567".to_string()),
                write_lsn: Some("0/1234560".to_string()),
                flush_lsn: Some("0/1234550".to_string()),
                replay_lsn: Some("0/1234540".to_string()),
                write_lag_secs: Some(0.001),
                flush_lag_secs: Some(0.002),
                replay_lag_secs: Some(0.005),
                sync_priority: Some(1),
                sync_state: Some("async".to_string()),
                reply_time: Some(chrono::Utc::now()),
            }],
            replication_slots: vec![ReplicationSlot {
                slot_name: "my_slot".to_string(),
                slot_type: "logical".to_string(),
                database: Some("testdb".to_string()),
                active: true,
                restart_lsn: Some("0/1234000".to_string()),
                confirmed_flush_lsn: Some("0/1234500".to_string()),
                wal_retained_bytes: Some(1048576),
                temporary: false,
                spill_txns: Some(0),
                spill_count: Some(0),
                spill_bytes: Some(0),
            }],
            subscriptions: vec![Subscription {
                subname: "my_sub".to_string(),
                pid: Some(8888),
                relcount: 5,
                received_lsn: Some("0/5555555".to_string()),
                last_msg_send_time: Some(chrono::Utc::now()),
                last_msg_receipt_time: Some(chrono::Utc::now()),
                latest_end_lsn: Some("0/5555550".to_string()),
                latest_end_time: Some(chrono::Utc::now()),
                enabled: true,
            }],
            vacuum_progress: vec![VacuumProgress {
                pid: 7777,
                datname: Some("testdb".to_string()),
                table_name: "public.large_table".to_string(),
                phase: "scanning heap".to_string(),
                heap_blks_total: 10000,
                heap_blks_vacuumed: 2500,
                progress_pct: 25.0,
                num_dead_tuples: 5000,
            }],
            wraparound: vec![WraparoundInfo {
                datname: "testdb".to_string(),
                xid_age: 500000000,
                xids_remaining: 1647483648,
                pct_towards_wraparound: 23.28,
            }],
            indexes: vec![IndexInfo {
                schemaname: "public".to_string(),
                table_name: "users".to_string(),
                index_name: "users_pkey".to_string(),
                index_size_bytes: 500000,
                idx_scan: 10000,
                idx_tup_read: 50000,
                idx_tup_fetch: 48000,
                index_definition: "CREATE UNIQUE INDEX users_pkey ON public.users USING btree (id)"
                    .to_string(),
                bloat_bytes: Some(25000),
                bloat_pct: Some(5.0),
            }],
            stat_statements: vec![StatStatement {
                queryid: 123456789,
                query: "SELECT * FROM users WHERE id = $1".to_string(),
                calls: 10000,
                total_exec_time: 5000.0,
                min_exec_time: 0.1,
                mean_exec_time: 0.5,
                max_exec_time: 10.0,
                stddev_exec_time: 0.25,
                rows: 10000,
                shared_blks_hit: 50000,
                shared_blks_read: 500,
                shared_blks_dirtied: 100,
                shared_blks_written: 50,
                local_blks_hit: 0,
                local_blks_read: 0,
                local_blks_dirtied: 0,
                local_blks_written: 0,
                temp_blks_read: 0,
                temp_blks_written: 0,
                blk_read_time: 10.5,
                blk_write_time: 5.2,
                hit_ratio: 0.99,
            }],
            stat_statements_error: None,
            extensions: DetectedExtensions {
                pg_stat_statements: true,
                pg_stat_statements_version: Some("1.10".to_string()),
                pg_stat_kcache: false,
                pg_wait_sampling: true,
                pg_buffercache: true,
            },
            db_size: 5000000000,
            checkpoint_stats: Some(CheckpointStats {
                checkpoints_timed: 100,
                checkpoints_req: 5,
                checkpoint_write_time: 50000.0,
                checkpoint_sync_time: 1000.0,
                buffers_checkpoint: 10000,
                buffers_backend: 500,
            }),
            wal_stats: Some(crate::db::models::WalStats {
                wal_records: 1000000,
                wal_fpi: 5000,
                wal_bytes: 1073741824,
                wal_buffers_full: 10,
                wal_write: 50000,
                wal_sync: 50000,
                wal_write_time: 100.5,
                wal_sync_time: 50.2,
            }),
            archiver_stats: Some(crate::db::models::ArchiverStats {
                archived_count: 1000,
                failed_count: 2,
                last_archived_wal: Some("000000010000000000000064".to_string()),
                last_archived_time: Some(chrono::Utc::now()),
                last_failed_wal: Some("000000010000000000000050".to_string()),
                last_failed_time: Some(chrono::Utc::now()),
            }),
            bgwriter_stats: Some(crate::db::models::BgwriterStats {
                buffers_clean: 5000,
                maxwritten_clean: 10,
                buffers_alloc: 100000,
            }),
            db_stats: Some(DatabaseStats {
                xact_commit: 500000,
                xact_rollback: 100,
                blks_read: 10000,
            }),
        };

        // Record the session
        let mut recorder = Recorder::new_with_path(
            path.clone(),
            "testhost",
            5432,
            "testdb",
            "testuser",
            &server_info,
        )
        .unwrap();

        recorder.record(&snapshot).unwrap();
        drop(recorder); // Ensure file is flushed

        // Load via ReplaySession
        let session = ReplaySession::load(&path).unwrap();

        // Verify header data
        assert_eq!(session.host, "testhost");
        assert_eq!(session.port, 5432);
        assert_eq!(session.dbname, "testdb");
        assert_eq!(session.user, "testuser");
        assert!(session.server_info.version.contains("15.3"));
        assert_eq!(session.server_info.max_connections, 200);
        assert!(session.server_info.extensions.pg_stat_statements);
        assert!(session.server_info.extensions.pg_buffercache);

        // Verify snapshot data
        assert_eq!(session.len(), 1);
        let loaded = session.current().unwrap();

        // Verify active queries
        assert_eq!(loaded.active_queries.len(), 2);
        assert_eq!(loaded.active_queries[0].pid, 12345);
        assert_eq!(
            loaded.active_queries[0].usename,
            Some("testuser".to_string())
        );
        assert_eq!(
            loaded.active_queries[0].query,
            Some("SELECT * FROM large_table".to_string())
        );
        assert_eq!(loaded.active_queries[1].pid, 12346);
        assert!((loaded.active_queries[1].duration_secs - 120.0).abs() < 0.001);

        // Verify summary
        assert_eq!(loaded.summary.total_backends, 25);
        assert_eq!(loaded.summary.active_query_count, 5);
        assert_eq!(loaded.summary.idle_in_transaction_count, 2);
        assert!((loaded.summary.oldest_xact_secs.unwrap() - 300.5).abs() < 0.001);

        // Verify buffer cache
        assert_eq!(loaded.buffer_cache.blks_hit, 99000);
        assert!((loaded.buffer_cache.hit_ratio - 0.99).abs() < 0.001);

        // Verify table stats with bloat
        assert_eq!(loaded.table_stats.len(), 1);
        assert_eq!(loaded.table_stats[0].schemaname, "public");
        assert_eq!(loaded.table_stats[0].relname, "users");
        assert_eq!(loaded.table_stats[0].bloat_bytes, Some(500000));
        assert!((loaded.table_stats[0].bloat_pct.unwrap() - 6.25).abs() < 0.001);

        // Verify indexes with bloat
        assert_eq!(loaded.indexes.len(), 1);
        assert_eq!(loaded.indexes[0].index_name, "users_pkey");
        assert_eq!(loaded.indexes[0].bloat_bytes, Some(25000));

        // Verify replication
        assert_eq!(loaded.replication.len(), 1);
        assert_eq!(
            loaded.replication[0].application_name,
            Some("replica1".to_string())
        );
        assert!(loaded.replication[0].replay_lag_secs.is_some());

        // Verify replication slots
        assert_eq!(loaded.replication_slots.len(), 1);
        assert_eq!(loaded.replication_slots[0].slot_name, "my_slot");
        assert!(loaded.replication_slots[0].active);

        // Verify subscriptions
        assert_eq!(loaded.subscriptions.len(), 1);
        assert_eq!(loaded.subscriptions[0].subname, "my_sub");

        // Verify vacuum progress
        assert_eq!(loaded.vacuum_progress.len(), 1);
        assert!((loaded.vacuum_progress[0].progress_pct - 25.0).abs() < 0.001);

        // Verify wraparound
        assert_eq!(loaded.wraparound.len(), 1);
        assert_eq!(loaded.wraparound[0].xid_age, 500000000);

        // Verify stat_statements
        assert_eq!(loaded.stat_statements.len(), 1);
        assert_eq!(loaded.stat_statements[0].calls, 10000);
        assert!((loaded.stat_statements[0].hit_ratio - 0.99).abs() < 0.001);

        // Verify other stats
        assert!(loaded.checkpoint_stats.is_some());
        assert_eq!(loaded.checkpoint_stats.as_ref().unwrap().checkpoints_timed, 100);

        assert!(loaded.wal_stats.is_some());
        assert_eq!(loaded.wal_stats.as_ref().unwrap().wal_records, 1000000);

        assert!(loaded.archiver_stats.is_some());
        assert_eq!(loaded.archiver_stats.as_ref().unwrap().archived_count, 1000);

        assert!(loaded.bgwriter_stats.is_some());
        assert_eq!(loaded.bgwriter_stats.as_ref().unwrap().buffers_clean, 5000);

        assert!(loaded.db_stats.is_some());
        assert_eq!(loaded.db_stats.as_ref().unwrap().xact_commit, 500000);

        assert_eq!(loaded.db_size, 5000000000);
        assert!(loaded.extensions.pg_stat_statements);
    }

    #[test]
    fn roundtrip_multiple_snapshots() {
        use crate::replay::ReplaySession;

        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("multi.jsonl");

        let server_info = make_server_info();

        let mut recorder =
            Recorder::new_with_path(path.clone(), "host", 5432, "db", "user", &server_info)
                .unwrap();

        // Record multiple snapshots with different values
        for i in 0..5 {
            let mut snap = make_snapshot();
            snap.summary.total_backends = (10 + i * 5) as i64;
            snap.buffer_cache.hit_ratio = 0.90 + (i as f64 * 0.02);
            recorder.record(&snap).unwrap();
        }
        drop(recorder);

        let session = ReplaySession::load(&path).unwrap();
        assert_eq!(session.len(), 5);

        // Verify each snapshot has correct values
        for (i, snap) in session.snapshots.iter().enumerate() {
            assert_eq!(snap.summary.total_backends, (10 + i * 5) as i64);
            let expected_ratio = 0.90 + (i as f64 * 0.02);
            assert!((snap.buffer_cache.hit_ratio - expected_ratio).abs() < 0.001);
        }
    }

    #[test]
    fn roundtrip_preserves_timestamps() {
        use crate::replay::ReplaySession;

        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("timestamps.jsonl");

        let server_info = make_server_info();
        let mut recorder =
            Recorder::new_with_path(path.clone(), "host", 5432, "db", "user", &server_info)
                .unwrap();

        let mut snap = make_snapshot();
        let original_timestamp = chrono::Utc::now();
        snap.timestamp = original_timestamp;
        recorder.record(&snap).unwrap();
        drop(recorder);

        let session = ReplaySession::load(&path).unwrap();
        let loaded = session.current().unwrap();

        // Timestamps should match (within microsecond precision due to serialization)
        let diff = (loaded.timestamp - original_timestamp).num_microseconds().unwrap_or(0).abs();
        assert!(diff < 1000); // Allow 1ms tolerance for serialization rounding
    }
}
