use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher};
use ratatui::widgets::TableState;

use crate::config::{AppConfig, ConfigItem};
use crate::db::models::{ActiveQuery, IndexInfo, PgSnapshot, ServerInfo, StatStatement};
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
}

impl BottomPanel {
    pub fn supports_filter(self) -> bool {
        matches!(self, Self::Queries | Self::Indexes | Self::Statements)
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
    ConfirmCancel(i32),
    ConfirmKill(i32),
    Config,
    Help,
}

#[derive(Debug, Clone)]
pub enum AppAction {
    CancelQuery(i32),
    TerminateBackend(i32),
    ForceRefresh,
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

    pub connection_history: RingBuffer<u64>,
    pub avg_query_time_history: RingBuffer<u64>,
    pub hit_ratio_history: RingBuffer<u64>,
    pub active_query_history: RingBuffer<u64>,
    pub lock_count_history: RingBuffer<u64>,

    pub server_info: ServerInfo,

    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub user: String,
    pub refresh_interval_secs: u64,

    pub last_error: Option<String>,
    pub status_message: Option<String>,
    pub pending_action: Option<AppAction>,

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
            connection_history: RingBuffer::new(history_len),
            avg_query_time_history: RingBuffer::new(history_len),
            hit_ratio_history: RingBuffer::new(history_len),
            active_query_history: RingBuffer::new(history_len),
            lock_count_history: RingBuffer::new(history_len),
            server_info,
            host,
            port,
            dbname,
            user,
            refresh_interval_secs: refresh,
            last_error: None,
            status_message: None,
            pending_action: None,
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
        }
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

    pub fn update(&mut self, snapshot: PgSnapshot) {
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
        self.snapshot = Some(snapshot);
        self.last_error = None;
    }

    pub fn update_error(&mut self, err: String) {
        self.last_error = Some(err);
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
                    self.view_mode = ViewMode::ConfirmKill(pid);
                }
            }
            KeyCode::Char('C') if !self.replay_mode => {
                if let Some(pid) = self.selected_query_pid() {
                    self.view_mode = ViewMode::ConfirmCancel(pid);
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
            _ => {}
        }
    }

    fn handle_panel_key(&mut self, key: KeyEvent) {
        match self.bottom_panel {
            BottomPanel::Queries => self.handle_queries_key(key),
            BottomPanel::Indexes => self.handle_indexes_key(key),
            BottomPanel::Statements => self.handle_statements_key(key),
            BottomPanel::TableStats => self.handle_table_stats_key(key),
            _ => {} // Static panels have no panel-specific keys
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
                self.running = false;
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
