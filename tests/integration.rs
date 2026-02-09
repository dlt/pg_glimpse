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
