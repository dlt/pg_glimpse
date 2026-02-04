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
