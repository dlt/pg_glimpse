# pg_glimpse

Terminal-based PostgreSQL monitoring tool. Real-time visibility into active queries, connections, locks, cache performance, replication lag, and more.

## Install

**Homebrew** (macOS):

```bash
brew install dlt/tap/pg_glimpse
```

**Cargo** (any platform with Rust 1.74+):

```bash
cargo install pg_glimpse
```

**Binary download**: grab a prebuilt binary from [GitHub Releases](https://github.com/dlt/pg_glimpse/releases).

## Usage

```bash
# Connect with individual parameters
pg_glimpse -H localhost -p 5432 -d mydb -U postgres

# Connection string
pg_glimpse -c "host=localhost port=5432 dbname=mydb user=postgres"

# PostgreSQL URI
pg_glimpse -c "postgresql://user:pass@host:5432/dbname"

# Custom refresh interval and history length
pg_glimpse -r 1 --history-length 240

# Replay a recorded session
pg_glimpse --replay ~/.local/share/pg_glimpse/recordings/localhost_5432_20260204_143022.jsonl
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-c`, `--connection` | Connection string (overrides individual params) | -- |
| `-H`, `--host` | PostgreSQL host | `localhost` |
| `-p`, `--port` | PostgreSQL port | `5432` |
| `-d`, `--dbname` | Database name | `postgres` |
| `-U`, `--user` | Database user | `postgres` |
| `-W`, `--password` | Database password | -- |
| `-r`, `--refresh` | Refresh interval in seconds | `2` |
| `--history-length` | Sparkline data points to keep | `120` |
| `--replay` | Replay a recorded JSONL session file | -- |

### Environment Variables

Standard PostgreSQL environment variables are supported: `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, `PGPASSWORD`, and `PG_GLIMPSE_CONNECTION`.

## Features

### Panels

| Key | Panel | Description |
|-----|-------|-------------|
| (default) | **Queries** | Active queries with PID, user, state, duration, wait events. Cancel or kill queries. |
| `Tab` | **Blocking** | Lock blocking chains showing blocked/blocker pairs |
| `w` | **Wait Events** | Breakdown of what backends are waiting on |
| `t` | **Table Stats** | Row counts, dead tuples, dead ratio, last vacuum, sizes |
| `R` | **Replication** | Streaming replication lag (write/flush/replay) |
| `v` | **Vacuum** | In-progress vacuum operations with phase and progress |
| `x` | **Wraparound** | Transaction ID age and XID wraparound risk |
| `I` | **Indexes** | Scan counts, tuple reads/fetches, sizes, definitions |
| `S` | **Statements** | pg_stat_statements: timing, calls, rows, buffer usage |

### Live Graphs

Sparkline graphs for connections, average query time, cache hit ratio, active queries, and lock count. Configurable marker style (Braille, HalfBlock, Block).

### Stats Panel

Server version, uptime, database size, connection usage, activity summary, cache hit ratio, dead tuple count, wraparound status, replication lag, and checkpoint stats.

### Recording & Replay

Every monitoring snapshot is automatically recorded to `~/.local/share/pg_glimpse/recordings/` as JSONL. Old recordings are pruned on startup (default: 1 hour retention, configurable).

Replay a recorded session:

```bash
pg_glimpse --replay recording.jsonl
```

All panels, sorting, inspection, filtering, and yank work identically in replay mode.

| Key | Action |
|-----|--------|
| `Space` | Play / pause |
| `Left` / `h` | Step back one snapshot |
| `Right` / `l` | Step forward one snapshot |
| `<` | Slow down (0.25x, 0.5x, 1x, 2x, 4x, 8x) |
| `>` | Speed up |
| `g` | Jump to start |
| `G` | Jump to end |

### Fuzzy Filter

Press `/` in Queries, Indexes, or Statements panels to fuzzy-filter rows. Matches against all visible fields.

### Clipboard

Press `y` to yank the selected query text, index definition, or statement to the system clipboard.

## Keyboard Reference

### Global

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `Esc` | Back to Queries (or quit) |
| `p` | Pause / resume |
| `r` | Force refresh |
| `?` | Help |
| `,` | Configuration |
| `y` | Yank to clipboard |
| `/` | Fuzzy filter |

### Panel Controls

| Key | Action |
|-----|--------|
| `Up` / `k` | Previous row |
| `Down` / `j` | Next row |
| `Enter` | Inspect |
| `s` | Cycle sort column |
| `C` | Cancel query (Queries panel) |
| `K` | Terminate backend (Queries panel) |

### Configuration

Press `,` to open the config overlay. Use `Left`/`Right` to adjust values. Settings are saved to `~/.config/pg_glimpse/config.toml`.

| Setting | Range | Default |
|---------|-------|---------|
| Graph Marker | Braille / HalfBlock / Block | Braille |
| Color Theme | Tokyo Night / Dracula / Nord / Solarized Dark | Tokyo Night |
| Refresh Interval | 1-60s | 2s |
| Warn Duration | 0.1s - danger threshold | 1.0s |
| Danger Duration | warn threshold - 300s | 10.0s |
| Recording Retention | 10m - 24h | 1h |

## Extension Support

pg_glimpse automatically detects and uses these PostgreSQL extensions when available:

- **pg_stat_statements** -- query-level statistics
- **pg_buffercache** -- buffer cache inspection
- **pg_stat_kcache** -- OS-level cache stats
- **pg_wait_sampling** -- wait event profiling

Panels that depend on missing extensions are gracefully skipped.

## License

[MIT](LICENSE)
