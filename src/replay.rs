use color_eyre::{eyre::eyre, Result};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::db::models::{PgSnapshot, ServerInfo};

#[derive(Deserialize)]
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
    },
    #[serde(rename = "snapshot")]
    Snapshot { data: PgSnapshot },
}

#[derive(Debug)]
pub struct ReplaySession {
    pub server_info: ServerInfo,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub user: String,
    pub snapshots: Vec<PgSnapshot>,
    pub position: usize,
}

impl ReplaySession {
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // First line must be header
        let header_line = lines
            .next()
            .ok_or_else(|| eyre!("Recording file is empty"))??;
        let header: RecordLine = serde_json::from_str(&header_line)?;
        let (host, port, dbname, user, server_info) = match header {
            RecordLine::Header {
                host,
                port,
                dbname,
                user,
                server_info,
                ..
            } => (host, port, dbname, user, server_info),
            _ => return Err(eyre!("First line must be a header")),
        };

        // Remaining lines are snapshots
        let mut snapshots = Vec::new();
        for line in lines {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: RecordLine = serde_json::from_str(&line)?;
            if let RecordLine::Snapshot { data } = record {
                snapshots.push(data);
            }
        }

        if snapshots.is_empty() {
            return Err(eyre!("Recording contains no snapshots"));
        }

        Ok(Self {
            server_info,
            host,
            port,
            dbname,
            user,
            snapshots,
            position: 0,
        })
    }

    pub fn current(&self) -> Option<&PgSnapshot> {
        self.snapshots.get(self.position)
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn step_forward(&mut self) -> bool {
        if self.position + 1 < self.snapshots.len() {
            self.position += 1;
            true
        } else {
            false
        }
    }

    pub fn step_back(&mut self) -> bool {
        if self.position > 0 {
            self.position -= 1;
            true
        } else {
            false
        }
    }

    pub fn jump_start(&mut self) {
        self.position = 0;
    }

    pub fn jump_end(&mut self) {
        if !self.snapshots.is_empty() {
            self.position = self.snapshots.len() - 1;
        }
    }

    pub fn at_end(&self) -> bool {
        self.position + 1 >= self.snapshots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_header_json(host: &str, port: u16, dbname: &str, user: &str) -> String {
        let server_info = serde_json::json!({
            "version": "PostgreSQL 14.5",
            "start_time": "2024-01-01T00:00:00Z",
            "max_connections": 100,
            "extensions": {
                "pg_stat_statements": false,
                "pg_stat_statements_version": null,
                "pg_stat_kcache": false,
                "pg_wait_sampling": false,
                "pg_buffercache": false
            },
            "settings": []
        });

        serde_json::json!({
            "type": "header",
            "host": host,
            "port": port,
            "dbname": dbname,
            "user": user,
            "server_info": server_info,
            "recorded_at": "2024-01-01T00:00:00Z"
        })
        .to_string()
    }

    fn make_snapshot_json(total_backends: i64) -> String {
        serde_json::json!({
            "type": "snapshot",
            "data": {
                "timestamp": "2024-01-01T00:00:00Z",
                "active_queries": [],
                "wait_events": [],
                "blocking_info": [],
                "buffer_cache": {
                    "blks_hit": 9900,
                    "blks_read": 100,
                    "hit_ratio": 0.99
                },
                "summary": {
                    "total_backends": total_backends,
                    "active_query_count": 0,
                    "idle_in_transaction_count": 0,
                    "waiting_count": 0,
                    "lock_count": 0,
                    "oldest_xact_secs": null,
                    "autovacuum_count": 0
                },
                "table_stats": [],
                "replication": [],
                "replication_slots": [],
                "subscriptions": [],
                "vacuum_progress": [],
                "wraparound": [],
                "indexes": [],
                "stat_statements": [],
                "stat_statements_error": null,
                "extensions": {
                    "pg_stat_statements": false,
                    "pg_stat_statements_version": null,
                    "pg_stat_kcache": false,
                    "pg_wait_sampling": false,
                    "pg_buffercache": false
                },
                "db_size": 1000000,
                "checkpoint_stats": null,
                "wal_stats": null,
                "archiver_stats": null,
                "bgwriter_stats": null,
                "db_stats": null
            }
        })
        .to_string()
    }

    fn create_recording_file(lines: &[&str]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        file.flush().unwrap();
        file
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Loading tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn load_valid_recording() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(15);
        let snap3 = make_snapshot_json(20);

        let file = create_recording_file(&[&header, &snap1, &snap2, &snap3]);
        let session = ReplaySession::load(file.path()).unwrap();

        assert_eq!(session.host, "localhost");
        assert_eq!(session.port, 5432);
        assert_eq!(session.dbname, "testdb");
        assert_eq!(session.user, "testuser");
        assert_eq!(session.len(), 3);
        assert_eq!(session.position, 0);
    }

    #[test]
    fn load_extracts_server_info() {
        let header = make_header_json("myhost", 5433, "mydb", "myuser");
        let snap = make_snapshot_json(5);

        let file = create_recording_file(&[&header, &snap]);
        let session = ReplaySession::load(file.path()).unwrap();

        assert!(session.server_info.version.contains("14.5"));
        assert_eq!(session.server_info.max_connections, 100);
    }

    #[test]
    fn load_empty_file_fails() {
        let file = create_recording_file(&[]);
        let result = ReplaySession::load(file.path());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn load_header_only_fails() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let file = create_recording_file(&[&header]);
        let result = ReplaySession::load(file.path());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no snapshots"));
    }

    #[test]
    fn load_snapshot_first_fails() {
        let snap = make_snapshot_json(10);
        let file = create_recording_file(&[&snap]);
        let result = ReplaySession::load(file.path());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("header"));
    }

    #[test]
    fn load_skips_empty_lines() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(20);

        let file = create_recording_file(&[&header, "", &snap1, "   ", &snap2, ""]);
        let session = ReplaySession::load(file.path()).unwrap();

        assert_eq!(session.len(), 2);
    }

    #[test]
    fn load_invalid_json_fails() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let file = create_recording_file(&[&header, "not valid json"]);
        let result = ReplaySession::load(file.path());

        assert!(result.is_err());
    }

    #[test]
    fn load_nonexistent_file_fails() {
        let result = ReplaySession::load(Path::new("/nonexistent/path/file.jsonl"));
        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Navigation tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn step_forward() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(20);
        let snap3 = make_snapshot_json(30);

        let file = create_recording_file(&[&header, &snap1, &snap2, &snap3]);
        let mut session = ReplaySession::load(file.path()).unwrap();

        assert_eq!(session.position, 0);
        assert!(session.step_forward());
        assert_eq!(session.position, 1);
        assert!(session.step_forward());
        assert_eq!(session.position, 2);
        assert!(!session.step_forward()); // At end, returns false
        assert_eq!(session.position, 2); // Position unchanged
    }

    #[test]
    fn step_back() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(20);

        let file = create_recording_file(&[&header, &snap1, &snap2]);
        let mut session = ReplaySession::load(file.path()).unwrap();

        session.position = 1;
        assert!(session.step_back());
        assert_eq!(session.position, 0);
        assert!(!session.step_back()); // At start, returns false
        assert_eq!(session.position, 0); // Position unchanged
    }

    #[test]
    fn jump_start() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(20);
        let snap3 = make_snapshot_json(30);

        let file = create_recording_file(&[&header, &snap1, &snap2, &snap3]);
        let mut session = ReplaySession::load(file.path()).unwrap();

        session.position = 2;
        session.jump_start();
        assert_eq!(session.position, 0);
    }

    #[test]
    fn jump_end() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(20);
        let snap3 = make_snapshot_json(30);

        let file = create_recording_file(&[&header, &snap1, &snap2, &snap3]);
        let mut session = ReplaySession::load(file.path()).unwrap();

        assert_eq!(session.position, 0);
        session.jump_end();
        assert_eq!(session.position, 2); // Last index (len - 1)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Current and state tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn current_returns_correct_snapshot() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(20);
        let snap3 = make_snapshot_json(30);

        let file = create_recording_file(&[&header, &snap1, &snap2, &snap3]);
        let mut session = ReplaySession::load(file.path()).unwrap();

        assert_eq!(session.current().unwrap().summary.total_backends, 10);
        session.step_forward();
        assert_eq!(session.current().unwrap().summary.total_backends, 20);
        session.step_forward();
        assert_eq!(session.current().unwrap().summary.total_backends, 30);
    }

    #[test]
    fn len_returns_snapshot_count() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(20);

        let file = create_recording_file(&[&header, &snap1, &snap2]);
        let session = ReplaySession::load(file.path()).unwrap();

        assert_eq!(session.len(), 2);
    }

    #[test]
    fn at_end_behavior() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap1 = make_snapshot_json(10);
        let snap2 = make_snapshot_json(20);

        let file = create_recording_file(&[&header, &snap1, &snap2]);
        let mut session = ReplaySession::load(file.path()).unwrap();

        assert!(!session.at_end()); // Position 0, len 2
        session.step_forward();
        assert!(session.at_end()); // Position 1, len 2
    }

    #[test]
    fn single_snapshot_at_end() {
        let header = make_header_json("localhost", 5432, "testdb", "testuser");
        let snap = make_snapshot_json(10);

        let file = create_recording_file(&[&header, &snap]);
        let session = ReplaySession::load(file.path()).unwrap();

        assert!(session.at_end()); // Single snapshot, always at end
        assert_eq!(session.len(), 1);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Fuzz tests for JSONL parsing robustness
    // ─────────────────────────────────────────────────────────────────────────────

    mod fuzz_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Parsing arbitrary strings as JSON should never panic
            #[test]
            fn json_parse_never_panics(input in ".*") {
                let _ = serde_json::from_str::<RecordLine>(&input);
            }

            /// Parsing arbitrary bytes as UTF-8 then JSON should never panic
            #[test]
            fn json_parse_arbitrary_bytes_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..500)) {
                if let Ok(input) = String::from_utf8(bytes) {
                    let _ = serde_json::from_str::<RecordLine>(&input);
                }
            }

            /// Loading a file with arbitrary content should return Err, not panic
            #[test]
            fn load_arbitrary_content_never_panics(content in ".*") {
                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{content}").unwrap();
                file.flush().unwrap();

                // Should return Err for invalid content, never panic
                let _ = ReplaySession::load(file.path());
            }

            /// Loading multiple arbitrary lines should never panic
            #[test]
            fn load_multiple_arbitrary_lines_never_panics(
                lines in proptest::collection::vec(".{0,200}", 1..20)
            ) {
                let mut file = NamedTempFile::new().unwrap();
                for line in &lines {
                    writeln!(file, "{line}").unwrap();
                }
                file.flush().unwrap();

                let _ = ReplaySession::load(file.path());
            }

            /// Valid header with corrupted snapshots should fail gracefully
            #[test]
            fn corrupted_snapshot_after_valid_header(corruption in ".{1,100}") {
                let header = make_header_json("localhost", 5432, "testdb", "testuser");
                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{header}").unwrap();
                writeln!(file, "{corruption}").unwrap();
                file.flush().unwrap();

                // Should return Err for corrupted snapshot
                let result = ReplaySession::load(file.path());
                // Either succeeds (if corruption is valid JSON that gets skipped)
                // or fails gracefully
                let _ = result;
            }

            /// Truncated JSON should fail gracefully
            #[test]
            fn truncated_json_handled(truncate_at in 1usize..100) {
                let header = make_header_json("localhost", 5432, "testdb", "testuser");
                let truncated = if truncate_at < header.len() {
                    &header[..truncate_at]
                } else {
                    &header
                };

                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{truncated}").unwrap();
                file.flush().unwrap();

                let _ = ReplaySession::load(file.path());
            }

            /// JSON with wrong type field should fail gracefully
            #[test]
            fn wrong_type_field(type_value in "[a-z]{1,20}") {
                let json = format!(r#"{{"type": "{type_value}"}}"#);
                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{json}").unwrap();
                file.flush().unwrap();

                let _ = ReplaySession::load(file.path());
            }

            /// Very deep nesting should not cause stack overflow
            #[test]
            fn deeply_nested_json(depth in 1usize..100) {
                let open_braces: String = "{\"a\":".repeat(depth);
                let close_braces: String = "}".repeat(depth);
                let json = format!("{open_braces}1{close_braces}");

                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{json}").unwrap();
                file.flush().unwrap();

                let _ = ReplaySession::load(file.path());
            }

            /// JSON with very long string values should be handled
            #[test]
            fn very_long_string_values(len in 100usize..5000) {
                let long_value = "x".repeat(len);
                let json = format!(r#"{{"type": "header", "host": "{long_value}"}}"#);

                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{json}").unwrap();
                file.flush().unwrap();

                let _ = ReplaySession::load(file.path());
            }

            /// Unicode in JSON values should be handled
            #[test]
            fn unicode_in_json(s in "\\PC{0,50}") {
                // Escape for JSON string
                let escaped = s.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                let json = format!(r#"{{"type": "header", "host": "{escaped}"}}"#);

                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{json}").unwrap();
                file.flush().unwrap();

                let _ = ReplaySession::load(file.path());
            }

            /// Null bytes and control characters should be handled
            #[test]
            fn control_characters_handled(bytes in proptest::collection::vec(0u8..32, 1..50)) {
                let s: String = bytes.iter()
                    .filter(|&&b| b != 0) // Skip null bytes for string creation
                    .map(|&b| b as char)
                    .collect();

                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{s}").unwrap();
                file.flush().unwrap();

                let _ = ReplaySession::load(file.path());
            }

            /// Empty lines mixed with content should be handled
            #[test]
            fn empty_lines_interspersed(num_empty in 0usize..20) {
                let header = make_header_json("localhost", 5432, "testdb", "testuser");
                let snap = make_snapshot_json(10);

                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{header}").unwrap();
                for _ in 0..num_empty {
                    writeln!(file).unwrap();
                }
                writeln!(file, "{snap}").unwrap();
                for _ in 0..num_empty {
                    writeln!(file).unwrap();
                }
                file.flush().unwrap();

                let result = ReplaySession::load(file.path());
                prop_assert!(result.is_ok());
            }

            /// Random valid-looking JSON objects should be handled
            #[test]
            fn random_json_objects(
                key in "[a-z]{1,10}",
                value in "[a-zA-Z0-9]{1,20}"
            ) {
                let json = format!(r#"{{"{key}" : "{value}"}}"#);

                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{json}").unwrap();
                file.flush().unwrap();

                // Should fail (missing required fields) but not panic
                let result = ReplaySession::load(file.path());
                prop_assert!(result.is_err());
            }

            /// Numbers at boundary values should be handled
            #[test]
            fn boundary_numbers(n in prop_oneof![
                Just(i64::MIN),
                Just(i64::MAX),
                Just(0i64),
                Just(-1i64),
                Just(1i64)
            ]) {
                let json = format!(r#"{{"type": "header", "port": {n}}}"#);

                let mut file = NamedTempFile::new().unwrap();
                writeln!(file, "{json}").unwrap();
                file.flush().unwrap();

                let _ = ReplaySession::load(file.path());
            }
        }
    }
}
