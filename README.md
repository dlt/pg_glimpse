```
        ██████╗  ██████╗     ██████╗ ██╗     ██╗███╗   ███╗██████╗ ███████╗███████╗
        ██╔══██╗██╔════╝    ██╔════╝ ██║     ██║████╗ ████║██╔══██╗██╔════╝██╔════╝
        ██████╔╝██║  ███╗   ██║  ███╗██║     ██║██╔████╔██║██████╔╝███████╗█████╗
        ██╔═══╝ ██║   ██║   ██║   ██║██║     ██║██║╚██╔╝██║██╔═══╝ ╚════██║██╔══╝
        ██║     ╚██████╔╝   ╚██████╔╝███████╗██║██║ ╚═╝ ██║██║     ███████║███████╗
        ╚═╝      ╚═════╝     ╚═════╝ ╚══════╝╚═╝╚═╝     ╚═╝╚═╝     ╚══════╝╚══════╝

                        Real-time PostgreSQL monitoring in your terminal
```

[Install](#install) • [Features](#features) • [Usage](#usage) • [Keys](#keyboard-reference) • [Replay](#recording--replay)

---

A blazing-fast TUI for PostgreSQL. Monitor active queries, connections, locks, cache performance, replication lag, vacuum progress, and more — all from your terminal.

## Install

**Homebrew** (macOS):
```bash
brew install dlt/tap/pg_glimpse
```

**Cargo** (any platform with Rust 1.74+):
```bash
cargo install pg_glimpse
```

**Binary**: grab a prebuilt binary from [Releases](https://github.com/dlt/pg_glimpse/releases).

## Features

### Panels

| Key | Panel | What you see |
|:---:|-------|--------------|
| — | **Queries** | Active queries with PID, user, state, duration, wait events |
| `Tab` | **Blocking** | Lock blocking chains — who's waiting on whom |
| `w` | **Wait Events** | What backends are waiting on |
| `t` | **Table Stats** | Dead tuples, bloat, sizes, last vacuum |
| `R` | **Replication** | Streaming replica lag (write/flush/replay) |
| `v` | **Vacuum** | Live vacuum progress with phase |
| `x` | **Wraparound** | XID age and wraparound risk |
| `I` | **Indexes** | Scan counts, tuple reads, sizes |
| `S` | **Statements** | pg_stat_statements metrics |

### Live Graphs

Sparkline graphs tracking:
- Connections
- Average query time
- Cache hit ratio
- Active queries
- Lock count

### Stats Overview

Server version, uptime, database size, connection usage, cache hit ratio, dead tuples, wraparound status, replication lag, checkpoint stats.

### More

- **Fuzzy filter** — press `/` to filter queries, indexes, or statements
- **Clipboard** — press `y` to yank SQL to clipboard
- **SQL highlighting** — syntax-highlighted queries in inspect views
- **Themes** — Tokyo Night, Dracula, Nord, Solarized, Catppuccin

## Usage

```bash
# Connect with parameters
pg_glimpse -H localhost -p 5432 -d mydb -U postgres

# Connection string
pg_glimpse -c "host=localhost port=5432 dbname=mydb user=postgres"

# PostgreSQL URI
pg_glimpse -c "postgresql://user:pass@host:5432/dbname"

# Custom refresh interval
pg_glimpse -r 1 --history-length 240
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-c`, `--connection` | Connection string | — |
| `-H`, `--host` | PostgreSQL host | `localhost` |
| `-p`, `--port` | PostgreSQL port | `5432` |
| `-d`, `--dbname` | Database name | `postgres` |
| `-U`, `--user` | Database user | `postgres` |
| `-W`, `--password` | Database password | — |
| `-r`, `--refresh` | Refresh interval (seconds) | `2` |
| `--history-length` | Sparkline data points | `120` |
| `--replay` | Replay a recorded session | — |

### Environment Variables

`PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, `PGPASSWORD`, `PG_GLIMPSE_CONNECTION`

## Keyboard Reference

### Global

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `Esc` | Back to Queries / Quit |
| `p` | Pause / resume |
| `r` | Force refresh |
| `?` | Help |
| `,` | Configuration |
| `y` | Yank to clipboard |
| `/` | Fuzzy filter |

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Previous row |
| `↓` / `j` | Next row |
| `Enter` | Inspect |
| `s` | Cycle sort column |
| `C` | Cancel query |
| `K` | Terminate backend |

## Recording & Replay

Sessions are automatically recorded to `~/.local/share/pg_glimpse/recordings/`.

```bash
pg_glimpse --replay recording.jsonl
```

| Key | Action |
|-----|--------|
| `Space` | Play / pause |
| `←` / `h` | Step back |
| `→` / `l` | Step forward |
| `<` / `>` | Adjust speed |
| `g` / `G` | Jump to start / end |

## Configuration

Press `,` to open settings. Saved to `~/.config/pg_glimpse/config.toml`.

| Setting | Options |
|---------|---------|
| Graph Marker | Braille / HalfBlock / Block |
| Color Theme | Tokyo Night / Dracula / Nord / Solarized / Catppuccin |
| Refresh Interval | 1–60s |
| Warn Duration | 0.1s+ |
| Danger Duration | warn threshold – 300s |
| Recording Retention | 10m – 24h |

## Extension Support

Automatically detects and uses:
- **pg_stat_statements** — query-level stats
- **pg_buffercache** — buffer cache inspection
- **pg_stat_kcache** — OS-level cache stats
- **pg_wait_sampling** — wait event profiling

## License

[MIT](LICENSE)
