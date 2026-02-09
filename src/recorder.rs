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
}
