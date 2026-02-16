# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Git Policy

**Never push to remote without explicit user confirmation.** Always wait for the user to say "push" or confirm before running `git push`.

## Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run -- -H localhost -d postgres   # Run with connection params
cargo run -- --ssl-insecure -H rds-host -d mydb -U user -W pass  # RDS/cloud with SSL
```

## Testing

```bash
cargo test                # Unit tests only

# Integration tests (requires Docker)
docker compose -f tests/docker-compose.yml up -d
cargo test --features integration --test integration
docker compose -f tests/docker-compose.yml down -v

# mTLS integration tests (requires test certificates)
./tests/fixtures/generate_test_certs.sh  # Generate certificates first
docker compose -f tests/docker-compose.yml up -d pg-mtls
cargo test --features integration --test integration test_mtls
docker compose -f tests/docker-compose.yml down -v
```

Integration tests run against PG11-PG18 to verify version-aware query logic. mTLS tests verify client certificate authentication against a PostgreSQL instance configured to require certificates.

## Architecture

Rust TUI application using ratatui + crossterm + tokio + tokio-postgres.

### Core Flow

- `src/main.rs` — Entry point with `run()` (live mode) and `run_replay()` (replay mode) async functions. Spawns background task for DB queries, handles event loop.
- `src/app.rs` — App state and key handling. Key dispatch is layered: modal → global → panel-switch → panel-specific.
- `src/cli.rs` — CLI argument parsing with clap.
- `src/config.rs` — AppConfig with ConfigItem enum for the settings overlay.

### Database Layer (`src/db/`)

- `models.rs` — All data structs (PgSnapshot, ServerInfo, DetectedExtensions, etc.). All derive `Serialize, Deserialize` for recording/replay.
- `queries.rs` — SQL queries with version-aware variants (PG11/13/14/15/17 differences). Key functions: `fetch_snapshot()`, `fetch_server_info()`, `detect_extensions()`.

### UI Layer (`src/ui/`)

- `panels.rs` — Main panel renderers (queries, blocking, tables, replication, etc.)
- `overlay.rs` — Modal overlays (help, config, query inspect, filter)
- `stats_panel.rs` — Server stats sidebar with sparklines
- `theme.rs` — Color themes (Tokyo Night, Dracula, Nord, etc.)

### Recording/Replay

- `src/recorder.rs` — JSONL writer to `~/.local/share/pg_glimpse/recordings/`
- `src/replay.rs` — JSONL reader, loads snapshots into memory
- Format: first line `{"type":"header",...}`, rest `{"type":"snapshot","data":{...}}`

### SSL/TLS and Client Certificates

- `src/ssl.rs` — Certificate loading and validation for client certificate authentication (mutual TLS)
- `src/connection.rs` — TLS configuration with optional client certificates, supports verified and insecure modes
- Certificate paths can be specified via CLI args (`--ssl-cert`, `--ssl-key`, `--ssl-root-cert`), environment variables (`PGSSLCERT`, `PGSSLKEY`, `PGSSLROOTCERT`), service file (`sslcert`, `sslkey`, `sslrootcert`), or auto-detected from `~/.postgresql/` directory

## Key Patterns

**Adding a new ConfigItem:** Update the enum, ALL array, `label()`, `config_adjust()` in app.rs, and the overlay.rs match block.

**Version-aware queries:** Check `queries.rs` for patterns. Use the PG major version or extension version to select the right SQL. Column names change between versions (e.g., `num_dead_tuples` → `dead_tuple_count` in PG17).

**SSL connections:** Use `--ssl` for verified TLS, `--ssl-insecure` for cloud DBs with custom CAs (AWS RDS, Aurora).

**Client certificate authentication:** Set both certificate path (`--ssl-cert`) and private key path (`--ssl-key`). The `SslCertConfig` struct in `ssl.rs` tracks certificate paths with precedence: CLI args > env vars > service file > defaults. The `build_tls_config()` function in `connection.rs` conditionally loads client certificates if `has_client_cert()` returns true.

## Release Process

```bash
# 1. Bump version in Cargo.toml
# 2. Build and commit
cargo build --release
git add Cargo.toml Cargo.lock && git commit -m "Bump version to X.Y.Z"
git push

# 3. Tag and push (triggers GitHub Actions release)
git tag -a vX.Y.Z -m "vX.Y.Z" && git push origin vX.Y.Z

# 4. After release assets are built, update formula
# Get SHA256 hashes from release assets, update Formula/pg_glimpse.rb
# Copy to tap: cp Formula/pg_glimpse.rb /opt/homebrew/Library/Taps/dlt/homebrew-tap/Formula/
```
