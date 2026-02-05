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

- **Inspect overlay** — press `Enter` to see full query details, index definitions, or statement stats
- **Fuzzy filter** — press `/` to filter queries, indexes, or statements
- **Clipboard** — press `y` to yank SQL to clipboard
- **SQL highlighting** — syntax-highlighted queries everywhere
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

Every live session is automatically recorded to `~/.local/share/pg_glimpse/recordings/`. This is useful for:

- **Incident investigation** — review what happened during an outage
- **Sharing with teammates** — send a recording file for async debugging
- **Post-mortem analysis** — step through events at your own pace

### How it works

- Recordings are saved as JSONL files named `host_port_YYYYMMDD_HHMMSS.jsonl`
- Each snapshot (every refresh interval) is captured with all panel data
- Old recordings are automatically cleaned up based on retention setting (default: 1 hour)

### Replay a session

```bash
pg_glimpse --replay ~/.local/share/pg_glimpse/recordings/localhost_5432_20260205_143022.jsonl
```

All panels, sorting, filtering, and inspection work identically in replay mode. Actions that modify the database (Cancel/Kill) are disabled.

### Replay controls

| Key | Action |
|-----|--------|
| `Space` | Play / pause |
| `←` / `h` | Step back one snapshot |
| `→` / `l` | Step forward one snapshot |
| `<` / `>` | Adjust playback speed (0.25x – 8x) |
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

## FAQ

**Did you build it or did Claude?**

Yes.

## License

[MIT](LICENSE)
