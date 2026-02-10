```
        ██████╗  ██████╗     ██████╗ ██╗     ██╗███╗   ███╗██████╗ ███████╗███████╗
        ██╔══██╗██╔════╝    ██╔════╝ ██║     ██║████╗ ████║██╔══██╗██╔════╝██╔════╝
        ██████╔╝██║  ███╗   ██║  ███╗██║     ██║██╔████╔██║██████╔╝███████╗█████╗
        ██╔═══╝ ██║   ██║   ██║   ██║██║     ██║██║╚██╔╝██║██╔═══╝ ╚════██║██╔══╝
        ██║     ╚██████╔╝   ╚██████╔╝███████╗██║██║ ╚═╝ ██║██║     ███████║███████╗
        ╚═╝      ╚═════╝     ╚═════╝ ╚══════╝╚═╝╚═╝     ╚═╝╚═╝     ╚══════╝╚══════╝

                        Real-time PostgreSQL monitoring in your terminal
```

<p align="center">
  <a href="https://github.com/dlt/pg_glimpse/actions/workflows/ci.yml"><img src="https://github.com/dlt/pg_glimpse/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/dlt/pg_glimpse"><img src="https://codecov.io/gh/dlt/pg_glimpse/branch/main/graph/badge.svg" alt="Coverage"></a>
  <a href="https://crates.io/crates/pg_glimpse"><img src="https://img.shields.io/crates/v/pg_glimpse.svg" alt="Crates.io"></a>
  <a href="https://crates.io/crates/pg_glimpse"><img src="https://img.shields.io/crates/d/pg_glimpse.svg" alt="Downloads"></a>
  <a href="LICENSE"><img src="https://img.shields.io/crates/l/pg_glimpse.svg" alt="License"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/rust-1.74%2B-blue.svg" alt="Rust"></a>
</p>

<p align="center">
  <a href="#install">Install</a> •
  <a href="#features">Features</a> •
  <a href="#usage">Usage</a> •
  <a href="#keyboard-reference">Keys</a> •
  <a href="#recording--replay">Replay</a>
</p>

---

> **Note:** This project is under active development. You may encounter bugs or unexpected behavior. If you find any issues, please [open an issue](https://github.com/dlt/pg_glimpse/issues).

A blazing-fast TUI for PostgreSQL. Monitor active queries, connections, locks, cache performance, replication lag, vacuum progress, and more — all from your terminal. Built with Rust and [ratatui](https://ratatui.rs).

![pg_glimpse demo](demo-final.gif)

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
| `A` | **WAL & I/O** | WAL rate, checkpoints, archiver stats (PG14+) |

### Live Graphs

Sparkline graphs tracking:
- Connections
- Average query time
- Cache hit ratio
- Active queries
- Lock count
- TPS (transactions per second)
- WAL write rate

### Stats Overview

Server version, uptime, database size, connection usage, cache hit ratio, dead tuples, wraparound status, replication lag, checkpoint stats, TPS, WAL rate, blocks read/sec, oldest transaction age, autovacuum workers.

### More

- **Inspect overlay** — press `Enter` to see full query details, index definitions, or statement stats
- **Fuzzy filter** — press `/` to filter with match highlighting
- **Clipboard** — press `y` to yank SQL to clipboard
- **SQL highlighting** — syntax-highlighted queries everywhere
- **Themes** — Tokyo Night, Dracula, Nord, Solarized, Catppuccin
- **Recordings browser** — press `L` to browse and replay past sessions

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
| `--ssl` | Enable SSL/TLS connection | — |
| `--ssl-insecure` | SSL without cert verification (RDS/Aurora) | — |
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
| `L` | Recordings browser |
| `y` | Yank to clipboard |
| `/` | Fuzzy filter |

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Previous row |
| `↓` / `j` | Next row |
| `Enter` | Inspect |
| `s` | Cycle sort column |
| `b` | Refresh bloat estimates |
| `C` | Cancel query (batch if filtered) |
| `K` | Terminate backend (batch if filtered) |

## Recording & Replay

Every live session is automatically recorded to `~/.local/share/pg_glimpse/recordings/` (configurable). This is useful for:

- **Incident investigation** — review what happened during an outage
- **Sharing with teammates** — send a recording file for async debugging
- **Post-mortem analysis** — step through events at your own pace

### How it works

- Recordings are saved as JSONL files named `host_port_YYYYMMDD_HHMMSS.jsonl`
- Each snapshot (every refresh interval) is captured with all panel data
- Old recordings are automatically cleaned up based on retention setting (default: 1 hour)

### Browse recordings

Press `L` during a live session to open the recordings browser. Navigate with `↑`/`↓`, press `Enter` to replay, or `d` to delete a recording.

### Replay a session

From the browser, or via command line:

```bash
pg_glimpse --replay ~/.local/share/pg_glimpse/recordings/localhost_5432_20260205_143022.jsonl
```

Recordings auto-play when opened. All panels, sorting, filtering, and inspection work identically in replay mode. Actions that modify the database (Cancel/Kill) are disabled. Press `q` to exit replay and return to live monitoring.

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
| Recordings Dir | Custom path (default: `~/.local/share/pg_glimpse/recordings/`) |

## Extension Support

Automatically detects and integrates with:
- **pg_stat_statements** — query-level stats (powers the Statements panel)

Detected (shown as indicators in stats panel):
- **pg_buffercache** — buffer cache inspection
- **pg_stat_kcache** — OS-level CPU/disk stats
- **pg_wait_sampling** — wait event profiling

## Troubleshooting

**Password with special characters**

If your password contains special characters (`!`, `$`, `"`, etc.), the shell may interpret them before pg_glimpse receives them. Use the `PGPASSWORD` environment variable with single quotes:

```bash
PGPASSWORD='my!pass$word' pg_glimpse -H myhost -d mydb -U myuser
```

**SSL connection to RDS/Aurora**

Cloud-hosted PostgreSQL typically requires SSL but uses certificates not in your system trust store. Use `--ssl-insecure`:

```bash
pg_glimpse --ssl-insecure -H myinstance.rds.amazonaws.com -d mydb -U myuser
```

## FAQ

**Did you build it or did Claude?**

Yes.

## License

[MIT](LICENSE)
