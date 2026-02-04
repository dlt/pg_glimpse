use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;

use crate::config::{AppConfig, ConfigItem};
use crate::db::models::PgSnapshot;
use crate::history::RingBuffer;
use crate::ui::theme;

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Normal,
    Inspect,
    Blocking,
    WaitEvents,
    TableStats,
    Replication,
    VacuumProgress,
    Wraparound,
    Indexes,
    IndexInspect,
    Statements,
    StatementInspect,
    ConfirmCancel(i32),
    ConfirmKill(i32),
    Config,
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
pub enum StatementSortColumn {
    TotalTime,
    MeanTime,
    MaxTime,
    Calls,
    Rows,
    RowsPerCall,
    Buffers,
}

impl StatementSortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::TotalTime => Self::MeanTime,
            Self::MeanTime => Self::MaxTime,
            Self::MaxTime => Self::Calls,
            Self::Calls => Self::Rows,
            Self::Rows => Self::RowsPerCall,
            Self::RowsPerCall => Self::Buffers,
            Self::Buffers => Self::TotalTime,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TotalTime => "Total Time",
            Self::MeanTime => "Mean Time",
            Self::MaxTime => "Max Time",
            Self::Calls => "Calls",
            Self::Rows => "Rows",
            Self::RowsPerCall => "Rows/Call",
            Self::Buffers => "Buffers",
        }
    }
}

pub struct App {
    pub running: bool,
    pub paused: bool,
    pub snapshot: Option<PgSnapshot>,
    pub query_table_state: TableState,
    pub view_mode: ViewMode,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub index_table_state: TableState,
    pub index_sort_column: IndexSortColumn,
    pub index_sort_ascending: bool,
    pub stmt_table_state: TableState,
    pub stmt_sort_column: StatementSortColumn,
    pub stmt_sort_ascending: bool,

    pub connection_history: RingBuffer<u64>,
    pub avg_query_time_history: RingBuffer<u64>,
    pub lock_count_history: RingBuffer<u64>,
    pub hit_ratio_history: RingBuffer<u64>,

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
}

impl App {
    pub fn new(
        host: String,
        port: u16,
        dbname: String,
        user: String,
        refresh: u64,
        history_len: usize,
        config: AppConfig,
    ) -> Self {
        Self {
            running: true,
            paused: false,
            snapshot: None,
            query_table_state: TableState::default(),
            view_mode: ViewMode::Normal,
            sort_column: SortColumn::Duration,
            sort_ascending: false,
            index_table_state: TableState::default(),
            index_sort_column: IndexSortColumn::Scans,
            index_sort_ascending: true,
            stmt_table_state: TableState::default(),
            stmt_sort_column: StatementSortColumn::TotalTime,
            stmt_sort_ascending: false,
            connection_history: RingBuffer::new(history_len),
            avg_query_time_history: RingBuffer::new(history_len),
            lock_count_history: RingBuffer::new(history_len),
            hit_ratio_history: RingBuffer::new(history_len),
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
        }
    }

    pub fn update(&mut self, snapshot: PgSnapshot) {
        self.connection_history
            .push(snapshot.summary.total_backends as u64);

        // Average duration of active queries in milliseconds
        let active: Vec<&_> = snapshot
            .active_queries
            .iter()
            .filter(|q| q.state.as_deref() == Some("active"))
            .collect();
        let avg_ms = if active.is_empty() {
            0u64
        } else {
            let sum: f64 = active.iter().map(|q| q.duration_secs).sum();
            (sum / active.len() as f64 * 1000.0) as u64
        };
        self.avg_query_time_history.push(avg_ms);

        self.lock_count_history
            .push(snapshot.summary.lock_count as u64);
        self.hit_ratio_history
            .push((snapshot.buffer_cache.hit_ratio * 1000.0) as u64);
        self.snapshot = Some(snapshot);
        self.last_error = None;
    }

    pub fn update_error(&mut self, err: String) {
        self.last_error = Some(err);
    }

    pub fn sorted_query_indices(&self) -> Vec<usize> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let mut indices: Vec<usize> = (0..snap.active_queries.len()).collect();
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
            StatementSortColumn::RowsPerCall => indices.sort_by(|&a, &b| {
                let rpc = |s: &crate::db::models::StatStatement| {
                    if s.calls > 0 { s.rows as f64 / s.calls as f64 } else { 0.0 }
                };
                let cmp = rpc(&snap.stat_statements[a])
                    .partial_cmp(&rpc(&snap.stat_statements[b]))
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { cmp } else { cmp.reverse() }
            }),
            StatementSortColumn::Buffers => indices.sort_by(|&a, &b| {
                let bufs = |s: &crate::db::models::StatStatement| {
                    s.shared_blks_hit + s.shared_blks_read
                };
                let cmp = bufs(&snap.stat_statements[a])
                    .cmp(&bufs(&snap.stat_statements[b]));
                if asc { cmp } else { cmp.reverse() }
            }),
        }
        indices
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
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
            ViewMode::Indexes => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = self.index_table_state.selected().unwrap_or(0);
                        self.index_table_state.select(Some(i.saturating_sub(1)));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let max = self
                            .snapshot
                            .as_ref()
                            .map_or(0, |s| s.indexes.len().saturating_sub(1));
                        let i = self.index_table_state.selected().unwrap_or(0);
                        self.index_table_state.select(Some((i + 1).min(max)));
                    }
                    KeyCode::Enter => {
                        if self.snapshot.as_ref().is_some_and(|s| !s.indexes.is_empty()) {
                            if self.index_table_state.selected().is_none() {
                                self.index_table_state.select(Some(0));
                            }
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
                return;
            }
            ViewMode::IndexInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.view_mode = ViewMode::Indexes;
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::Statements => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.view_mode = ViewMode::Normal;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = self.stmt_table_state.selected().unwrap_or(0);
                        self.stmt_table_state.select(Some(i.saturating_sub(1)));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let max = self
                            .snapshot
                            .as_ref()
                            .map_or(0, |s| s.stat_statements.len().saturating_sub(1));
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
                            if self.stmt_sort_ascending { "↑" } else { "↓" }
                        ));
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::StatementInspect => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.view_mode = ViewMode::Statements;
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
            ViewMode::Inspect
            | ViewMode::Blocking
            | ViewMode::WaitEvents
            | ViewMode::TableStats
            | ViewMode::Replication
            | ViewMode::VacuumProgress
            | ViewMode::Wraparound => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        self.view_mode = ViewMode::Normal;
                    }
                    _ => {}
                }
                return;
            }
            ViewMode::Normal => {}
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            KeyCode::Char('p') => self.paused = !self.paused,
            KeyCode::Char('r') => {
                self.pending_action = Some(AppAction::ForceRefresh);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.query_table_state.selected().unwrap_or(0);
                self.query_table_state.select(Some(i.saturating_sub(1)));
                self.status_message = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self
                    .snapshot
                    .as_ref()
                    .map_or(0, |s| s.active_queries.len().saturating_sub(1));
                let i = self.query_table_state.selected().unwrap_or(0);
                self.query_table_state.select(Some((i + 1).min(max)));
                self.status_message = None;
            }
            KeyCode::Enter | KeyCode::Char('i') => {
                if self.selected_query_pid().is_some() {
                    self.view_mode = ViewMode::Inspect;
                }
            }
            KeyCode::Char('K') => {
                if let Some(pid) = self.selected_query_pid() {
                    self.view_mode = ViewMode::ConfirmKill(pid);
                }
            }
            KeyCode::Char('C') => {
                if let Some(pid) = self.selected_query_pid() {
                    self.view_mode = ViewMode::ConfirmCancel(pid);
                }
            }
            KeyCode::Tab => {
                self.view_mode = ViewMode::Blocking;
            }
            KeyCode::Char('w') => {
                self.view_mode = ViewMode::WaitEvents;
            }
            KeyCode::Char('t') => {
                self.view_mode = ViewMode::TableStats;
            }
            KeyCode::Char('R') => {
                self.view_mode = ViewMode::Replication;
            }
            KeyCode::Char('v') => {
                self.view_mode = ViewMode::VacuumProgress;
            }
            KeyCode::Char('x') => {
                self.view_mode = ViewMode::Wraparound;
            }
            KeyCode::Char('I') => {
                self.view_mode = ViewMode::Indexes;
            }
            KeyCode::Char('S') => {
                self.view_mode = ViewMode::Statements;
            }
            KeyCode::Char(',') => {
                self.view_mode = ViewMode::Config;
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
                    if self.sort_ascending { "↑" } else { "↓" }
                ));
            }
            _ => {}
        }
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
        }
    }
}
