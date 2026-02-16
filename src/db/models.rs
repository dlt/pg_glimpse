use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DetectedExtensions {
    pub pg_stat_statements: bool,
    pub pg_stat_statements_version: Option<String>,
    pub pg_stat_kcache: bool,
    pub pg_wait_sampling: bool,
    pub pg_buffercache: bool,
    pub pgstattuple: bool,
    pub pgstattuple_version: Option<String>,
}

/// Source of bloat estimation - indicates accuracy level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BloatSource {
    /// Accurate measurement using pgstattuple extension
    Pgstattuple,
    /// Estimated from pg_stats column widths (ioguix method)
    Statistical,
    /// Simple formula based on assumed row size (legacy, least accurate)
    #[default]
    Naive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgSetting {
    pub name: String,
    pub setting: String,
    pub unit: Option<String>,
    pub category: String,
    pub short_desc: String,
    pub context: String,        // postmaster, sighup, superuser, user
    pub source: String,         // default, configuration file, etc.
    pub pending_restart: bool,  // PG 9.5+
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgExtension {
    pub name: String,
    pub version: String,
    pub schema: String,
    pub relocatable: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    pub start_time: DateTime<Utc>,
    pub max_connections: i64,
    pub extensions: DetectedExtensions,
    #[serde(default)]
    pub settings: Vec<PgSetting>,
    #[serde(default)]
    pub extensions_list: Vec<PgExtension>,
}

impl ServerInfo {
    /// Extract the major `PostgreSQL` version number (e.g., 14 from "`PostgreSQL` 14.5 on ...")
    #[must_use] 
    pub fn major_version(&self) -> u32 {
        self.version
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.split('.').next())
            .and_then(|v| v.parse().ok())
            .unwrap_or(11)
    }
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct WalStats {
    pub wal_records: i64,
    pub wal_fpi: i64,
    pub wal_bytes: i64,
    pub wal_buffers_full: i64,
    pub wal_write: i64,
    pub wal_sync: i64,
    pub wal_write_time: f64,
    pub wal_sync_time: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchiverStats {
    pub archived_count: i64,
    pub failed_count: i64,
    pub last_archived_wal: Option<String>,
    pub last_archived_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_failed_wal: Option<String>,
    pub last_failed_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BgwriterStats {
    pub buffers_clean: i64,
    pub maxwritten_clean: i64,
    pub buffers_alloc: i64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub xact_commit: i64,
    pub xact_rollback: i64,
    pub blks_read: i64,
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
    pub oldest_xact_secs: Option<f64>,
    pub autovacuum_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStat {
    pub schemaname: String,
    pub relname: String,
    pub total_size_bytes: i64,
    pub table_size_bytes: i64,
    pub indexes_size_bytes: i64,
    pub seq_scan: i64,
    pub seq_tup_read: i64,
    pub idx_scan: i64,
    pub idx_tup_fetch: i64,
    pub n_live_tup: i64,
    pub n_dead_tup: i64,
    pub dead_ratio: f64,
    pub n_tup_ins: i64,
    pub n_tup_upd: i64,
    pub n_tup_del: i64,
    pub n_tup_hot_upd: i64,
    pub last_vacuum: Option<DateTime<Utc>>,
    pub last_autovacuum: Option<DateTime<Utc>>,
    pub last_analyze: Option<DateTime<Utc>>,
    pub last_autoanalyze: Option<DateTime<Utc>>,
    pub vacuum_count: i64,
    pub autovacuum_count: i64,
    // Bloat estimation (populated on-demand)
    #[serde(default)]
    pub bloat_bytes: Option<i64>,
    #[serde(default)]
    pub bloat_pct: Option<f64>,
    #[serde(default)]
    pub bloat_source: Option<BloatSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationInfo {
    pub pid: i32,
    pub usesysid: Option<i64>,
    pub usename: Option<String>,
    pub application_name: Option<String>,
    pub client_addr: Option<String>,
    pub client_hostname: Option<String>,
    pub client_port: Option<i32>,
    pub backend_start: Option<chrono::DateTime<chrono::Utc>>,
    pub backend_xmin: Option<String>,
    pub state: Option<String>,
    pub sent_lsn: Option<String>,
    pub write_lsn: Option<String>,
    pub flush_lsn: Option<String>,
    pub replay_lsn: Option<String>,
    pub write_lag_secs: Option<f64>,
    pub flush_lag_secs: Option<f64>,
    pub replay_lag_secs: Option<f64>,
    pub sync_priority: Option<i32>,
    pub sync_state: Option<String>,
    pub reply_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationSlot {
    pub slot_name: String,
    pub slot_type: String,
    pub database: Option<String>,
    pub active: bool,
    pub restart_lsn: Option<String>,
    pub confirmed_flush_lsn: Option<String>,
    pub wal_retained_bytes: Option<i64>,
    pub temporary: bool,
    // PG 14+ stats from pg_stat_replication_slots
    pub spill_txns: Option<i64>,
    pub spill_count: Option<i64>,
    pub spill_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub subname: String,
    pub pid: Option<i32>,
    pub relcount: i64,
    pub received_lsn: Option<String>,
    pub last_msg_send_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_msg_receipt_time: Option<chrono::DateTime<chrono::Utc>>,
    pub latest_end_lsn: Option<String>,
    pub latest_end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
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
    // Bloat estimation (populated on-demand)
    #[serde(default)]
    pub bloat_bytes: Option<i64>,
    #[serde(default)]
    pub bloat_pct: Option<f64>,
    #[serde(default)]
    pub bloat_source: Option<BloatSource>,
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
pub struct ForeignKeyConstraint {
    pub constraint_name: String,
    pub table_schema: String,
    pub table_name: String,
    pub column_name: String,
    pub foreign_table_schema: String,
    pub foreign_table_name: String,
    pub foreign_column_name: String,
    pub delete_rule: String,
    pub update_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub column_name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub schema_name: String,
    pub table_name: String,
    pub columns: Vec<ColumnInfo>,
    pub primary_keys: Vec<String>,
    pub foreign_keys_out: Vec<ForeignKeyConstraint>,
    pub foreign_keys_in: Vec<ForeignKeyConstraint>,
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
    pub replication_slots: Vec<ReplicationSlot>,
    pub subscriptions: Vec<Subscription>,
    pub vacuum_progress: Vec<VacuumProgress>,
    pub wraparound: Vec<WraparoundInfo>,
    pub indexes: Vec<IndexInfo>,
    pub stat_statements: Vec<StatStatement>,
    pub stat_statements_error: Option<String>,
    pub extensions: DetectedExtensions,
    pub db_size: i64,
    pub checkpoint_stats: Option<CheckpointStats>,
    pub wal_stats: Option<WalStats>,
    pub archiver_stats: Option<ArchiverStats>,
    pub bgwriter_stats: Option<BgwriterStats>,
    pub db_stats: Option<DatabaseStats>,
    #[serde(default)]
    pub table_schemas: Vec<TableSchema>,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKeyConstraint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────────
    // ServerInfo::major_version tests
    // ─────────────────────────────────────────────────────────────────────────────

    fn server_info_with_version(version: &str) -> ServerInfo {
        ServerInfo {
            version: version.to_string(),
            start_time: Utc::now(),
            max_connections: 100,
            extensions: DetectedExtensions::default(),
            settings: vec![],
            extensions_list: vec![],
        }
    }

    #[test]
    fn major_version_pg14() {
        let info = server_info_with_version("PostgreSQL 14.5 on x86_64-pc-linux-gnu");
        assert_eq!(info.major_version(), 14);
    }

    #[test]
    fn major_version_pg11() {
        let info = server_info_with_version("PostgreSQL 11.21 on x86_64-pc-linux-gnu");
        assert_eq!(info.major_version(), 11);
    }

    #[test]
    fn major_version_pg17() {
        let info = server_info_with_version("PostgreSQL 17.0 on x86_64-apple-darwin");
        assert_eq!(info.major_version(), 17);
    }

    #[test]
    fn major_version_pg9_6() {
        let info = server_info_with_version("PostgreSQL 9.6.24 on x86_64-pc-linux-gnu");
        assert_eq!(info.major_version(), 9);
    }

    #[test]
    fn major_version_with_devel_suffix() {
        let info = server_info_with_version("PostgreSQL 18devel on x86_64");
        // "18devel".parse() will fail, should return default 11
        assert_eq!(info.major_version(), 11);
    }

    #[test]
    fn major_version_aurora() {
        let info =
            server_info_with_version("PostgreSQL 15.4 on x86_64-pc-linux-gnu, compiled by gcc");
        assert_eq!(info.major_version(), 15);
    }

    #[test]
    fn major_version_empty_string() {
        let info = server_info_with_version("");
        assert_eq!(info.major_version(), 11); // Default fallback
    }

    #[test]
    fn major_version_garbage() {
        let info = server_info_with_version("not a version string at all");
        assert_eq!(info.major_version(), 11); // Default fallback
    }

    #[test]
    fn major_version_just_postgresql() {
        let info = server_info_with_version("PostgreSQL");
        assert_eq!(info.major_version(), 11); // Default fallback
    }

    #[test]
    fn major_version_no_minor() {
        let info = server_info_with_version("PostgreSQL 16 on linux");
        assert_eq!(info.major_version(), 16);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // DetectedExtensions default
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn detected_extensions_default() {
        let ext = DetectedExtensions::default();
        assert!(!ext.pg_stat_statements);
        assert!(!ext.pg_stat_kcache);
        assert!(!ext.pg_wait_sampling);
        assert!(!ext.pg_buffercache);
        assert!(!ext.pgstattuple);
        assert!(ext.pg_stat_statements_version.is_none());
        assert!(ext.pgstattuple_version.is_none());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Serde roundtrip tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn server_info_serde_roundtrip() {
        let info = ServerInfo {
            version: "PostgreSQL 15.2".to_string(),
            start_time: Utc::now(),
            max_connections: 200,
            extensions: DetectedExtensions {
                pg_stat_statements: true,
                pg_stat_statements_version: Some("1.10".to_string()),
                pg_stat_kcache: false,
                pg_wait_sampling: true,
                pg_buffercache: false,
                pgstattuple: false,
                pgstattuple_version: None,
            },
            settings: vec![PgSetting {
                name: "max_connections".to_string(),
                setting: "200".to_string(),
                unit: None,
                category: "Connections".to_string(),
                short_desc: "Max connections".to_string(),
                context: "postmaster".to_string(),
                source: "configuration file".to_string(),
                pending_restart: false,
            }],
            extensions_list: vec![],
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: ServerInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, info.version);
        assert_eq!(parsed.max_connections, info.max_connections);
        assert_eq!(
            parsed.extensions.pg_stat_statements,
            info.extensions.pg_stat_statements
        );
        assert_eq!(parsed.settings.len(), 1);
        assert_eq!(parsed.settings[0].name, "max_connections");
    }

    #[test]
    fn activity_summary_default_values() {
        let summary = ActivitySummary {
            active_query_count: 5,
            idle_in_transaction_count: 2,
            total_backends: 10,
            lock_count: 3,
            waiting_count: 1,
            oldest_xact_secs: Some(120.5),
            autovacuum_count: 0,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: ActivitySummary = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.active_query_count, 5);
        assert_eq!(parsed.oldest_xact_secs, Some(120.5));
    }

    #[test]
    fn active_query_with_nulls() {
        let query = ActiveQuery {
            pid: 12345,
            usename: None,
            datname: None,
            state: Some("active".to_string()),
            wait_event_type: None,
            wait_event: None,
            query_start: None,
            duration_secs: 5.5,
            query: None,
            backend_type: None,
        };

        let json = serde_json::to_string(&query).unwrap();
        let parsed: ActiveQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.pid, 12345);
        assert!(parsed.usename.is_none());
        assert_eq!(parsed.state, Some("active".to_string()));
        assert_eq!(parsed.duration_secs, 5.5);
    }

    #[test]
    fn buffer_cache_stats_serde() {
        let stats = BufferCacheStats {
            blks_hit: 9900,
            blks_read: 100,
            hit_ratio: 0.99,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let parsed: BufferCacheStats = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.blks_hit, 9900);
        assert_eq!(parsed.blks_read, 100);
        assert!((parsed.hit_ratio - 0.99).abs() < 0.001);
    }

    #[test]
    fn table_stat_bloat_fields_default() {
        // Test that bloat fields default to None when missing from JSON
        let json = r#"{
            "schemaname": "public",
            "relname": "users",
            "total_size_bytes": 1000000,
            "table_size_bytes": 800000,
            "indexes_size_bytes": 200000,
            "seq_scan": 100,
            "seq_tup_read": 5000,
            "idx_scan": 500,
            "idx_tup_fetch": 2000,
            "n_live_tup": 1000,
            "n_dead_tup": 50,
            "dead_ratio": 5.0,
            "n_tup_ins": 100,
            "n_tup_upd": 50,
            "n_tup_del": 10,
            "n_tup_hot_upd": 20,
            "last_vacuum": null,
            "last_autovacuum": null,
            "last_analyze": null,
            "last_autoanalyze": null,
            "vacuum_count": 5,
            "autovacuum_count": 10
        }"#;

        let parsed: TableStat = serde_json::from_str(json).unwrap();
        assert!(parsed.bloat_bytes.is_none());
        assert!(parsed.bloat_pct.is_none());
    }

    #[test]
    fn index_info_bloat_fields_default() {
        // Test that bloat fields default to None when missing from JSON
        let json = r#"{
            "schemaname": "public",
            "table_name": "users",
            "index_name": "users_pkey",
            "index_size_bytes": 50000,
            "idx_scan": 1000,
            "idx_tup_read": 5000,
            "idx_tup_fetch": 4500,
            "index_definition": "CREATE UNIQUE INDEX users_pkey ON public.users USING btree (id)"
        }"#;

        let parsed: IndexInfo = serde_json::from_str(json).unwrap();
        assert!(parsed.bloat_bytes.is_none());
        assert!(parsed.bloat_pct.is_none());
    }

    #[test]
    fn wal_stats_default() {
        let stats = WalStats::default();
        assert_eq!(stats.wal_records, 0);
        assert_eq!(stats.wal_bytes, 0);
        assert_eq!(stats.wal_write_time, 0.0);
    }

    #[test]
    fn replication_slot_serde() {
        let slot = ReplicationSlot {
            slot_name: "my_slot".to_string(),
            slot_type: "logical".to_string(),
            database: Some("mydb".to_string()),
            active: true,
            restart_lsn: Some("0/1234567".to_string()),
            confirmed_flush_lsn: Some("0/1234000".to_string()),
            wal_retained_bytes: Some(1_048_576),
            temporary: false,
            spill_txns: Some(0),
            spill_count: Some(0),
            spill_bytes: Some(0),
        };

        let json = serde_json::to_string(&slot).unwrap();
        let parsed: ReplicationSlot = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.slot_name, "my_slot");
        assert!(parsed.active);
        assert_eq!(parsed.wal_retained_bytes, Some(1_048_576));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // BloatSource tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn bloat_source_default_is_naive() {
        assert_eq!(BloatSource::default(), BloatSource::Naive);
    }

    #[test]
    fn bloat_source_serde_roundtrip() {
        for source in [BloatSource::Pgstattuple, BloatSource::Statistical, BloatSource::Naive] {
            let json = serde_json::to_string(&source).unwrap();
            let parsed: BloatSource = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, source);
        }
    }

    #[test]
    fn table_stat_bloat_source_defaults_to_none() {
        let json = r#"{
            "schemaname": "public",
            "relname": "users",
            "total_size_bytes": 1000000,
            "table_size_bytes": 800000,
            "indexes_size_bytes": 200000,
            "seq_scan": 100,
            "seq_tup_read": 5000,
            "idx_scan": 500,
            "idx_tup_fetch": 2000,
            "n_live_tup": 1000,
            "n_dead_tup": 50,
            "dead_ratio": 5.0,
            "n_tup_ins": 100,
            "n_tup_upd": 50,
            "n_tup_del": 10,
            "n_tup_hot_upd": 20,
            "last_vacuum": null,
            "last_autovacuum": null,
            "last_analyze": null,
            "last_autoanalyze": null,
            "vacuum_count": 5,
            "autovacuum_count": 10
        }"#;

        let parsed: TableStat = serde_json::from_str(json).unwrap();
        assert!(parsed.bloat_source.is_none());
    }

    #[test]
    fn table_stat_bloat_source_with_value() {
        let json = r#"{
            "schemaname": "public",
            "relname": "users",
            "total_size_bytes": 1000000,
            "table_size_bytes": 800000,
            "indexes_size_bytes": 200000,
            "seq_scan": 100,
            "seq_tup_read": 5000,
            "idx_scan": 500,
            "idx_tup_fetch": 2000,
            "n_live_tup": 1000,
            "n_dead_tup": 50,
            "dead_ratio": 5.0,
            "n_tup_ins": 100,
            "n_tup_upd": 50,
            "n_tup_del": 10,
            "n_tup_hot_upd": 20,
            "last_vacuum": null,
            "last_autovacuum": null,
            "last_analyze": null,
            "last_autoanalyze": null,
            "vacuum_count": 5,
            "autovacuum_count": 10,
            "bloat_bytes": 50000,
            "bloat_pct": 5.0,
            "bloat_source": "Pgstattuple"
        }"#;

        let parsed: TableStat = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.bloat_bytes, Some(50000));
        assert_eq!(parsed.bloat_pct, Some(5.0));
        assert_eq!(parsed.bloat_source, Some(BloatSource::Pgstattuple));
    }

    #[test]
    fn detected_extensions_missing_pgstattuple_defaults() {
        // Old JSON without pgstattuple fields should deserialize with defaults
        let json = r#"{
            "pg_stat_statements": true,
            "pg_stat_statements_version": "1.10",
            "pg_stat_kcache": false,
            "pg_wait_sampling": false,
            "pg_buffercache": true
        }"#;

        let parsed: DetectedExtensions = serde_json::from_str(json).unwrap();
        assert!(!parsed.pgstattuple);
        assert!(parsed.pgstattuple_version.is_none());
    }

    #[test]
    fn detected_extensions_with_pgstattuple() {
        let json = r#"{
            "pg_stat_statements": true,
            "pg_stat_statements_version": "1.10",
            "pg_stat_kcache": false,
            "pg_wait_sampling": false,
            "pg_buffercache": true,
            "pgstattuple": true,
            "pgstattuple_version": "1.5"
        }"#;

        let parsed: DetectedExtensions = serde_json::from_str(json).unwrap();
        assert!(parsed.pgstattuple);
        assert_eq!(parsed.pgstattuple_version, Some("1.5".to_string()));
    }
}
