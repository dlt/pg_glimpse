use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DetectedExtensions {
    pub pg_stat_statements: bool,
    pub pg_stat_kcache: bool,
    pub pg_wait_sampling: bool,
    pub pg_buffercache: bool,
}

impl Default for DetectedExtensions {
    fn default() -> Self {
        Self {
            pg_stat_statements: false,
            pg_stat_kcache: false,
            pg_wait_sampling: false,
            pg_buffercache: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    pub start_time: DateTime<Utc>,
    pub max_connections: i64,
    pub extensions: DetectedExtensions,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CheckpointStats {
    pub checkpoints_timed: i64,
    pub checkpoints_req: i64,
    pub checkpoint_write_time: f64,
    pub checkpoint_sync_time: f64,
    pub buffers_checkpoint: i64,
    pub buffers_backend: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveQuery {
    pub pid: i32,
    pub usename: Option<String>,
    pub datname: Option<String>,
    pub state: Option<String>,
    pub wait_event_type: Option<String>,
    pub wait_event: Option<String>,
    pub query_start: Option<DateTime<Utc>>,
    pub duration_secs: f64,
    pub query: Option<String>,
    pub backend_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitEventCount {
    pub wait_event_type: String,
    pub wait_event: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingInfo {
    pub blocked_pid: i32,
    pub blocked_user: Option<String>,
    pub blocked_query: Option<String>,
    pub blocked_duration_secs: f64,
    pub blocker_pid: i32,
    pub blocker_user: Option<String>,
    pub blocker_query: Option<String>,
    pub blocker_state: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BufferCacheStats {
    pub blks_hit: i64,
    pub blks_read: i64,
    pub hit_ratio: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub active_query_count: i64,
    pub idle_in_transaction_count: i64,
    pub total_backends: i64,
    pub lock_count: i64,
    pub waiting_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStat {
    pub schemaname: String,
    pub relname: String,
    pub total_size_bytes: i64,
    pub seq_scan: i64,
    pub idx_scan: i64,
    pub n_live_tup: i64,
    pub n_dead_tup: i64,
    pub dead_ratio: f64,
    pub last_autovacuum: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationInfo {
    pub pid: i32,
    pub usename: Option<String>,
    pub application_name: Option<String>,
    pub client_addr: Option<String>,
    pub state: Option<String>,
    pub write_lag_secs: Option<f64>,
    pub flush_lag_secs: Option<f64>,
    pub replay_lag_secs: Option<f64>,
    pub sync_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VacuumProgress {
    pub pid: i32,
    pub datname: Option<String>,
    pub table_name: String,
    pub phase: String,
    pub heap_blks_total: i64,
    pub heap_blks_vacuumed: i64,
    pub progress_pct: f64,
    pub num_dead_tuples: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WraparoundInfo {
    pub datname: String,
    pub xid_age: i32,
    pub xids_remaining: i64,
    pub pct_towards_wraparound: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub schemaname: String,
    pub table_name: String,
    pub index_name: String,
    pub index_size_bytes: i64,
    pub idx_scan: i64,
    pub idx_tup_read: i64,
    pub idx_tup_fetch: i64,
    pub index_definition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatStatement {
    pub queryid: i64,
    pub query: String,
    pub calls: i64,
    pub total_exec_time: f64,
    pub min_exec_time: f64,
    pub mean_exec_time: f64,
    pub max_exec_time: f64,
    pub stddev_exec_time: f64,
    pub rows: i64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
    pub shared_blks_dirtied: i64,
    pub shared_blks_written: i64,
    pub local_blks_hit: i64,
    pub local_blks_read: i64,
    pub local_blks_dirtied: i64,
    pub local_blks_written: i64,
    pub temp_blks_read: i64,
    pub temp_blks_written: i64,
    pub blk_read_time: f64,
    pub blk_write_time: f64,
    pub hit_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgSnapshot {
    pub timestamp: DateTime<Utc>,
    pub active_queries: Vec<ActiveQuery>,
    pub wait_events: Vec<WaitEventCount>,
    pub blocking_info: Vec<BlockingInfo>,
    pub buffer_cache: BufferCacheStats,
    pub summary: ActivitySummary,
    pub table_stats: Vec<TableStat>,
    pub replication: Vec<ReplicationInfo>,
    pub vacuum_progress: Vec<VacuumProgress>,
    pub wraparound: Vec<WraparoundInfo>,
    pub indexes: Vec<IndexInfo>,
    pub stat_statements: Vec<StatStatement>,
    pub extensions: DetectedExtensions,
    pub db_size: i64,
    pub checkpoint_stats: Option<CheckpointStats>,
}
