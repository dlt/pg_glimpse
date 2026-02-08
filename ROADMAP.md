# pg_glimpse Roadmap

Feature comparison with pg_activity, pgcenter, and pg_top. Items are roughly prioritized within each section.

## High-Impact Features

### Rate/Delta Statistics
- [x] Per-second rates (TPS, blocks read/sec)
- [x] Delta calculations between snapshots
- [ ] Tuples inserted/updated/deleted per second
- ~~Currently shows absolute counts only~~ Now shows live rates

### WAL & Checkpoint Monitoring
- [x] `pg_stat_wal` (PG14+) - WAL generation rate, buffers, sync time
- [x] `pg_stat_archiver` - archive lag, failed count
- [x] WAL write rate display in sidebar
- [x] Dedicated WAL & I/O panel (key: A)

### System-Level Process Info
- [ ] Per-backend CPU usage
- [ ] Per-backend memory consumption
- [ ] I/O stats per process (reads/writes)
- pg_activity shows these via psutil integration

### Dynamic Connection Switching
- [ ] Connect to multiple databases without restart
- [ ] Switch between monitored instances
- [ ] Connection profiles/bookmarks
- pgcenter excels here

### EXPLAIN Integration
- [ ] Run EXPLAIN on selected query
- [ ] Run EXPLAIN ANALYZE on selected query
- [ ] Plan visualization
- pg_activity has this built-in

### Batch Operations
- [x] Cancel all queries matching current filter
- [x] Kill all queries from a specific user
- [x] Bulk operations with confirmation
- Filter queries, then C/K shows choice: one or all matching

## Medium-Impact Features

### pg_stat_io (PG16+)
- [ ] Detailed I/O statistics by backend type and context
- [ ] Buffer vs direct I/O patterns
- [ ] I/O panel or overlay

### Bloat Estimation
- [x] Table bloat percentage estimates
- [x] Index bloat percentage estimates
- [ ] Suggest VACUUM/REINDEX targets
- [x] Integration with table stats panel (press 'b' to refresh)

### Long Transaction Warnings
- [x] Highlight transactions holding XIDs too long (oldest txn in sidebar)
- [ ] Show impact on wraparound risk
- [x] Color warning when transactions exceed threshold (1h/6h)

### Autovacuum Worker Details
- [ ] List currently running autovacuum workers
- [ ] Show phase, progress, table being processed
- [ ] Distinguish from manual VACUUM in vacuum panel

### Logical Replication
- [ ] `pg_stat_subscription` monitoring
- [ ] `pg_replication_slots` (all types, not just streaming)
- [ ] Publication/subscription lag
- [ ] Slot disk usage

### Log File Tailing
- [ ] View PostgreSQL logs in-app
- [ ] Filter by severity
- [ ] Search within logs
- pgcenter has this

### Export/Save
- [ ] Export current view to CSV
- [ ] Export current view to JSON
- [ ] Save snapshot for later analysis
- [ ] Scheduled exports

## Nice to Have

### Visualization Improvements
- [ ] Blocker tree visualization (graphical lock chain)
- [ ] Session grouping by application_name or user
- [ ] Flame graph style for wait events

### Query Analysis
- [ ] Query fingerprinting - group similar queries
- [ ] Normalized query display
- [ ] Query plan cache stats

### Historical Features
- [ ] Historical comparison (current vs N minutes ago)
- [ ] Trend analysis and anomaly detection
- [ ] Longer-term statistics storage

### Alerts & Notifications
- [ ] Configurable thresholds
- [ ] Desktop notifications
- [ ] Webhook/external alerting

### Configuration
- [ ] PostgreSQL config file viewer
- [ ] Show current postgresql.conf settings
- [ ] Highlight non-default values

### UI Enhancements
- [ ] Custom column selection per panel
- [ ] Resizable panels
- [ ] Multiple layout options

## Quick Wins

Small improvements that add value with minimal effort:

- [x] **TPS counter** in sidebar (commits + rollbacks per second)
- [x] **WAL rate** - bytes/sec of WAL generated
- [x] **Blocks read/sec** - physical I/O rate in sidebar
- [x] **Oldest transaction age** - highlight potential wraparound contributors
- [x] **Autovacuum count** - number of running autovacuum workers (AV in sidebar)
- [ ] **Backend type filter** - filter out background workers from queries panel

## Completed Features

- [x] Active queries panel with sorting and filtering
- [x] Blocking chains visualization
- [x] Wait events panel
- [x] Table statistics with dead tuple tracking
- [x] Replication lag monitoring
- [x] Vacuum progress tracking
- [x] Transaction wraparound monitoring
- [x] Index usage statistics
- [x] pg_stat_statements integration
- [x] Recording and replay
- [x] Multiple color themes
- [x] Fuzzy filtering with match highlighting
- [x] Query cancel/terminate with confirmation
- [x] Batch cancel/kill for filtered queries
- [x] Clipboard copy support
- [x] Configurable refresh interval
- [x] SSL/TLS connection support
- [x] Sparkline graphs for historical data
- [x] SQL syntax highlighting in overlays
- [x] Scroll indicators in overlays
- [x] TPS and WAL rate in sidebar with sparklines
- [x] WAL & I/O panel (pg_stat_wal, checkpoints, archiver)
- [x] Oldest transaction age tracking
- [x] Autovacuum worker count in sidebar
- [x] Bloat estimation for tables and indexes (on-demand with 'b' key)
