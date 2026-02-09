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
            writeln!(file, "{}", line).unwrap();
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
}
