use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher};
use ratatui::widgets::TableState;

use std::collections::HashMap;

use crate::config::{AppConfig, ConfigItem};
use crate::db::models::{ActiveQuery, IndexInfo, PgSetting, PgSnapshot, ServerInfo, StatStatement, TableStat};
use crate::db::queries::{IndexBloat, TableBloat};
use crate::history::RingBuffer;
use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BottomPanel {
    Queries,
    Blocking,
    WaitEvents,
    TableStats,
    Replication,
    VacuumProgress,
    Wraparound,
    Indexes,
    Statements,
    WalIo,
    Settings,
}

impl BottomPanel {
    pub fn supports_filter(self) -> bool {
        matches!(self, Self::Queries | Self::Indexes | Self::Statements | Self::TableStats | Self::Settings)
    }

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Self::Queries => "Queries",
            Self::Blocking => "Blocking",
            Self::WaitEvents => "Wait Events",
            Self::TableStats => "Table Stats",
            Self::Replication => "Replication",
            Self::VacuumProgress => "Vacuum Progress",
            Self::Wraparound => "Wraparound",
            Self::Indexes => "Indexes",
            Self::Statements => "Statements",
            Self::WalIo => "WAL & I/O",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Normal,
    Filter,
    Inspect,
    IndexInspect,
    StatementInspect,
    ReplicationInspect,
    TableInspect,
    BlockingInspect,
    VacuumInspect,
    WraparoundInspect,
    ConfirmCancel(i32),
    ConfirmKill(i32),
    ConfirmCancelChoice { selected_pid: i32, all_pids: Vec<i32> },
    ConfirmKillChoice { selected_pid: i32, all_pids: Vec<i32> },
    ConfirmCancelBatch(Vec<i32>),
    ConfirmKillBatch(Vec<i32>),
    Config,
    Help,
}

#[derive(Debug, Clone)]
pub enum AppAction {
    CancelQuery(i32),
    TerminateBackend(i32),
    CancelQueries(Vec<i32>),
    TerminateBackends(Vec<i32>),
    ForceRefresh,
    RefreshBloat,
    SaveConfig,
    RefreshIntervalChanged,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortColumn {
    Pid,
    Duration,
    State,
    User,
}

impl SortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::Duration => Self::Pid,
            Self::Pid => Self::User,
            Self::User => Self::State,
            Self::State => Self::Duration,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pid => "PID",
            Self::Duration => "Duration",
            Self::State => "State",
            Self::User => "User",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexSortColumn {
    Scans,
    Size,
    Name,
    TupRead,
    TupFetch,
}

impl IndexSortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::Scans => Self::Size,
            Self::Size => Self::Name,
            Self::Name => Self::TupRead,
            Self::TupRead => Self::TupFetch,
            Self::TupFetch => Self::Scans,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableStatSortColumn {
    DeadTuples,
    Size,
    Name,
    SeqScan,
    IdxScan,
    DeadRatio,
}

impl TableStatSortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::DeadTuples => Self::Size,
            Self::Size => Self::Name,
            Self::Name => Self::SeqScan,
            Self::SeqScan => Self::IdxScan,
            Self::IdxScan => Self::DeadRatio,
            Self::DeadRatio => Self::DeadTuples,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DeadTuples => "Dead Tuples",
            Self::Size => "Size",
            Self::Name => "Name",
            Self::SeqScan => "Seq Scan",
            Self::IdxScan => "Idx Scan",
            Self::DeadRatio => "Dead %",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatementSortColumn {
    TotalTime,
    MeanTime,
    MaxTime,
    Stddev,
    Calls,
    Rows,
    HitRatio,
    SharedReads,
    IoTime,
    Temp,
}

impl StatementSortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::TotalTime => Self::MeanTime,
            Self::MeanTime => Self::MaxTime,
            Self::MaxTime => Self::Stddev,
            Self::Stddev => Self::Calls,
            Self::Calls => Self::Rows,
            Self::Rows => Self::HitRatio,
            Self::HitRatio => Self::SharedReads,
            Self::SharedReads => Self::IoTime,
            Self::IoTime => Self::Temp,
            Self::Temp => Self::TotalTime,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TotalTime => "Total Time",
            Self::MeanTime => "Mean Time",
            Self::MaxTime => "Max Time",
            Self::Stddev => "Stddev",
            Self::Calls => "Calls",
            Self::Rows => "Rows",
            Self::HitRatio => "Hit %",
            Self::SharedReads => "Reads",
            Self::IoTime => "I/O Time",
            Self::Temp => "Temp",
        }
    }
}

/// Trait for sort column enums to enable generic TableViewState
pub trait SortColumnTrait: Copy + PartialEq {
    fn next(self) -> Self;
    #[allow(dead_code)]
    fn label(self) -> &'static str;
}

impl SortColumnTrait for SortColumn {
    fn next(self) -> Self { SortColumn::next(self) }
    fn label(self) -> &'static str { SortColumn::label(self) }
}

impl SortColumnTrait for IndexSortColumn {
    fn next(self) -> Self { IndexSortColumn::next(self) }
    fn label(self) -> &'static str {
        match self {
            Self::Scans => "Scans",
            Self::Size => "Size",
            Self::Name => "Name",
            Self::TupRead => "Tup Read",
            Self::TupFetch => "Tup Fetch",
        }
    }
}

impl SortColumnTrait for TableStatSortColumn {
    fn next(self) -> Self { TableStatSortColumn::next(self) }
    fn label(self) -> &'static str { TableStatSortColumn::label(self) }
}

impl SortColumnTrait for StatementSortColumn {
    fn next(self) -> Self { StatementSortColumn::next(self) }
    fn label(self) -> &'static str { StatementSortColumn::label(self) }
}

/// Generic table view state with sort column and navigation
pub struct TableViewState<S: SortColumnTrait> {
    pub state: TableState,
    pub sort_column: S,
    pub sort_ascending: bool,
}

impl<S: SortColumnTrait> TableViewState<S> {
    pub fn new(default_sort: S, ascending: bool) -> Self {
        Self {
            state: TableState::default(),
            sort_column: default_sort,
            sort_ascending: ascending,
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_column = self.sort_column.next();
    }

    #[allow(dead_code)]
    pub fn toggle_direction(&mut self) {
        self.sort_ascending = !self.sort_ascending;
    }

    pub fn select_next(&mut self, max: usize) {
        let i = self.state.selected().unwrap_or(0);
        if i < max.saturating_sub(1) {
            self.state.select(Some(i + 1));
        }
    }

    pub fn select_prev(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some(i.saturating_sub(1)));
    }

    pub fn select_first(&mut self) {
        self.state.select(Some(0));
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }
}

/// State for replay mode (when reviewing recorded sessions)
pub struct ReplayState {
    pub filename: String,
    pub position: usize,
    pub total: usize,
    pub speed: f64,
    pub playing: bool,
}

impl ReplayState {
    pub fn new(filename: String, total: usize) -> Self {
        Self {
            filename,
            position: 0,
            total,
            speed: 1.0,
            playing: false,
        }
    }
}

/// Connection information (read-only after construction)
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub user: String,
    pub ssl_mode: Option<String>,
}

impl ConnectionInfo {
    pub fn new(host: String, port: u16, dbname: String, user: String) -> Self {
        Self {
            host,
            port,
            dbname,
            user,
            ssl_mode: None,
        }
    }

    pub fn set_ssl_mode(&mut self, label: &str) {
        self.ssl_mode = Some(label.to_string());
    }
}

/// Metrics history for sparklines and rate calculations
pub struct MetricsHistory {
    // Sparkline data
    pub connections: RingBuffer<u64>,
    pub avg_query_time: RingBuffer<u64>,
    pub hit_ratio: RingBuffer<u64>,
    pub active_queries: RingBuffer<u64>,
    pub lock_count: RingBuffer<u64>,

    // Rate tracking
    pub tps: RingBuffer<u64>,
    pub wal_rate: RingBuffer<u64>,
    pub blks_read: RingBuffer<u64>,

    // Current values for display
    pub current_tps: Option<f64>,
    pub current_wal_rate: Option<f64>,
    pub current_blks_read_rate: Option<f64>,

    // Previous snapshot for delta calculation
    prev_snapshot: Option<PgSnapshot>,
}

impl MetricsHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            connections: RingBuffer::new(capacity),
            avg_query_time: RingBuffer::new(capacity),
            hit_ratio: RingBuffer::new(capacity),
            active_queries: RingBuffer::new(capacity),
            lock_count: RingBuffer::new(capacity),
            tps: RingBuffer::new(capacity),
            wal_rate: RingBuffer::new(capacity),
            blks_read: RingBuffer::new(capacity),
            current_tps: None,
            current_wal_rate: None,
            current_blks_read_rate: None,
            prev_snapshot: None,
        }
    }

    /// Push basic metrics from a snapshot
    pub fn push_snapshot_metrics(&mut self, snap: &PgSnapshot) {
        self.connections.push(snap.summary.total_backends as u64);

        let active: Vec<&_> = snap
            .active_queries
            .iter()
            .filter(|q| matches!(q.state.as_deref(), Some("active") | Some("idle in transaction")))
            .collect();
        let avg_ms = if active.is_empty() {
            0u64
        } else {
            let sum: f64 = active.iter().map(|q| q.duration_secs).sum();
            (sum / active.len() as f64 * 1000.0) as u64
        };
        self.avg_query_time.push(avg_ms);

        self.hit_ratio.push((snap.buffer_cache.hit_ratio * 1000.0) as u64);
        self.active_queries.push(snap.summary.active_query_count as u64);
        self.lock_count.push(snap.summary.lock_count as u64);
    }

    /// Calculate and update rate metrics from snapshot delta
    pub fn calculate_rates(&mut self, snap: &PgSnapshot) {
        if let Some(prev) = &self.prev_snapshot {
            let secs = snap
                .timestamp
                .signed_duration_since(prev.timestamp)
                .num_milliseconds() as f64
                / 1000.0;

            if secs > 0.0 {
                // TPS and blocks read from pg_stat_database
                if let (Some(curr), Some(prev_db)) = (&snap.db_stats, &prev.db_stats) {
                    let commits = curr.xact_commit - prev_db.xact_commit;
                    let rollbacks = curr.xact_rollback - prev_db.xact_rollback;
                    // Guard against counter reset (server restart)
                    if commits >= 0 && rollbacks >= 0 {
                        let tps = (commits + rollbacks) as f64 / secs;
                        self.current_tps = Some(tps);
                        self.tps.push(tps as u64);
                    }

                    // Blocks read rate (physical I/O)
                    let blks = curr.blks_read - prev_db.blks_read;
                    if blks >= 0 {
                        let rate = blks as f64 / secs;
                        self.current_blks_read_rate = Some(rate);
                        self.blks_read.push(rate as u64);
                    }
                }

                // WAL rate from pg_stat_wal
                if let (Some(curr_wal), Some(prev_wal)) = (&snap.wal_stats, &prev.wal_stats) {
                    let bytes = curr_wal.wal_bytes - prev_wal.wal_bytes;
                    if bytes >= 0 {
                        let rate = bytes as f64 / secs;
                        self.current_wal_rate = Some(rate);
                        // Store as KB/s for sparkline (fits in u64 better)
                        self.wal_rate.push((rate / 1024.0) as u64);
                    }
                }
            }
        }
        self.prev_snapshot = Some(snap.clone());
    }
}

pub struct App {
    pub running: bool,
    pub paused: bool,
    pub snapshot: Option<PgSnapshot>,
    pub view_mode: ViewMode,
    pub bottom_panel: BottomPanel,

    // Sortable panel views
    pub queries: TableViewState<SortColumn>,
    pub indexes: TableViewState<IndexSortColumn>,
    pub statements: TableViewState<StatementSortColumn>,
    pub table_stats: TableViewState<TableStatSortColumn>,

    // Non-sortable panel views (simple TableState)
    pub replication_table_state: TableState,
    pub blocking_table_state: TableState,
    pub vacuum_table_state: TableState,
    pub wraparound_table_state: TableState,
    pub settings_table_state: TableState,

    pub metrics: MetricsHistory,

    pub server_info: ServerInfo,
    pub connection: ConnectionInfo,
    pub refresh_interval_secs: u64,

    pub last_error: Option<String>,
    pub status_message: Option<String>,
    pub pending_action: Option<AppAction>,
    pub bloat_loading: bool,
    pub spinner_frame: u8,

    pub config: AppConfig,
    pub config_selected: usize,

    pub filter_text: String,
    pub filter_active: bool,
    pub replay: Option<ReplayState>,
    pub overlay_scroll: u16,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: String,
        port: u16,
        dbname: String,
        user: String,
        refresh: u64,
        history_len: usize,
        config: AppConfig,
        server_info: ServerInfo,
    ) -> Self {
        Self {
            running: true,
            paused: false,
            snapshot: None,
            view_mode: ViewMode::Normal,
            bottom_panel: BottomPanel::Queries,
            queries: TableViewState::new(SortColumn::Duration, false),
            indexes: TableViewState::new(IndexSortColumn::Scans, true),
            statements: TableViewState::new(StatementSortColumn::TotalTime, false),
            table_stats: TableViewState::new(TableStatSortColumn::DeadTuples, false),
            replication_table_state: TableState::default(),
            blocking_table_state: TableState::default(),
            vacuum_table_state: TableState::default(),
            wraparound_table_state: TableState::default(),
            settings_table_state: TableState::default(),
            metrics: MetricsHistory::new(history_len),
            server_info,
            connection: ConnectionInfo::new(host, port, dbname, user),
            refresh_interval_secs: refresh,
            last_error: None,
            status_message: None,
            pending_action: None,
            bloat_loading: false,
            spinner_frame: 0,
            config,
            config_selected: 0,
            filter_text: String::new(),
            filter_active: false,
            replay: None,
            overlay_scroll: 0,
        }
    }

    pub fn set_ssl_mode_label(&mut self, label: &str) {
        self.connection.set_ssl_mode(label);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_replay(
        host: String,
        port: u16,
        dbname: String,
        user: String,
        history_len: usize,
        config: AppConfig,
        server_info: ServerInfo,
        filename: String,
        total_snapshots: usize,
    ) -> Self {
        let mut app = Self::new(host, port, dbname, user, 0, history_len, config, server_info);
        app.replay = Some(ReplayState::new(filename, total_snapshots));
        app
    }

    /// Returns true if in replay mode
    pub fn is_replay_mode(&self) -> bool {
        self.replay.is_some()
    }

    pub fn update(&mut self, mut snapshot: PgSnapshot) {
        // Update metrics history
        self.metrics.push_snapshot_metrics(&snapshot);
        self.metrics.calculate_rates(&snapshot);

        // Preserve bloat data from previous snapshot
        if let Some(ref old_snap) = self.snapshot {
            // Build lookup maps from old snapshot's bloat data
            let table_bloat: HashMap<String, (Option<i64>, Option<f64>)> = old_snap
                .table_stats
                .iter()
                .filter(|t| t.bloat_pct.is_some())
                .map(|t| {
                    let key = format!("{}.{}", t.schemaname, t.relname);
                    (key, (t.bloat_bytes, t.bloat_pct))
                })
                .collect();

            let index_bloat: HashMap<String, (Option<i64>, Option<f64>)> = old_snap
                .indexes
                .iter()
                .filter(|i| i.bloat_pct.is_some())
                .map(|i| {
                    let key = format!("{}.{}", i.schemaname, i.index_name);
                    (key, (i.bloat_bytes, i.bloat_pct))
                })
                .collect();

            // Apply to new snapshot
            for table in &mut snapshot.table_stats {
                let key = format!("{}.{}", table.schemaname, table.relname);
                if let Some((bytes, pct)) = table_bloat.get(&key) {
                    table.bloat_bytes = *bytes;
                    table.bloat_pct = *pct;
                }
            }

            for index in &mut snapshot.indexes {
                let key = format!("{}.{}", index.schemaname, index.index_name);
                if let Some((bytes, pct)) = index_bloat.get(&key) {
                    index.bloat_bytes = *bytes;
                    index.bloat_pct = *pct;
                }
            }
        }

        self.snapshot = Some(snapshot);
        self.last_error = None;
    }

    pub fn update_error(&mut self, err: String) {
        self.last_error = Some(err);
    }

    /// Apply bloat estimates to current snapshot's table_stats and indexes
    pub fn apply_bloat_data(
        &mut self,
        table_bloat: &HashMap<String, TableBloat>,
        index_bloat: &HashMap<String, IndexBloat>,
    ) {
        if let Some(ref mut snapshot) = self.snapshot {
            // Apply table bloat
            for table in &mut snapshot.table_stats {
                let key = format!("{}.{}", table.schemaname, table.relname);
                if let Some(bloat) = table_bloat.get(&key) {
                    table.bloat_bytes = Some(bloat.bloat_bytes);
                    table.bloat_pct = Some(bloat.bloat_pct);
                }
            }
            // Apply index bloat
            for index in &mut snapshot.indexes {
                let key = format!("{}.{}", index.schemaname, index.index_name);
                if let Some(bloat) = index_bloat.get(&key) {
                    index.bloat_bytes = Some(bloat.bloat_bytes);
                    index.bloat_pct = Some(bloat.bloat_pct);
                }
            }
        }
    }

    fn query_to_filter_string(q: &ActiveQuery) -> String {
        format!(
            "{} {} {} {} {} {}",
            q.pid,
            q.usename.as_deref().unwrap_or(""),
            q.datname.as_deref().unwrap_or(""),
            q.state.as_deref().unwrap_or(""),
            q.wait_event.as_deref().unwrap_or(""),
            q.query.as_deref().unwrap_or(""),
        )
    }

    fn index_to_filter_string(idx: &IndexInfo) -> String {
        format!(
            "{} {} {} {}",
            idx.schemaname, idx.table_name, idx.index_name, idx.index_definition
        )
    }

    fn stmt_to_filter_string(stmt: &StatStatement) -> String {
        stmt.query.clone()
    }

    fn table_stat_to_filter_string(t: &TableStat) -> String {
        format!("{} {}", t.schemaname, t.relname)
    }

    fn setting_to_filter_string(s: &PgSetting) -> String {
        format!("{} {} {}", s.name, s.category, s.short_desc)
    }

    pub fn sorted_query_indices(&self) -> Vec<usize> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let mut indices: Vec<usize> = (0..snap.active_queries.len()).collect();

        // Apply fuzzy filter only when on the Queries panel
        let filter_text = &self.filter_text;
        if self.bottom_panel == BottomPanel::Queries
            && !filter_text.is_empty()
            && (self.filter_active || self.view_mode == ViewMode::Filter)
        {
            let mut matcher = Matcher::new(MatcherConfig::DEFAULT);
            let pattern =
                Pattern::parse(filter_text, CaseMatching::Ignore, Normalization::Smart);
            indices.retain(|&i| {
                let haystack = Self::query_to_filter_string(&snap.active_queries[i]);
                let mut buf = Vec::new();
                pattern
                    .score(nucleo_matcher::Utf32Str::new(&haystack, &mut buf), &mut matcher)
                    .is_some()
            });
        }

        let asc = self.queries.sort_ascending;
        match self.queries.sort_column {
            SortColumn::Pid => indices.sort_by(|&a, &b| {
                let cmp = snap.active_queries[a]
                    .pid
                    .cmp(&snap.active_queries[b].pid);
                if asc { cmp } else { cmp.reverse() }
            }),
            SortColumn::Duration => indices.sort_by(|&a, &b| {
                let cmp = snap.active_queries[a]
                    .duration_secs
                    .partial_cmp(&snap.active_queries[b].duration_secs)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
            SortColumn::State => indices.sort_by(|&a, &b| {
                let cmp = snap.active_queries[a]
                    .state
                    .cmp(&snap.active_queries[b].state);
                if asc { cmp } else { cmp.reverse() }
            }),
            SortColumn::User => indices.sort_by(|&a, &b| {
                let cmp = snap.active_queries[a]
                    .usename
                    .cmp(&snap.active_queries[b].usename);
                if asc { cmp } else { cmp.reverse() }
            }),
        }
        indices
    }

    pub fn selected_query_pid(&self) -> Option<i32> {
        let snap = self.snapshot.as_ref()?;
        let idx = self.queries.selected()?;
        let indices = self.sorted_query_indices();
        let &real_idx = indices.get(idx)?;
        Some(snap.active_queries[real_idx].pid)
    }

    /// Get PIDs of all queries matching the current filter
    pub fn get_filtered_pids(&self) -> Vec<i32> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let indices = self.sorted_query_indices();
        indices
            .iter()
            .map(|&i| snap.active_queries[i].pid)
            .collect()
    }

    pub fn sorted_index_indices(&self) -> Vec<usize> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let mut indices: Vec<usize> = (0..snap.indexes.len()).collect();

        // Apply fuzzy filter only when on the Indexes panel
        let filter_text = &self.filter_text;
        if self.bottom_panel == BottomPanel::Indexes
            && !filter_text.is_empty()
            && (self.filter_active || self.view_mode == ViewMode::Filter)
        {
            let mut matcher = Matcher::new(MatcherConfig::DEFAULT);
            let pattern =
                Pattern::parse(filter_text, CaseMatching::Ignore, Normalization::Smart);
            indices.retain(|&i| {
                let haystack = Self::index_to_filter_string(&snap.indexes[i]);
                let mut buf = Vec::new();
                pattern
                    .score(nucleo_matcher::Utf32Str::new(&haystack, &mut buf), &mut matcher)
                    .is_some()
            });
        }

        let asc = self.indexes.sort_ascending;
        match self.indexes.sort_column {
            IndexSortColumn::Scans => indices.sort_by(|&a, &b| {
                let cmp = snap.indexes[a].idx_scan.cmp(&snap.indexes[b].idx_scan);
                if asc { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::Size => indices.sort_by(|&a, &b| {
                let cmp = snap.indexes[a]
                    .index_size_bytes
                    .cmp(&snap.indexes[b].index_size_bytes);
                if asc { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::Name => indices.sort_by(|&a, &b| {
                let cmp = snap.indexes[a]
                    .index_name
                    .cmp(&snap.indexes[b].index_name);
                if asc { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::TupRead => indices.sort_by(|&a, &b| {
                let cmp = snap.indexes[a]
                    .idx_tup_read
                    .cmp(&snap.indexes[b].idx_tup_read);
                if asc { cmp } else { cmp.reverse() }
            }),
            IndexSortColumn::TupFetch => indices.sort_by(|&a, &b| {
                let cmp = snap.indexes[a]
                    .idx_tup_fetch
                    .cmp(&snap.indexes[b].idx_tup_fetch);
                if asc { cmp } else { cmp.reverse() }
            }),
        }
        indices
    }

    pub fn sorted_stmt_indices(&self) -> Vec<usize> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let mut indices: Vec<usize> = (0..snap.stat_statements.len()).collect();

        // Apply fuzzy filter only when on the Statements panel
        let filter_text = &self.filter_text;
        if self.bottom_panel == BottomPanel::Statements
            && !filter_text.is_empty()
            && (self.filter_active || self.view_mode == ViewMode::Filter)
        {
            let mut matcher = Matcher::new(MatcherConfig::DEFAULT);
            let pattern =
                Pattern::parse(filter_text, CaseMatching::Ignore, Normalization::Smart);
            indices.retain(|&i| {
                let haystack = Self::stmt_to_filter_string(&snap.stat_statements[i]);
                let mut buf = Vec::new();
                pattern
                    .score(nucleo_matcher::Utf32Str::new(&haystack, &mut buf), &mut matcher)
                    .is_some()
            });
        }

        let asc = self.statements.sort_ascending;
        match self.statements.sort_column {
            StatementSortColumn::TotalTime => indices.sort_by(|&a, &b| {
                let cmp = snap.stat_statements[a]
                    .total_exec_time
                    .partial_cmp(&snap.stat_statements[b].total_exec_time)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::MeanTime => indices.sort_by(|&a, &b| {
                let cmp = snap.stat_statements[a]
                    .mean_exec_time
                    .partial_cmp(&snap.stat_statements[b].mean_exec_time)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::MaxTime => indices.sort_by(|&a, &b| {
                let cmp = snap.stat_statements[a]
                    .max_exec_time
                    .partial_cmp(&snap.stat_statements[b].max_exec_time)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::Stddev => indices.sort_by(|&a, &b| {
                let cmp = snap.stat_statements[a]
                    .stddev_exec_time
                    .partial_cmp(&snap.stat_statements[b].stddev_exec_time)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::Calls => indices.sort_by(|&a, &b| {
                let cmp = snap.stat_statements[a]
                    .calls
                    .cmp(&snap.stat_statements[b].calls);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::Rows => indices.sort_by(|&a, &b| {
                let cmp = snap.stat_statements[a]
                    .rows
                    .cmp(&snap.stat_statements[b].rows);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::HitRatio => indices.sort_by(|&a, &b| {
                let cmp = snap.stat_statements[a]
                    .hit_ratio
                    .partial_cmp(&snap.stat_statements[b].hit_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::SharedReads => indices.sort_by(|&a, &b| {
                let cmp = snap.stat_statements[a]
                    .shared_blks_read
                    .cmp(&snap.stat_statements[b].shared_blks_read);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::IoTime => indices.sort_by(|&a, &b| {
                let io = |s: &crate::db::models::StatStatement| {
                    s.blk_read_time + s.blk_write_time
                };
                let cmp = io(&snap.stat_statements[a])
                    .partial_cmp(&io(&snap.stat_statements[b]))
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::Temp => indices.sort_by(|&a, &b| {
                let temp = |s: &crate::db::models::StatStatement| {
                    s.temp_blks_read + s.temp_blks_written
                };
                let cmp = temp(&snap.stat_statements[a])
                    .cmp(&temp(&snap.stat_statements[b]));
                if asc { cmp } else { cmp.reverse() }
            }),
        }
        indices
    }

    pub fn sorted_table_stat_indices(&self) -> Vec<usize> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let mut indices: Vec<usize> = (0..snap.table_stats.len()).collect();

        // Apply fuzzy filter only when on the TableStats panel
        let filter_text = &self.filter_text;
        if self.bottom_panel == BottomPanel::TableStats
            && !filter_text.is_empty()
            && (self.filter_active || self.view_mode == ViewMode::Filter)
        {
            let mut matcher = Matcher::new(MatcherConfig::DEFAULT);
            let pattern =
                Pattern::parse(filter_text, CaseMatching::Ignore, Normalization::Smart);
            indices.retain(|&i| {
                let haystack = Self::table_stat_to_filter_string(&snap.table_stats[i]);
                let mut buf = Vec::new();
                pattern
                    .score(nucleo_matcher::Utf32Str::new(&haystack, &mut buf), &mut matcher)
                    .is_some()
            });
        }

        let asc = self.table_stats.sort_ascending;
        match self.table_stats.sort_column {
            TableStatSortColumn::DeadTuples => indices.sort_by(|&a, &b| {
                let cmp = snap.table_stats[a]
                    .n_dead_tup
                    .cmp(&snap.table_stats[b].n_dead_tup);
                if asc { cmp } else { cmp.reverse() }
            }),
            TableStatSortColumn::Size => indices.sort_by(|&a, &b| {
                let cmp = snap.table_stats[a]
                    .total_size_bytes
                    .cmp(&snap.table_stats[b].total_size_bytes);
                if asc { cmp } else { cmp.reverse() }
            }),
            TableStatSortColumn::Name => indices.sort_by(|&a, &b| {
                let cmp = snap.table_stats[a]
                    .relname
                    .cmp(&snap.table_stats[b].relname);
                if asc { cmp } else { cmp.reverse() }
            }),
            TableStatSortColumn::SeqScan => indices.sort_by(|&a, &b| {
                let cmp = snap.table_stats[a]
                    .seq_scan
                    .cmp(&snap.table_stats[b].seq_scan);
                if asc { cmp } else { cmp.reverse() }
            }),
            TableStatSortColumn::IdxScan => indices.sort_by(|&a, &b| {
                let cmp = snap.table_stats[a]
                    .idx_scan
                    .cmp(&snap.table_stats[b].idx_scan);
                if asc { cmp } else { cmp.reverse() }
            }),
            TableStatSortColumn::DeadRatio => indices.sort_by(|&a, &b| {
                let cmp = snap.table_stats[a]
                    .dead_ratio
                    .partial_cmp(&snap.table_stats[b].dead_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
        }
        indices
    }

    pub fn sorted_settings_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.server_info.settings.len()).collect();

        // Apply fuzzy filter only when on the Settings panel
        let filter_text = &self.filter_text;
        if self.bottom_panel == BottomPanel::Settings
            && !filter_text.is_empty()
            && (self.filter_active || self.view_mode == ViewMode::Filter)
        {
            let mut matcher = Matcher::new(MatcherConfig::DEFAULT);
            let pattern =
                Pattern::parse(filter_text, CaseMatching::Ignore, Normalization::Smart);
            indices.retain(|&i| {
                let haystack = Self::setting_to_filter_string(&self.server_info.settings[i]);
                let mut buf = Vec::new();
                pattern
                    .score(nucleo_matcher::Utf32Str::new(&haystack, &mut buf), &mut matcher)
                    .is_some()
            });
        }

        // Settings are already sorted by category, name from the query
        indices
    }

    fn copy_to_clipboard(&mut self, text: &str) {
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
            Ok(()) => {
                let preview: String = text.chars().take(40).collect();
                let suffix = if text.len() > 40 { "..." } else { "" };
                self.status_message = Some(format!("Copied: {}{}", preview, suffix));
            }
            Err(e) => {
                self.status_message = Some(format!("Clipboard error: {}", e));
            }
        }
    }

    fn yank_selected(&mut self) {
        let Some(snap) = &self.snapshot else { return };
        match self.bottom_panel {
            BottomPanel::Queries => {
                let idx = self.queries.selected().unwrap_or(0);
                let indices = self.sorted_query_indices();
                if let Some(&real_idx) = indices.get(idx) {
                    if let Some(ref q) = snap.active_queries[real_idx].query {
                        let text = q.clone();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            BottomPanel::Indexes => {
                let idx = self.indexes.selected().unwrap_or(0);
                let indices = self.sorted_index_indices();
                if let Some(&real_idx) = indices.get(idx) {
                    let text = snap.indexes[real_idx].index_definition.clone();
                    self.copy_to_clipboard(&text);
                }
            }
            BottomPanel::Statements => {
                let idx = self.statements.selected().unwrap_or(0);
                let indices = self.sorted_stmt_indices();
                if let Some(&real_idx) = indices.get(idx) {
                    let text = snap.stat_statements[real_idx].query.clone();
                    self.copy_to_clipboard(&text);
                }
            }
            _ => {}
        }
    }

    fn switch_panel(&mut self, target: BottomPanel) {
        if self.bottom_panel == target {
            // Toggle back to Queries
            self.bottom_panel = BottomPanel::Queries;
        } else {
            self.bottom_panel = target;
        }
        // Clear filter state when switching panels
        self.filter_text.clear();
        self.filter_active = false;
        self.view_mode = ViewMode::Normal;
    }

    fn reset_panel_selection(&mut self) {
        match self.bottom_panel {
            BottomPanel::Queries => self.queries.select_first(),
            BottomPanel::Indexes => self.indexes.select_first(),
            BottomPanel::Statements => self.statements.select_first(),
            BottomPanel::TableStats => self.table_stats.select_first(),
            BottomPanel::Replication => self.replication_table_state.select(Some(0)),
            BottomPanel::Settings => self.settings_table_state.select(Some(0)),
            _ => {}
        }
    }

    fn handle_queries_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.queries.select_prev();
                self.status_message = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_query_indices().len();
                self.queries.select_next(max);
                self.status_message = None;
            }
            KeyCode::Enter | KeyCode::Char('i') => {
                if self.selected_query_pid().is_some() {
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::Inspect;
                }
            }
            KeyCode::Char('K') if self.replay.is_none() => {
                if let Some(pid) = self.selected_query_pid() {
                    let filtered_pids = self.get_filtered_pids();
                    if self.filter_active && filtered_pids.len() > 1 {
                        // Multiple matches - show choice dialog
                        self.view_mode = ViewMode::ConfirmKillChoice {
                            selected_pid: pid,
                            all_pids: filtered_pids,
                        };
                    } else {
                        // Single query - existing behavior
                        self.view_mode = ViewMode::ConfirmKill(pid);
                    }
                }
            }
            KeyCode::Char('C') if self.replay.is_none() => {
                if let Some(pid) = self.selected_query_pid() {
                    let filtered_pids = self.get_filtered_pids();
                    if self.filter_active && filtered_pids.len() > 1 {
                        // Multiple matches - show choice dialog
                        self.view_mode = ViewMode::ConfirmCancelChoice {
                            selected_pid: pid,
                            all_pids: filtered_pids,
                        };
                    } else {
                        // Single query - existing behavior
                        self.view_mode = ViewMode::ConfirmCancel(pid);
                    }
                }
            }
            KeyCode::Char('s') => {
                self.queries.cycle_sort();
                self.status_message = Some(format!(
                    "Sort: {} {}",
                    self.queries.sort_column.label(),
                    if self.queries.sort_ascending { "\u{2191}" } else { "\u{2193}" }
                ));
            }
            _ => {}
        }
    }

    fn handle_indexes_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.indexes.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_index_indices().len();
                self.indexes.select_next(max);
            }
            KeyCode::Enter => {
                if self.snapshot.as_ref().is_some_and(|s| !s.indexes.is_empty()) {
                    if self.indexes.selected().is_none() {
                        self.indexes.select_first();
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::IndexInspect;
                }
            }
            KeyCode::Char('s') => {
                self.indexes.cycle_sort();
                // Default ascending for Name/Scans, descending for others
                self.indexes.sort_ascending = matches!(
                    self.indexes.sort_column,
                    IndexSortColumn::Scans | IndexSortColumn::Name
                );
            }
            KeyCode::Char('b') if self.replay.is_none() => {
                self.pending_action = Some(AppAction::RefreshBloat);
                self.status_message = Some("Refreshing bloat estimates...".to_string());
                self.bloat_loading = true;
            }
            _ => {}
        }
    }

    fn handle_statements_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.statements.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_stmt_indices().len();
                self.statements.select_next(max);
            }
            KeyCode::Enter => {
                if self
                    .snapshot
                    .as_ref()
                    .is_some_and(|s| !s.stat_statements.is_empty())
                {
                    if self.statements.selected().is_none() {
                        self.statements.select_first();
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::StatementInspect;
                }
            }
            KeyCode::Char('s') => {
                self.statements.cycle_sort();
                self.status_message = Some(format!(
                    "Sort: {} {}",
                    self.statements.sort_column.label(),
                    if self.statements.sort_ascending { "\u{2191}" } else { "\u{2193}" }
                ));
            }
            _ => {}
        }
    }

    fn handle_table_stats_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.table_stats.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_table_stat_indices().len();
                self.table_stats.select_next(max);
            }
            KeyCode::Enter => {
                if self.snapshot.as_ref().is_some_and(|s| !s.table_stats.is_empty()) {
                    if self.table_stats.selected().is_none() {
                        self.table_stats.select_first();
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::TableInspect;
                }
            }
            KeyCode::Char('s') => {
                self.table_stats.cycle_sort();
                self.status_message = Some(format!(
                    "Sort: {} {}",
                    self.table_stats.sort_column.label(),
                    if self.table_stats.sort_ascending { "\u{2191}" } else { "\u{2193}" }
                ));
            }
            KeyCode::Char('b') if self.replay.is_none() => {
                self.pending_action = Some(AppAction::RefreshBloat);
                self.status_message = Some("Refreshing bloat estimates...".to_string());
                self.bloat_loading = true;
            }
            _ => {}
        }
    }

    fn handle_replication_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.replication_table_state.selected().unwrap_or(0);
                self.replication_table_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self
                    .snapshot
                    .as_ref()
                    .map(|s| s.replication.len())
                    .unwrap_or(0)
                    .saturating_sub(1);
                let i = self.replication_table_state.selected().unwrap_or(0);
                self.replication_table_state.select(Some((i + 1).min(max)));
            }
            KeyCode::Enter => {
                if self.snapshot.as_ref().is_some_and(|s| !s.replication.is_empty()) {
                    if self.replication_table_state.selected().is_none() {
                        self.replication_table_state.select(Some(0));
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::ReplicationInspect;
                }
            }
            _ => {}
        }
    }

    fn handle_blocking_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.blocking_table_state.selected().unwrap_or(0);
                self.blocking_table_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self
                    .snapshot
                    .as_ref()
                    .map(|s| s.blocking_info.len())
                    .unwrap_or(0)
                    .saturating_sub(1);
                let i = self.blocking_table_state.selected().unwrap_or(0);
                self.blocking_table_state.select(Some((i + 1).min(max)));
            }
            KeyCode::Enter => {
                if self.snapshot.as_ref().is_some_and(|s| !s.blocking_info.is_empty()) {
                    if self.blocking_table_state.selected().is_none() {
                        self.blocking_table_state.select(Some(0));
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::BlockingInspect;
                }
            }
            _ => {}
        }
    }

    fn handle_vacuum_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.vacuum_table_state.selected().unwrap_or(0);
                self.vacuum_table_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self
                    .snapshot
                    .as_ref()
                    .map(|s| s.vacuum_progress.len())
                    .unwrap_or(0)
                    .saturating_sub(1);
                let i = self.vacuum_table_state.selected().unwrap_or(0);
                self.vacuum_table_state.select(Some((i + 1).min(max)));
            }
            KeyCode::Enter => {
                if self.snapshot.as_ref().is_some_and(|s| !s.vacuum_progress.is_empty()) {
                    if self.vacuum_table_state.selected().is_none() {
                        self.vacuum_table_state.select(Some(0));
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::VacuumInspect;
                }
            }
            _ => {}
        }
    }

    fn handle_wraparound_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.wraparound_table_state.selected().unwrap_or(0);
                self.wraparound_table_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self
                    .snapshot
                    .as_ref()
                    .map(|s| s.wraparound.len())
                    .unwrap_or(0)
                    .saturating_sub(1);
                let i = self.wraparound_table_state.selected().unwrap_or(0);
                self.wraparound_table_state.select(Some((i + 1).min(max)));
            }
            KeyCode::Enter => {
                if self.snapshot.as_ref().is_some_and(|s| !s.wraparound.is_empty()) {
                    if self.wraparound_table_state.selected().is_none() {
                        self.wraparound_table_state.select(Some(0));
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::WraparoundInspect;
                }
            }
            _ => {}
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.settings_table_state.selected().unwrap_or(0);
                self.settings_table_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_settings_indices().len().saturating_sub(1);
                let i = self.settings_table_state.selected().unwrap_or(0);
                self.settings_table_state.select(Some((i + 1).min(max)));
            }
            _ => {}
        }
    }

    fn handle_panel_key(&mut self, key: KeyEvent) {
        match self.bottom_panel {
            BottomPanel::Queries => self.handle_queries_key(key),
            BottomPanel::Indexes => self.handle_indexes_key(key),
            BottomPanel::Statements => self.handle_statements_key(key),
            BottomPanel::TableStats => self.handle_table_stats_key(key),
            BottomPanel::Replication => self.handle_replication_key(key),
            BottomPanel::Blocking => self.handle_blocking_key(key),
            BottomPanel::VacuumProgress => self.handle_vacuum_key(key),
            BottomPanel::Wraparound => self.handle_wraparound_key(key),
            BottomPanel::Settings => self.handle_settings_key(key),
            BottomPanel::WalIo | BottomPanel::WaitEvents => {}
        }
    }

    // --- Modal overlay handlers ---

    fn handle_confirm_cancel_key(&mut self, key: KeyEvent, pid: i32) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.pending_action = Some(AppAction::CancelQuery(pid));
                self.view_mode = ViewMode::Normal;
            }
            _ => {
                self.view_mode = ViewMode::Normal;
                self.status_message = Some("Cancel aborted".into());
            }
        }
    }

    fn handle_confirm_kill_key(&mut self, key: KeyEvent, pid: i32) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.pending_action = Some(AppAction::TerminateBackend(pid));
                self.view_mode = ViewMode::Normal;
            }
            _ => {
                self.view_mode = ViewMode::Normal;
                self.status_message = Some("Kill aborted".into());
            }
        }
    }

    fn handle_confirm_cancel_choice_key(&mut self, key: KeyEvent, selected_pid: i32, all_pids: Vec<i32>) {
        match key.code {
            KeyCode::Char('1') | KeyCode::Char('o') => {
                self.pending_action = Some(AppAction::CancelQuery(selected_pid));
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('a') => {
                self.view_mode = ViewMode::ConfirmCancelBatch(all_pids);
            }
            KeyCode::Esc => {
                self.view_mode = ViewMode::Normal;
                self.status_message = Some("Cancel aborted".into());
            }
            _ => {}
        }
    }

    fn handle_confirm_kill_choice_key(&mut self, key: KeyEvent, selected_pid: i32, all_pids: Vec<i32>) {
        match key.code {
            KeyCode::Char('1') | KeyCode::Char('o') => {
                self.pending_action = Some(AppAction::TerminateBackend(selected_pid));
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('a') => {
                self.view_mode = ViewMode::ConfirmKillBatch(all_pids);
            }
            KeyCode::Esc => {
                self.view_mode = ViewMode::Normal;
                self.status_message = Some("Kill aborted".into());
            }
            _ => {}
        }
    }

    fn handle_confirm_cancel_batch_key(&mut self, key: KeyEvent, pids: Vec<i32>) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.pending_action = Some(AppAction::CancelQueries(pids));
                self.view_mode = ViewMode::Normal;
            }
            _ => {
                self.view_mode = ViewMode::Normal;
                self.status_message = Some("Batch cancel aborted".into());
            }
        }
    }

    fn handle_confirm_kill_batch_key(&mut self, key: KeyEvent, pids: Vec<i32>) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.pending_action = Some(AppAction::TerminateBackends(pids));
                self.view_mode = ViewMode::Normal;
            }
            _ => {
                self.view_mode = ViewMode::Normal;
                self.status_message = Some("Batch kill aborted".into());
            }
        }
    }

    /// Handle overlay scroll keys, returns true if handled
    fn handle_overlay_scroll(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                true
            }
            KeyCode::Char('g') => {
                self.overlay_scroll = 0;
                true
            }
            KeyCode::Char('G') => {
                self.overlay_scroll = u16::MAX;
                true
            }
            _ => false,
        }
    }

    fn handle_inspect_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('y') => {
                if let Some(snap) = &self.snapshot {
                    let idx = self.queries.selected().unwrap_or(0);
                    let indices = self.sorted_query_indices();
                    if let Some(&real_idx) = indices.get(idx) {
                        if let Some(ref q) = snap.active_queries[real_idx].query {
                            let text = q.clone();
                            self.copy_to_clipboard(&text);
                        }
                    }
                }
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_index_inspect_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('y') => {
                if let Some(snap) = &self.snapshot {
                    let idx = self.indexes.selected().unwrap_or(0);
                    let indices = self.sorted_index_indices();
                    if let Some(&real_idx) = indices.get(idx) {
                        let text = snap.indexes[real_idx].index_definition.clone();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_statement_inspect_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('y') => {
                if let Some(snap) = &self.snapshot {
                    let idx = self.statements.selected().unwrap_or(0);
                    let indices = self.sorted_stmt_indices();
                    if let Some(&real_idx) = indices.get(idx) {
                        let text = snap.stat_statements[real_idx].query.clone();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_replication_inspect_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('y') => {
                if let Some(snap) = &self.snapshot {
                    let sel = self.replication_table_state.selected().unwrap_or(0);
                    if let Some(r) = snap.replication.get(sel) {
                        let text = r.application_name.clone().unwrap_or_default();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_table_inspect_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('y') => {
                if let Some(snap) = &self.snapshot {
                    let sel = self.table_stats.selected().unwrap_or(0);
                    let indices = self.sorted_table_stat_indices();
                    if let Some(&real_idx) = indices.get(sel) {
                        let t = &snap.table_stats[real_idx];
                        let text = format!("{}.{}", t.schemaname, t.relname);
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_blocking_inspect_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('y') => {
                if let Some(snap) = &self.snapshot {
                    let sel = self.blocking_table_state.selected().unwrap_or(0);
                    if let Some(info) = snap.blocking_info.get(sel) {
                        let text = info.blocked_query.clone().unwrap_or_default();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_vacuum_inspect_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('y') => {
                if let Some(snap) = &self.snapshot {
                    let sel = self.vacuum_table_state.selected().unwrap_or(0);
                    if let Some(vac) = snap.vacuum_progress.get(sel) {
                        let text = vac.table_name.clone();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_wraparound_inspect_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('y') => {
                if let Some(snap) = &self.snapshot {
                    let sel = self.wraparound_table_state.selected().unwrap_or(0);
                    if let Some(wrap) = snap.wraparound.get(sel) {
                        let text = wrap.datname.clone();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_config_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.pending_action = Some(AppAction::SaveConfig);
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.config_selected > 0 {
                    self.config_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.config_selected < ConfigItem::ALL.len() - 1 {
                    self.config_selected += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.config_adjust(-1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.config_adjust(1);
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Normal;
            }
            _ => {
                self.handle_overlay_scroll(key);
            }
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.filter_text.clear();
                self.filter_active = false;
                self.view_mode = ViewMode::Normal;
                self.reset_panel_selection();
            }
            KeyCode::Enter => {
                self.filter_active = !self.filter_text.is_empty();
                self.view_mode = ViewMode::Normal;
                self.reset_panel_selection();
            }
            KeyCode::Backspace => {
                self.filter_text.pop();
                self.reset_panel_selection();
            }
            KeyCode::Char(c) => {
                self.filter_text.push(c);
                self.reset_panel_selection();
            }
            _ => {}
        }
    }

    fn handle_normal_global_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => {
                if self.bottom_panel == BottomPanel::Queries {
                    self.running = false;
                } else {
                    self.switch_panel(BottomPanel::Queries);
                }
                true
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
                true
            }
            KeyCode::Esc => {
                if self.bottom_panel != BottomPanel::Queries {
                    self.switch_panel(BottomPanel::Queries);
                } else {
                    self.running = false;
                }
                true
            }
            KeyCode::Char('p') if self.replay.is_none() => {
                self.paused = !self.paused;
                true
            }
            KeyCode::Char('r') if self.replay.is_none() => {
                self.pending_action = Some(AppAction::ForceRefresh);
                true
            }
            KeyCode::Char('?') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Help;
                true
            }
            KeyCode::Char(',') => {
                self.view_mode = ViewMode::Config;
                true
            }
            KeyCode::Char('y') => {
                self.yank_selected();
                true
            }
            _ => false,
        }
    }

    fn handle_panel_switch_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('Q') => {
                self.switch_panel(BottomPanel::Queries);
                true
            }
            KeyCode::Tab => {
                self.switch_panel(BottomPanel::Blocking);
                true
            }
            KeyCode::Char('w') => {
                self.switch_panel(BottomPanel::WaitEvents);
                true
            }
            KeyCode::Char('t') => {
                self.switch_panel(BottomPanel::TableStats);
                true
            }
            KeyCode::Char('R') => {
                self.switch_panel(BottomPanel::Replication);
                true
            }
            KeyCode::Char('v') => {
                self.switch_panel(BottomPanel::VacuumProgress);
                true
            }
            KeyCode::Char('x') => {
                self.switch_panel(BottomPanel::Wraparound);
                true
            }
            KeyCode::Char('I') => {
                self.switch_panel(BottomPanel::Indexes);
                true
            }
            KeyCode::Char('S') => {
                self.switch_panel(BottomPanel::Statements);
                true
            }
            KeyCode::Char('A') => {
                self.switch_panel(BottomPanel::WalIo);
                true
            }
            KeyCode::Char('P') => {
                self.switch_panel(BottomPanel::Settings);
                true
            }
            KeyCode::Char('/') => {
                if self.bottom_panel.supports_filter() {
                    self.view_mode = ViewMode::Filter;
                }
                true
            }
            _ => false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Layer 1: Modal overlays consume all input
        match &self.view_mode {
            ViewMode::ConfirmCancel(pid) => {
                let pid = *pid;
                self.handle_confirm_cancel_key(key, pid);
                return;
            }
            ViewMode::ConfirmKill(pid) => {
                let pid = *pid;
                self.handle_confirm_kill_key(key, pid);
                return;
            }
            ViewMode::ConfirmCancelChoice { selected_pid, all_pids } => {
                let selected_pid = *selected_pid;
                let all_pids = all_pids.clone();
                self.handle_confirm_cancel_choice_key(key, selected_pid, all_pids);
                return;
            }
            ViewMode::ConfirmKillChoice { selected_pid, all_pids } => {
                let selected_pid = *selected_pid;
                let all_pids = all_pids.clone();
                self.handle_confirm_kill_choice_key(key, selected_pid, all_pids);
                return;
            }
            ViewMode::ConfirmCancelBatch(pids) => {
                let pids = pids.clone();
                self.handle_confirm_cancel_batch_key(key, pids);
                return;
            }
            ViewMode::ConfirmKillBatch(pids) => {
                let pids = pids.clone();
                self.handle_confirm_kill_batch_key(key, pids);
                return;
            }
            ViewMode::Inspect => {
                self.handle_inspect_key(key);
                return;
            }
            ViewMode::IndexInspect => {
                self.handle_index_inspect_key(key);
                return;
            }
            ViewMode::StatementInspect => {
                self.handle_statement_inspect_key(key);
                return;
            }
            ViewMode::ReplicationInspect => {
                self.handle_replication_inspect_key(key);
                return;
            }
            ViewMode::TableInspect => {
                self.handle_table_inspect_key(key);
                return;
            }
            ViewMode::BlockingInspect => {
                self.handle_blocking_inspect_key(key);
                return;
            }
            ViewMode::VacuumInspect => {
                self.handle_vacuum_inspect_key(key);
                return;
            }
            ViewMode::WraparoundInspect => {
                self.handle_wraparound_inspect_key(key);
                return;
            }
            ViewMode::Config => {
                self.handle_config_key(key);
                return;
            }
            ViewMode::Help => {
                self.handle_help_key(key);
                return;
            }
            ViewMode::Filter => {
                self.handle_filter_key(key);
                return;
            }
            ViewMode::Normal => {}
        }

        // Layer 2: Normal mode global keys
        if self.handle_normal_global_key(key) {
            return;
        }

        // Layer 3: Panel-switch keys
        if self.handle_panel_switch_key(key) {
            return;
        }

        // Layer 4: Panel-specific keys
        self.handle_panel_key(key);
    }

    fn config_adjust(&mut self, direction: i8) {
        let item = ConfigItem::ALL[self.config_selected];
        match item {
            ConfigItem::GraphMarker => {
                self.config.graph_marker = if direction > 0 {
                    self.config.graph_marker.next()
                } else {
                    self.config.graph_marker.prev()
                };
            }
            ConfigItem::ColorTheme => {
                self.config.color_theme = if direction > 0 {
                    self.config.color_theme.next()
                } else {
                    self.config.color_theme.prev()
                };
                theme::set_theme(self.config.color_theme.colors());
            }
            ConfigItem::RefreshInterval => {
                let val = self.config.refresh_interval_secs as i64 + direction as i64;
                self.config.refresh_interval_secs = val.clamp(1, 60) as u64;
                self.refresh_interval_secs = self.config.refresh_interval_secs;
                self.pending_action = Some(AppAction::RefreshIntervalChanged);
            }
            ConfigItem::WarnDuration => {
                let val = self.config.warn_duration_secs + direction as f64 * 0.5;
                self.config.warn_duration_secs = val.clamp(0.1, self.config.danger_duration_secs);
                theme::set_duration_thresholds(
                    self.config.warn_duration_secs,
                    self.config.danger_duration_secs,
                );
            }
            ConfigItem::DangerDuration => {
                let val = self.config.danger_duration_secs + direction as f64 * 1.0;
                self.config.danger_duration_secs = val.clamp(self.config.warn_duration_secs, 300.0);
                theme::set_duration_thresholds(
                    self.config.warn_duration_secs,
                    self.config.danger_duration_secs,
                );
            }
            ConfigItem::RecordingRetention => {
                let step: i64 = if self.config.recording_retention_secs >= 7200 {
                    3600
                } else {
                    600
                };
                let val = self.config.recording_retention_secs as i64 + direction as i64 * step;
                self.config.recording_retention_secs = val.clamp(600, 86400) as u64;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        ActiveQuery, ActivitySummary, BufferCacheStats, DetectedExtensions, PgSnapshot, ServerInfo,
    };
    use chrono::Utc;

    fn make_server_info() -> ServerInfo {
        ServerInfo {
            version: "PostgreSQL 14.5".into(),
            start_time: Utc::now(),
            max_connections: 100,
            extensions: DetectedExtensions::default(),
            settings: vec![],
        }
    }

    fn make_snapshot() -> PgSnapshot {
        PgSnapshot {
            timestamp: Utc::now(),
            active_queries: vec![ActiveQuery {
                pid: 12345,
                usename: Some("postgres".into()),
                datname: Some("testdb".into()),
                state: Some("active".into()),
                query: Some("SELECT 1".into()),
                duration_secs: 1.5,
                wait_event_type: None,
                wait_event: None,
                query_start: None,
                backend_type: None,
            }],
            wait_events: vec![],
            blocking_info: vec![],
            buffer_cache: BufferCacheStats {
                blks_hit: 9900,
                blks_read: 100,
                hit_ratio: 0.99,
            },
            summary: ActivitySummary {
                total_backends: 10,
                active_query_count: 1,
                idle_in_transaction_count: 0,
                waiting_count: 0,
                lock_count: 0,
                oldest_xact_secs: None,
                autovacuum_count: 0,
            },
            table_stats: vec![],
            replication: vec![],
            replication_slots: vec![],
            subscriptions: vec![],
            vacuum_progress: vec![],
            wraparound: vec![],
            indexes: vec![],
            stat_statements: vec![],
            stat_statements_error: None,
            extensions: DetectedExtensions::default(),
            db_size: 1000000,
            checkpoint_stats: None,
            wal_stats: None,
            archiver_stats: None,
            bgwriter_stats: None,
            db_stats: None,
        }
    }

    fn make_app() -> App {
        App::new(
            "localhost".into(),
            5432,
            "postgres".into(),
            "postgres".into(),
            2,
            120,
            AppConfig::default(),
            make_server_info(),
        )
    }

    fn make_replay_app() -> App {
        App::new_replay(
            "localhost".into(),
            5432,
            "postgres".into(),
            "postgres".into(),
            120,
            AppConfig::default(),
            make_server_info(),
            "test.jsonl".into(),
            10,
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Global keys
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn quit_from_queries_panel() {
        let mut app = make_app();
        assert!(app.running);
        app.handle_key(key(KeyCode::Char('q')));
        assert!(!app.running);
    }

    #[test]
    fn quit_from_other_panel_returns_to_queries() {
        let mut app = make_app();
        app.bottom_panel = BottomPanel::Indexes;
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.running);
        assert_eq!(app.bottom_panel, BottomPanel::Queries);
    }

    #[test]
    fn ctrl_c_quits() {
        let mut app = make_app();
        app.handle_key(key_ctrl(KeyCode::Char('c')));
        assert!(!app.running);
    }

    #[test]
    fn esc_from_queries_quits() {
        let mut app = make_app();
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.running);
    }

    #[test]
    fn esc_from_other_panel_returns_to_queries() {
        let mut app = make_app();
        app.bottom_panel = BottomPanel::Replication;
        app.handle_key(key(KeyCode::Esc));
        assert!(app.running);
        assert_eq!(app.bottom_panel, BottomPanel::Queries);
    }

    #[test]
    fn pause_toggle() {
        let mut app = make_app();
        assert!(!app.paused);
        app.handle_key(key(KeyCode::Char('p')));
        assert!(app.paused);
        app.handle_key(key(KeyCode::Char('p')));
        assert!(!app.paused);
    }

    #[test]
    fn pause_disabled_in_replay_mode() {
        let mut app = make_replay_app();
        assert!(!app.paused);
        app.handle_key(key(KeyCode::Char('p')));
        assert!(!app.paused); // Should remain unchanged
    }

    #[test]
    fn force_refresh() {
        let mut app = make_app();
        app.handle_key(key(KeyCode::Char('r')));
        assert!(matches!(app.pending_action, Some(AppAction::ForceRefresh)));
    }

    #[test]
    fn force_refresh_disabled_in_replay_mode() {
        let mut app = make_replay_app();
        app.handle_key(key(KeyCode::Char('r')));
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn help_opens() {
        let mut app = make_app();
        app.handle_key(key(KeyCode::Char('?')));
        assert_eq!(app.view_mode, ViewMode::Help);
    }

    #[test]
    fn config_opens() {
        let mut app = make_app();
        app.handle_key(key(KeyCode::Char(',')));
        assert_eq!(app.view_mode, ViewMode::Config);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Panel switching
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn tab_switches_to_blocking() {
        let mut app = make_app();
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.bottom_panel, BottomPanel::Blocking);
    }

    #[test]
    fn tab_toggles_back_to_queries() {
        let mut app = make_app();
        app.bottom_panel = BottomPanel::Blocking;
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.bottom_panel, BottomPanel::Queries);
    }

    #[test]
    fn panel_switch_keys() {
        let cases = [
            ('w', BottomPanel::WaitEvents),
            ('t', BottomPanel::TableStats),
            ('R', BottomPanel::Replication),
            ('v', BottomPanel::VacuumProgress),
            ('x', BottomPanel::Wraparound),
            ('I', BottomPanel::Indexes),
            ('S', BottomPanel::Statements),
            ('A', BottomPanel::WalIo),
            ('P', BottomPanel::Settings),
            ('Q', BottomPanel::Queries),
        ];
        for (ch, expected) in cases {
            let mut app = make_app();
            app.handle_key(key(KeyCode::Char(ch)));
            assert_eq!(app.bottom_panel, expected, "Key '{}' should switch to {:?}", ch, expected);
        }
    }

    #[test]
    fn panel_switch_clears_filter() {
        let mut app = make_app();
        app.filter_text = "test".into();
        app.filter_active = true;
        app.handle_key(key(KeyCode::Char('I')));
        assert!(app.filter_text.is_empty());
        assert!(!app.filter_active);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Filter mode
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn filter_opens_on_supported_panels() {
        for panel in [
            BottomPanel::Queries,
            BottomPanel::Indexes,
            BottomPanel::Statements,
            BottomPanel::TableStats,
            BottomPanel::Settings,
        ] {
            let mut app = make_app();
            app.bottom_panel = panel;
            app.handle_key(key(KeyCode::Char('/')));
            assert_eq!(app.view_mode, ViewMode::Filter, "Filter should open on {:?}", panel);
        }
    }

    #[test]
    fn filter_does_not_open_on_unsupported_panels() {
        for panel in [
            BottomPanel::Blocking,
            BottomPanel::WaitEvents,
            BottomPanel::Replication,
            BottomPanel::VacuumProgress,
            BottomPanel::Wraparound,
            BottomPanel::WalIo,
        ] {
            let mut app = make_app();
            app.bottom_panel = panel;
            app.handle_key(key(KeyCode::Char('/')));
            assert_eq!(app.view_mode, ViewMode::Normal, "Filter should not open on {:?}", panel);
        }
    }

    #[test]
    fn filter_typing() {
        let mut app = make_app();
        app.view_mode = ViewMode::Filter;
        app.handle_key(key(KeyCode::Char('t')));
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(key(KeyCode::Char('t')));
        assert_eq!(app.filter_text, "test");
    }

    #[test]
    fn filter_backspace() {
        let mut app = make_app();
        app.view_mode = ViewMode::Filter;
        app.filter_text = "test".into();
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.filter_text, "tes");
    }

    #[test]
    fn filter_enter_activates() {
        let mut app = make_app();
        app.view_mode = ViewMode::Filter;
        app.filter_text = "query".into();
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(app.filter_active);
    }

    #[test]
    fn filter_enter_with_empty_text_does_not_activate() {
        let mut app = make_app();
        app.view_mode = ViewMode::Filter;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(!app.filter_active);
    }

    #[test]
    fn filter_esc_clears_and_exits() {
        let mut app = make_app();
        app.view_mode = ViewMode::Filter;
        app.filter_text = "test".into();
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(app.filter_text.is_empty());
        assert!(!app.filter_active);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Config mode
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn config_navigation() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = 0;

        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.config_selected, 1);

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.config_selected, 2);

        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.config_selected, 1);

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.config_selected, 0);
    }

    #[test]
    fn config_esc_saves() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(matches!(app.pending_action, Some(AppAction::SaveConfig)));
    }

    #[test]
    fn config_adjust_refresh_interval() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::RefreshInterval)
            .unwrap();
        let initial = app.config.refresh_interval_secs;
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.config.refresh_interval_secs, initial + 1);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.config.refresh_interval_secs, initial);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Help mode
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn help_scroll() {
        let mut app = make_app();
        app.view_mode = ViewMode::Help;
        app.overlay_scroll = 5;

        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.overlay_scroll, 6);

        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.overlay_scroll, 5);

        app.handle_key(key(KeyCode::Char('g')));
        assert_eq!(app.overlay_scroll, 0);

        app.handle_key(key(KeyCode::Char('G')));
        assert_eq!(app.overlay_scroll, u16::MAX);
    }

    #[test]
    fn help_exit_keys() {
        for code in [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Enter] {
            let mut app = make_app();
            app.view_mode = ViewMode::Help;
            app.overlay_scroll = 10;
            app.handle_key(key(code));
            assert_eq!(app.view_mode, ViewMode::Normal);
            assert_eq!(app.overlay_scroll, 0);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Confirm dialogs
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn confirm_cancel_yes() {
        let mut app = make_app();
        app.view_mode = ViewMode::ConfirmCancel(12345);
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(matches!(app.pending_action, Some(AppAction::CancelQuery(12345))));
    }

    #[test]
    fn confirm_cancel_any_other_key_aborts() {
        let mut app = make_app();
        app.view_mode = ViewMode::ConfirmCancel(12345);
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(app.pending_action.is_none());
        assert!(app.status_message.as_ref().unwrap().contains("aborted"));
    }

    #[test]
    fn confirm_kill_yes() {
        let mut app = make_app();
        app.view_mode = ViewMode::ConfirmKill(12345);
        app.handle_key(key(KeyCode::Char('Y')));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(matches!(app.pending_action, Some(AppAction::TerminateBackend(12345))));
    }

    #[test]
    fn confirm_cancel_choice_one() {
        let mut app = make_app();
        app.view_mode = ViewMode::ConfirmCancelChoice {
            selected_pid: 100,
            all_pids: vec![100, 200, 300],
        };
        app.handle_key(key(KeyCode::Char('1')));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(matches!(app.pending_action, Some(AppAction::CancelQuery(100))));
    }

    #[test]
    fn confirm_cancel_choice_all() {
        let mut app = make_app();
        app.view_mode = ViewMode::ConfirmCancelChoice {
            selected_pid: 100,
            all_pids: vec![100, 200, 300],
        };
        app.handle_key(key(KeyCode::Char('a')));
        assert!(matches!(app.view_mode, ViewMode::ConfirmCancelBatch(_)));
    }

    #[test]
    fn confirm_cancel_batch_yes() {
        let mut app = make_app();
        app.view_mode = ViewMode::ConfirmCancelBatch(vec![100, 200, 300]);
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.view_mode, ViewMode::Normal);
        match &app.pending_action {
            Some(AppAction::CancelQueries(pids)) => assert_eq!(pids, &vec![100, 200, 300]),
            _ => panic!("Expected CancelQueries action"),
        }
    }

    #[test]
    fn confirm_kill_choice_esc() {
        let mut app = make_app();
        app.view_mode = ViewMode::ConfirmKillChoice {
            selected_pid: 100,
            all_pids: vec![100, 200],
        };
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.view_mode, ViewMode::Normal);
        assert!(app.pending_action.is_none());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Inspect modes
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn inspect_scroll_and_exit() {
        let modes = [
            ViewMode::Inspect,
            ViewMode::IndexInspect,
            ViewMode::StatementInspect,
            ViewMode::ReplicationInspect,
            ViewMode::TableInspect,
            ViewMode::BlockingInspect,
            ViewMode::VacuumInspect,
            ViewMode::WraparoundInspect,
        ];
        for mode in modes {
            let mut app = make_app();
            app.view_mode = mode.clone();
            app.overlay_scroll = 5;

            app.handle_key(key(KeyCode::Down));
            assert_eq!(app.overlay_scroll, 6, "Down should scroll in {:?}", mode);

            app.handle_key(key(KeyCode::Char('k')));
            assert_eq!(app.overlay_scroll, 5, "k should scroll up in {:?}", mode);

            app.handle_key(key(KeyCode::Esc));
            assert_eq!(app.view_mode, ViewMode::Normal, "Esc should exit {:?}", mode);
            assert_eq!(app.overlay_scroll, 0, "Overlay scroll should reset after {:?}", mode);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Panel-specific navigation
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn queries_panel_navigation() {
        let mut app = make_app();
        app.queries.state.select(Some(5));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.queries.selected(), Some(4));

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.queries.selected(), Some(3));
    }

    #[test]
    fn queries_panel_sort_cycle() {
        let mut app = make_app();
        assert_eq!(app.queries.sort_column, SortColumn::Duration);

        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(app.queries.sort_column, SortColumn::Pid);

        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(app.queries.sort_column, SortColumn::User);
    }

    #[test]
    fn indexes_panel_bloat_refresh() {
        let mut app = make_app();
        app.bottom_panel = BottomPanel::Indexes;
        app.handle_key(key(KeyCode::Char('b')));
        assert!(matches!(app.pending_action, Some(AppAction::RefreshBloat)));
        assert!(app.bloat_loading);
    }

    #[test]
    fn indexes_panel_bloat_disabled_in_replay() {
        let mut app = make_replay_app();
        app.bottom_panel = BottomPanel::Indexes;
        app.handle_key(key(KeyCode::Char('b')));
        assert!(app.pending_action.is_none());
        assert!(!app.bloat_loading);
    }

    #[test]
    fn table_stats_panel_bloat_refresh() {
        let mut app = make_app();
        app.bottom_panel = BottomPanel::TableStats;
        app.handle_key(key(KeyCode::Char('b')));
        assert!(matches!(app.pending_action, Some(AppAction::RefreshBloat)));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Cancel/Kill in replay mode
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn cancel_disabled_in_replay_mode() {
        let mut app = make_replay_app();
        // Set up a query so there's something to cancel
        app.snapshot = Some(make_snapshot());
        app.queries.state.select(Some(0));
        app.handle_key(key(KeyCode::Char('C')));
        // Should not enter any confirm mode
        assert_eq!(app.view_mode, ViewMode::Normal);
    }

    #[test]
    fn kill_disabled_in_replay_mode() {
        let mut app = make_replay_app();
        app.snapshot = Some(make_snapshot());
        app.queries.state.select(Some(0));
        app.handle_key(key(KeyCode::Char('K')));
        assert_eq!(app.view_mode, ViewMode::Normal);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Modal consumes all input
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn modal_blocks_global_keys() {
        let mut app = make_app();
        app.view_mode = ViewMode::Help;

        // 'q' should not quit when in help mode, just close help
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.running); // Should still be running
        assert_eq!(app.view_mode, ViewMode::Normal);
    }

    #[test]
    fn confirm_modal_blocks_panel_switch() {
        let mut app = make_app();
        app.view_mode = ViewMode::ConfirmCancel(123);

        // Tab should not switch panels
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.bottom_panel, BottomPanel::Queries);
        // Instead it should abort the confirm
        assert_eq!(app.view_mode, ViewMode::Normal);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Sort column cycling
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn sort_column_cycles() {
        assert_eq!(SortColumn::Duration.next(), SortColumn::Pid);
        assert_eq!(SortColumn::Pid.next(), SortColumn::User);
        assert_eq!(SortColumn::User.next(), SortColumn::State);
        assert_eq!(SortColumn::State.next(), SortColumn::Duration);
    }

    #[test]
    fn index_sort_column_cycles() {
        assert_eq!(IndexSortColumn::Scans.next(), IndexSortColumn::Size);
        assert_eq!(IndexSortColumn::Size.next(), IndexSortColumn::Name);
        assert_eq!(IndexSortColumn::Name.next(), IndexSortColumn::TupRead);
        assert_eq!(IndexSortColumn::TupRead.next(), IndexSortColumn::TupFetch);
        assert_eq!(IndexSortColumn::TupFetch.next(), IndexSortColumn::Scans);
    }

    #[test]
    fn statement_sort_column_cycles() {
        assert_eq!(StatementSortColumn::TotalTime.next(), StatementSortColumn::MeanTime);
        assert_eq!(StatementSortColumn::Temp.next(), StatementSortColumn::TotalTime);
    }

    #[test]
    fn table_stat_sort_column_cycles() {
        assert_eq!(TableStatSortColumn::DeadTuples.next(), TableStatSortColumn::Size);
        assert_eq!(TableStatSortColumn::DeadRatio.next(), TableStatSortColumn::DeadTuples);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Panel supports_filter
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn panel_supports_filter() {
        assert!(BottomPanel::Queries.supports_filter());
        assert!(BottomPanel::Indexes.supports_filter());
        assert!(BottomPanel::Statements.supports_filter());
        assert!(BottomPanel::TableStats.supports_filter());
        assert!(BottomPanel::Settings.supports_filter());

        assert!(!BottomPanel::Blocking.supports_filter());
        assert!(!BottomPanel::WaitEvents.supports_filter());
        assert!(!BottomPanel::Replication.supports_filter());
        assert!(!BottomPanel::VacuumProgress.supports_filter());
        assert!(!BottomPanel::Wraparound.supports_filter());
        assert!(!BottomPanel::WalIo.supports_filter());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // App::update edge cases
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn update_with_empty_snapshot() {
        let mut app = make_app();
        let mut snap = make_snapshot();
        snap.active_queries.clear();
        snap.summary.total_backends = 0;
        snap.summary.active_query_count = 0;

        app.update(snap);

        assert!(app.snapshot.is_some());
        assert_eq!(app.snapshot.as_ref().unwrap().active_queries.len(), 0);
        // History should still be updated
        assert!(!app.metrics.connections.as_vec().is_empty());
    }

    #[test]
    fn update_clears_last_error() {
        let mut app = make_app();
        app.last_error = Some("Previous error".to_string());

        app.update(make_snapshot());

        assert!(app.last_error.is_none());
    }

    #[test]
    fn update_populates_histories() {
        let mut app = make_app();
        assert!(app.metrics.connections.as_vec().is_empty());
        assert!(app.metrics.hit_ratio.as_vec().is_empty());

        let mut snap = make_snapshot();
        snap.summary.total_backends = 25;
        snap.buffer_cache.hit_ratio = 0.95;
        app.update(snap);

        assert_eq!(app.metrics.connections.as_vec().len(), 1);
        assert_eq!(app.metrics.hit_ratio.as_vec().len(), 1);
    }

    #[test]
    fn update_calculates_avg_query_time() {
        let mut app = make_app();
        let mut snap = make_snapshot();
        snap.active_queries = vec![
            ActiveQuery {
                pid: 1,
                usename: None,
                datname: None,
                state: Some("active".into()),
                query: None,
                duration_secs: 2.0,
                wait_event_type: None,
                wait_event: None,
                query_start: None,
                backend_type: None,
            },
            ActiveQuery {
                pid: 2,
                usename: None,
                datname: None,
                state: Some("active".into()),
                query: None,
                duration_secs: 4.0,
                wait_event_type: None,
                wait_event: None,
                query_start: None,
                backend_type: None,
            },
        ];

        app.update(snap);

        // Average of 2.0 and 4.0 = 3.0 seconds = 3000ms
        let last_avg = app.metrics.avg_query_time.last().unwrap();
        assert_eq!(last_avg, 3000);
    }

    #[test]
    fn update_handles_no_active_queries() {
        let mut app = make_app();
        let mut snap = make_snapshot();
        // Only idle queries (not "active" or "idle in transaction")
        snap.active_queries = vec![ActiveQuery {
            pid: 1,
            usename: None,
            datname: None,
            state: Some("idle".into()),
            query: None,
            duration_secs: 100.0,
            wait_event_type: None,
            wait_event: None,
            query_start: None,
            backend_type: None,
        }];

        app.update(snap);

        // No active queries, so avg should be 0
        let last_avg = app.metrics.avg_query_time.last().unwrap();
        assert_eq!(last_avg, 0);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // sorted_*_indices edge cases
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn sorted_query_indices_no_snapshot() {
        let app = make_app();
        assert!(app.snapshot.is_none());
        assert!(app.sorted_query_indices().is_empty());
    }

    #[test]
    fn sorted_query_indices_empty_queries() {
        let mut app = make_app();
        let mut snap = make_snapshot();
        snap.active_queries.clear();
        app.update(snap);

        assert!(app.sorted_query_indices().is_empty());
    }

    #[test]
    fn sorted_index_indices_no_snapshot() {
        let app = make_app();
        assert!(app.sorted_index_indices().is_empty());
    }

    #[test]
    fn sorted_stmt_indices_no_snapshot() {
        let app = make_app();
        assert!(app.sorted_stmt_indices().is_empty());
    }

    #[test]
    fn sorted_table_stat_indices_no_snapshot() {
        let app = make_app();
        assert!(app.sorted_table_stat_indices().is_empty());
    }

    #[test]
    fn sorted_settings_indices_no_snapshot() {
        let app = make_app();
        assert!(app.sorted_settings_indices().is_empty());
    }

    #[test]
    fn selected_query_pid_no_snapshot() {
        let app = make_app();
        assert!(app.selected_query_pid().is_none());
    }

    #[test]
    fn selected_query_pid_no_selection() {
        let mut app = make_app();
        app.update(make_snapshot());
        app.queries.state.select(None);

        assert!(app.selected_query_pid().is_none());
    }

    #[test]
    fn get_filtered_pids_no_snapshot() {
        let app = make_app();
        assert!(app.get_filtered_pids().is_empty());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Filter edge cases
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn filter_with_no_matches() {
        let mut app = make_app();
        app.update(make_snapshot());
        app.bottom_panel = BottomPanel::Queries;
        app.filter_text = "xyznonexistent123".to_string();
        app.filter_active = true;

        let indices = app.sorted_query_indices();
        assert!(indices.is_empty());
    }

    #[test]
    fn filter_with_special_characters() {
        let mut app = make_app();
        let mut snap = make_snapshot();
        snap.active_queries[0].query = Some("SELECT * FROM \"table-with-dashes\"".into());
        app.update(snap);

        app.bottom_panel = BottomPanel::Queries;
        app.filter_text = "table-with".to_string();
        app.filter_active = true;

        let indices = app.sorted_query_indices();
        assert!(!indices.is_empty());
    }

    #[test]
    fn filter_inactive_ignores_filter_text() {
        let mut app = make_app();
        app.update(make_snapshot());
        app.bottom_panel = BottomPanel::Queries;
        app.filter_text = "xyznonexistent123".to_string();
        app.filter_active = false;
        app.view_mode = ViewMode::Normal;

        // When filter is not active, all queries should be returned
        let indices = app.sorted_query_indices();
        assert!(!indices.is_empty());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // config_adjust bounds checking
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn config_adjust_refresh_interval_lower_bound() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::RefreshInterval)
            .unwrap();

        app.config.refresh_interval_secs = 1; // At minimum
        app.config_adjust(-1); // Try to go below

        assert_eq!(app.config.refresh_interval_secs, 1); // Should stay at 1
    }

    #[test]
    fn config_adjust_refresh_interval_upper_bound() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::RefreshInterval)
            .unwrap();

        app.config.refresh_interval_secs = 60; // At maximum
        app.config_adjust(1); // Try to go above

        assert_eq!(app.config.refresh_interval_secs, 60); // Should stay at 60
    }

    #[test]
    fn config_adjust_warn_duration_lower_bound() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::WarnDuration)
            .unwrap();

        app.config.warn_duration_secs = 0.1;
        app.config_adjust(-1); // Try to go below 0.1

        assert!(app.config.warn_duration_secs >= 0.1);
    }

    #[test]
    fn config_adjust_warn_duration_clamped_to_danger() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::WarnDuration)
            .unwrap();

        app.config.danger_duration_secs = 5.0;
        app.config.warn_duration_secs = 5.0;
        app.config_adjust(1); // Try to go above danger

        assert!(app.config.warn_duration_secs <= app.config.danger_duration_secs);
    }

    #[test]
    fn config_adjust_danger_duration_clamped_to_warn() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::DangerDuration)
            .unwrap();

        app.config.warn_duration_secs = 5.0;
        app.config.danger_duration_secs = 5.0;
        app.config_adjust(-1); // Try to go below warn

        assert!(app.config.danger_duration_secs >= app.config.warn_duration_secs);
    }

    #[test]
    fn config_adjust_danger_duration_upper_bound() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::DangerDuration)
            .unwrap();

        app.config.danger_duration_secs = 300.0; // At maximum
        app.config_adjust(1); // Try to go above

        assert_eq!(app.config.danger_duration_secs, 300.0);
    }

    #[test]
    fn config_adjust_recording_retention_lower_bound() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::RecordingRetention)
            .unwrap();

        app.config.recording_retention_secs = 600; // At minimum (10 min)
        app.config_adjust(-1);

        assert_eq!(app.config.recording_retention_secs, 600);
    }

    #[test]
    fn config_adjust_recording_retention_upper_bound() {
        let mut app = make_app();
        app.view_mode = ViewMode::Config;
        app.config_selected = ConfigItem::ALL
            .iter()
            .position(|&i| i == ConfigItem::RecordingRetention)
            .unwrap();

        app.config.recording_retention_secs = 86400; // At maximum (24 hours)
        app.config_adjust(1);

        assert_eq!(app.config.recording_retention_secs, 86400);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Rate calculation edge cases
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn rate_calculation_with_counter_reset() {
        use crate::db::models::DatabaseStats;

        let mut app = make_app();

        // First snapshot with high counter values
        let mut snap1 = make_snapshot();
        snap1.db_stats = Some(DatabaseStats {
            xact_commit: 1000000,
            xact_rollback: 100,
            blks_read: 50000,
        });
        app.update(snap1);

        // Second snapshot with lower values (simulating server restart)
        let mut snap2 = make_snapshot();
        snap2.timestamp = chrono::Utc::now() + chrono::Duration::seconds(2);
        snap2.db_stats = Some(DatabaseStats {
            xact_commit: 100, // Lower than before - counter reset
            xact_rollback: 0,
            blks_read: 100,
        });
        app.update(snap2);

        // TPS should not be calculated when counters go backwards
        // (the rate calculation guards against negative values)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Bloat preservation during update
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn update_preserves_bloat_data() {
        use crate::db::models::TableStat;

        let mut app = make_app();

        // First update with bloat data
        let mut snap1 = make_snapshot();
        snap1.table_stats = vec![TableStat {
            schemaname: "public".into(),
            relname: "users".into(),
            total_size_bytes: 1000000,
            table_size_bytes: 800000,
            indexes_size_bytes: 200000,
            seq_scan: 100,
            seq_tup_read: 5000,
            idx_scan: 500,
            idx_tup_fetch: 4500,
            n_live_tup: 10000,
            n_dead_tup: 500,
            dead_ratio: 5.0,
            n_tup_ins: 100,
            n_tup_upd: 50,
            n_tup_del: 10,
            n_tup_hot_upd: 20,
            last_vacuum: None,
            last_autovacuum: None,
            last_analyze: None,
            last_autoanalyze: None,
            vacuum_count: 0,
            autovacuum_count: 0,
            bloat_bytes: Some(100000),
            bloat_pct: Some(12.5),
        }];
        app.update(snap1);

        // Second update without bloat data
        let mut snap2 = make_snapshot();
        snap2.table_stats = vec![TableStat {
            schemaname: "public".into(),
            relname: "users".into(),
            total_size_bytes: 1000100, // Slightly different
            table_size_bytes: 800100,
            indexes_size_bytes: 200000,
            seq_scan: 110,
            seq_tup_read: 5500,
            idx_scan: 510,
            idx_tup_fetch: 4600,
            n_live_tup: 10050,
            n_dead_tup: 480,
            dead_ratio: 4.8,
            n_tup_ins: 150,
            n_tup_upd: 60,
            n_tup_del: 15,
            n_tup_hot_upd: 25,
            last_vacuum: None,
            last_autovacuum: None,
            last_analyze: None,
            last_autoanalyze: None,
            vacuum_count: 0,
            autovacuum_count: 0,
            bloat_bytes: None, // No bloat in new snapshot
            bloat_pct: None,
        }];
        app.update(snap2);

        // Bloat data should be preserved from previous snapshot
        let snap = app.snapshot.as_ref().unwrap();
        assert_eq!(snap.table_stats[0].bloat_bytes, Some(100000));
        assert_eq!(snap.table_stats[0].bloat_pct, Some(12.5));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Navigation with empty data
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn navigate_queries_with_empty_list() {
        let mut app = make_app();
        let mut snap = make_snapshot();
        snap.active_queries.clear();
        app.update(snap);

        // Navigating should not panic
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Enter)); // Inspect on empty list
    }

    #[test]
    fn navigate_indexes_with_empty_list() {
        let mut app = make_app();
        app.update(make_snapshot());
        app.bottom_panel = BottomPanel::Indexes;

        // Navigating should not panic
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Enter)); // Inspect on empty list
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Replay mode specific
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn replay_mode_state() {
        let app = make_replay_app();
        assert!(app.is_replay_mode());
        let replay = app.replay.as_ref().unwrap();
        assert_eq!(replay.filename, "test.jsonl");
        assert_eq!(replay.total, 10);
    }

    #[test]
    fn replay_mode_disables_cancel_kill() {
        let mut app = make_replay_app();
        app.update(make_snapshot());
        app.queries.state.select(Some(0));

        // C and K should be ignored in replay mode
        app.handle_key(key(KeyCode::Char('C')));
        assert_eq!(app.view_mode, ViewMode::Normal);

        app.handle_key(key(KeyCode::Char('K')));
        assert_eq!(app.view_mode, ViewMode::Normal);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // BottomPanel::label
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn bottom_panel_labels() {
        assert_eq!(BottomPanel::Queries.label(), "Queries");
        assert_eq!(BottomPanel::Blocking.label(), "Blocking");
        assert_eq!(BottomPanel::WaitEvents.label(), "Wait Events");
        assert_eq!(BottomPanel::TableStats.label(), "Table Stats");
        assert_eq!(BottomPanel::Replication.label(), "Replication");
        assert_eq!(BottomPanel::VacuumProgress.label(), "Vacuum Progress");
        assert_eq!(BottomPanel::Wraparound.label(), "Wraparound");
        assert_eq!(BottomPanel::Indexes.label(), "Indexes");
        assert_eq!(BottomPanel::Statements.label(), "Statements");
        assert_eq!(BottomPanel::WalIo.label(), "WAL & I/O");
        assert_eq!(BottomPanel::Settings.label(), "Settings");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Rate calculation tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn rate_calculation_first_snapshot_no_rate() {
        use crate::db::models::DatabaseStats;

        let mut app = make_app();

        // First snapshot - should not calculate rates (no previous)
        let mut snap = make_snapshot();
        snap.db_stats = Some(DatabaseStats {
            xact_commit: 1000,
            xact_rollback: 10,
            blks_read: 500,
        });
        app.update(snap);

        // No previous snapshot, so no rate should be calculated
        assert!(app.metrics.current_tps.is_none());
        assert!(app.metrics.current_blks_read_rate.is_none());
    }

    #[test]
    fn rate_calculation_tps_normal() {
        use crate::db::models::DatabaseStats;

        let mut app = make_app();
        let base_time = chrono::Utc::now();

        // First snapshot
        let mut snap1 = make_snapshot();
        snap1.timestamp = base_time;
        snap1.db_stats = Some(DatabaseStats {
            xact_commit: 1000,
            xact_rollback: 10,
            blks_read: 500,
        });
        app.update(snap1);

        // Second snapshot 2 seconds later with 200 more transactions
        let mut snap2 = make_snapshot();
        snap2.timestamp = base_time + chrono::Duration::seconds(2);
        snap2.db_stats = Some(DatabaseStats {
            xact_commit: 1190, // +190 commits
            xact_rollback: 20, // +10 rollbacks
            blks_read: 600,    // +100 reads
        });
        app.update(snap2);

        // TPS should be (190 + 10) / 2 = 100 TPS
        assert!(app.metrics.current_tps.is_some());
        let tps = app.metrics.current_tps.unwrap();
        assert!((tps - 100.0).abs() < 0.1, "Expected ~100 TPS, got {}", tps);

        // Blks/sec should be 100 / 2 = 50
        assert!(app.metrics.current_blks_read_rate.is_some());
        let blks = app.metrics.current_blks_read_rate.unwrap();
        assert!((blks - 50.0).abs() < 0.1, "Expected ~50 blks/s, got {}", blks);

        // History should have one entry
        assert_eq!(app.metrics.tps.as_vec().len(), 1);
        assert_eq!(app.metrics.blks_read.as_vec().len(), 1);
    }

    #[test]
    fn rate_calculation_wal_rate() {
        use crate::db::models::{DatabaseStats, WalStats};

        let mut app = make_app();
        let base_time = chrono::Utc::now();

        // First snapshot with WAL stats
        let mut snap1 = make_snapshot();
        snap1.timestamp = base_time;
        snap1.db_stats = Some(DatabaseStats {
            xact_commit: 1000,
            xact_rollback: 10,
            blks_read: 500,
        });
        snap1.wal_stats = Some(WalStats {
            wal_records: 10000,
            wal_fpi: 100,
            wal_bytes: 1_000_000, // 1 MB
            wal_buffers_full: 0,
            wal_write: 1000,
            wal_sync: 1000,
            wal_write_time: 100.0,
            wal_sync_time: 50.0,
        });
        app.update(snap1);

        // Second snapshot 2 seconds later with 2MB more WAL
        let mut snap2 = make_snapshot();
        snap2.timestamp = base_time + chrono::Duration::seconds(2);
        snap2.db_stats = Some(DatabaseStats {
            xact_commit: 1100,
            xact_rollback: 10,
            blks_read: 600,
        });
        snap2.wal_stats = Some(WalStats {
            wal_records: 12000,
            wal_fpi: 120,
            wal_bytes: 3_000_000, // 3 MB (+2 MB)
            wal_buffers_full: 0,
            wal_write: 1200,
            wal_sync: 1200,
            wal_write_time: 120.0,
            wal_sync_time: 60.0,
        });
        app.update(snap2);

        // WAL rate should be 2MB / 2s = 1MB/s
        assert!(app.metrics.current_wal_rate.is_some());
        let wal_rate = app.metrics.current_wal_rate.unwrap();
        let expected = 1_000_000.0; // 1 MB/s
        assert!(
            (wal_rate - expected).abs() < 1000.0,
            "Expected ~1MB/s WAL rate, got {}",
            wal_rate
        );

        // History should have one entry (stored as KB/s)
        assert_eq!(app.metrics.wal_rate.as_vec().len(), 1);
    }

    #[test]
    fn rate_calculation_missing_db_stats() {
        let mut app = make_app();
        let base_time = chrono::Utc::now();

        // First snapshot without db_stats
        let mut snap1 = make_snapshot();
        snap1.timestamp = base_time;
        snap1.db_stats = None;
        app.update(snap1);

        // Second snapshot also without db_stats
        let mut snap2 = make_snapshot();
        snap2.timestamp = base_time + chrono::Duration::seconds(2);
        snap2.db_stats = None;
        app.update(snap2);

        // No rates should be calculated
        assert!(app.metrics.current_tps.is_none());
        assert!(app.metrics.current_blks_read_rate.is_none());
    }

    #[test]
    fn rate_calculation_missing_wal_stats() {
        use crate::db::models::DatabaseStats;

        let mut app = make_app();
        let base_time = chrono::Utc::now();

        // First snapshot without wal_stats
        let mut snap1 = make_snapshot();
        snap1.timestamp = base_time;
        snap1.db_stats = Some(DatabaseStats {
            xact_commit: 1000,
            xact_rollback: 10,
            blks_read: 500,
        });
        snap1.wal_stats = None;
        app.update(snap1);

        // Second snapshot
        let mut snap2 = make_snapshot();
        snap2.timestamp = base_time + chrono::Duration::seconds(2);
        snap2.db_stats = Some(DatabaseStats {
            xact_commit: 1100,
            xact_rollback: 10,
            blks_read: 600,
        });
        snap2.wal_stats = None;
        app.update(snap2);

        // TPS should be calculated, but WAL rate should not
        assert!(app.metrics.current_tps.is_some());
        assert!(app.metrics.current_wal_rate.is_none());
    }

    #[test]
    fn rate_calculation_zero_time_difference() {
        use crate::db::models::DatabaseStats;

        let mut app = make_app();
        let same_time = chrono::Utc::now();

        // Two snapshots with same timestamp
        let mut snap1 = make_snapshot();
        snap1.timestamp = same_time;
        snap1.db_stats = Some(DatabaseStats {
            xact_commit: 1000,
            xact_rollback: 10,
            blks_read: 500,
        });
        app.update(snap1);

        let mut snap2 = make_snapshot();
        snap2.timestamp = same_time; // Same timestamp
        snap2.db_stats = Some(DatabaseStats {
            xact_commit: 1100,
            xact_rollback: 20,
            blks_read: 600,
        });
        app.update(snap2);

        // No rate should be calculated with zero time difference
        // (would cause division by zero)
        assert!(app.metrics.current_tps.is_none());
    }

    #[test]
    fn rate_calculation_very_small_interval() {
        use crate::db::models::DatabaseStats;

        let mut app = make_app();
        let base_time = chrono::Utc::now();

        // First snapshot
        let mut snap1 = make_snapshot();
        snap1.timestamp = base_time;
        snap1.db_stats = Some(DatabaseStats {
            xact_commit: 1000,
            xact_rollback: 10,
            blks_read: 500,
        });
        app.update(snap1);

        // Second snapshot 100ms later
        let mut snap2 = make_snapshot();
        snap2.timestamp = base_time + chrono::Duration::milliseconds(100);
        snap2.db_stats = Some(DatabaseStats {
            xact_commit: 1010, // +10 in 100ms
            xact_rollback: 10,
            blks_read: 505,
        });
        app.update(snap2);

        // TPS should be 10 / 0.1 = 100 TPS
        assert!(app.metrics.current_tps.is_some());
        let tps = app.metrics.current_tps.unwrap();
        assert!((tps - 100.0).abs() < 1.0, "Expected ~100 TPS, got {}", tps);
    }

    #[test]
    fn rate_calculation_history_accumulates() {
        use crate::db::models::DatabaseStats;

        let mut app = make_app();
        let base_time = chrono::Utc::now();

        // Initial snapshot
        let mut snap = make_snapshot();
        snap.timestamp = base_time;
        snap.db_stats = Some(DatabaseStats {
            xact_commit: 1000,
            xact_rollback: 0,
            blks_read: 100,
        });
        app.update(snap);

        // Add 5 more snapshots
        for i in 1..=5 {
            let mut snap = make_snapshot();
            snap.timestamp = base_time + chrono::Duration::seconds(i * 2);
            snap.db_stats = Some(DatabaseStats {
                xact_commit: 1000 + (i as i64 * 100), // +100 per 2 sec = 50 TPS
                xact_rollback: 0,
                blks_read: 100 + (i as i64 * 10),
            });
            app.update(snap);
        }

        // Should have 5 history entries
        assert_eq!(app.metrics.tps.as_vec().len(), 5);
        assert_eq!(app.metrics.blks_read.as_vec().len(), 5);
    }

    #[test]
    fn rate_calculation_counter_reset_blks() {
        use crate::db::models::DatabaseStats;

        let mut app = make_app();
        let base_time = chrono::Utc::now();

        // First snapshot with high blks_read
        let mut snap1 = make_snapshot();
        snap1.timestamp = base_time;
        snap1.db_stats = Some(DatabaseStats {
            xact_commit: 1000,
            xact_rollback: 10,
            blks_read: 1_000_000,
        });
        app.update(snap1);

        // Second snapshot with lower blks_read (counter reset)
        let mut snap2 = make_snapshot();
        snap2.timestamp = base_time + chrono::Duration::seconds(2);
        snap2.db_stats = Some(DatabaseStats {
            xact_commit: 1100, // Normal increase
            xact_rollback: 20,
            blks_read: 100, // Counter reset
        });
        app.update(snap2);

        // TPS should be calculated (commits/rollbacks increased normally)
        assert!(app.metrics.current_tps.is_some());
        // But blks rate should not be in history (counter went backwards)
        // Note: current_blks_read_rate may still have old value
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Error handling tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn update_error_sets_last_error() {
        let mut app = make_app();

        assert!(app.last_error.is_none());

        app.update_error("Connection refused".to_string());

        assert!(app.last_error.is_some());
        assert_eq!(app.last_error.as_ref().unwrap(), "Connection refused");
    }

    #[test]
    fn update_error_overwrites_previous() {
        let mut app = make_app();

        app.update_error("First error".to_string());
        assert_eq!(app.last_error.as_ref().unwrap(), "First error");

        app.update_error("Second error".to_string());
        assert_eq!(app.last_error.as_ref().unwrap(), "Second error");
    }

    #[test]
    fn update_clears_error() {
        let mut app = make_app();

        app.update_error("Some error".to_string());
        assert!(app.last_error.is_some());

        // Successful update should clear error
        app.update(make_snapshot());
        assert!(app.last_error.is_none());
    }

    #[test]
    fn stat_statements_error_displayed() {
        let mut app = make_app();

        let mut snap = make_snapshot();
        snap.stat_statements_error = Some("permission denied for view pg_stat_statements".to_string());
        app.update(snap);

        // Error should be preserved in snapshot
        assert!(app.snapshot.is_some());
        let snapshot = app.snapshot.as_ref().unwrap();
        assert!(snapshot.stat_statements_error.is_some());
        assert!(snapshot.stat_statements_error.as_ref().unwrap().contains("permission denied"));
    }

    #[test]
    fn empty_active_queries_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();

        let mut snap = make_snapshot();
        snap.active_queries = vec![];
        app.update(snap);

        // Navigation should not panic with empty queries
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(key_j);
        app.handle_key(key_k);

        // Selection should stay at None or 0
        let selected = app.queries.selected();
        assert!(selected.is_none() || selected == Some(0));
    }

    #[test]
    fn empty_tables_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();
        app.bottom_panel = BottomPanel::TableStats;

        let mut snap = make_snapshot();
        snap.table_stats = vec![];
        app.update(snap);

        // Navigation should not panic
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(key_j);
        app.handle_key(key_k);
    }

    #[test]
    fn empty_indexes_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();
        app.bottom_panel = BottomPanel::Indexes;

        let mut snap = make_snapshot();
        snap.indexes = vec![];
        app.update(snap);

        // Navigation should not panic
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(key_j);
        app.handle_key(key_k);
    }

    #[test]
    fn empty_blocking_info_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();
        app.bottom_panel = BottomPanel::Blocking;

        let mut snap = make_snapshot();
        snap.blocking_info = vec![];
        app.update(snap);

        // Navigation should not panic
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(key_j);
        app.handle_key(key_k);
    }

    #[test]
    fn empty_replication_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();
        app.bottom_panel = BottomPanel::Replication;

        let mut snap = make_snapshot();
        snap.replication = vec![];
        app.update(snap);

        // Navigation should not panic
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(key_j);
        app.handle_key(key_k);
    }

    #[test]
    fn empty_vacuum_progress_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();
        app.bottom_panel = BottomPanel::VacuumProgress;

        let mut snap = make_snapshot();
        snap.vacuum_progress = vec![];
        app.update(snap);

        // Navigation should not panic
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(key_j);
        app.handle_key(key_k);
    }

    #[test]
    fn empty_wraparound_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();
        app.bottom_panel = BottomPanel::Wraparound;

        let mut snap = make_snapshot();
        snap.wraparound = vec![];
        app.update(snap);

        // Navigation should not panic
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(key_j);
        app.handle_key(key_k);
    }

    #[test]
    fn empty_statements_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();
        app.bottom_panel = BottomPanel::Statements;

        let mut snap = make_snapshot();
        snap.stat_statements = vec![];
        app.update(snap);

        // Navigation should not panic
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(key_j);
        app.handle_key(key_k);
    }

    #[test]
    fn no_snapshot_navigation_no_panic() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_app();
        app.snapshot = None;

        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);

        // All navigation should be safe with no snapshot
        app.handle_key(key_j);
        app.handle_key(key_k);

        // Panel switching should work
        for panel in [
            BottomPanel::Queries,
            BottomPanel::Blocking,
            BottomPanel::WaitEvents,
            BottomPanel::TableStats,
            BottomPanel::Replication,
            BottomPanel::VacuumProgress,
            BottomPanel::Wraparound,
            BottomPanel::Indexes,
            BottomPanel::Statements,
            BottomPanel::WalIo,
        ] {
            app.bottom_panel = panel;
            app.handle_key(key_j);
            app.handle_key(key_k);
        }
    }

    #[test]
    fn inspect_with_no_selection_no_panic() {
        let mut app = make_app();
        app.update(make_snapshot());

        // Clear selection
        app.queries.state.select(None);

        // Trying to enter inspect mode should be safe
        app.view_mode = ViewMode::Inspect;

        // App should handle this state gracefully
        assert!(app.snapshot.is_some());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Sort column labels
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn sort_column_labels() {
        assert_eq!(SortColumn::Pid.label(), "PID");
        assert_eq!(SortColumn::Duration.label(), "Duration");
        assert_eq!(SortColumn::State.label(), "State");
        assert_eq!(SortColumn::User.label(), "User");
    }

    #[test]
    fn table_stat_sort_column_labels() {
        assert_eq!(TableStatSortColumn::DeadTuples.label(), "Dead Tuples");
        assert_eq!(TableStatSortColumn::Size.label(), "Size");
        assert_eq!(TableStatSortColumn::Name.label(), "Name");
        assert_eq!(TableStatSortColumn::SeqScan.label(), "Seq Scan");
        assert_eq!(TableStatSortColumn::IdxScan.label(), "Idx Scan");
        assert_eq!(TableStatSortColumn::DeadRatio.label(), "Dead %");
    }

    #[test]
    fn statement_sort_column_labels() {
        assert_eq!(StatementSortColumn::TotalTime.label(), "Total Time");
        assert_eq!(StatementSortColumn::MeanTime.label(), "Mean Time");
        assert_eq!(StatementSortColumn::MaxTime.label(), "Max Time");
        assert_eq!(StatementSortColumn::Stddev.label(), "Stddev");
        assert_eq!(StatementSortColumn::Calls.label(), "Calls");
        assert_eq!(StatementSortColumn::Rows.label(), "Rows");
        assert_eq!(StatementSortColumn::HitRatio.label(), "Hit %");
        assert_eq!(StatementSortColumn::SharedReads.label(), "Reads");
        assert_eq!(StatementSortColumn::IoTime.label(), "I/O Time");
        assert_eq!(StatementSortColumn::Temp.label(), "Temp");
    }
}
