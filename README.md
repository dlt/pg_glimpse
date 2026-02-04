```
┌─────────────────────────────────────────────┐
│                                             │
│   ╔═╗╔═╗  ╔═╗╦  ╦╔╦╗╔═╗╔═╗╔═╗             │
│   ╠═╝║ ╦  ║ ╦║  ║║║║╠═╝╚═╗║╣              │
│   ╩  ╚═╝  ╚═╝╩═╝╩╩ ╩╩  ╚═╝╚═╝             │
│                                             │
│   Terminal-based PostgreSQL monitoring       │
│                                             │
└─────────────────────────────────────────────┘
```

pg_glimpse gives you real-time visibility into your PostgreSQL database — active queries, connections, locks, cache performance, replication lag, and more — all from your terminal.

## Features

- **Active queries** — view, inspect, cancel, or terminate running queries
- **Live sparkline graphs** — connections, average query time, cache hit ratio, lock count
- **Lock blocking chains** — see which queries are blocking others
- **Wait events** — breakdown of what backends are waiting on
- **Table statistics** — row counts, dead tuples, last vacuum/analyze
- **Replication status** — streaming replication lag monitoring
- **Vacuum progress** — track in-progress vacuum operations
- **XID wraparound** — transaction ID age monitoring
- **Index stats** — scan counts, tuple reads, index sizes

## Installation

Requires Rust 1.74+.

```bash
cargo build --release
```

The binary will be at `target/release/pg_glimpse`.

## Usage

```bash
# Connect using individual parameters
pg_glimpse -H localhost -p 5432 -d mydb -U postgres

# Connect using a connection string
pg_glimpse -c "host=localhost port=5432 dbname=mydb user=postgres"

# Connect using a PostgreSQL URI
pg_glimpse -c "postgresql://user:pass@host:5432/dbname"

# Custom refresh interval (seconds) and history length
pg_glimpse -r 1 --history-length 240
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-c`, `--connection` | Connection string (overrides individual params) | — |
| `-H`, `--host` | PostgreSQL host | `localhost` |
| `-p`, `--port` | PostgreSQL port | `5432` |
| `-d`, `--dbname` | Database name | `postgres` |
| `-U`, `--user` | Database user | `postgres` |
| `-W`, `--password` | Database password | — |
| `-r`, `--refresh` | Refresh interval in seconds | `2` |
| `--history-length` | Number of sparkline data points | `120` |

### Environment Variables

Connection parameters can also be set via standard PostgreSQL environment variables:

- `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, `PGPASSWORD`
- `PG_GLIMPSE_CONNECTION` — full connection string

## Keyboard Shortcuts

### Main View

| Key | Action |
|-----|--------|
| `q` / `Esc` / `Ctrl+C` | Quit |
| `p` | Pause/resume refresh |
| `r` | Force refresh |
| `↑` / `k` | Select previous query |
| `↓` / `j` | Select next query |
| `Enter` / `i` | Inspect selected query |
| `C` | Cancel selected query (`pg_cancel_backend`) |
| `K` | Terminate selected backend (`pg_terminate_backend`) |
| `s` | Cycle sort column (Duration → PID → User → State) |
| `Tab` | Lock blocking chains |
| `w` | Wait events |
| `t` | Table statistics |
| `R` | Replication status |
| `v` | Vacuum progress |
| `x` | XID wraparound |
| `I` | Indexes |

### Overlay Views

| Key | Action |
|-----|--------|
| `Esc` / `q` | Close overlay |
| `↑↓` / `jk` | Navigate (Indexes view) |
| `Enter` | Inspect selected index (Indexes view) |
| `s` | Cycle sort column (Indexes view) |

## License

MIT
