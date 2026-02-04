use color_eyre::Result;
use tokio_postgres::Client;

use super::models::*;

const ACTIVE_QUERIES_SQL: &str = "
SELECT
    pid,
    usename,
    datname,
    state,
    wait_event_type,
    wait_event,
    query_start,
    COALESCE(EXTRACT(EPOCH FROM (clock_timestamp() - query_start)), 0) AS duration_secs,
    LEFT(query, 120) AS query,
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

const BLOCKING_SQL: &str = "
SELECT
    blocked.pid AS blocked_pid,
    blocked.usename AS blocked_user,
    LEFT(blocked.query, 100) AS blocked_query,
    COALESCE(EXTRACT(EPOCH FROM (clock_timestamp() - blocked.query_start)), 0) AS blocked_duration_secs,
    blocker.pid AS blocker_pid,
    blocker.usename AS blocker_user,
    LEFT(blocker.query, 100) AS blocker_query,
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

const TABLE_STATS_SQL: &str = "
SELECT schemaname, relname,
    pg_total_relation_size(relid) AS total_size_bytes,
    COALESCE(seq_scan, 0) AS seq_scan,
    COALESCE(idx_scan, 0) AS idx_scan,
    COALESCE(n_live_tup, 0) AS n_live_tup,
    COALESCE(n_dead_tup, 0) AS n_dead_tup,
    (CASE WHEN n_live_tup > 0 THEN (100.0 * n_dead_tup / n_live_tup) ELSE 0 END)::float8 AS dead_ratio,
    last_autovacuum
FROM pg_stat_user_tables ORDER BY n_dead_tup DESC LIMIT 30
";

const REPLICATION_SQL: &str = "
SELECT pid, usename, application_name, client_addr::text,
    state,
    EXTRACT(EPOCH FROM write_lag) AS write_lag_secs,
    EXTRACT(EPOCH FROM flush_lag) AS flush_lag_secs,
    EXTRACT(EPOCH FROM replay_lag) AS replay_lag_secs,
    sync_state
FROM pg_stat_replication ORDER BY replay_lag DESC NULLS LAST
";

const VACUUM_PROGRESS_SQL: &str = "
SELECT p.pid, a.datname,
    COALESCE(n.nspname || '.' || c.relname, p.relid::text) AS table_name,
    p.phase,
    p.heap_blks_total, p.heap_blks_vacuumed,
    (CASE WHEN p.heap_blks_total > 0 THEN (100.0 * p.heap_blks_vacuumed / p.heap_blks_total) ELSE 0 END)::float8 AS progress_pct,
    p.num_dead_tuples
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
    pg_relation_size(s.indexrelid)::bigint AS index_size_bytes,
    COALESCE(s.idx_scan, 0)::bigint AS idx_scan,
    COALESCE(s.idx_tup_read, 0)::bigint AS idx_tup_read,
    COALESCE(s.idx_tup_fetch, 0)::bigint AS idx_tup_fetch,
    pg_get_indexdef(s.indexrelid) AS index_definition
FROM pg_stat_user_indexes s
ORDER BY pg_relation_size(s.indexrelid) DESC
";

const ACTIVITY_SUMMARY_SQL: &str = "
SELECT
    COUNT(*) FILTER (WHERE state = 'active' AND pid <> pg_backend_pid()) AS active_query_count,
    COUNT(*) FILTER (WHERE state = 'idle in transaction') AS idle_in_transaction_count,
    COUNT(*) AS total_backends,
    (SELECT COUNT(*) FROM pg_locks WHERE NOT granted) AS lock_count,
    COUNT(*) FILTER (WHERE wait_event_type = 'Lock') AS waiting_count
FROM pg_stat_activity
WHERE backend_type = 'client backend'
";

pub async fn fetch_active_queries(client: &Client) -> Result<Vec<ActiveQuery>> {
    let rows = client.query(ACTIVE_QUERIES_SQL, &[]).await?;
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

pub async fn fetch_wait_events(client: &Client) -> Result<Vec<WaitEventCount>> {
    let rows = client.query(WAIT_EVENTS_SQL, &[]).await?;
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

pub async fn fetch_blocking_info(client: &Client) -> Result<Vec<BlockingInfo>> {
    let rows = client.query(BLOCKING_SQL, &[]).await?;
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

pub async fn fetch_buffer_cache(client: &Client) -> Result<BufferCacheStats> {
    let row = client.query_one(BUFFER_CACHE_SQL, &[]).await?;
    Ok(BufferCacheStats {
        blks_hit: row.get("blks_hit"),
        blks_read: row.get("blks_read"),
        hit_ratio: row.get("hit_ratio"),
    })
}

pub async fn fetch_activity_summary(client: &Client) -> Result<ActivitySummary> {
    let row = client.query_one(ACTIVITY_SUMMARY_SQL, &[]).await?;
    Ok(ActivitySummary {
        active_query_count: row.get("active_query_count"),
        idle_in_transaction_count: row.get("idle_in_transaction_count"),
        total_backends: row.get("total_backends"),
        lock_count: row.get("lock_count"),
        waiting_count: row.get("waiting_count"),
    })
}

pub async fn fetch_table_stats(client: &Client) -> Result<Vec<TableStat>> {
    let rows = client.query(TABLE_STATS_SQL, &[]).await?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(TableStat {
            schemaname: row.get("schemaname"),
            relname: row.get("relname"),
            total_size_bytes: row.get("total_size_bytes"),
            seq_scan: row.get("seq_scan"),
            idx_scan: row.get("idx_scan"),
            n_live_tup: row.get("n_live_tup"),
            n_dead_tup: row.get("n_dead_tup"),
            dead_ratio: row.get("dead_ratio"),
            last_autovacuum: row.get("last_autovacuum"),
        });
    }
    Ok(results)
}

pub async fn fetch_replication(client: &Client) -> Result<Vec<ReplicationInfo>> {
    let rows = client.query(REPLICATION_SQL, &[]).await?;
    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        results.push(ReplicationInfo {
            pid: row.get("pid"),
            usename: row.get("usename"),
            application_name: row.get("application_name"),
            client_addr: row.get("client_addr"),
            state: row.get("state"),
            write_lag_secs: row.get("write_lag_secs"),
            flush_lag_secs: row.get("flush_lag_secs"),
            replay_lag_secs: row.get("replay_lag_secs"),
            sync_state: row.get("sync_state"),
        });
    }
    Ok(results)
}

pub async fn fetch_vacuum_progress(client: &Client) -> Result<Vec<VacuumProgress>> {
    let rows = client.query(VACUUM_PROGRESS_SQL, &[]).await?;
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

pub async fn fetch_wraparound(client: &Client) -> Result<Vec<WraparoundInfo>> {
    let rows = client.query(WRAPAROUND_SQL, &[]).await?;
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

pub async fn fetch_indexes(client: &Client) -> Result<Vec<IndexInfo>> {
    let rows = client.query(INDEXES_SQL, &[]).await?;
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
        });
    }
    Ok(results)
}

pub async fn cancel_backend(client: &Client, pid: i32) -> Result<bool> {
    let row = client
        .query_one("SELECT pg_cancel_backend($1)", &[&pid])
        .await?;
    Ok(row.get(0))
}

pub async fn terminate_backend(client: &Client, pid: i32) -> Result<bool> {
    let row = client
        .query_one("SELECT pg_terminate_backend($1)", &[&pid])
        .await?;
    Ok(row.get(0))
}

pub async fn fetch_snapshot(client: &Client) -> Result<PgSnapshot> {
    let (active, waits, blocks, cache, summary, tables, repl, vacuum, wrap, indexes) =
        tokio::try_join!(
            fetch_active_queries(client),
            fetch_wait_events(client),
            fetch_blocking_info(client),
            fetch_buffer_cache(client),
            fetch_activity_summary(client),
            fetch_table_stats(client),
            fetch_replication(client),
            fetch_vacuum_progress(client),
            fetch_wraparound(client),
            fetch_indexes(client),
        )?;
    Ok(PgSnapshot {
        timestamp: chrono::Utc::now(),
        active_queries: active,
        wait_events: waits,
        blocking_info: blocks,
        buffer_cache: cache,
        summary,
        table_stats: tables,
        replication: repl,
        vacuum_progress: vacuum,
        wraparound: wrap,
        indexes,
    })
}
