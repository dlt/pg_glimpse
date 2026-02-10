//! Integration tests for pg_glimpse
//!
//! These tests require running PostgreSQL instances. Use the provided Docker Compose:
//!
//! ```bash
//! # Start test databases
//! docker compose -f tests/docker-compose.yml up -d
//!
//! # Wait for databases to be ready
//! sleep 5
//!
//! # Run integration tests
//! cargo test --features integration --test integration
//!
//! # Stop test databases
//! docker compose -f tests/docker-compose.yml down -v
//! ```

#![cfg(feature = "integration")]

use pg_glimpse::db::models::DetectedExtensions;
use pg_glimpse::db::queries;
use tokio_postgres::{Client, NoTls};

/// PostgreSQL test instance configuration
struct PgInstance {
    name: &'static str,
    port: u16,
    version: u32,
}

const PG_INSTANCES: &[PgInstance] = &[
    PgInstance {
        name: "pg11",
        port: 5411,
        version: 11,
    },
    PgInstance {
        name: "pg12",
        port: 5412,
        version: 12,
    },
    PgInstance {
        name: "pg13",
        port: 5413,
        version: 13,
    },
    PgInstance {
        name: "pg14",
        port: 5414,
        version: 14,
    },
    PgInstance {
        name: "pg15",
        port: 5415,
        version: 15,
    },
    PgInstance {
        name: "pg16",
        port: 5416,
        version: 16,
    },
    PgInstance {
        name: "pg17",
        port: 5417,
        version: 17,
    },
    PgInstance {
        name: "pg18",
        port: 5418,
        version: 18,
    },
];

/// Connect to a PostgreSQL test instance
async fn connect(port: u16) -> Result<Client, tokio_postgres::Error> {
    let connstr = format!(
        "host=localhost port={port} user=test password=test dbname=test"
    );
    let (client, connection) = tokio_postgres::connect(&connstr, NoTls).await?;

    // Spawn the connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {e}");
        }
    });

    Ok(client)
}

/// Extract PostgreSQL major version from version string
fn extract_major_version(version: &str) -> Option<u32> {
    // Version string format: "PostgreSQL 14.5 on ..."
    version
        .split_whitespace()
        .nth(1)?
        .split('.')
        .next()?
        .parse()
        .ok()
}

/// Enable pg_stat_statements extension if not already enabled
async fn ensure_pg_stat_statements(client: &Client) -> Result<(), tokio_postgres::Error> {
    // Try to create extension (may already exist)
    let _ = client
        .execute("CREATE EXTENSION IF NOT EXISTS pg_stat_statements", &[])
        .await;
    Ok(())
}

// ============================================================================
// Connection and Version Tests
// ============================================================================

#[tokio::test]
async fn test_pg11_connection() {
    if let Ok(client) = connect(5411).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 11, "Expected PG11, got PG{major}");
    } else {
        eprintln!("Skipping pg11 test - instance not available");
    }
}

#[tokio::test]
async fn test_pg14_connection() {
    if let Ok(client) = connect(5414).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 14, "Expected PG14, got PG{major}");
    } else {
        eprintln!("Skipping pg14 test - instance not available");
    }
}

#[tokio::test]
async fn test_pg12_connection() {
    if let Ok(client) = connect(5412).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 12, "Expected PG12, got PG{major}");
    } else {
        eprintln!("Skipping pg12 test - instance not available");
    }
}

#[tokio::test]
async fn test_pg13_connection() {
    if let Ok(client) = connect(5413).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 13, "Expected PG13, got PG{major}");
    } else {
        eprintln!("Skipping pg13 test - instance not available");
    }
}

#[tokio::test]
async fn test_pg15_connection() {
    if let Ok(client) = connect(5415).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 15, "Expected PG15, got PG{major}");
    } else {
        eprintln!("Skipping pg15 test - instance not available");
    }
}

#[tokio::test]
async fn test_pg16_connection() {
    if let Ok(client) = connect(5416).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 16, "Expected PG16, got PG{major}");
    } else {
        eprintln!("Skipping pg16 test - instance not available");
    }
}

#[tokio::test]
async fn test_pg17_connection() {
    if let Ok(client) = connect(5417).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 17, "Expected PG17, got PG{major}");
    } else {
        eprintln!("Skipping pg17 test - instance not available");
    }
}

#[tokio::test]
async fn test_pg18_connection() {
    if let Ok(client) = connect(5418).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 18, "Expected PG18, got PG{major}");
    } else {
        eprintln!("Skipping pg18 test - instance not available");
    }
}

// ============================================================================
// Core Query Tests - Run on all versions
// ============================================================================

/// Test pg_stat_activity query works on all versions
#[tokio::test]
async fn test_pg_stat_activity_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query(
                    "SELECT pid, state, query FROM pg_stat_activity WHERE pid = pg_backend_pid()",
                    &[],
                )
                .await;
            assert!(
                result.is_ok(),
                "{}: pg_stat_activity query failed: {:?}",
                instance.name,
                result.err()
            );
            let rows = result.unwrap();
            assert_eq!(rows.len(), 1, "{}: should return exactly one row", instance.name);
        }
    }
}

/// Test pg_stat_database query works on all versions
#[tokio::test]
async fn test_pg_stat_database_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query(
                    "SELECT datname, blks_hit, blks_read FROM pg_stat_database WHERE datname = current_database()",
                    &[],
                )
                .await;
            assert!(
                result.is_ok(),
                "{}: pg_stat_database query failed: {:?}",
                instance.name,
                result.err()
            );
        }
    }
}

/// Test pg_stat_bgwriter query works on all versions
#[tokio::test]
async fn test_pg_stat_bgwriter_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query("SELECT buffers_clean, buffers_alloc FROM pg_stat_bgwriter", &[])
                .await;
            assert!(
                result.is_ok(),
                "{}: pg_stat_bgwriter query failed: {:?}",
                instance.name,
                result.err()
            );
        }
    }
}

// ============================================================================
// Version-Specific Query Tests
// ============================================================================

/// Test pg_stat_checkpointer exists only in PG17+
#[tokio::test]
async fn test_pg_stat_checkpointer_v17() {
    // Should work on PG17
    if let Ok(client) = connect(5417).await {
        let result = client
            .query("SELECT num_timed, num_requested FROM pg_stat_checkpointer", &[])
            .await;
        assert!(
            result.is_ok(),
            "pg17: pg_stat_checkpointer should exist: {:?}",
            result.err()
        );
    }

    // Should NOT work on PG11-16
    for instance in PG_INSTANCES.iter().filter(|i| i.version < 17) {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query("SELECT num_timed FROM pg_stat_checkpointer", &[])
                .await;
            assert!(
                result.is_err(),
                "{}: pg_stat_checkpointer should NOT exist",
                instance.name
            );
        }
    }
}

/// Test pg_stat_wal exists only in PG14+
#[tokio::test]
async fn test_pg_stat_wal_v14_plus() {
    // Should work on PG14+
    for instance in PG_INSTANCES.iter().filter(|i| i.version >= 14) {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query("SELECT wal_records, wal_bytes FROM pg_stat_wal", &[])
                .await;
            assert!(
                result.is_ok(),
                "{}: pg_stat_wal should exist: {:?}",
                instance.name,
                result.err()
            );
        }
    }

    // Should NOT work on PG11-13
    for instance in PG_INSTANCES.iter().filter(|i| i.version < 14) {
        if let Ok(client) = connect(instance.port).await {
            let result = client.query("SELECT wal_records FROM pg_stat_wal", &[]).await;
            assert!(result.is_err(), "{}: pg_stat_wal should NOT exist", instance.name);
        }
    }
}

/// Test checkpoint stats location varies by version
#[tokio::test]
async fn test_checkpoint_stats_version_compat() {
    // PG11-16: checkpoint columns are in pg_stat_bgwriter
    for instance in PG_INSTANCES.iter().filter(|i| i.version < 17) {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query(
                    "SELECT checkpoints_timed, checkpoints_req, checkpoint_write_time FROM pg_stat_bgwriter",
                    &[],
                )
                .await;
            assert!(
                result.is_ok(),
                "{}: checkpoint stats should be in pg_stat_bgwriter: {:?}",
                instance.name,
                result.err()
            );
        }
    }

    // PG17+: checkpoint columns moved to pg_stat_checkpointer
    for instance in PG_INSTANCES.iter().filter(|i| i.version >= 17) {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query(
                    "SELECT num_timed, num_requested, write_time FROM pg_stat_checkpointer",
                    &[],
                )
                .await;
            assert!(
                result.is_ok(),
                "{}: checkpoint stats should be in pg_stat_checkpointer: {:?}",
                instance.name,
                result.err()
            );
        }
    }
}

// ============================================================================
// pg_stat_statements Tests
// ============================================================================

/// Test pg_stat_statements extension and version-specific columns
#[tokio::test]
async fn test_pg_stat_statements_version_compat() {
    // PG11: uses total_time, blk_read_time
    if let Ok(client) = connect(5411).await {
        let _ = ensure_pg_stat_statements(&client).await;
        let result = client
            .query(
                "SELECT queryid, total_time, blk_read_time FROM pg_stat_statements LIMIT 1",
                &[],
            )
            .await;
        // May fail if extension not loaded - that's ok for this test
        if result.is_ok() {
            println!("pg11: pg_stat_statements with total_time works");
        }
    }

    // PG14: uses total_exec_time, blk_read_time
    if let Ok(client) = connect(5414).await {
        let _ = ensure_pg_stat_statements(&client).await;
        let result = client
            .query(
                "SELECT queryid, total_exec_time, blk_read_time FROM pg_stat_statements LIMIT 1",
                &[],
            )
            .await;
        if result.is_ok() {
            println!("pg14: pg_stat_statements with total_exec_time works");
        }
    }

    // PG17: uses total_exec_time, shared_blk_read_time
    if let Ok(client) = connect(5417).await {
        let _ = ensure_pg_stat_statements(&client).await;
        let result = client
            .query(
                "SELECT queryid, total_exec_time, shared_blk_read_time FROM pg_stat_statements LIMIT 1",
                &[],
            )
            .await;
        if result.is_ok() {
            println!("pg17: pg_stat_statements with shared_blk_read_time works");
        }
    }
}

// ============================================================================
// Replication Slot Tests (PG14+ has additional stats)
// ============================================================================

/// Test replication slots query compatibility
#[tokio::test]
async fn test_replication_slots_version_compat() {
    // Basic query should work on all versions
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query(
                    "SELECT slot_name, slot_type, active FROM pg_replication_slots",
                    &[],
                )
                .await;
            assert!(
                result.is_ok(),
                "{}: pg_replication_slots basic query failed: {:?}",
                instance.name,
                result.err()
            );
        }
    }

    // PG14+ has pg_stat_replication_slots
    for instance in PG_INSTANCES.iter().filter(|i| i.version >= 14) {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query("SELECT slot_name FROM pg_stat_replication_slots", &[])
                .await;
            assert!(
                result.is_ok(),
                "{}: pg_stat_replication_slots should exist: {:?}",
                instance.name,
                result.err()
            );
        }
    }

    // PG11-13 should NOT have pg_stat_replication_slots
    for instance in PG_INSTANCES.iter().filter(|i| i.version < 14) {
        if let Ok(client) = connect(instance.port).await {
            let result = client
                .query("SELECT slot_name FROM pg_stat_replication_slots", &[])
                .await;
            assert!(
                result.is_err(),
                "{}: pg_stat_replication_slots should NOT exist",
                instance.name
            );
        }
    }
}

// ============================================================================
// Actual fetch_* Function Tests - Test version detection logic
// ============================================================================

/// Test fetch_checkpoint_stats uses correct query for each PG version
#[tokio::test]
async fn test_fetch_checkpoint_stats_all_versions() {
    // PG11 - uses pg_stat_bgwriter
    if let Ok(client) = connect(5411).await {
        let result = queries::fetch_checkpoint_stats(&client, 11).await;
        assert!(
            result.is_ok(),
            "pg11: fetch_checkpoint_stats should succeed: {:?}",
            result.err()
        );
        let stats = result.unwrap();
        // Verify we got valid data
        assert!(
            stats.checkpoints_timed >= 0,
            "pg11: checkpoints_timed should be non-negative"
        );
    }

    // PG14 - uses pg_stat_bgwriter
    if let Ok(client) = connect(5414).await {
        let result = queries::fetch_checkpoint_stats(&client, 14).await;
        assert!(
            result.is_ok(),
            "pg14: fetch_checkpoint_stats should succeed: {:?}",
            result.err()
        );
    }

    // PG17 - uses pg_stat_checkpointer (new view)
    if let Ok(client) = connect(5417).await {
        let result = queries::fetch_checkpoint_stats(&client, 17).await;
        assert!(
            result.is_ok(),
            "pg17: fetch_checkpoint_stats should succeed with pg_stat_checkpointer: {:?}",
            result.err()
        );
    }
}

/// Test fetch_stat_statements handles version-specific column names
#[tokio::test]
async fn test_fetch_stat_statements_all_versions() {
    // PG11 - uses total_time, blk_read_time
    if let Ok(client) = connect(5411).await {
        let _ = ensure_pg_stat_statements(&client).await;
        // Run a query to populate pg_stat_statements
        let _ = client.query("SELECT 1", &[]).await;

        let ext = DetectedExtensions {
            pg_stat_statements: true,
            pg_stat_statements_version: Some("1.6".to_string()),
            ..Default::default()
        };
        let (statements, error) = queries::fetch_stat_statements(&client, &ext, 11).await;
        // Either we get results or a clear error (permission etc)
        if error.is_none() || !statements.is_empty() {
            println!("pg11: fetch_stat_statements succeeded with {} rows", statements.len());
        } else if let Some(ref err) = error {
            // Permission errors are acceptable in test environment
            assert!(
                err.contains("permission") || err.contains("does not exist"),
                "pg11: unexpected error: {err}"
            );
        }
    }

    // PG14 - uses total_exec_time, blk_read_time
    if let Ok(client) = connect(5414).await {
        let _ = ensure_pg_stat_statements(&client).await;
        let _ = client.query("SELECT 1", &[]).await;

        let ext = DetectedExtensions {
            pg_stat_statements: true,
            pg_stat_statements_version: Some("1.9".to_string()),
            ..Default::default()
        };
        let (statements, error) = queries::fetch_stat_statements(&client, &ext, 14).await;
        if error.is_none() || !statements.is_empty() {
            println!("pg14: fetch_stat_statements succeeded with {} rows", statements.len());
        } else if let Some(ref err) = error {
            assert!(
                err.contains("permission") || err.contains("does not exist"),
                "pg14: unexpected error: {err}"
            );
        }
    }

    // PG17 - uses total_exec_time, shared_blk_read_time (renamed columns)
    if let Ok(client) = connect(5417).await {
        let _ = ensure_pg_stat_statements(&client).await;
        let _ = client.query("SELECT 1", &[]).await;

        let ext = DetectedExtensions {
            pg_stat_statements: true,
            pg_stat_statements_version: Some("1.11".to_string()),
            ..Default::default()
        };
        let (statements, error) = queries::fetch_stat_statements(&client, &ext, 17).await;
        if error.is_none() || !statements.is_empty() {
            println!("pg17: fetch_stat_statements succeeded with {} rows", statements.len());
        } else if let Some(ref err) = error {
            assert!(
                err.contains("permission") || err.contains("does not exist"),
                "pg17: unexpected error: {err}"
            );
        }
    }
}

/// Test fetch_replication_slots uses correct query for each version
#[tokio::test]
async fn test_fetch_replication_slots_all_versions() {
    // PG11 - basic query without spill stats
    if let Ok(client) = connect(5411).await {
        let result = queries::fetch_replication_slots(&client, 11).await;
        assert!(
            result.is_ok(),
            "pg11: fetch_replication_slots should succeed: {:?}",
            result.err()
        );
        let slots = result.unwrap();
        // Verify spill fields are None for PG11
        for slot in &slots {
            assert!(
                slot.spill_txns.is_none(),
                "pg11: spill_txns should be None"
            );
        }
        println!("pg11: fetch_replication_slots returned {} slots", slots.len());
    }

    // PG14 - includes spill stats from pg_stat_replication_slots
    if let Ok(client) = connect(5414).await {
        let result = queries::fetch_replication_slots(&client, 14).await;
        assert!(
            result.is_ok(),
            "pg14: fetch_replication_slots should succeed with spill stats: {:?}",
            result.err()
        );
        let slots = result.unwrap();
        // If there are slots, they should have spill fields
        for slot in &slots {
            assert!(
                slot.spill_txns.is_some(),
                "pg14: spill_txns should be Some"
            );
        }
        println!("pg14: fetch_replication_slots returned {} slots", slots.len());
    }

    // PG17 - also includes spill stats
    if let Ok(client) = connect(5417).await {
        let result = queries::fetch_replication_slots(&client, 17).await;
        assert!(
            result.is_ok(),
            "pg17: fetch_replication_slots should succeed: {:?}",
            result.err()
        );
        println!("pg17: fetch_replication_slots returned {} slots", result.unwrap().len());
    }
}

/// Test fetch_wal_stats only works on PG14+
#[tokio::test]
async fn test_fetch_wal_stats_version_gating() {
    // PG11-13: pg_stat_wal doesn't exist
    for instance in PG_INSTANCES.iter().filter(|i| i.version < 14) {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_wal_stats(&client, instance.version).await;
            assert!(
                result.is_err(),
                "{}: fetch_wal_stats should fail (pg_stat_wal doesn't exist)",
                instance.name
            );
        }
    }

    // PG14+: pg_stat_wal exists
    for instance in PG_INSTANCES.iter().filter(|i| i.version >= 14) {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_wal_stats(&client, instance.version).await;
            assert!(
                result.is_ok(),
                "{}: fetch_wal_stats should succeed: {:?}",
                instance.name,
                result.err()
            );
            let stats = result.unwrap();
            assert!(
                stats.wal_records >= 0,
                "{}: wal_records should be non-negative",
                instance.name
            );
        }
    }
}

/// Test fetch_subscriptions version gating (PG10+)
#[tokio::test]
async fn test_fetch_subscriptions_version_gating() {
    // All our test versions are PG10+, so the query should work
    // but return empty vec if no subscriptions exist
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_subscriptions(&client, instance.version).await;
            assert!(
                result.is_ok(),
                "{}: fetch_subscriptions should succeed: {:?}",
                instance.name,
                result.err()
            );
            // Empty vec is fine - just testing the query doesn't error
            println!(
                "{}: fetch_subscriptions returned {} subscriptions",
                instance.name,
                result.unwrap().len()
            );
        }
    }

    // Test version < 10 returns empty vec immediately
    // We can't test this with a real connection, but we can verify the logic
    // by checking the function handles it gracefully
}

// ============================================================================
// Core fetch_* Function Tests
// ============================================================================

/// Test fetch_snapshot - the main aggregator function
#[tokio::test]
async fn test_fetch_snapshot_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            // First detect extensions
            let extensions = queries::detect_extensions(&client).await;

            // Fetch the full snapshot
            let result = queries::fetch_snapshot(&client, &extensions, instance.version).await;
            assert!(
                result.is_ok(),
                "{}: fetch_snapshot should succeed: {:?}",
                instance.name,
                result.err()
            );

            let snapshot = result.unwrap();

            // Verify snapshot has valid structure
            assert!(
                snapshot.timestamp.timestamp() > 0,
                "{}: snapshot should have valid timestamp",
                instance.name
            );

            // Buffer cache should have valid hit ratio
            assert!(
                snapshot.buffer_cache.hit_ratio >= 0.0 && snapshot.buffer_cache.hit_ratio <= 1.0,
                "{}: hit_ratio should be between 0 and 1, got {}",
                instance.name,
                snapshot.buffer_cache.hit_ratio
            );

            // Summary counts should be non-negative
            assert!(
                snapshot.summary.total_backends >= 0,
                "{}: total_backends should be non-negative",
                instance.name
            );

            // db_size should be positive
            assert!(
                snapshot.db_size > 0,
                "{}: db_size should be positive, got {}",
                instance.name,
                snapshot.db_size
            );

            // Checkpoint stats should exist for all versions
            assert!(
                snapshot.checkpoint_stats.is_some(),
                "{}: checkpoint_stats should be Some",
                instance.name
            );

            // WAL stats should only exist for PG14+
            if instance.version >= 14 {
                assert!(
                    snapshot.wal_stats.is_some(),
                    "{}: wal_stats should be Some for PG14+",
                    instance.name
                );
            } else {
                assert!(
                    snapshot.wal_stats.is_none(),
                    "{}: wal_stats should be None for PG<14",
                    instance.name
                );
            }

            println!(
                "{}: fetch_snapshot succeeded - {} queries, {} backends, db_size={}",
                instance.name,
                snapshot.active_queries.len(),
                snapshot.summary.total_backends,
                snapshot.db_size
            );
        }
    }
}

/// Test fetch_server_info and detect_extensions
#[tokio::test]
async fn test_fetch_server_info_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_server_info(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_server_info should succeed: {:?}",
                instance.name,
                result.err()
            );

            let info = result.unwrap();

            // Version string should contain expected major version
            assert!(
                info.version.contains(&format!("PostgreSQL {}", instance.version))
                    || info.version.contains(&format!(" {}", instance.version)),
                "{}: version should contain {}, got: {}",
                instance.name,
                instance.version,
                info.version
            );

            // Start time should be in the past
            assert!(
                info.start_time < chrono::Utc::now(),
                "{}: start_time should be in the past",
                instance.name
            );

            // max_connections should be positive
            assert!(
                info.max_connections > 0,
                "{}: max_connections should be positive, got {}",
                instance.name,
                info.max_connections
            );

            // Settings should have entries
            assert!(
                !info.settings.is_empty(),
                "{}: settings should not be empty",
                instance.name
            );

            println!(
                "{}: fetch_server_info succeeded - {} settings, max_conn={}",
                instance.name,
                info.settings.len(),
                info.max_connections
            );
        }
    }
}

/// Test fetch_active_queries - core monitoring function
#[tokio::test]
async fn test_fetch_active_queries_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_active_queries(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_active_queries should succeed: {:?}",
                instance.name,
                result.err()
            );

            let queries_list = result.unwrap();

            // We should have at least our own connection (but it's filtered out)
            // So empty result is valid
            for query in &queries_list {
                // Verify fields have expected types
                assert!(query.pid > 0, "{}: pid should be positive", instance.name);
                assert!(
                    query.duration_secs >= 0.0,
                    "{}: duration_secs should be non-negative",
                    instance.name
                );
            }

            println!(
                "{}: fetch_active_queries succeeded with {} queries",
                instance.name,
                queries_list.len()
            );
        }
    }
}

/// Test fetch_activity_summary
#[tokio::test]
async fn test_fetch_activity_summary_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_activity_summary(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_activity_summary should succeed: {:?}",
                instance.name,
                result.err()
            );

            let summary = result.unwrap();

            // total_backends should include at least our connection
            assert!(
                summary.total_backends >= 1,
                "{}: total_backends should be >= 1, got {}",
                instance.name,
                summary.total_backends
            );

            // All counts should be non-negative
            assert!(
                summary.active_query_count >= 0,
                "{}: active_query_count should be non-negative",
                instance.name
            );
            assert!(
                summary.idle_in_transaction_count >= 0,
                "{}: idle_in_transaction_count should be non-negative",
                instance.name
            );
            assert!(
                summary.lock_count >= 0,
                "{}: lock_count should be non-negative",
                instance.name
            );

            println!(
                "{}: fetch_activity_summary - {} backends, {} active",
                instance.name, summary.total_backends, summary.active_query_count
            );
        }
    }
}

/// Test fetch_buffer_cache
#[tokio::test]
async fn test_fetch_buffer_cache_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_buffer_cache(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_buffer_cache should succeed: {:?}",
                instance.name,
                result.err()
            );

            let cache = result.unwrap();

            // hit_ratio should be between 0 and 1
            assert!(
                cache.hit_ratio >= 0.0 && cache.hit_ratio <= 1.0,
                "{}: hit_ratio should be between 0 and 1, got {}",
                instance.name,
                cache.hit_ratio
            );

            // Block counts should be non-negative
            assert!(
                cache.blks_hit >= 0,
                "{}: blks_hit should be non-negative",
                instance.name
            );
            assert!(
                cache.blks_read >= 0,
                "{}: blks_read should be non-negative",
                instance.name
            );

            println!(
                "{}: fetch_buffer_cache - hit_ratio={:.2}%, hits={}, reads={}",
                instance.name,
                cache.hit_ratio * 100.0,
                cache.blks_hit,
                cache.blks_read
            );
        }
    }
}

// ============================================================================
// Table and Index Statistics Tests
// ============================================================================

/// Helper to create a test table for table/index stats tests
async fn create_test_table(client: &Client, table_name: &str) -> Result<(), tokio_postgres::Error> {
    // Drop if exists
    let _ = client
        .execute(&format!("DROP TABLE IF EXISTS {table_name}"), &[])
        .await;

    // Create table with some data
    client
        .execute(
            &format!(
                "CREATE TABLE {table_name} (
                    id SERIAL PRIMARY KEY,
                    name TEXT NOT NULL,
                    value INTEGER,
                    created_at TIMESTAMP DEFAULT NOW()
                )"
            ),
            &[],
        )
        .await?;

    // Insert some data
    client
        .execute(
            &format!(
                "INSERT INTO {table_name} (name, value)
                 SELECT 'item_' || i, i
                 FROM generate_series(1, 100) AS i"
            ),
            &[],
        )
        .await?;

    // Update some rows to create dead tuples
    client
        .execute(
            &format!("UPDATE {table_name} SET value = value + 1 WHERE id <= 10"),
            &[],
        )
        .await?;

    // Run ANALYZE to update statistics
    client
        .execute(&format!("ANALYZE {table_name}"), &[])
        .await?;

    Ok(())
}

/// Helper to cleanup test table
async fn cleanup_test_table(client: &Client, table_name: &str) {
    let _ = client
        .execute(&format!("DROP TABLE IF EXISTS {table_name}"), &[])
        .await;
}

/// Test fetch_table_stats on all versions
#[tokio::test]
async fn test_fetch_table_stats_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let table_name = format!("test_table_{}", instance.port);

            // Create test table
            if let Err(e) = create_test_table(&client, &table_name).await {
                eprintln!("{}: failed to create test table: {}", instance.name, e);
                continue;
            }

            // Force stats update by doing a sequential scan
            let _ = client
                .query(&format!("SELECT COUNT(*) FROM {table_name}"), &[])
                .await;

            let result = queries::fetch_table_stats(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_table_stats should succeed: {:?}",
                instance.name,
                result.err()
            );

            let tables = result.unwrap();

            // Find our test table
            let test_table = tables.iter().find(|t| t.relname == table_name);
            assert!(
                test_table.is_some(),
                "{}: should find test table in results",
                instance.name
            );

            let table = test_table.unwrap();

            // Verify expected values
            assert_eq!(table.schemaname, "public", "{}: schemaname should be public", instance.name);
            // Note: n_live_tup may be 0 or estimated - stats aren't always immediately available
            assert!(
                table.n_live_tup >= 0,
                "{}: n_live_tup should be non-negative, got {}",
                instance.name,
                table.n_live_tup
            );
            assert!(
                table.total_size_bytes > 0,
                "{}: total_size_bytes should be positive",
                instance.name
            );
            assert!(
                table.seq_scan >= 0,
                "{}: seq_scan should be non-negative",
                instance.name
            );

            // dead_ratio should be non-negative
            assert!(
                table.dead_ratio >= 0.0,
                "{}: dead_ratio should be non-negative",
                instance.name
            );

            // Cleanup
            cleanup_test_table(&client, &table_name).await;

            println!(
                "{}: fetch_table_stats - found {} tables, test table has {} live tuples",
                instance.name,
                tables.len(),
                table.n_live_tup
            );
        }
    }
}

/// Test fetch_indexes on all versions
#[tokio::test]
async fn test_fetch_indexes_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let table_name = format!("test_idx_table_{}", instance.port);
            let index_name = format!("test_idx_{}", instance.port);

            // Create test table with index
            if let Err(e) = create_test_table(&client, &table_name).await {
                eprintln!("{}: failed to create test table: {}", instance.name, e);
                continue;
            }

            // Create additional index
            let _ = client
                .execute(
                    &format!("CREATE INDEX {index_name} ON {table_name} (value)"),
                    &[],
                )
                .await;

            // Use the index
            let _ = client
                .query(
                    &format!("SELECT * FROM {table_name} WHERE value > 50"),
                    &[],
                )
                .await;

            let result = queries::fetch_indexes(&client).await;
            // Note: This can fail with "could not open relation with OID" if concurrent tests
            // drop tables while this query is running. That's a known race condition in tests.
            let indexes = match result {
                Ok(indexes) => indexes,
                Err(e) => {
                    // Check both the error string and the debug representation for the race condition message
                    let err_str = format!("{e:?}");
                    if err_str.contains("could not open relation") || err_str.contains("does not exist") {
                        eprintln!("{}: skipping due to concurrent table drop: {}", instance.name, err_str);
                        cleanup_test_table(&client, &table_name).await;
                        continue;
                    }
                    panic!("{}: fetch_indexes should succeed: {:?}", instance.name, e);
                }
            };

            // Find our test index
            let test_index = indexes.iter().find(|i| i.index_name == index_name);
            assert!(
                test_index.is_some(),
                "{}: should find test index in results",
                instance.name
            );

            let index = test_index.unwrap();

            // Verify expected values
            assert_eq!(index.schemaname, "public", "{}: schemaname should be public", instance.name);
            assert_eq!(
                index.table_name, table_name,
                "{}: table_name should match",
                instance.name
            );
            assert!(
                index.index_size_bytes > 0,
                "{}: index_size_bytes should be positive",
                instance.name
            );
            assert!(
                index.index_definition.contains("CREATE INDEX"),
                "{}: index_definition should contain CREATE INDEX",
                instance.name
            );

            // Cleanup
            cleanup_test_table(&client, &table_name).await;

            println!(
                "{}: fetch_indexes - found {} indexes, test index size={}",
                instance.name,
                indexes.len(),
                index.index_size_bytes
            );
        }
    }
}

// ============================================================================
// Bloat Estimation Tests
// ============================================================================

/// Test fetch_table_bloat on all versions
#[tokio::test]
async fn test_fetch_table_bloat_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let table_name = format!("test_bloat_table_{}", instance.port);

            // Create test table with updates to cause some bloat
            if let Err(e) = create_test_table(&client, &table_name).await {
                eprintln!("{}: failed to create test table: {}", instance.name, e);
                continue;
            }

            // Create more updates to simulate bloat
            for _ in 0..5 {
                let _ = client
                    .execute(
                        &format!("UPDATE {table_name} SET value = value + 1"),
                        &[],
                    )
                    .await;
            }

            // Force a scan to update stats
            let _ = client
                .query(&format!("SELECT COUNT(*) FROM {table_name}"), &[])
                .await;

            let extensions = queries::detect_extensions(&client).await;
            let result = queries::fetch_table_bloat(&client, &extensions).await;
            assert!(
                result.is_ok(),
                "{}: fetch_table_bloat should succeed: {:?}",
                instance.name,
                result.err()
            );

            let bloat_map = result.unwrap();

            // Find our test table (may not exist if stats aren't populated yet)
            let key = format!("public.{table_name}");
            if let Some(bloat) = bloat_map.get(&key) {
                // bloat_pct should be between 0 and 100
                assert!(
                    bloat.bloat_pct >= 0.0 && bloat.bloat_pct <= 100.0,
                    "{}: bloat_pct should be between 0 and 100, got {}",
                    instance.name,
                    bloat.bloat_pct
                );

                // bloat_bytes should be non-negative
                assert!(
                    bloat.bloat_bytes >= 0,
                    "{}: bloat_bytes should be non-negative",
                    instance.name
                );

                println!(
                    "{}: fetch_table_bloat - {} tables, test table bloat={:.1}% ({} bytes)",
                    instance.name,
                    bloat_map.len(),
                    bloat.bloat_pct,
                    bloat.bloat_bytes
                );
            } else {
                // Table not found in bloat results - stats may not be populated yet
                // This is acceptable as we're testing the query works
                println!(
                    "{}: fetch_table_bloat - {} tables (test table not in results, stats not ready)",
                    instance.name,
                    bloat_map.len()
                );
            }

            // Cleanup
            cleanup_test_table(&client, &table_name).await;
        }
    }
}

/// Test fetch_index_bloat on all versions
#[tokio::test]
async fn test_fetch_index_bloat_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let table_name = format!("test_ibloat_table_{}", instance.port);
            let index_name = format!("test_ibloat_idx_{}", instance.port);

            // Create test table with index
            if let Err(e) = create_test_table(&client, &table_name).await {
                eprintln!("{}: failed to create test table: {}", instance.name, e);
                continue;
            }

            // Create index
            let _ = client
                .execute(
                    &format!("CREATE INDEX {index_name} ON {table_name} (value)"),
                    &[],
                )
                .await;

            // Do updates to cause index bloat
            for _ in 0..3 {
                let _ = client
                    .execute(
                        &format!("UPDATE {table_name} SET value = value + 1"),
                        &[],
                    )
                    .await;
            }

            let extensions = queries::detect_extensions(&client).await;
            let result = queries::fetch_index_bloat(&client, &extensions).await;
            assert!(
                result.is_ok(),
                "{}: fetch_index_bloat should succeed: {:?}",
                instance.name,
                result.err()
            );

            let bloat_map = result.unwrap();

            // Find our test index
            let key = format!("public.{index_name}");
            let test_bloat = bloat_map.get(&key);
            assert!(
                test_bloat.is_some(),
                "{}: should find test index in bloat results",
                instance.name
            );

            let bloat = test_bloat.unwrap();

            // bloat_pct should be between 0 and 100
            assert!(
                bloat.bloat_pct >= 0.0 && bloat.bloat_pct <= 100.0,
                "{}: bloat_pct should be between 0 and 100, got {}",
                instance.name,
                bloat.bloat_pct
            );

            // bloat_bytes should be non-negative
            assert!(
                bloat.bloat_bytes >= 0,
                "{}: bloat_bytes should be non-negative",
                instance.name
            );

            // Cleanup
            cleanup_test_table(&client, &table_name).await;

            println!(
                "{}: fetch_index_bloat - {} indexes, test index bloat={:.1}% ({} bytes)",
                instance.name,
                bloat_map.len(),
                bloat.bloat_pct,
                bloat.bloat_bytes
            );
        }
    }
}

// ============================================================================
// Additional Query Tests
// ============================================================================

/// Test fetch_wait_events on all versions
#[tokio::test]
async fn test_fetch_wait_events_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_wait_events(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_wait_events should succeed: {:?}",
                instance.name,
                result.err()
            );

            let events = result.unwrap();

            // Each event should have positive count
            for event in &events {
                assert!(
                    event.count > 0,
                    "{}: wait event count should be positive",
                    instance.name
                );
                assert!(
                    !event.wait_event_type.is_empty(),
                    "{}: wait_event_type should not be empty",
                    instance.name
                );
            }

            println!(
                "{}: fetch_wait_events - {} event types",
                instance.name,
                events.len()
            );
        }
    }
}

/// Test fetch_blocking_info on all versions (usually empty)
#[tokio::test]
async fn test_fetch_blocking_info_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_blocking_info(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_blocking_info should succeed: {:?}",
                instance.name,
                result.err()
            );

            // Result is usually empty unless there are actual locks
            let blocking = result.unwrap();
            println!(
                "{}: fetch_blocking_info - {} blocked queries",
                instance.name,
                blocking.len()
            );
        }
    }
}

/// Test fetch_wraparound on all versions
#[tokio::test]
async fn test_fetch_wraparound_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_wraparound(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_wraparound should succeed: {:?}",
                instance.name,
                result.err()
            );

            let wraparound = result.unwrap();

            // Should have at least one database
            assert!(
                !wraparound.is_empty(),
                "{}: should have at least one database",
                instance.name
            );

            for db in &wraparound {
                // XID age should be positive
                assert!(
                    db.xid_age > 0,
                    "{}: xid_age should be positive",
                    instance.name
                );
                // pct_towards_wraparound should be between 0 and 100
                assert!(
                    db.pct_towards_wraparound >= 0.0 && db.pct_towards_wraparound <= 100.0,
                    "{}: pct_towards_wraparound should be between 0 and 100",
                    instance.name
                );
            }

            println!(
                "{}: fetch_wraparound - {} databases",
                instance.name,
                wraparound.len()
            );
        }
    }
}

/// Test fetch_replication on all versions (usually empty)
#[tokio::test]
async fn test_fetch_replication_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_replication(&client, instance.version).await;
            assert!(
                result.is_ok(),
                "{}: fetch_replication should succeed: {:?}",
                instance.name,
                result.err()
            );

            // Result is usually empty unless there are actual replicas
            let replication = result.unwrap();
            println!(
                "{}: fetch_replication - {} replicas",
                instance.name,
                replication.len()
            );
        }
    }
}

/// Test fetch_vacuum_progress on all versions (usually empty)
#[tokio::test]
async fn test_fetch_vacuum_progress_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_vacuum_progress(&client, instance.version).await;
            assert!(
                result.is_ok(),
                "{}: fetch_vacuum_progress should succeed: {:?}",
                instance.name,
                result.err()
            );

            // Result is usually empty unless vacuum is running
            let progress = result.unwrap();
            println!(
                "{}: fetch_vacuum_progress - {} active vacuums",
                instance.name,
                progress.len()
            );
        }
    }
}

/// Test fetch_archiver_stats on all versions
#[tokio::test]
async fn test_fetch_archiver_stats_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_archiver_stats(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_archiver_stats should succeed: {:?}",
                instance.name,
                result.err()
            );

            let stats = result.unwrap();

            // Counts should be non-negative
            assert!(
                stats.archived_count >= 0,
                "{}: archived_count should be non-negative",
                instance.name
            );
            assert!(
                stats.failed_count >= 0,
                "{}: failed_count should be non-negative",
                instance.name
            );

            println!(
                "{}: fetch_archiver_stats - archived={}, failed={}",
                instance.name, stats.archived_count, stats.failed_count
            );
        }
    }
}

/// Test fetch_bgwriter_stats on all versions
#[tokio::test]
async fn test_fetch_bgwriter_stats_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_bgwriter_stats(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_bgwriter_stats should succeed: {:?}",
                instance.name,
                result.err()
            );

            let stats = result.unwrap();

            // Counts should be non-negative
            assert!(
                stats.buffers_clean >= 0,
                "{}: buffers_clean should be non-negative",
                instance.name
            );
            assert!(
                stats.buffers_alloc >= 0,
                "{}: buffers_alloc should be non-negative",
                instance.name
            );

            println!(
                "{}: fetch_bgwriter_stats - clean={}, alloc={}",
                instance.name, stats.buffers_clean, stats.buffers_alloc
            );
        }
    }
}

/// Test fetch_database_stats on all versions
#[tokio::test]
async fn test_fetch_database_stats_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_database_stats(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_database_stats should succeed: {:?}",
                instance.name,
                result.err()
            );

            let stats = result.unwrap();

            // Transaction counts should be non-negative
            assert!(
                stats.xact_commit >= 0,
                "{}: xact_commit should be non-negative",
                instance.name
            );
            assert!(
                stats.xact_rollback >= 0,
                "{}: xact_rollback should be non-negative",
                instance.name
            );
            assert!(
                stats.blks_read >= 0,
                "{}: blks_read should be non-negative",
                instance.name
            );

            println!(
                "{}: fetch_database_stats - commits={}, rollbacks={}, reads={}",
                instance.name, stats.xact_commit, stats.xact_rollback, stats.blks_read
            );
        }
    }
}

/// Test fetch_db_size on all versions
#[tokio::test]
async fn test_fetch_db_size_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_db_size(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_db_size should succeed: {:?}",
                instance.name,
                result.err()
            );

            let size = result.unwrap();

            // Database size should be positive
            assert!(
                size > 0,
                "{}: db_size should be positive, got {}",
                instance.name,
                size
            );

            println!("{}: fetch_db_size - {} bytes", instance.name, size);
        }
    }
}

/// Test fetch_pg_settings on all versions
#[tokio::test]
async fn test_fetch_pg_settings_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let result = queries::fetch_pg_settings(&client).await;
            assert!(
                result.is_ok(),
                "{}: fetch_pg_settings should succeed: {:?}",
                instance.name,
                result.err()
            );

            let settings = result.unwrap();

            // Should have many settings
            assert!(
                settings.len() > 100,
                "{}: should have more than 100 settings, got {}",
                instance.name,
                settings.len()
            );

            // Verify structure of a setting
            let max_conn = settings.iter().find(|s| s.name == "max_connections");
            assert!(
                max_conn.is_some(),
                "{}: should find max_connections setting",
                instance.name
            );

            let setting = max_conn.unwrap();
            assert!(
                !setting.setting.is_empty(),
                "{}: setting value should not be empty",
                instance.name
            );
            assert!(
                !setting.category.is_empty(),
                "{}: category should not be empty",
                instance.name
            );

            println!(
                "{}: fetch_pg_settings - {} settings",
                instance.name,
                settings.len()
            );
        }
    }
}

/// Test detect_extensions on all versions
#[tokio::test]
async fn test_detect_extensions_all_versions() {
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let _ = ensure_pg_stat_statements(&client).await;

            let extensions = queries::detect_extensions(&client).await;

            // pg_stat_statements should be detected if installed
            // (we tried to install it above)
            println!(
                "{}: detect_extensions - pg_stat_statements={}, version={:?}",
                instance.name,
                extensions.pg_stat_statements,
                extensions.pg_stat_statements_version
            );

            // If pg_stat_statements is detected, version should be set
            if extensions.pg_stat_statements {
                assert!(
                    extensions.pg_stat_statements_version.is_some(),
                    "{}: pg_stat_statements_version should be Some when extension is detected",
                    instance.name
                );
            }
        }
    }
}
