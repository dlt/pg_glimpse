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

pub struct App {
    pub running: bool,
    pub paused: bool,
    pub snapshot: Option<PgSnapshot>,
    pub query_table_state: TableState,
    pub view_mode: ViewMode,
    pub bottom_panel: BottomPanel,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub index_table_state: TableState,
    pub index_sort_column: IndexSortColumn,
    pub index_sort_ascending: bool,
    pub stmt_table_state: TableState,
    pub stmt_sort_column: StatementSortColumn,
    pub stmt_sort_ascending: bool,
    pub table_stat_table_state: TableState,
    pub table_stat_sort_column: TableStatSortColumn,
    pub table_stat_sort_ascending: bool,
    pub replication_table_state: TableState,
    pub blocking_table_state: TableState,
    pub vacuum_table_state: TableState,
    pub wraparound_table_state: TableState,
    pub settings_table_state: TableState,

    pub connection_history: RingBuffer<u64>,
    pub avg_query_time_history: RingBuffer<u64>,
    pub hit_ratio_history: RingBuffer<u64>,
    pub active_query_history: RingBuffer<u64>,
    pub lock_count_history: RingBuffer<u64>,

    // Rate tracking
    pub prev_snapshot: Option<PgSnapshot>,
    pub tps_history: RingBuffer<u64>,
    pub wal_rate_history: RingBuffer<u64>,
    pub blks_read_history: RingBuffer<u64>,

    // Current rates (for display)
    pub current_tps: Option<f64>,
    pub current_wal_rate: Option<f64>,
    pub current_blks_read_rate: Option<f64>,

    pub server_info: ServerInfo,

    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub user: String,
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
    pub replay_mode: bool,
    pub replay_filename: Option<String>,
    pub replay_position: usize,
    pub replay_total: usize,
    pub replay_speed: f64,
    pub replay_playing: bool,
    pub overlay_scroll: u16,
    pub ssl_mode_label: Option<String>,
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
            query_table_state: TableState::default(),
            view_mode: ViewMode::Normal,
            bottom_panel: BottomPanel::Queries,
            sort_column: SortColumn::Duration,
            sort_ascending: false,
            index_table_state: TableState::default(),
            index_sort_column: IndexSortColumn::Scans,
            index_sort_ascending: true,
            stmt_table_state: TableState::default(),
            stmt_sort_column: StatementSortColumn::TotalTime,
            stmt_sort_ascending: false,
            table_stat_table_state: TableState::default(),
            table_stat_sort_column: TableStatSortColumn::DeadTuples,
            table_stat_sort_ascending: false,
            replication_table_state: TableState::default(),
            blocking_table_state: TableState::default(),
            vacuum_table_state: TableState::default(),
            wraparound_table_state: TableState::default(),
            settings_table_state: TableState::default(),
            connection_history: RingBuffer::new(history_len),
            avg_query_time_history: RingBuffer::new(history_len),
            hit_ratio_history: RingBuffer::new(history_len),
            active_query_history: RingBuffer::new(history_len),
            lock_count_history: RingBuffer::new(history_len),
            prev_snapshot: None,
            tps_history: RingBuffer::new(history_len),
            wal_rate_history: RingBuffer::new(history_len),
            blks_read_history: RingBuffer::new(history_len),
            current_tps: None,
            current_wal_rate: None,
            current_blks_read_rate: None,
            server_info,
            host,
            port,
            dbname,
            user,
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
            replay_mode: false,
            replay_filename: None,
            replay_position: 0,
            replay_total: 0,
            replay_speed: 1.0,
            replay_playing: false,
            overlay_scroll: 0,
            ssl_mode_label: None,
        }
    }

    pub fn set_ssl_mode_label(&mut self, label: &str) {
        self.ssl_mode_label = Some(label.to_string());
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
        app.replay_mode = true;
        app.replay_filename = Some(filename);
        app.replay_total = total_snapshots;
        app
    }

    pub fn update(&mut self, mut snapshot: PgSnapshot) {
        self.connection_history
            .push(snapshot.summary.total_backends as u64);

        let active: Vec<&_> = snapshot
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
        self.avg_query_time_history.push(avg_ms);

        self.hit_ratio_history
            .push((snapshot.buffer_cache.hit_ratio * 1000.0) as u64);
        self.active_query_history
            .push(snapshot.summary.active_query_count as u64);
        self.lock_count_history
            .push(snapshot.summary.lock_count as u64);

        // Calculate rates from delta
        self.calculate_rates(&snapshot);

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

    fn calculate_rates(&mut self, snap: &PgSnapshot) {
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
                        self.tps_history.push(tps as u64);
                    }

                    // Blocks read rate (physical I/O)
                    let blks = curr.blks_read - prev_db.blks_read;
                    if blks >= 0 {
                        let rate = blks as f64 / secs;
                        self.current_blks_read_rate = Some(rate);
                        self.blks_read_history.push(rate as u64);
                    }
                }

                // WAL rate from pg_stat_wal
                if let (Some(curr_wal), Some(prev_wal)) = (&snap.wal_stats, &prev.wal_stats) {
                    let bytes = curr_wal.wal_bytes - prev_wal.wal_bytes;
                    if bytes >= 0 {
                        let rate = bytes as f64 / secs;
                        self.current_wal_rate = Some(rate);
                        // Store as KB/s for sparkline (fits in u64 better)
                        self.wal_rate_history.push((rate / 1024.0) as u64);
                    }
                }
            }
        }
        self.prev_snapshot = Some(snap.clone());
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

        let asc = self.sort_ascending;
        match self.sort_column {
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
        let idx = self.query_table_state.selected()?;
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

        let asc = self.index_sort_ascending;
        match self.index_sort_column {
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

        let asc = self.stmt_sort_ascending;
        match self.stmt_sort_column {
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

        let asc = self.table_stat_sort_ascending;
        match self.table_stat_sort_column {
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
                let idx = self.query_table_state.selected().unwrap_or(0);
                let indices = self.sorted_query_indices();
                if let Some(&real_idx) = indices.get(idx) {
                    if let Some(ref q) = snap.active_queries[real_idx].query {
                        let text = q.clone();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            BottomPanel::Indexes => {
                let idx = self.index_table_state.selected().unwrap_or(0);
                let indices = self.sorted_index_indices();
                if let Some(&real_idx) = indices.get(idx) {
                    let text = snap.indexes[real_idx].index_definition.clone();
                    self.copy_to_clipboard(&text);
                }
            }
            BottomPanel::Statements => {
                let idx = self.stmt_table_state.selected().unwrap_or(0);
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
            BottomPanel::Queries => self.query_table_state.select(Some(0)),
            BottomPanel::Indexes => self.index_table_state.select(Some(0)),
            BottomPanel::Statements => self.stmt_table_state.select(Some(0)),
            BottomPanel::TableStats => self.table_stat_table_state.select(Some(0)),
            BottomPanel::Replication => self.replication_table_state.select(Some(0)),
            BottomPanel::Settings => self.settings_table_state.select(Some(0)),
            _ => {}
        }
    }

    fn handle_queries_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.query_table_state.selected().unwrap_or(0);
                self.query_table_state.select(Some(i.saturating_sub(1)));
                self.status_message = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_query_indices().len().saturating_sub(1);
                let i = self.query_table_state.selected().unwrap_or(0);
                self.query_table_state.select(Some((i + 1).min(max)));
                self.status_message = None;
            }
            KeyCode::Enter | KeyCode::Char('i') => {
                if self.selected_query_pid().is_some() {
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::Inspect;
                }
            }
            KeyCode::Char('K') if !self.replay_mode => {
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
            KeyCode::Char('C') if !self.replay_mode => {
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
                let next = self.sort_column.next();
                if next == self.sort_column {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = next;
                    self.sort_ascending = false;
                }
                self.status_message = Some(format!(
                    "Sort: {} {}",
                    self.sort_column.label(),
                    if self.sort_ascending { "\u{2191}" } else { "\u{2193}" }
                ));
            }
            _ => {}
        }
    }

    fn handle_indexes_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.index_table_state.selected().unwrap_or(0);
                self.index_table_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self
                    .sorted_index_indices()
                    .len()
                    .saturating_sub(1);
                let i = self.index_table_state.selected().unwrap_or(0);
                self.index_table_state.select(Some((i + 1).min(max)));
            }
            KeyCode::Enter => {
                if self.snapshot.as_ref().is_some_and(|s| !s.indexes.is_empty()) {
                    if self.index_table_state.selected().is_none() {
                        self.index_table_state.select(Some(0));
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::IndexInspect;
                }
            }
            KeyCode::Char('s') => {
                let next = self.index_sort_column.next();
                if next == self.index_sort_column {
                    self.index_sort_ascending = !self.index_sort_ascending;
                } else {
                    self.index_sort_column = next;
                    self.index_sort_ascending = matches!(
                        next,
                        IndexSortColumn::Scans | IndexSortColumn::Name
                    );
                }
            }
            KeyCode::Char('b') if !self.replay_mode => {
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
                let i = self.stmt_table_state.selected().unwrap_or(0);
                self.stmt_table_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self
                    .sorted_stmt_indices()
                    .len()
                    .saturating_sub(1);
                let i = self.stmt_table_state.selected().unwrap_or(0);
                self.stmt_table_state.select(Some((i + 1).min(max)));
            }
            KeyCode::Enter => {
                if self
                    .snapshot
                    .as_ref()
                    .is_some_and(|s| !s.stat_statements.is_empty())
                {
                    if self.stmt_table_state.selected().is_none() {
                        self.stmt_table_state.select(Some(0));
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::StatementInspect;
                }
            }
            KeyCode::Char('s') => {
                let next = self.stmt_sort_column.next();
                if next == self.stmt_sort_column {
                    self.stmt_sort_ascending = !self.stmt_sort_ascending;
                } else {
                    self.stmt_sort_column = next;
                    self.stmt_sort_ascending = false;
                }
                self.status_message = Some(format!(
                    "Sort: {} {}",
                    self.stmt_sort_column.label(),
                    if self.stmt_sort_ascending { "\u{2191}" } else { "\u{2193}" }
                ));
            }
            _ => {}
        }
    }

    fn handle_table_stats_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.table_stat_table_state.selected().unwrap_or(0);
                self.table_stat_table_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self
                    .sorted_table_stat_indices()
                    .len()
                    .saturating_sub(1);
                let i = self.table_stat_table_state.selected().unwrap_or(0);
                self.table_stat_table_state.select(Some((i + 1).min(max)));
            }
            KeyCode::Enter => {
                if self.snapshot.as_ref().is_some_and(|s| !s.table_stats.is_empty()) {
                    if self.table_stat_table_state.selected().is_none() {
                        self.table_stat_table_state.select(Some(0));
                    }
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::TableInspect;
                }
            }
            KeyCode::Char('s') => {
                let next = self.table_stat_sort_column.next();
                if next == self.table_stat_sort_column {
                    self.table_stat_sort_ascending = !self.table_stat_sort_ascending;
                } else {
                    self.table_stat_sort_column = next;
                    self.table_stat_sort_ascending = false;
                }
                self.status_message = Some(format!(
                    "Sort: {} {}",
                    self.table_stat_sort_column.label(),
                    if self.table_stat_sort_ascending { "\u{2191}" } else { "\u{2193}" }
                ));
            }
            KeyCode::Char('b') if !self.replay_mode => {
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

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Layer 1: Modal overlays consume all input
        match &self.view_mode {
            ViewMode::ConfirmCancel(pid) => {
                let pid = *pid;
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
                return;
            }
            ViewMode::ConfirmKill(pid) => {
                let pid = *pid;
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
                return;
            }
            ViewMode::ConfirmCancelChoice { selected_pid, all_pids } => {
                let selected_pid = *selected_pid;
                let all_pids = all_pids.clone();
                match key.code {
                    KeyCode::Char('1') | KeyCode::Char('o') => {
                        // Cancel ONE (selected)
                        self.pending_action = Some(AppAction::CancelQuery(selected_pid));
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Char('a') => {
                        // Cancel ALL matching - show batch confirmation
                        self.view_mode = ViewMode::ConfirmCancelBatch(all_pids);
                    }
                    KeyCode::Esc => {
                        self.view_mode = ViewMode::Normal;
                        self.status_message = Some("Cancel aborted".into());
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::ConfirmKillChoice { selected_pid, all_pids } => {
                let selected_pid = *selected_pid;
                let all_pids = all_pids.clone();
                match key.code {
                    KeyCode::Char('1') | KeyCode::Char('o') => {
                        // Kill ONE (selected)
                        self.pending_action = Some(AppAction::TerminateBackend(selected_pid));
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Char('a') => {
                        // Kill ALL matching - show batch confirmation
                        self.view_mode = ViewMode::ConfirmKillBatch(all_pids);
                    }
                    KeyCode::Esc => {
                        self.view_mode = ViewMode::Normal;
                        self.status_message = Some("Kill aborted".into());
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::ConfirmCancelBatch(pids) => {
                let pids = pids.clone();
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
                return;
            }
            ViewMode::ConfirmKillBatch(pids) => {
                let pids = pids.clone();
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
                return;
            }
            ViewMode::Inspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
                    }
                    KeyCode::Char('y') => {
                        if let Some(snap) = &self.snapshot {
                            let idx = self.query_table_state.selected().unwrap_or(0);
                            let indices = self.sorted_query_indices();
                            if let Some(&real_idx) = indices.get(idx) {
                                if let Some(ref q) = snap.active_queries[real_idx].query {
                                    let text = q.clone();
                                    self.copy_to_clipboard(&text);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::IndexInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
                    }
                    KeyCode::Char('y') => {
                        if let Some(snap) = &self.snapshot {
                            let idx = self.index_table_state.selected().unwrap_or(0);
                            let indices = self.sorted_index_indices();
                            if let Some(&real_idx) = indices.get(idx) {
                                let text = snap.indexes[real_idx].index_definition.clone();
                                self.copy_to_clipboard(&text);
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::StatementInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
                    }
                    KeyCode::Char('y') => {
                        if let Some(snap) = &self.snapshot {
                            let idx = self.stmt_table_state.selected().unwrap_or(0);
                            let indices = self.sorted_stmt_indices();
                            if let Some(&real_idx) = indices.get(idx) {
                                let text = snap.stat_statements[real_idx].query.clone();
                                self.copy_to_clipboard(&text);
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::ReplicationInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
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
                    _ => {}
                }
                return;
            }
            ViewMode::TableInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
                    }
                    KeyCode::Char('y') => {
                        if let Some(snap) = &self.snapshot {
                            let sel = self.table_stat_table_state.selected().unwrap_or(0);
                            let indices = self.sorted_table_stat_indices();
                            if let Some(&real_idx) = indices.get(sel) {
                                let t = &snap.table_stats[real_idx];
                                let text = format!("{}.{}", t.schemaname, t.relname);
                                self.copy_to_clipboard(&text);
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::BlockingInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
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
                    _ => {}
                }
                return;
            }
            ViewMode::VacuumInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
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
                    _ => {}
                }
                return;
            }
            ViewMode::WraparoundInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
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
                    _ => {}
                }
                return;
            }
            ViewMode::Config => {
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
                return;
            }
            ViewMode::Help => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        self.overlay_scroll = 0;
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('g') => {
                        self.overlay_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.overlay_scroll = u16::MAX;
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::Filter => {
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
                return;
            }
            ViewMode::Normal => {}
        }

        // Layer 2: Normal mode global keys
        match key.code {
            KeyCode::Char('q') => {
                if self.bottom_panel == BottomPanel::Queries {
                    self.running = false;
                } else {
                    self.switch_panel(BottomPanel::Queries);
                }
                return;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
                return;
            }
            KeyCode::Esc => {
                if self.bottom_panel != BottomPanel::Queries {
                    // Go back to Queries panel
                    self.switch_panel(BottomPanel::Queries);
                } else {
                    self.running = false;
                }
                return;
            }
            KeyCode::Char('p') if !self.replay_mode => {
                self.paused = !self.paused;
                return;
            }
            KeyCode::Char('r') if !self.replay_mode => {
                self.pending_action = Some(AppAction::ForceRefresh);
                return;
            }
            KeyCode::Char('?') => {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Help;
                return;
            }
            KeyCode::Char(',') => {
                self.view_mode = ViewMode::Config;
                return;
            }
            KeyCode::Char('y') => {
                self.yank_selected();
                return;
            }
            _ => {}
        }

        // Layer 3: Panel-switch keys
        match key.code {
            KeyCode::Char('Q') => {
                self.switch_panel(BottomPanel::Queries);
                return;
            }
            KeyCode::Tab => {
                self.switch_panel(BottomPanel::Blocking);
                return;
            }
            KeyCode::Char('w') => {
                self.switch_panel(BottomPanel::WaitEvents);
                return;
            }
            KeyCode::Char('t') => {
                self.switch_panel(BottomPanel::TableStats);
                return;
            }
            KeyCode::Char('R') => {
                self.switch_panel(BottomPanel::Replication);
                return;
            }
            KeyCode::Char('v') => {
                self.switch_panel(BottomPanel::VacuumProgress);
                return;
            }
            KeyCode::Char('x') => {
                self.switch_panel(BottomPanel::Wraparound);
                return;
            }
            KeyCode::Char('I') => {
                self.switch_panel(BottomPanel::Indexes);
                return;
            }
            KeyCode::Char('S') => {
                self.switch_panel(BottomPanel::Statements);
                return;
            }
            KeyCode::Char('A') => {
                self.switch_panel(BottomPanel::WalIo);
                return;
            }
            KeyCode::Char('P') => {
                self.switch_panel(BottomPanel::Settings);
                return;
            }
            KeyCode::Char('/') => {
                if self.bottom_panel.supports_filter() {
                    self.view_mode = ViewMode::Filter;
                }
                return;
            }
            _ => {}
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
        app.query_table_state.select(Some(5));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.query_table_state.selected(), Some(4));

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.query_table_state.selected(), Some(3));
    }

    #[test]
    fn queries_panel_sort_cycle() {
        let mut app = make_app();
        assert_eq!(app.sort_column, SortColumn::Duration);

        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(app.sort_column, SortColumn::Pid);

        app.handle_key(key(KeyCode::Char('s')));
        assert_eq!(app.sort_column, SortColumn::User);
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
        app.query_table_state.select(Some(0));
        app.handle_key(key(KeyCode::Char('C')));
        // Should not enter any confirm mode
        assert_eq!(app.view_mode, ViewMode::Normal);
    }

    #[test]
    fn kill_disabled_in_replay_mode() {
        let mut app = make_replay_app();
        app.snapshot = Some(make_snapshot());
        app.query_table_state.select(Some(0));
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
}
