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
}

const PG_INSTANCES: &[PgInstance] = &[
    PgInstance {
        name: "pg11",
        port: 5411,
    },
    PgInstance {
        name: "pg14",
        port: 5414,
    },
    PgInstance {
        name: "pg17",
        port: 5417,
    },
];

/// Connect to a PostgreSQL test instance
async fn connect(port: u16) -> Result<Client, tokio_postgres::Error> {
    let connstr = format!(
        "host=localhost port={} user=test password=test dbname=test",
        port
    );
    let (client, connection) = tokio_postgres::connect(&connstr, NoTls).await?;

    // Spawn the connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
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
        assert_eq!(major, 11, "Expected PG11, got PG{}", major);
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
        assert_eq!(major, 14, "Expected PG14, got PG{}", major);
    } else {
        eprintln!("Skipping pg14 test - instance not available");
    }
}

#[tokio::test]
async fn test_pg17_connection() {
    if let Ok(client) = connect(5417).await {
        let row = client.query_one("SELECT version()", &[]).await.unwrap();
        let version: String = row.get(0);
        let major = extract_major_version(&version).unwrap();
        assert_eq!(major, 17, "Expected PG17, got PG{}", major);
    } else {
        eprintln!("Skipping pg17 test - instance not available");
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

    // Should NOT work on PG11 and PG14
    for port in [5411, 5414] {
        if let Ok(client) = connect(port).await {
            let result = client
                .query("SELECT num_timed FROM pg_stat_checkpointer", &[])
                .await;
            assert!(
                result.is_err(),
                "pg{}: pg_stat_checkpointer should NOT exist",
                if port == 5411 { 11 } else { 14 }
            );
        }
    }
}

/// Test pg_stat_wal exists only in PG14+
#[tokio::test]
async fn test_pg_stat_wal_v14_plus() {
    // Should work on PG14 and PG17
    for port in [5414, 5417] {
        if let Ok(client) = connect(port).await {
            let result = client
                .query("SELECT wal_records, wal_bytes FROM pg_stat_wal", &[])
                .await;
            assert!(
                result.is_ok(),
                "pg{}: pg_stat_wal should exist: {:?}",
                if port == 5414 { 14 } else { 17 },
                result.err()
            );
        }
    }

    // Should NOT work on PG11
    if let Ok(client) = connect(5411).await {
        let result = client.query("SELECT wal_records FROM pg_stat_wal", &[]).await;
        assert!(result.is_err(), "pg11: pg_stat_wal should NOT exist");
    }
}

/// Test checkpoint stats location varies by version
#[tokio::test]
async fn test_checkpoint_stats_version_compat() {
    // PG11/14: checkpoint columns are in pg_stat_bgwriter
    for port in [5411, 5414] {
        if let Ok(client) = connect(port).await {
            let result = client
                .query(
                    "SELECT checkpoints_timed, checkpoints_req, checkpoint_write_time FROM pg_stat_bgwriter",
                    &[],
                )
                .await;
            assert!(
                result.is_ok(),
                "pg{}: checkpoint stats should be in pg_stat_bgwriter",
                if port == 5411 { 11 } else { 14 }
            );
        }
    }

    // PG17: checkpoint columns moved to pg_stat_checkpointer
    if let Ok(client) = connect(5417).await {
        let result = client
            .query(
                "SELECT num_timed, num_requested, write_time FROM pg_stat_checkpointer",
                &[],
            )
            .await;
        assert!(
            result.is_ok(),
            "pg17: checkpoint stats should be in pg_stat_checkpointer: {:?}",
            result.err()
        );
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
    for port in [5414, 5417] {
        if let Ok(client) = connect(port).await {
            let result = client
                .query("SELECT slot_name FROM pg_stat_replication_slots", &[])
                .await;
            assert!(
                result.is_ok(),
                "pg{}: pg_stat_replication_slots should exist",
                if port == 5414 { 14 } else { 17 }
            );
        }
    }

    // PG11 should NOT have pg_stat_replication_slots
    if let Ok(client) = connect(5411).await {
        let result = client
            .query("SELECT slot_name FROM pg_stat_replication_slots", &[])
            .await;
        assert!(
            result.is_err(),
            "pg11: pg_stat_replication_slots should NOT exist"
        );
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
                "pg11: unexpected error: {}",
                err
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
                "pg14: unexpected error: {}",
                err
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
                "pg17: unexpected error: {}",
                err
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
    // PG11 - pg_stat_wal doesn't exist
    if let Ok(client) = connect(5411).await {
        let result = queries::fetch_wal_stats(&client).await;
        assert!(
            result.is_err(),
            "pg11: fetch_wal_stats should fail (pg_stat_wal doesn't exist)"
        );
    }

    // PG14 - pg_stat_wal exists
    if let Ok(client) = connect(5414).await {
        let result = queries::fetch_wal_stats(&client).await;
        assert!(
            result.is_ok(),
            "pg14: fetch_wal_stats should succeed: {:?}",
            result.err()
        );
        let stats = result.unwrap();
        assert!(
            stats.wal_records >= 0,
            "pg14: wal_records should be non-negative"
        );
    }

    // PG17 - pg_stat_wal exists
    if let Ok(client) = connect(5417).await {
        let result = queries::fetch_wal_stats(&client).await;
        assert!(
            result.is_ok(),
            "pg17: fetch_wal_stats should succeed: {:?}",
            result.err()
        );
    }
}

/// Test fetch_subscriptions version gating (PG10+)
#[tokio::test]
async fn test_fetch_subscriptions_version_gating() {
    // All our test versions are PG10+, so the query should work
    // but return empty vec if no subscriptions exist
    for instance in PG_INSTANCES {
        if let Ok(client) = connect(instance.port).await {
            let version = match instance.port {
                5411 => 11,
                5414 => 14,
                5417 => 17,
                _ => continue,
            };
            let result = queries::fetch_subscriptions(&client, version).await;
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
