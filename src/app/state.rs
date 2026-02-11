//! Application state types.

use chrono::{DateTime, Utc};
use ratatui::widgets::TableState;

use crate::db::models::PgSnapshot;
use crate::history::RingBuffer;

use super::sorting::SortColumnTrait;

/// Generic table view state with sort column and navigation
pub struct TableViewState<S: SortColumnTrait> {
    pub state: TableState,
    pub sort_column: S,
    pub sort_ascending: bool,
}

impl<S: SortColumnTrait> TableViewState<S> {
    pub fn new(default_sort: S, ascending: bool) -> Self {
        Self {
            state: TableState::default(),
            sort_column: default_sort,
            sort_ascending: ascending,
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_column = self.sort_column.next();
    }

    pub fn select_next(&mut self, max: usize) {
        let i = self.state.selected().unwrap_or(0);
        if i < max.saturating_sub(1) {
            self.state.select(Some(i + 1));
        }
    }

    pub fn select_prev(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some(i.saturating_sub(1)));
    }

    pub fn select_first(&mut self) {
        self.state.select(Some(0));
    }

    pub const fn selected(&self) -> Option<usize> {
        self.state.selected()
    }
}

/// State for replay mode (when reviewing recorded sessions)
pub struct ReplayState {
    pub filename: String,
    pub position: usize,
    pub total: usize,
    pub speed: f64,
    pub playing: bool,
}

impl ReplayState {
    pub const fn new(filename: String, total: usize) -> Self {
        Self {
            filename,
            position: 0,
            total,
            speed: 1.0,
            playing: false,
        }
    }
}

/// Connection information (read-only after construction)
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub user: String,
    pub ssl_mode: Option<String>,
}

impl ConnectionInfo {
    pub const fn new(host: String, port: u16, dbname: String, user: String) -> Self {
        Self {
            host,
            port,
            dbname,
            user,
            ssl_mode: None,
        }
    }

    pub fn set_ssl_mode(&mut self, label: &str) {
        self.ssl_mode = Some(label.to_string());
    }
}

/// Filter state for panel filtering
#[derive(Default)]
pub struct FilterState {
    pub text: String,
    pub active: bool,
}

impl FilterState {
    pub fn clear(&mut self) {
        self.text.clear();
        self.active = false;
    }

    pub fn push_char(&mut self, c: char) {
        self.text.push(c);
    }

    pub fn pop_char(&mut self) {
        self.text.pop();
    }
}

/// Lightweight struct for rate delta calculations (avoids cloning full `PgSnapshot`)
pub(super) struct PrevMetrics {
    pub timestamp: DateTime<Utc>,
    pub xact_commit: i64,
    pub xact_rollback: i64,
    pub blks_read: i64,
    pub wal_bytes: Option<i64>,
}

/// Metrics history for sparklines and rate calculations
pub struct MetricsHistory {
    // Sparkline data
    pub connections: RingBuffer<u64>,
    pub avg_query_time: RingBuffer<u64>,
    pub hit_ratio: RingBuffer<u64>,
    pub active_queries: RingBuffer<u64>,
    pub lock_count: RingBuffer<u64>,

    // Rate tracking
    pub tps: RingBuffer<u64>,
    pub wal_rate: RingBuffer<u64>,
    pub blks_read: RingBuffer<u64>,

    // Current values for display
    pub current_tps: Option<f64>,
    pub current_wal_rate: Option<f64>,
    pub current_blks_read_rate: Option<f64>,

    // Previous metrics for delta calculation
    pub(super) prev_metrics: Option<PrevMetrics>,
}

impl MetricsHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            connections: RingBuffer::new(capacity),
            avg_query_time: RingBuffer::new(capacity),
            hit_ratio: RingBuffer::new(capacity),
            active_queries: RingBuffer::new(capacity),
            lock_count: RingBuffer::new(capacity),
            tps: RingBuffer::new(capacity),
            wal_rate: RingBuffer::new(capacity),
            blks_read: RingBuffer::new(capacity),
            current_tps: None,
            current_wal_rate: None,
            current_blks_read_rate: None,
            prev_metrics: None,
        }
    }

    /// Push basic metrics from a snapshot
    pub fn push_snapshot_metrics(&mut self, snap: &PgSnapshot) {
        self.connections.push(snap.summary.total_backends as u64);

        let active: Vec<&_> = snap
            .active_queries
            .iter()
            .filter(|q| matches!(q.state.as_deref(), Some("active" | "idle in transaction")))
            .collect();
        let avg_ms = if active.is_empty() {
            0u64
        } else {
            let sum: f64 = active.iter().map(|q| q.duration_secs).sum();
            (sum / active.len() as f64 * 1000.0) as u64
        };
        self.avg_query_time.push(avg_ms);

        self.hit_ratio
            .push((snap.buffer_cache.hit_ratio * 1000.0) as u64);
        self.active_queries
            .push(snap.summary.active_query_count as u64);
        self.lock_count.push(snap.summary.lock_count as u64);
    }

    /// Calculate and update rate metrics from snapshot delta
    pub fn calculate_rates(&mut self, snap: &PgSnapshot) {
        if let (Some(prev), Some(curr_db)) = (&self.prev_metrics, &snap.db_stats) {
            let secs = snap
                .timestamp
                .signed_duration_since(prev.timestamp)
                .num_milliseconds() as f64
                / 1000.0;

            if secs > 0.0 {
                // TPS and blocks read from pg_stat_database
                let commits = curr_db.xact_commit - prev.xact_commit;
                let rollbacks = curr_db.xact_rollback - prev.xact_rollback;
                // Guard against counter reset (server restart)
                if commits >= 0 && rollbacks >= 0 {
                    let tps = (commits + rollbacks) as f64 / secs;
                    self.current_tps = Some(tps);
                    self.tps.push(tps as u64);
                }

                // Blocks read rate (physical I/O)
                let blks = curr_db.blks_read - prev.blks_read;
                if blks >= 0 {
                    let rate = blks as f64 / secs;
                    self.current_blks_read_rate = Some(rate);
                    self.blks_read.push(rate as u64);
                }

                // WAL rate from pg_stat_wal
                if let (Some(curr_wal_bytes), Some(prev_wal_bytes)) =
                    (snap.wal_stats.as_ref().map(|w| w.wal_bytes), prev.wal_bytes)
                {
                    let bytes = curr_wal_bytes - prev_wal_bytes;
                    if bytes >= 0 {
                        let rate = bytes as f64 / secs;
                        self.current_wal_rate = Some(rate);
                        // Store as KB/s for sparkline (fits in u64 better)
                        self.wal_rate.push((rate / 1024.0) as u64);
                    }
                }
            }
        }

        // Store only the fields needed for next delta calculation
        if let Some(db) = &snap.db_stats {
            self.prev_metrics = Some(PrevMetrics {
                timestamp: snap.timestamp,
                xact_commit: db.xact_commit,
                xact_rollback: db.xact_rollback,
                blks_read: db.blks_read,
                wal_bytes: snap.wal_stats.as_ref().map(|w| w.wal_bytes),
            });
        }
    }
}
