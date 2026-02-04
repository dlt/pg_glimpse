use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::db::models::{PgSnapshot, ServerInfo};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
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
        let filename = filename.replace('/', "_").replace('\\', "_");
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
}
