use chrono::{DateTime, Utc};
use color_eyre::Result;
use tokio_postgres::Client;

use super::error::{DbError, Result as DbResult};
use super::models::{
    ActiveQuery, ActivitySummary, ArchiverStats, BgwriterStats, BlockingInfo, BloatSource,
    BufferCacheStats, CheckpointStats, DatabaseStats, DetectedExtensions, IndexInfo,
    PgExtension, PgSetting, PgSnapshot, ReplicationInfo, ReplicationSlot, ServerInfo,
    StatStatement, Subscription, TableStat, VacuumProgress, WaitEventCount, WalStats,
    WraparoundInfo,
};

/// Query result limits - these values are embedded in the SQL constants below.
/// Change both the constant and the corresponding SQL if adjusting limits.
pub mod limits {
    /// Maximum active queries to fetch from `pg_stat_activity`
    pub const MAX_ACTIVE_QUERIES: u32 = 100;
    /// Maximum blocking chains to fetch
    pub const MAX_BLOCKING_CHAINS: u32 = 50;
    /// Maximum table stats entries to fetch
    pub const MAX_TABLE_STATS: u32 = 30;
    /// Maximum `pg_stat_statements` entries to fetch
    pub const MAX_STAT_STATEMENTS: u32 = 100;
}

/// See `limits::MAX_ACTIVE_QUERIES`
const ACTIVE_QUERIES_SQL: &str = "
SELECT
    pid,
    usename,
    datname,
    state,
    wait_event_type,
    wait_event,
    query_start,
    COALESCE(EXTRACT(EPOCH FROM (clock_timestamp() - query_start))::float8, 0) AS duration_secs,
    query,
    backend_type
FROM pg_stat_activity
WHERE pid <> pg_backend_pid()
  AND state IS NOT NULL
  AND backend_type = 'client backend'
ORDER BY
    CASE state
        WHEN 'active' THEN 0
        WHEN 'idle in transaction' THEN 1
        WHEN 'idle in transaction (aborted)' THEN 2
        ELSE 3
    END,
    duration_secs DESC
LIMIT 100
";

const WAIT_EVENTS_SQL: &str = "
SELECT
    COALESCE(wait_event_type, 'CPU/Running') AS wait_event_type,
    COALESCE(wait_event, 'CPU/Running') AS wait_event,
    COUNT(*) AS count
FROM pg_stat_activity
WHERE pid <> pg_backend_pid()
  AND state = 'active'
  AND backend_type = 'client backend'
GROUP BY wait_event_type, wait_event
ORDER BY count DESC
";

/// See `limits::MAX_BLOCKING_CHAINS`
const BLOCKING_SQL: &str = "
SELECT
    blocked.pid AS blocked_pid,
    blocked.usename AS blocked_user,
    blocked.query AS blocked_query,
    COALESCE(EXTRACT(EPOCH FROM (clock_timestamp() - blocked.query_start))::float8, 0) AS blocked_duration_secs,
    blocker.pid AS blocker_pid,
    blocker.usename AS blocker_user,
    blocker.query AS blocker_query,
    blocker.state AS blocker_state
FROM pg_stat_activity AS blocked
JOIN LATERAL unnest(pg_blocking_pids(blocked.pid)) AS blocker_pid ON TRUE
JOIN pg_stat_activity AS blocker ON blocker.pid = blocker_pid
WHERE blocked.pid <> pg_backend_pid()
  AND cardinality(pg_blocking_pids(blocked.pid)) > 0
ORDER BY blocked_duration_secs DESC
LIMIT 50
";

const BUFFER_CACHE_SQL: &str = "
SELECT
    COALESCE(blks_hit, 0) AS blks_hit,
    COALESCE(blks_read, 0) AS blks_read,
    CASE
        WHEN COALESCE(blks_hit, 0) + COALESCE(blks_read, 0) = 0 THEN 1.0
        ELSE blks_hit::float / (blks_hit + blks_read)
    END AS hit_ratio
FROM pg_stat_database
WHERE datname = current_database()
";

/// See `limits::MAX_TABLE_STATS`
const TABLE_STATS_SQL: &str = "
SELECT schemaname, relname,
    COALESCE(pg_total_relation_size(relid), 0) AS total_size_bytes,
    COALESCE(pg_table_size(relid), 0) AS table_size_bytes,
    COALESCE(pg_indexes_size(relid), 0) AS indexes_size_bytes,
    COALESCE(seq_scan, 0) AS seq_scan,
    COALESCE(seq_tup_read, 0) AS seq_tup_read,
    COALESCE(idx_scan, 0) AS idx_scan,
    COALESCE(idx_tup_fetch, 0) AS idx_tup_fetch,
    COALESCE(n_live_tup, 0) AS n_live_tup,
    COALESCE(n_dead_tup, 0) AS n_dead_tup,
    COALESCE((CASE WHEN n_live_tup > 0 THEN (100.0 * n_dead_tup / n_live_tup) ELSE 0 END)::float8, 0) AS dead_ratio,
    COALESCE(n_tup_ins, 0) AS n_tup_ins,
    COALESCE(n_tup_upd, 0) AS n_tup_upd,
    COALESCE(n_tup_del, 0) AS n_tup_del,
    COALESCE(n_tup_hot_upd, 0) AS n_tup_hot_upd,
    last_vacuum,
    last_autovacuum,
    last_analyze,
    last_autoanalyze,
    COALESCE(vacuum_count, 0) AS vacuum_count,
    COALESCE(autovacuum_count, 0) AS autovacuum_count
FROM pg_stat_user_tables ORDER BY n_dead_tup DESC LIMIT 30
";

/// Replication query for PG12+: includes `reply_time`
const REPLICATION_SQL_V12: &str = "
SELECT pid,
    usesysid::bigint AS usesysid,
    usename,
    application_name,
    host(client_addr) AS client_addr,
    client_hostname,
    client_port,
    backend_start,
    backend_xmin::text AS backend_xmin,
    state::text AS state,
    sent_lsn::text AS sent_lsn,
    write_lsn::text AS write_lsn,
    flush_lsn::text AS flush_lsn,
    replay_lsn::text AS replay_lsn,
    EXTRACT(EPOCH FROM write_lag)::float8 AS write_lag_secs,
    EXTRACT(EPOCH FROM flush_lag)::float8 AS flush_lag_secs,
    EXTRACT(EPOCH FROM replay_lag)::float8 AS replay_lag_secs,
    sync_priority,
    sync_state::text AS sync_state,
    reply_time
FROM pg_stat_replication ORDER BY replay_lag DESC NULLS LAST
";

/// Replication query for PG10-11: no `reply_time` column
const REPLICATION_SQL_V10: &str = "
SELECT pid,
    usesysid::bigint AS usesysid,
    usename,
    application_name,
    host(client_addr) AS client_addr,
    client_hostname,
    client_port,
    backend_start,
    backend_xmin::text AS backend_xmin,
    state::text AS state,
    sent_lsn::text AS sent_lsn,
    write_lsn::text AS write_lsn,
    flush_lsn::text AS flush_lsn,
    replay_lsn::text AS replay_lsn,
    EXTRACT(EPOCH FROM write_lag)::float8 AS write_lag_secs,
    EXTRACT(EPOCH FROM flush_lag)::float8 AS flush_lag_secs,
    EXTRACT(EPOCH FROM replay_lag)::float8 AS replay_lag_secs,
    sync_priority,
    sync_state::text AS sync_state
FROM pg_stat_replication ORDER BY replay_lag DESC NULLS LAST
";

/// Replication slots query (all PG versions with slots support)
const REPLICATION_SLOTS_SQL: &str = "
SELECT
    slot_name,
    slot_type::text AS slot_type,
    database,
    active,
    restart_lsn::text AS restart_lsn,
    confirmed_flush_lsn::text AS confirmed_flush_lsn,
    (pg_wal_lsn_diff(pg_current_wal_lsn(), restart_lsn))::bigint AS wal_retained_bytes,
    temporary
FROM pg_replication_slots
ORDER BY slot_name
";

/// Replication slots query for PG 14+ (includes stats from `pg_stat_replication_slots`)
const REPLICATION_SLOTS_SQL_V14: &str = "
SELECT
    s.slot_name,
    s.slot_type::text AS slot_type,
    s.database,
    s.active,
    s.restart_lsn::text AS restart_lsn,
    s.confirmed_flush_lsn::text AS confirmed_flush_lsn,
    (pg_wal_lsn_diff(pg_current_wal_lsn(), s.restart_lsn))::bigint AS wal_retained_bytes,
    s.temporary,
    COALESCE(st.spill_txns, 0)::bigint AS spill_txns,
    COALESCE(st.spill_count, 0)::bigint AS spill_count,
    COALESCE(st.spill_bytes, 0)::bigint AS spill_bytes
FROM pg_replication_slots s
LEFT JOIN pg_stat_replication_slots st ON s.slot_name = st.slot_name
ORDER BY s.slot_name
";

/// Subscriptions query for PG 10+ (logical replication subscriber side)
const SUBSCRIPTIONS_SQL: &str = "
SELECT
    sub.subname,
    stat.pid,
    (SELECT COUNT(*) FROM pg_subscription_rel WHERE srsubid = sub.oid) AS relcount,
    stat.received_lsn::text AS received_lsn,
    stat.last_msg_send_time,
    stat.last_msg_receipt_time,
    stat.latest_end_lsn::text AS latest_end_lsn,
    stat.latest_end_time,
    sub.subenabled AS enabled
FROM pg_subscription sub
LEFT JOIN pg_stat_subscription stat ON sub.oid = stat.subid
WHERE stat.relid IS NULL
ORDER BY sub.subname
";

/// Vacuum progress query - uses 0 for `num_dead_tuples` for compatibility
/// (column name varies across PG versions and cloud providers)
const VACUUM_PROGRESS_SQL: &str = "
SELECT p.pid, a.datname,
    COALESCE(n.nspname || '.' || c.relname, p.relid::text) AS table_name,
    p.phase,
    p.heap_blks_total, p.heap_blks_vacuumed,
    (CASE WHEN p.heap_blks_total > 0 THEN (100.0 * p.heap_blks_vacuumed / p.heap_blks_total) ELSE 0 END)::float8 AS progress_pct,
    0::bigint AS num_dead_tuples
FROM pg_stat_progress_vacuum p
JOIN pg_stat_activity a ON a.pid = p.pid
LEFT JOIN pg_class c ON c.oid = p.relid
LEFT JOIN pg_namespace n ON n.oid = c.relnamespace
ORDER BY p.pid
";

const WRAPAROUND_SQL: &str = "
SELECT datname,
    age(datfrozenxid) AS xid_age,
    (2147483647 - age(datfrozenxid))::bigint AS xids_remaining,
    round(100.0 * age(datfrozenxid) / 2147483647, 2)::float8 AS pct_towards_wraparound
FROM pg_database WHERE datallowconn
ORDER BY age(datfrozenxid) DESC
";

const INDEXES_SQL: &str = "
SELECT
    s.schemaname,
    s.relname AS table_name,
    s.indexrelname AS index_name,
    COALESCE(pg_relation_size(s.indexrelid), 0)::bigint AS index_size_bytes,
    COALESCE(s.idx_scan, 0)::bigint AS idx_scan,
    COALESCE(s.idx_tup_read, 0)::bigint AS idx_tup_read,
    COALESCE(s.idx_tup_fetch, 0)::bigint AS idx_tup_fetch,
    pg_get_indexdef(s.indexrelid) AS index_definition
FROM pg_stat_user_indexes s
ORDER BY pg_relation_size(s.indexrelid) DESC NULLS LAST
";

/// Column naming variants for `pg_stat_statements` across PG versions.
/// - PG11-12: `total_time`, `min_time`, etc. + `blk_read_time`
/// - PG13-16: `total_exec_time`, `min_exec_time`, etc. + `blk_read_time`
/// - PG17+: `total_exec_time`, etc. + `shared_blk_read_time`
#[derive(Clone, Copy)]
struct StatStatementsColumns {
    /// Column prefix for time stats (empty for V11's "time", "exec_" for V13+)
    time_prefix: &'static str,
    /// Column name for block read time
    blk_read_time: &'static str,
    /// Column name for block write time
    blk_write_time: &'static str,
    /// ORDER BY column name
    order_by: &'static str,
}

const STAT_STATEMENTS_V11: StatStatementsColumns = StatStatementsColumns {
    time_prefix: "",
    blk_read_time: "blk_read_time",
    blk_write_time: "blk_write_time",
    order_by: "total_time",
};

const STAT_STATEMENTS_V13: StatStatementsColumns = StatStatementsColumns {
    time_prefix: "exec_",
    blk_read_time: "blk_read_time",
    blk_write_time: "blk_write_time",
    order_by: "total_exec_time",
};

const STAT_STATEMENTS_V17: StatStatementsColumns = StatStatementsColumns {
    time_prefix: "exec_",
    blk_read_time: "shared_blk_read_time",
    blk_write_time: "shared_blk_write_time",
    order_by: "total_exec_time",
};

/// Build `pg_stat_statements` query with version-specific column names.
/// See `limits::MAX_STAT_STATEMENTS`
fn build_stat_statements_sql(cols: StatStatementsColumns) -> String {
    format!(
        "SELECT
    COALESCE(queryid, 0) AS queryid,
    query,
    COALESCE(calls, 0) AS calls,
    COALESCE(total_{tp}time, 0) AS total_exec_time,
    COALESCE(min_{tp}time, 0) AS min_exec_time,
    COALESCE(mean_{tp}time, 0) AS mean_exec_time,
    COALESCE(max_{tp}time, 0) AS max_exec_time,
    COALESCE(stddev_{tp}time, 0) AS stddev_exec_time,
    COALESCE(rows, 0) AS rows,
    COALESCE(shared_blks_hit, 0) AS shared_blks_hit,
    COALESCE(shared_blks_read, 0) AS shared_blks_read,
    COALESCE(shared_blks_dirtied, 0) AS shared_blks_dirtied,
    COALESCE(shared_blks_written, 0) AS shared_blks_written,
    COALESCE(local_blks_hit, 0) AS local_blks_hit,
    COALESCE(local_blks_read, 0) AS local_blks_read,
    COALESCE(local_blks_dirtied, 0) AS local_blks_dirtied,
    COALESCE(local_blks_written, 0) AS local_blks_written,
    COALESCE(temp_blks_read, 0) AS temp_blks_read,
    COALESCE(temp_blks_written, 0) AS temp_blks_written,
    COALESCE({blk_read}, 0) AS blk_read_time,
    COALESCE({blk_write}, 0) AS blk_write_time,
    CASE
        WHEN COALESCE(shared_blks_hit, 0) + COALESCE(shared_blks_read, 0) = 0 THEN 1.0
        ELSE COALESCE(shared_blks_hit, 0)::float / (COALESCE(shared_blks_hit, 0) + COALESCE(shared_blks_read, 0))
    END AS hit_ratio
FROM pg_stat_statements
ORDER BY {order_by} DESC
LIMIT 100",
        tp = cols.time_prefix,
        blk_read = cols.blk_read_time,
        blk_write = cols.blk_write_time,
        order_by = cols.order_by,
    )
}

/// Parse extension version like "1.8" or "1.10" and return (major, minor)
pub(crate) fn parse_ext_version(v: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() >= 2 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        Some((major, minor))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ext_version_valid() {
        assert_eq!(parse_ext_version("1.8"), Some((1, 8)));
        assert_eq!(parse_ext_version("1.10"), Some((1, 10)));
        assert_eq!(parse_ext_version("2.0"), Some((2, 0)));
        assert_eq!(parse_ext_version("10.5"), Some((10, 5)));
    }

    #[test]
    fn parse_ext_version_with_patch() {
        // Should still parse major.minor even with patch version
        assert_eq!(parse_ext_version("1.8.3"), Some((1, 8)));
    }

    #[test]
    fn parse_ext_version_invalid() {
        assert_eq!(parse_ext_version(""), None);
        assert_eq!(parse_ext_version("1"), None);
        assert_eq!(parse_ext_version("abc"), None);
        assert_eq!(parse_ext_version("a.b"), None);
        assert_eq!(parse_ext_version("1.abc"), None);
    }

    #[test]
    fn parse_ext_version_edge_cases() {
        assert_eq!(parse_ext_version("0.0"), Some((0, 0)));
        assert_eq!(parse_ext_version("99.99"), Some((99, 99)));
    }

    #[test]
    fn stat_statements_sql_v11_uses_total_time() {
        let sql = build_stat_statements_sql(STAT_STATEMENTS_V11);
        assert!(sql.contains("total_time"), "V11 should use total_time");
        assert!(sql.contains("min_time"), "V11 should use min_time");
        assert!(sql.contains("blk_read_time"), "V11 should use blk_read_time");
        assert!(sql.contains("ORDER BY total_time DESC"));
    }

    #[test]
    fn stat_statements_sql_v13_uses_exec_time() {
        let sql = build_stat_statements_sql(STAT_STATEMENTS_V13);
        assert!(sql.contains("total_exec_time"), "V13 should use total_exec_time");
        assert!(sql.contains("min_exec_time"), "V13 should use min_exec_time");
        assert!(sql.contains("blk_read_time"), "V13 should use blk_read_time");
        assert!(sql.contains("ORDER BY total_exec_time DESC"));
    }

    #[test]
    fn stat_statements_sql_v17_uses_shared_blk() {
        let sql = build_stat_statements_sql(STAT_STATEMENTS_V17);
        assert!(sql.contains("total_exec_time"), "V17 should use total_exec_time");
        assert!(sql.contains("shared_blk_read_time"), "V17 should use shared_blk_read_time");
        assert!(sql.contains("shared_blk_write_time"), "V17 should use shared_blk_write_time");
        assert!(sql.contains("ORDER BY total_exec_time DESC"));
    }

    #[test]
    fn stat_statements_sql_all_versions_have_same_output_columns() {
        // All versions should alias to the same output column names
        for cols in [STAT_STATEMENTS_V11, STAT_STATEMENTS_V13, STAT_STATEMENTS_V17] {
            let sql = build_stat_statements_sql(cols);
            assert!(sql.contains("AS total_exec_time"), "Should alias to total_exec_time");
            assert!(sql.contains("AS blk_read_time"), "Should alias to blk_read_time");
            assert!(sql.contains("AS blk_write_time"), "Should alias to blk_write_time");
        }
    }
}


const ACTIVITY_SUMMARY_SQL: &str = "
SELECT
    COUNT(*) FILTER (WHERE state = 'active' AND pid <> pg_backend_pid()) AS active_query_count,
    COUNT(*) FILTER (WHERE state = 'idle in transaction') AS idle_in_transaction_count,
    COUNT(*) AS total_backends,
    (SELECT COUNT(*) FROM pg_locks WHERE NOT granted) AS lock_count,
    COUNT(*) FILTER (WHERE wait_event_type = 'Lock') AS waiting_count,
    MAX(EXTRACT(EPOCH FROM (clock_timestamp() - xact_start)))::float8 AS oldest_xact_secs,
    (SELECT COUNT(*) FROM pg_stat_activity WHERE backend_type = 'autovacuum worker') AS autovacuum_count
FROM pg_stat_activity
WHERE backend_type = 'client backend'
";

const EXTENSIONS_SQL: &str = "
SELECT extname, extversion FROM pg_extension
WHERE extname IN ('pg_stat_statements', 'pg_stat_kcache', 'pg_wait_sampling', 'pg_buffercache', 'pgstattuple')
";

const SERVER_INFO_SQL: &str = "
SELECT
    version(),
    pg_postmaster_start_time(),
    (SELECT setting::bigint FROM pg_settings WHERE name = 'max_connections') AS max_connections
";

const PG_SETTINGS_SQL: &str = "
SELECT
    name,
    setting,
    unit,
    category,
    short_desc,
    context,
    source,
    COALESCE(pending_restart, false) AS pending_restart
FROM pg_settings
ORDER BY category, name
";

const PG_EXTENSIONS_LIST_SQL: &str = "
SELECT
    e.extname AS name,
    e.extversion AS version,
    n.nspname AS schema,
    e.extrelocatable AS relocatable,
    a.comment AS description
FROM pg_extension e
JOIN pg_namespace n ON n.oid = e.extnamespace
LEFT JOIN pg_available_extensions a ON a.name = e.extname
ORDER BY e.extname
";

const DB_SIZE_SQL: &str = "
SELECT pg_database_size(current_database()) AS db_size
";

/// Checkpoint stats query for PG11-16: uses `pg_stat_bgwriter`
const CHECKPOINT_STATS_SQL_V11: &str = "
SELECT
    COALESCE(checkpoints_timed, 0) AS checkpoints_timed,
    COALESCE(checkpoints_req, 0) AS checkpoints_req,
    COALESCE(checkpoint_write_time, 0) AS checkpoint_write_time,
    COALESCE(checkpoint_sync_time, 0) AS checkpoint_sync_time,
    COALESCE(buffers_checkpoint, 0) AS buffers_checkpoint,
    COALESCE(buffers_backend, 0) AS buffers_backend
FROM pg_stat_bgwriter
";

/// Checkpoint stats query for PG17+: uses `pg_stat_checkpointer` (columns moved from `pg_stat_bgwriter`)
const CHECKPOINT_STATS_SQL_V17: &str = "
SELECT
    COALESCE(num_timed, 0) AS checkpoints_timed,
    COALESCE(num_requested, 0) AS checkpoints_req,
    COALESCE(write_time, 0) AS checkpoint_write_time,
    COALESCE(sync_time, 0) AS checkpoint_sync_time,
    COALESCE(buffers_written, 0) AS buffers_checkpoint,
    0::bigint AS buffers_backend
FROM pg_stat_checkpointer
";

const fn checkpoint_stats_sql(version: u32) -> &'static str {
    if version < 17 {
        CHECKPOINT_STATS_SQL_V11
    } else {
        CHECKPOINT_STATS_SQL_V17
    }
}

/// WAL stats query for PG14-17 (`pg_stat_wal` with full columns)
const WAL_STATS_SQL_V14: &str = "
SELECT
    COALESCE(wal_records, 0) AS wal_records,
    COALESCE(wal_fpi, 0) AS wal_fpi,
    COALESCE(wal_bytes, 0)::bigint AS wal_bytes,
    COALESCE(wal_buffers_full, 0) AS wal_buffers_full,
    COALESCE(wal_write, 0) AS wal_write,
    COALESCE(wal_sync, 0) AS wal_sync,
    COALESCE(wal_write_time, 0)::float8 AS wal_write_time,
    COALESCE(wal_sync_time, 0)::float8 AS wal_sync_time
FROM pg_stat_wal
";

/// WAL stats query for PG18+ (`wal_write`, `wal_sync`, `wal_write_time`, `wal_sync_time` removed)
const WAL_STATS_SQL_V18: &str = "
SELECT
    COALESCE(wal_records, 0) AS wal_records,
    COALESCE(wal_fpi, 0) AS wal_fpi,
    COALESCE(wal_bytes, 0)::bigint AS wal_bytes,
    COALESCE(wal_buffers_full, 0) AS wal_buffers_full,
    0::bigint AS wal_write,
    0::bigint AS wal_sync,
    0::float8 AS wal_write_time,
    0::float8 AS wal_sync_time
FROM pg_stat_wal
";

/// Archiver stats query (all versions)
const ARCHIVER_STATS_SQL: &str = "
SELECT
    COALESCE(archived_count, 0) AS archived_count,
    COALESCE(failed_count, 0) AS failed_count,
    last_archived_wal,
    last_archived_time,
    last_failed_wal,
    last_failed_time
FROM pg_stat_archiver
";

/// Background writer stats query (all versions)
const BGWRITER_STATS_SQL: &str = "
SELECT
    COALESCE(buffers_clean, 0) AS buffers_clean,
    COALESCE(maxwritten_clean, 0) AS maxwritten_clean,
    COALESCE(buffers_alloc, 0) AS buffers_alloc
FROM pg_stat_bgwriter
";

/// Database stats query for rate calculations (TPS, blocks read)
const DATABASE_STATS_SQL: &str = "
SELECT
    COALESCE(xact_commit, 0) AS xact_commit,
    COALESCE(xact_rollback, 0) AS xact_rollback,
    COALESCE(blks_read, 0) AS blks_read
FROM pg_stat_database
WHERE datname = current_database()
";

/// Table bloat estimation using pgstattuple_approx (most accurate)
/// Requires pgstattuple extension and appropriate permissions
const TABLE_BLOAT_PGSTATTUPLE_SQL: &str = "
SELECT
    s.schemaname,
    s.relname,
    (t.dead_tuple_percent + t.free_percent) AS bloat_pct,
    ((t.table_len * (t.dead_tuple_percent + t.free_percent) / 100.0))::bigint AS bloat_bytes
FROM pg_stat_user_tables s,
LATERAL pgstattuple_approx(s.relid) t
WHERE s.n_live_tup > 100
ORDER BY bloat_bytes DESC
";

/// Index bloat estimation using pgstatindex (accurate for B-tree indexes)
/// Requires pgstattuple extension
const INDEX_BLOAT_PGSTATTUPLE_SQL: &str = "
SELECT
    sui.schemaname,
    sui.relname AS table_name,
    sui.indexrelname AS index_name,
    (100.0 - t.avg_leaf_density) AS bloat_pct,
    ((pg_relation_size(sui.indexrelid) * (100.0 - t.avg_leaf_density) / 100.0))::bigint AS bloat_bytes
FROM pg_stat_user_indexes sui
JOIN pg_class c ON c.oid = sui.indexrelid
JOIN pg_index i ON i.indexrelid = sui.indexrelid,
LATERAL pgstatindex(sui.indexrelid) t
WHERE pg_relation_size(sui.indexrelid) > 65536
  AND i.indisvalid
  AND c.relam = (SELECT oid FROM pg_am WHERE amname = 'btree')
ORDER BY bloat_bytes DESC
";

/// Statistical table bloat estimation (ioguix method)
/// Uses pg_stats to calculate expected row widths and compare to actual table size
/// More accurate than naive but less accurate than pgstattuple
const TABLE_BLOAT_STATISTICAL_SQL: &str = "
WITH constants AS (
    SELECT
        current_setting('block_size')::numeric AS bs,
        23 AS page_hdr,
        8 AS tuple_hdr
),
table_stats AS (
    SELECT
        s.schemaname,
        s.relname,
        s.relid,
        c.relpages,
        c.reltuples,
        COALESCE(
            (SELECT (CASE WHEN regexp_replace(reloptions::text, '.*fillfactor=([0-9]+).*', '\\1') ~ '^[0-9]+$'
                          THEN regexp_replace(reloptions::text, '.*fillfactor=([0-9]+).*', '\\1')::int
                          ELSE 100 END)
             FROM pg_class WHERE oid = s.relid), 100
        ) AS fillfactor
    FROM pg_stat_user_tables s
    JOIN pg_class c ON c.oid = s.relid
    WHERE c.reltuples > 100
),
col_stats AS (
    SELECT
        ts.schemaname,
        ts.relname,
        ts.relid,
        ts.relpages,
        ts.reltuples,
        ts.fillfactor,
        SUM(
            (1 - COALESCE(s.null_frac, 0)) *
            COALESCE(s.avg_width,
                CASE
                    WHEN a.atttypid = 'int4'::regtype THEN 4
                    WHEN a.atttypid = 'int8'::regtype THEN 8
                    WHEN a.atttypid = 'int2'::regtype THEN 2
                    WHEN a.atttypid = 'bool'::regtype THEN 1
                    WHEN a.atttypid = 'float4'::regtype THEN 4
                    WHEN a.atttypid = 'float8'::regtype THEN 8
                    WHEN a.atttypid = 'timestamp'::regtype THEN 8
                    WHEN a.atttypid = 'timestamptz'::regtype THEN 8
                    WHEN a.atttypid = 'uuid'::regtype THEN 16
                    ELSE 10
                END
            )
        ) AS avg_row_width
    FROM table_stats ts
    JOIN pg_attribute a ON a.attrelid = ts.relid AND a.attnum > 0 AND NOT a.attisdropped
    LEFT JOIN pg_stats s ON s.schemaname = ts.schemaname
                        AND s.tablename = ts.relname
                        AND s.attname = a.attname
    GROUP BY ts.schemaname, ts.relname, ts.relid, ts.relpages, ts.reltuples, ts.fillfactor
),
bloat_calc AS (
    SELECT
        cs.schemaname,
        cs.relname,
        cs.relpages,
        cs.reltuples,
        c.bs,
        cs.fillfactor,
        -- Tuple size with alignment (round up to 8 bytes)
        (c.tuple_hdr + cs.avg_row_width + 7)::int / 8 * 8 AS tpl_size,
        -- Usable page size accounting for page header and fillfactor
        ((c.bs - c.page_hdr) * cs.fillfactor / 100)::int AS usable_page
    FROM col_stats cs
    CROSS JOIN constants c
),
expected AS (
    SELECT
        schemaname,
        relname,
        relpages,
        bs,
        -- Expected pages needed
        CEIL(reltuples * tpl_size / NULLIF(usable_page, 0)) AS expected_pages
    FROM bloat_calc
    WHERE tpl_size > 0 AND usable_page > 0
)
SELECT
    schemaname,
    relname,
    GREATEST(0.0, 100.0 * (relpages - expected_pages) / NULLIF(relpages, 0))::float8 AS bloat_pct,
    GREATEST(0, (relpages - expected_pages) * bs)::bigint AS bloat_bytes
FROM expected
WHERE relpages > 0
ORDER BY bloat_bytes DESC
";

/// Statistical index bloat estimation
/// Estimates based on relation size vs expected entries
const INDEX_BLOAT_STATISTICAL_SQL: &str = "
WITH index_stats AS (
    SELECT
        sui.schemaname,
        sui.relname AS table_name,
        sui.indexrelname AS index_name,
        pg_relation_size(sui.indexrelid) AS index_size,
        c.reltuples AS table_tuples,
        -- Estimate index tuple size: key width + tuple overhead
        COALESCE(
            (SELECT SUM(COALESCE(s.avg_width, 8))
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             LEFT JOIN pg_stats s ON s.schemaname = sui.schemaname
                                  AND s.tablename = sui.relname
                                  AND s.attname = a.attname
             WHERE i.indexrelid = sui.indexrelid),
            24
        ) + 8 AS est_idx_tuple_size
    FROM pg_stat_user_indexes sui
    JOIN pg_class c ON c.oid = sui.relid
    JOIN pg_index i ON i.indexrelid = sui.indexrelid
    WHERE pg_relation_size(sui.indexrelid) > 65536
      AND i.indisvalid
),
bloat_calc AS (
    SELECT
        schemaname,
        table_name,
        index_name,
        index_size,
        -- Expected index size (tuples * tuple size, with some overhead for B-tree structure ~1.3x)
        GREATEST(8192, (table_tuples * est_idx_tuple_size * 1.3)::bigint) AS expected_size
    FROM index_stats
    WHERE table_tuples > 0
)
SELECT
    schemaname,
    table_name,
    index_name,
    GREATEST(0.0, 100.0 * (index_size - expected_size) / NULLIF(index_size, 0))::float8 AS bloat_pct,
    GREATEST(0, index_size - expected_size)::bigint AS bloat_bytes
FROM bloat_calc
ORDER BY bloat_bytes DESC
";

/// Naive table bloat estimation - simplified version (fallback)
/// Estimates bloat by comparing actual table size to expected size based on row count
const TABLE_BLOAT_NAIVE_SQL: &str = "
SELECT
    schemaname,
    relname,
    GREATEST(0, pg_table_size(relid) - (n_live_tup * 100))::bigint AS bloat_bytes,
    (CASE
        WHEN pg_table_size(relid) > 0 AND n_live_tup > 0
        THEN GREATEST(0.0, 100.0 * (1.0 - (n_live_tup * 100.0 / pg_table_size(relid))))
        ELSE 0.0
    END)::float8 AS bloat_pct
FROM pg_stat_user_tables
WHERE n_live_tup > 0
ORDER BY bloat_bytes DESC
";

/// Naive index bloat estimation - simplified version (fallback)
const INDEX_BLOAT_NAIVE_SQL: &str = "
SELECT
    sui.schemaname,
    sui.relname AS table_name,
    sui.indexrelname AS index_name,
    GREATEST(0, pg_relation_size(sui.indexrelid) - GREATEST(c.reltuples * 50, 8192))::bigint AS bloat_bytes,
    (CASE
        WHEN pg_relation_size(sui.indexrelid) > 8192 AND c.reltuples > 0
        THEN GREATEST(0.0, 100.0 * (1.0 - (c.reltuples * 50.0 / pg_relation_size(sui.indexrelid))))
        ELSE 0.0
    END)::float8 AS bloat_pct
FROM pg_stat_user_indexes sui
JOIN pg_class c ON c.oid = sui.indexrelid
WHERE pg_relation_size(sui.indexrelid) > 0
ORDER BY bloat_bytes DESC
";

pub async fn detect_extensions(client: &Client) -> DetectedExtensions {
    let Ok(rows) = client.query(EXTENSIONS_SQL, &[]).await else {
        return DetectedExtensions::default();
    };
    let mut ext = DetectedExtensions::default();
    for row in rows {
        let name: String = row.get("extname");
        let version: String = row.get("extversion");
        match name.as_str() {
            "pg_stat_statements" => {
                ext.pg_stat_statements = true;
                ext.pg_stat_statements_version = Some(version);
            }
            "pg_stat_kcache" => ext.pg_stat_kcache = true,
            "pg_wait_sampling" => ext.pg_wait_sampling = true,
            "pg_buffercache" => ext.pg_buffercache = true,
            "pgstattuple" => {
                ext.pgstattuple = true;
                ext.pgstattuple_version = Some(version);
            }
            _ => {}
        }
    }
    ext
}

pub async fn fetch_pg_settings(client: &Client) -> DbResult<Vec<PgSetting>> {
    let rows = client
        .query(PG_SETTINGS_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_pg_settings",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(PgSetting {
            name: row.get("name"),
            setting: row.get("setting"),
            unit: row.get("unit"),
            category: row.get("category"),
            short_desc: row.get("short_desc"),
            context: row.get("context"),
            source: row.get("source"),
            pending_restart: row.get("pending_restart"),
        });
    }
    Ok(results)
}

pub async fn fetch_extensions_list(client: &Client) -> DbResult<Vec<PgExtension>> {
    let rows = client
        .query(PG_EXTENSIONS_LIST_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_extensions_list",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(PgExtension {
            name: row.get("name"),
            version: row.get("version"),
            schema: row.get("schema"),
            relocatable: row.get("relocatable"),
            description: row.get("description"),
        });
    }
    Ok(results)
}

pub async fn fetch_server_info(client: &Client) -> DbResult<ServerInfo> {
    let extensions = detect_extensions(client).await;
    let settings = fetch_pg_settings(client).await.unwrap_or_default();
    let extensions_list = fetch_extensions_list(client).await.unwrap_or_default();
    let row = client
        .query_one(SERVER_INFO_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_server_info",
            source: e,
        })?;
    let version: String = row.get(0);
    let start_time: DateTime<Utc> = row.get(1);
    let max_connections: i64 = row.get(2);
    Ok(ServerInfo {
        version,
        start_time,
        max_connections,
        extensions,
        settings,
        extensions_list,
    })
}

pub async fn fetch_db_size(client: &Client) -> DbResult<i64> {
    let row = client
        .query_one(DB_SIZE_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_db_size",
            source: e,
        })?;
    Ok(row.get("db_size"))
}

pub async fn fetch_checkpoint_stats(client: &Client, version: u32) -> DbResult<CheckpointStats> {
    let sql = checkpoint_stats_sql(version);
    let row = client
        .query_one(sql, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_checkpoint_stats",
            source: e,
        })?;
    Ok(CheckpointStats {
        checkpoints_timed: row.get("checkpoints_timed"),
        checkpoints_req: row.get("checkpoints_req"),
        checkpoint_write_time: row.get("checkpoint_write_time"),
        checkpoint_sync_time: row.get("checkpoint_sync_time"),
        buffers_checkpoint: row.get("buffers_checkpoint"),
        buffers_backend: row.get("buffers_backend"),
    })
}

pub async fn fetch_wal_stats(client: &Client, version: u32) -> DbResult<WalStats> {
    let sql = if version >= 18 {
        WAL_STATS_SQL_V18
    } else {
        WAL_STATS_SQL_V14
    };
    let row = client
        .query_one(sql, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_wal_stats",
            source: e,
        })?;
    Ok(WalStats {
        wal_records: row.get("wal_records"),
        wal_fpi: row.get("wal_fpi"),
        wal_bytes: row.get("wal_bytes"),
        wal_buffers_full: row.get("wal_buffers_full"),
        wal_write: row.get("wal_write"),
        wal_sync: row.get("wal_sync"),
        wal_write_time: row.get("wal_write_time"),
        wal_sync_time: row.get("wal_sync_time"),
    })
}

pub async fn fetch_archiver_stats(client: &Client) -> DbResult<ArchiverStats> {
    let row = client
        .query_one(ARCHIVER_STATS_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_archiver_stats",
            source: e,
        })?;
    Ok(ArchiverStats {
        archived_count: row.get("archived_count"),
        failed_count: row.get("failed_count"),
        last_archived_wal: row.get("last_archived_wal"),
        last_archived_time: row.get("last_archived_time"),
        last_failed_wal: row.get("last_failed_wal"),
        last_failed_time: row.get("last_failed_time"),
    })
}

pub async fn fetch_bgwriter_stats(client: &Client) -> DbResult<BgwriterStats> {
    let row = client
        .query_one(BGWRITER_STATS_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_bgwriter_stats",
            source: e,
        })?;
    Ok(BgwriterStats {
        buffers_clean: row.get("buffers_clean"),
        maxwritten_clean: row.get("maxwritten_clean"),
        buffers_alloc: row.get("buffers_alloc"),
    })
}

pub async fn fetch_database_stats(client: &Client) -> DbResult<DatabaseStats> {
    let row = client
        .query_one(DATABASE_STATS_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_database_stats",
            source: e,
        })?;
    Ok(DatabaseStats {
        xact_commit: row.get("xact_commit"),
        xact_rollback: row.get("xact_rollback"),
        blks_read: row.get("blks_read"),
    })
}

pub async fn fetch_active_queries(client: &Client) -> DbResult<Vec<ActiveQuery>> {
    let rows = client
        .query(ACTIVE_QUERIES_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_active_queries",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(ActiveQuery {
            pid: row.get("pid"),
            usename: row.get("usename"),
            datname: row.get("datname"),
            state: row.get("state"),
            wait_event_type: row.get("wait_event_type"),
            wait_event: row.get("wait_event"),
            query_start: row.get("query_start"),
            duration_secs: row.get("duration_secs"),
            query: row.get("query"),
            backend_type: row.get("backend_type"),
        });
    }
    Ok(results)
}

pub async fn fetch_wait_events(client: &Client) -> DbResult<Vec<WaitEventCount>> {
    let rows = client
        .query(WAIT_EVENTS_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_wait_events",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(WaitEventCount {
            wait_event_type: row.get("wait_event_type"),
            wait_event: row.get("wait_event"),
            count: row.get("count"),
        });
    }
    Ok(results)
}

pub async fn fetch_blocking_info(client: &Client) -> DbResult<Vec<BlockingInfo>> {
    let rows = client
        .query(BLOCKING_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_blocking_info",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(BlockingInfo {
            blocked_pid: row.get("blocked_pid"),
            blocked_user: row.get("blocked_user"),
            blocked_query: row.get("blocked_query"),
            blocked_duration_secs: row.get("blocked_duration_secs"),
            blocker_pid: row.get("blocker_pid"),
            blocker_user: row.get("blocker_user"),
            blocker_query: row.get("blocker_query"),
            blocker_state: row.get("blocker_state"),
        });
    }
    Ok(results)
}

pub async fn fetch_buffer_cache(client: &Client) -> DbResult<BufferCacheStats> {
    let row = client
        .query_one(BUFFER_CACHE_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_buffer_cache",
            source: e,
        })?;
    Ok(BufferCacheStats {
        blks_hit: row.get("blks_hit"),
        blks_read: row.get("blks_read"),
        hit_ratio: row.get("hit_ratio"),
    })
}

pub async fn fetch_activity_summary(client: &Client) -> DbResult<ActivitySummary> {
    let row = client
        .query_one(ACTIVITY_SUMMARY_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_activity_summary",
            source: e,
        })?;
    Ok(ActivitySummary {
        active_query_count: row.get("active_query_count"),
        idle_in_transaction_count: row.get("idle_in_transaction_count"),
        total_backends: row.get("total_backends"),
        lock_count: row.get("lock_count"),
        waiting_count: row.get("waiting_count"),
        oldest_xact_secs: row.get("oldest_xact_secs"),
        autovacuum_count: row.get("autovacuum_count"),
    })
}

pub async fn fetch_table_stats(client: &Client) -> DbResult<Vec<TableStat>> {
    let rows = client
        .query(TABLE_STATS_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_table_stats",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(TableStat {
            schemaname: row.get("schemaname"),
            relname: row.get("relname"),
            total_size_bytes: row.get("total_size_bytes"),
            table_size_bytes: row.get("table_size_bytes"),
            indexes_size_bytes: row.get("indexes_size_bytes"),
            seq_scan: row.get("seq_scan"),
            seq_tup_read: row.get("seq_tup_read"),
            idx_scan: row.get("idx_scan"),
            idx_tup_fetch: row.get("idx_tup_fetch"),
            n_live_tup: row.get("n_live_tup"),
            n_dead_tup: row.get("n_dead_tup"),
            dead_ratio: row.get("dead_ratio"),
            n_tup_ins: row.get("n_tup_ins"),
            n_tup_upd: row.get("n_tup_upd"),
            n_tup_del: row.get("n_tup_del"),
            n_tup_hot_upd: row.get("n_tup_hot_upd"),
            last_vacuum: row.get("last_vacuum"),
            last_autovacuum: row.get("last_autovacuum"),
            last_analyze: row.get("last_analyze"),
            last_autoanalyze: row.get("last_autoanalyze"),
            vacuum_count: row.get("vacuum_count"),
            autovacuum_count: row.get("autovacuum_count"),
            bloat_bytes: None,
            bloat_pct: None,
            bloat_source: None,
        });
    }
    Ok(results)
}

pub async fn fetch_replication(client: &Client, version: u32) -> DbResult<Vec<ReplicationInfo>> {
    let sql = if version >= 12 {
        REPLICATION_SQL_V12
    } else {
        REPLICATION_SQL_V10
    };
    let rows = client
        .query(sql, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_replication",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(ReplicationInfo {
            pid: row.get(0),
            usesysid: row.get(1),
            usename: row.get(2),
            application_name: row.get(3),
            client_addr: row.get(4),
            client_hostname: row.get(5),
            client_port: row.get(6),
            backend_start: row.get(7),
            backend_xmin: row.get(8),
            state: row.get(9),
            sent_lsn: row.get(10),
            write_lsn: row.get(11),
            flush_lsn: row.get(12),
            replay_lsn: row.get(13),
            write_lag_secs: row.get(14),
            flush_lag_secs: row.get(15),
            replay_lag_secs: row.get(16),
            sync_priority: row.get(17),
            sync_state: row.get(18),
            reply_time: if version >= 12 { row.get(19) } else { None },
        });
    }
    Ok(results)
}

pub async fn fetch_replication_slots(client: &Client, version: u32) -> DbResult<Vec<ReplicationSlot>> {
    let sql = if version >= 14 {
        REPLICATION_SLOTS_SQL_V14
    } else {
        REPLICATION_SLOTS_SQL
    };
    let Ok(rows) = client.query(sql, &[]).await else {
        return Ok(vec![]); // Graceful fallback if query fails
    };
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(ReplicationSlot {
            slot_name: row.get("slot_name"),
            slot_type: row.get("slot_type"),
            database: row.get("database"),
            active: row.get("active"),
            restart_lsn: row.get("restart_lsn"),
            confirmed_flush_lsn: row.get("confirmed_flush_lsn"),
            wal_retained_bytes: row.get("wal_retained_bytes"),
            temporary: row.get("temporary"),
            spill_txns: if version >= 14 { row.get("spill_txns") } else { None },
            spill_count: if version >= 14 { row.get("spill_count") } else { None },
            spill_bytes: if version >= 14 { row.get("spill_bytes") } else { None },
        });
    }
    Ok(results)
}

pub async fn fetch_subscriptions(client: &Client, version: u32) -> DbResult<Vec<Subscription>> {
    // Logical replication subscriptions only available in PG 10+
    if version < 10 {
        return Ok(vec![]);
    }
    let Ok(rows) = client.query(SUBSCRIPTIONS_SQL, &[]).await else {
        return Ok(vec![]); // Graceful fallback if query fails
    };
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(Subscription {
            subname: row.get("subname"),
            pid: row.get("pid"),
            relcount: row.get("relcount"),
            received_lsn: row.get("received_lsn"),
            last_msg_send_time: row.get("last_msg_send_time"),
            last_msg_receipt_time: row.get("last_msg_receipt_time"),
            latest_end_lsn: row.get("latest_end_lsn"),
            latest_end_time: row.get("latest_end_time"),
            enabled: row.get("enabled"),
        });
    }
    Ok(results)
}

pub async fn fetch_vacuum_progress(client: &Client, _version: u32) -> DbResult<Vec<VacuumProgress>> {
    let rows = client
        .query(VACUUM_PROGRESS_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_vacuum_progress",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(VacuumProgress {
            pid: row.get("pid"),
            datname: row.get("datname"),
            table_name: row.get("table_name"),
            phase: row.get("phase"),
            heap_blks_total: row.get("heap_blks_total"),
            heap_blks_vacuumed: row.get("heap_blks_vacuumed"),
            progress_pct: row.get("progress_pct"),
            num_dead_tuples: row.get("num_dead_tuples"),
        });
    }
    Ok(results)
}

pub async fn fetch_wraparound(client: &Client) -> DbResult<Vec<WraparoundInfo>> {
    let rows = client
        .query(WRAPAROUND_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_wraparound",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(WraparoundInfo {
            datname: row.get("datname"),
            xid_age: row.get("xid_age"),
            xids_remaining: row.get("xids_remaining"),
            pct_towards_wraparound: row.get("pct_towards_wraparound"),
        });
    }
    Ok(results)
}

pub async fn fetch_indexes(client: &Client) -> DbResult<Vec<IndexInfo>> {
    let rows = client
        .query(INDEXES_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_indexes",
            source: e,
        })?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(IndexInfo {
            schemaname: row.get("schemaname"),
            table_name: row.get("table_name"),
            index_name: row.get("index_name"),
            index_size_bytes: row.get("index_size_bytes"),
            idx_scan: row.get("idx_scan"),
            idx_tup_read: row.get("idx_tup_read"),
            idx_tup_fetch: row.get("idx_tup_fetch"),
            index_definition: row.get("index_definition"),
            bloat_bytes: None,
            bloat_pct: None,
            bloat_source: None,
        });
    }
    Ok(results)
}

/// Returns (statements, `error_message`)
pub async fn fetch_stat_statements(
    client: &Client,
    extensions: &DetectedExtensions,
    pg_major_version: u32,
) -> (Vec<StatStatement>, Option<String>) {
    if !extensions.pg_stat_statements {
        return (vec![], None);
    }
    let ext_version = extensions.pg_stat_statements_version.as_deref();

    // First, check if we can access the view and get row count
    let count_check = client
        .query_one("SELECT COUNT(*)::bigint AS cnt FROM pg_stat_statements", &[])
        .await;

    match count_check {
        Err(e) => {
            // Can't even count rows - permission or access issue
            let msg = e.as_db_error().map_or_else(
                || e.to_string(),
                |db_err| {
                    let mut parts = vec![db_err.message().to_string()];
                    if let Some(detail) = db_err.detail() {
                        parts.push(format!("Detail: {detail}"));
                    }
                    if let Some(hint) = db_err.hint() {
                        parts.push(format!("Hint: {hint}"));
                    }
                    parts.join(" - ")
                },
            );
            let hint = if msg.contains("permission denied") {
                format!("{msg} (Try: GRANT pg_read_all_stats TO your_user;)")
            } else if msg.contains("does not exist") {
                format!("{msg} (Extension may be in a different schema)")
            } else {
                msg
            };
            return (vec![], Some(hint));
        }
        Ok(row) => {
            let cnt: i64 = row.get("cnt");
            if cnt == 0 {
                // View is accessible but empty - this is expected for fresh installs
                return (vec![], None);
            }
        }
    }

    // Try queries in order: version-appropriate first, then fallbacks
    // Important: blk_read_time → shared_blk_read_time rename happened in PG17 (server version),
    // while total_time → total_exec_time happened in extension version 1.8 (PG13)
    let columns_to_try = if pg_major_version >= 17 {
        // PG17+ uses shared_blk_read_time
        vec![STAT_STATEMENTS_V17, STAT_STATEMENTS_V13, STAT_STATEMENTS_V11]
    } else {
        // PG13-16: uses total_exec_time but old blk_read_time
        match ext_version.and_then(parse_ext_version) {
            Some((major, minor)) if major > 1 || (major == 1 && minor >= 8) => {
                vec![STAT_STATEMENTS_V13, STAT_STATEMENTS_V11]
            }
            _ => vec![STAT_STATEMENTS_V11],
        }
    };

    let mut last_error = String::new();
    for cols in columns_to_try {
        let sql = build_stat_statements_sql(cols);
        match client.query(&sql, &[]).await {
            Ok(rows) => {
                let mut results = Vec::with_capacity(rows.len());
                for row in rows {
                    results.push(StatStatement {
                        queryid: row.get("queryid"),
                        query: row.get("query"),
                        calls: row.get("calls"),
                        total_exec_time: row.get("total_exec_time"),
                        min_exec_time: row.get("min_exec_time"),
                        mean_exec_time: row.get("mean_exec_time"),
                        max_exec_time: row.get("max_exec_time"),
                        stddev_exec_time: row.get("stddev_exec_time"),
                        rows: row.get("rows"),
                        shared_blks_hit: row.get("shared_blks_hit"),
                        shared_blks_read: row.get("shared_blks_read"),
                        shared_blks_dirtied: row.get("shared_blks_dirtied"),
                        shared_blks_written: row.get("shared_blks_written"),
                        local_blks_hit: row.get("local_blks_hit"),
                        local_blks_read: row.get("local_blks_read"),
                        local_blks_dirtied: row.get("local_blks_dirtied"),
                        local_blks_written: row.get("local_blks_written"),
                        temp_blks_read: row.get("temp_blks_read"),
                        temp_blks_written: row.get("temp_blks_written"),
                        blk_read_time: row.get("blk_read_time"),
                        blk_write_time: row.get("blk_write_time"),
                        hit_ratio: row.get("hit_ratio"),
                    });
                }
                return (results, None);
            }
            Err(e) => {
                // If it's a column error, try next query variant
                let msg = e.to_string();
                if msg.contains("column") && msg.contains("does not exist") {
                    last_error = msg;
                    continue;
                }
                // For other errors, return immediately
                let detailed = e.as_db_error().map_or(msg, |db_err| {
                    let mut parts = vec![db_err.message().to_string()];
                    if let Some(detail) = db_err.detail() {
                        parts.push(format!("Detail: {detail}"));
                    }
                    if let Some(hint) = db_err.hint() {
                        parts.push(format!("Hint: {hint}"));
                    }
                    parts.join(" - ")
                });
                let version_info = ext_version.unwrap_or("unknown");
                let hint = if detailed.contains("permission denied") {
                    format!("{detailed} (Try: GRANT pg_read_all_stats TO your_user;)")
                } else {
                    format!("{detailed} (PG{pg_major_version}, ext {version_info})")
                };
                return (vec![], Some(hint));
            }
        }
    }

    // All queries failed with column errors
    let version_info = ext_version.unwrap_or("unknown");
    (vec![], Some(format!("{last_error} (PG{pg_major_version}, ext {version_info}, tried all query variants)")))
}

use std::collections::HashMap;

/// Bloat estimation result for a table
#[derive(Debug, Clone)]
pub struct TableBloat {
    pub bloat_bytes: i64,
    pub bloat_pct: f64,
    pub source: BloatSource,
}

/// Bloat estimation result for an index
#[derive(Debug, Clone)]
pub struct IndexBloat {
    pub bloat_bytes: i64,
    pub bloat_pct: f64,
    pub source: BloatSource,
}

/// Try pgstattuple-based table bloat query
async fn try_pgstattuple_table_bloat(client: &Client) -> Option<HashMap<String, TableBloat>> {
    let rows = client.query(TABLE_BLOAT_PGSTATTUPLE_SQL, &[]).await.ok()?;
    let mut results = HashMap::with_capacity(rows.len());
    for row in rows {
        let schema: String = row.get("schemaname");
        let table: String = row.get("relname");
        let key = format!("{schema}.{table}");
        results.insert(
            key,
            TableBloat {
                bloat_bytes: row.get("bloat_bytes"),
                bloat_pct: row.get("bloat_pct"),
                source: BloatSource::Pgstattuple,
            },
        );
    }
    Some(results)
}

/// Try statistical table bloat estimation
async fn try_statistical_table_bloat(client: &Client) -> Option<HashMap<String, TableBloat>> {
    let rows = client.query(TABLE_BLOAT_STATISTICAL_SQL, &[]).await.ok()?;
    let mut results = HashMap::with_capacity(rows.len());
    for row in rows {
        let schema: String = row.get("schemaname");
        let table: String = row.get("relname");
        let key = format!("{schema}.{table}");
        results.insert(
            key,
            TableBloat {
                bloat_bytes: row.get("bloat_bytes"),
                bloat_pct: row.get("bloat_pct"),
                source: BloatSource::Statistical,
            },
        );
    }
    Some(results)
}

/// Naive table bloat estimation (fallback)
async fn naive_table_bloat(client: &Client) -> DbResult<HashMap<String, TableBloat>> {
    let rows = client
        .query(TABLE_BLOAT_NAIVE_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_table_bloat",
            source: e,
        })?;
    let mut results = HashMap::with_capacity(rows.len());
    for row in rows {
        let schema: String = row.get("schemaname");
        let table: String = row.get("relname");
        let key = format!("{schema}.{table}");
        results.insert(
            key,
            TableBloat {
                bloat_bytes: row.get("bloat_bytes"),
                bloat_pct: row.get("bloat_pct"),
                source: BloatSource::Naive,
            },
        );
    }
    Ok(results)
}

/// Fetch table bloat estimates. Returns map of "schema.table" -> bloat info.
/// Uses pgstattuple if available, falls back to statistical, then naive estimation.
pub async fn fetch_table_bloat(
    client: &Client,
    extensions: &DetectedExtensions,
) -> DbResult<HashMap<String, TableBloat>> {
    // Try pgstattuple first if available
    if extensions.pgstattuple {
        if let Some(results) = try_pgstattuple_table_bloat(client).await {
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    // Try statistical estimation
    if let Some(results) = try_statistical_table_bloat(client).await {
        if !results.is_empty() {
            return Ok(results);
        }
    }

    // Fall back to naive estimation
    naive_table_bloat(client).await
}

/// Try pgstattuple-based index bloat query
async fn try_pgstattuple_index_bloat(client: &Client) -> Option<HashMap<String, IndexBloat>> {
    let rows = client.query(INDEX_BLOAT_PGSTATTUPLE_SQL, &[]).await.ok()?;
    let mut results = HashMap::with_capacity(rows.len());
    for row in rows {
        let schema: String = row.get("schemaname");
        let index: String = row.get("index_name");
        let key = format!("{schema}.{index}");
        results.insert(
            key,
            IndexBloat {
                bloat_bytes: row.get("bloat_bytes"),
                bloat_pct: row.get("bloat_pct"),
                source: BloatSource::Pgstattuple,
            },
        );
    }
    Some(results)
}

/// Try statistical index bloat estimation
async fn try_statistical_index_bloat(client: &Client) -> Option<HashMap<String, IndexBloat>> {
    let rows = client.query(INDEX_BLOAT_STATISTICAL_SQL, &[]).await.ok()?;
    let mut results = HashMap::with_capacity(rows.len());
    for row in rows {
        let schema: String = row.get("schemaname");
        let index: String = row.get("index_name");
        let key = format!("{schema}.{index}");
        results.insert(
            key,
            IndexBloat {
                bloat_bytes: row.get("bloat_bytes"),
                bloat_pct: row.get("bloat_pct"),
                source: BloatSource::Statistical,
            },
        );
    }
    Some(results)
}

/// Naive index bloat estimation (fallback)
async fn naive_index_bloat(client: &Client) -> DbResult<HashMap<String, IndexBloat>> {
    let rows = client
        .query(INDEX_BLOAT_NAIVE_SQL, &[])
        .await
        .map_err(|e| DbError::Query {
            context: "fetch_index_bloat",
            source: e,
        })?;
    let mut results = HashMap::with_capacity(rows.len());
    for row in rows {
        let schema: String = row.get("schemaname");
        let index: String = row.get("index_name");
        let key = format!("{schema}.{index}");
        results.insert(
            key,
            IndexBloat {
                bloat_bytes: row.get("bloat_bytes"),
                bloat_pct: row.get("bloat_pct"),
                source: BloatSource::Naive,
            },
        );
    }
    Ok(results)
}

/// Fetch index bloat estimates. Returns map of "`schema.index_name`" -> bloat info.
/// Uses pgstattuple if available, falls back to statistical, then naive estimation.
pub async fn fetch_index_bloat(
    client: &Client,
    extensions: &DetectedExtensions,
) -> DbResult<HashMap<String, IndexBloat>> {
    // Try pgstattuple first if available
    if extensions.pgstattuple {
        if let Some(results) = try_pgstattuple_index_bloat(client).await {
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    // Try statistical estimation
    if let Some(results) = try_statistical_index_bloat(client).await {
        if !results.is_empty() {
            return Ok(results);
        }
    }

    // Fall back to naive estimation
    naive_index_bloat(client).await
}

pub async fn cancel_backend(client: &Client, pid: i32) -> DbResult<bool> {
    let row = client
        .query_one("SELECT pg_cancel_backend($1)", &[&pid])
        .await
        .map_err(|e| DbError::Query {
            context: "cancel_backend",
            source: e,
        })?;
    Ok(row.get(0))
}

pub async fn terminate_backend(client: &Client, pid: i32) -> DbResult<bool> {
    let row = client
        .query_one("SELECT pg_terminate_backend($1)", &[&pid])
        .await
        .map_err(|e| DbError::Query {
            context: "terminate_backend",
            source: e,
        })?;
    Ok(row.get(0))
}

/// Cancel multiple backends. Returns (pid, success) for each.
pub async fn cancel_backends(client: &Client, pids: &[i32]) -> Vec<(i32, bool)> {
    let mut results = Vec::with_capacity(pids.len());
    for &pid in pids {
        let ok = cancel_backend(client, pid).await.unwrap_or(false);
        results.push((pid, ok));
    }
    results
}

/// Terminate multiple backends. Returns (pid, success) for each.
pub async fn terminate_backends(client: &Client, pids: &[i32]) -> Vec<(i32, bool)> {
    let mut results = Vec::with_capacity(pids.len());
    for &pid in pids {
        let ok = terminate_backend(client, pid).await.unwrap_or(false);
        results.push((pid, ok));
    }
    results
}

pub async fn fetch_snapshot(
    client: &Client,
    extensions: &DetectedExtensions,
    version: u32,
) -> Result<PgSnapshot> {
    let ext = extensions.clone();
    let (active, waits, blocks, cache, summary, tables, repl, repl_slots, subs, vacuum, wrap, indexes, ss, db_size, chkpt, wal, archiver, bgwriter, db_stats) =
        tokio::try_join!(
            async { fetch_active_queries(client).await.map_err(color_eyre::Report::from) },
            async { fetch_wait_events(client).await.map_err(color_eyre::Report::from) },
            async { fetch_blocking_info(client).await.map_err(color_eyre::Report::from) },
            async { fetch_buffer_cache(client).await.map_err(color_eyre::Report::from) },
            async { fetch_activity_summary(client).await.map_err(color_eyre::Report::from) },
            // Table stats can fail if tables are dropped during query - return empty on error
            async { Ok::<_, color_eyre::Report>(fetch_table_stats(client).await.unwrap_or_default()) },
            async { fetch_replication(client, version).await.map_err(color_eyre::Report::from) },
            async { fetch_replication_slots(client, version).await.map_err(color_eyre::Report::from) },
            async { fetch_subscriptions(client, version).await.map_err(color_eyre::Report::from) },
            async { fetch_vacuum_progress(client, version).await.map_err(color_eyre::Report::from) },
            async { fetch_wraparound(client).await.map_err(color_eyre::Report::from) },
            // Index stats can fail if tables are dropped during query - return empty on error
            async { Ok::<_, color_eyre::Report>(fetch_indexes(client).await.unwrap_or_default()) },
            async { Ok(fetch_stat_statements(client, &ext, version).await) },
            async { fetch_db_size(client).await.map_err(color_eyre::Report::from) },
            async { Ok(fetch_checkpoint_stats(client, version).await.ok()) },
            async {
                // pg_stat_wal only available in PG14+
                if version >= 14 {
                    Ok(fetch_wal_stats(client, version).await.ok())
                } else {
                    Ok(None)
                }
            },
            async { Ok(fetch_archiver_stats(client).await.ok()) },
            async { Ok(fetch_bgwriter_stats(client).await.ok()) },
            async { Ok(fetch_database_stats(client).await.ok()) },
        )?;
    let (stat_statements, stat_statements_error) = ss;
    Ok(PgSnapshot {
        timestamp: chrono::Utc::now(),
        active_queries: active,
        wait_events: waits,
        blocking_info: blocks,
        buffer_cache: cache,
        summary,
        table_stats: tables,
        replication: repl,
        replication_slots: repl_slots,
        subscriptions: subs,
        vacuum_progress: vacuum,
        wraparound: wrap,
        indexes,
        stat_statements,
        stat_statements_error,
        extensions: ext,
        db_size,
        checkpoint_stats: chkpt,
        wal_stats: wal,
        archiver_stats: archiver,
        bgwriter_stats: bgwriter,
        db_stats,
    })
}
