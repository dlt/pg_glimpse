//! Application state and key handling.

mod actions;
mod panels;
mod sorting;
mod state;

pub use actions::AppAction;
pub use panels::{BottomPanel, ConfirmAction, InspectTarget, ViewMode};
pub use sorting::{
    IndexSortColumn, SortColumn, SortColumnTrait, StatementSortColumn, TableStatSortColumn,
};
pub use state::{ConfigOverlay, ConnectionInfo, FilterState, MetricsHistory, PanelStates, RecordingsBrowser, ReplayState, TableViewState, UiFeedback};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{AppConfig, ConfigItem};
use crate::db::models::{PgSnapshot, ServerInfo};
use crate::db::queries::{IndexBloat, TableBloat};
use crate::ui::theme;

use sorting::{sort_by_key, sort_by_key_partial, Filterable};

/// Max characters to show in clipboard preview messages
const CLIPBOARD_PREVIEW_LEN: usize = 40;

pub struct App {
    // Core runtime
    pub running: bool,
    pub paused: bool,
    pub snapshot: Option<PgSnapshot>,
    pub view_mode: ViewMode,
    pub bottom_panel: BottomPanel,

    // Panel states (consolidated)
    pub panels: PanelStates,

    pub metrics: MetricsHistory,

    pub server_info: ServerInfo,
    pub connection: ConnectionInfo,
    pub refresh_interval_secs: u64,

    // UI feedback (errors, status, loading)
    pub feedback: UiFeedback,

    pub config: AppConfig,
    pub config_overlay: ConfigOverlay,

    pub filter: FilterState,
    pub replay: Option<ReplayState>,
    pub overlay_scroll: u16,

    // Recordings browser state
    pub recordings: RecordingsBrowser,
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
            panels: PanelStates::new(),
            metrics: MetricsHistory::new(history_len),
            server_info,
            connection: ConnectionInfo::new(host, port, dbname, user),
            refresh_interval_secs: refresh,
            feedback: UiFeedback::new(),
            config,
            config_overlay: ConfigOverlay::new(),
            filter: FilterState::default(),
            replay: None,
            overlay_scroll: 0,
            recordings: RecordingsBrowser::new(),
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
    pub const fn is_replay_mode(&self) -> bool {
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
        self.feedback.last_error = None;
    }

    pub fn update_error(&mut self, err: String) {
        self.feedback.last_error = Some(err);
    }

    /// Apply bloat estimates to current snapshot's `table_stats` and indexes
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
                    table.bloat_source = Some(bloat.source);
                }
            }
            // Apply index bloat
            for index in &mut snapshot.indexes {
                let key = format!("{}.{}", index.schemaname, index.index_name);
                if let Some(bloat) = index_bloat.get(&key) {
                    index.bloat_bytes = Some(bloat.bloat_bytes);
                    index.bloat_pct = Some(bloat.bloat_pct);
                    index.bloat_source = Some(bloat.source);
                }
            }
        }
    }

    /// Check if fuzzy filter should be applied for the given panel
    fn should_apply_filter(&self, panel: BottomPanel) -> bool {
        self.bottom_panel == panel
            && !self.filter.text.is_empty()
            && (self.filter.active || self.view_mode == ViewMode::Filter)
    }

    /// Build indices for items, optionally applying fuzzy filter.
    fn filtered_indices<T: Filterable>(&self, items: &[T], panel: BottomPanel) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..items.len()).collect();
        if self.should_apply_filter(panel) {
            let mut matcher = Matcher::new(MatcherConfig::DEFAULT);
            let pattern =
                Pattern::parse(&self.filter.text, CaseMatching::Ignore, Normalization::Smart);
            indices.retain(|&i| {
                let haystack = items[i].filter_string();
                let mut buf = Vec::new();
                pattern
                    .score(
                        nucleo_matcher::Utf32Str::new(&haystack, &mut buf),
                        &mut matcher,
                    )
                    .is_some()
            });
        }
        indices
    }

    pub fn sorted_query_indices(&self) -> Vec<usize> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let mut indices = self.filtered_indices(&snap.active_queries, BottomPanel::Queries);

        let asc = self.panels.queries.sort_ascending;
        let q = &snap.active_queries;
        match self.panels.queries.sort_column {
            SortColumn::Pid => sort_by_key(&mut indices, q, asc, |x| x.pid),
            SortColumn::Duration => sort_by_key_partial(&mut indices, q, asc, |x| x.duration_secs),
            SortColumn::State => sort_by_key(&mut indices, q, asc, |x| x.state.clone()),
            SortColumn::User => sort_by_key(&mut indices, q, asc, |x| x.usename.clone()),
        }
        indices
    }

    pub fn selected_query_pid(&self) -> Option<i32> {
        let snap = self.snapshot.as_ref()?;
        let idx = self.panels.queries.selected()?;
        let indices = self.sorted_query_indices();
        let &real_idx = indices.get(idx)?;
        Some(snap.active_queries[real_idx].pid)
    }

    pub fn selected_index_key(&self) -> Option<String> {
        let snap = self.snapshot.as_ref()?;
        let idx = self.panels.indexes.selected().or(Some(0))?;
        let indices = self.sorted_index_indices();
        let &real_idx = indices.get(idx)?;
        let index = &snap.indexes[real_idx];
        Some(format!("{}.{}", index.schemaname, index.index_name))
    }

    pub fn selected_statement_queryid(&self) -> Option<i64> {
        let snap = self.snapshot.as_ref()?;
        let idx = self.panels.statements.selected().or(Some(0))?;
        let indices = self.sorted_stmt_indices();
        let &real_idx = indices.get(idx)?;
        Some(snap.stat_statements[real_idx].queryid)
    }

    pub fn selected_table_key(&self) -> Option<String> {
        let snap = self.snapshot.as_ref()?;
        let idx = self.panels.table_stats.selected().or(Some(0))?;
        let indices = self.sorted_table_stat_indices();
        let &real_idx = indices.get(idx)?;
        let table = &snap.table_stats[real_idx];
        Some(format!("{}.{}", table.schemaname, table.relname))
    }

    pub fn selected_replication_pid(&self) -> Option<i32> {
        let snap = self.snapshot.as_ref()?;
        let sel = self.panels.replication.selected().or(Some(0))?;
        Some(snap.replication.get(sel)?.pid)
    }

    pub fn selected_blocking_pid(&self) -> Option<i32> {
        let snap = self.snapshot.as_ref()?;
        let sel = self.panels.blocking.selected().or(Some(0))?;
        Some(snap.blocking_info.get(sel)?.blocked_pid)
    }

    pub fn selected_vacuum_pid(&self) -> Option<i32> {
        let snap = self.snapshot.as_ref()?;
        let sel = self.panels.vacuum.selected().or(Some(0))?;
        Some(snap.vacuum_progress.get(sel)?.pid)
    }

    pub fn selected_wraparound_datname(&self) -> Option<String> {
        let snap = self.snapshot.as_ref()?;
        let sel = self.panels.wraparound.selected().or(Some(0))?;
        Some(snap.wraparound.get(sel)?.datname.clone())
    }

    pub fn selected_setting_name(&self) -> Option<String> {
        let indices = self.sorted_settings_indices();
        let idx = self.panels.settings.selected().or(Some(0))?;
        let &real_idx = indices.get(idx)?;
        Some(self.server_info.settings[real_idx].name.clone())
    }

    pub fn selected_extension_name(&self) -> Option<String> {
        let indices = self.sorted_extensions_indices();
        let idx = self.panels.extensions.selected().or(Some(0))?;
        let &real_idx = indices.get(idx)?;
        Some(self.server_info.extensions_list[real_idx].name.clone())
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
        let mut indices = self.filtered_indices(&snap.indexes, BottomPanel::Indexes);

        let asc = self.panels.indexes.sort_ascending;
        let idx = &snap.indexes;
        match self.panels.indexes.sort_column {
            IndexSortColumn::Scans => sort_by_key(&mut indices, idx, asc, |x| x.idx_scan),
            IndexSortColumn::Size => sort_by_key(&mut indices, idx, asc, |x| x.index_size_bytes),
            IndexSortColumn::Name => sort_by_key(&mut indices, idx, asc, |x| x.index_name.clone()),
            IndexSortColumn::TupRead => sort_by_key(&mut indices, idx, asc, |x| x.idx_tup_read),
            IndexSortColumn::TupFetch => sort_by_key(&mut indices, idx, asc, |x| x.idx_tup_fetch),
        }
        indices
    }

    pub fn sorted_stmt_indices(&self) -> Vec<usize> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let mut indices = self.filtered_indices(&snap.stat_statements, BottomPanel::Statements);

        let asc = self.panels.statements.sort_ascending;
        let s = &snap.stat_statements;
        match self.panels.statements.sort_column {
            StatementSortColumn::TotalTime => {
                sort_by_key_partial(&mut indices, s, asc, |x| x.total_exec_time)
            }
            StatementSortColumn::MeanTime => {
                sort_by_key_partial(&mut indices, s, asc, |x| x.mean_exec_time)
            }
            StatementSortColumn::MaxTime => {
                sort_by_key_partial(&mut indices, s, asc, |x| x.max_exec_time)
            }
            StatementSortColumn::Stddev => {
                sort_by_key_partial(&mut indices, s, asc, |x| x.stddev_exec_time)
            }
            StatementSortColumn::Calls => sort_by_key(&mut indices, s, asc, |x| x.calls),
            StatementSortColumn::Rows => sort_by_key(&mut indices, s, asc, |x| x.rows),
            StatementSortColumn::HitRatio => {
                sort_by_key_partial(&mut indices, s, asc, |x| x.hit_ratio)
            }
            StatementSortColumn::SharedReads => {
                sort_by_key(&mut indices, s, asc, |x| x.shared_blks_read)
            }
            StatementSortColumn::IoTime => {
                sort_by_key_partial(&mut indices, s, asc, |x| x.blk_read_time + x.blk_write_time)
            }
            StatementSortColumn::Temp => {
                sort_by_key(&mut indices, s, asc, |x| x.temp_blks_read + x.temp_blks_written)
            }
        }
        indices
    }

    pub fn sorted_table_stat_indices(&self) -> Vec<usize> {
        let Some(snap) = &self.snapshot else {
            return vec![];
        };
        let mut indices = self.filtered_indices(&snap.table_stats, BottomPanel::TableStats);

        let asc = self.panels.table_stats.sort_ascending;
        let t = &snap.table_stats;
        match self.panels.table_stats.sort_column {
            TableStatSortColumn::DeadTuples => sort_by_key(&mut indices, t, asc, |x| x.n_dead_tup),
            TableStatSortColumn::Size => sort_by_key(&mut indices, t, asc, |x| x.total_size_bytes),
            TableStatSortColumn::Name => sort_by_key(&mut indices, t, asc, |x| x.relname.clone()),
            TableStatSortColumn::SeqScan => sort_by_key(&mut indices, t, asc, |x| x.seq_scan),
            TableStatSortColumn::IdxScan => sort_by_key(&mut indices, t, asc, |x| x.idx_scan),
            TableStatSortColumn::DeadRatio => {
                sort_by_key_partial(&mut indices, t, asc, |x| x.dead_ratio)
            }
        }
        indices
    }

    pub fn sorted_settings_indices(&self) -> Vec<usize> {
        // Settings are already sorted by category, name from the query
        self.filtered_indices(&self.server_info.settings, BottomPanel::Settings)
    }

    pub fn sorted_extensions_indices(&self) -> Vec<usize> {
        // Extensions are already sorted by name from the query
        self.filtered_indices(&self.server_info.extensions_list, BottomPanel::Extensions)
    }

    fn copy_to_clipboard(&mut self, text: &str) {
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
            Ok(()) => {
                let preview: String = text.chars().take(CLIPBOARD_PREVIEW_LEN).collect();
                let suffix = if text.len() > CLIPBOARD_PREVIEW_LEN { "..." } else { "" };
                self.feedback.status_message = Some(format!("Copied: {preview}{suffix}"));
            }
            Err(e) => {
                self.feedback.status_message = Some(format!("Clipboard error: {e}"));
            }
        }
    }

    fn yank_selected(&mut self) {
        let Some(snap) = &self.snapshot else {
            return;
        };
        match self.bottom_panel {
            BottomPanel::Queries => {
                let idx = self.panels.queries.selected().unwrap_or(0);
                let indices = self.sorted_query_indices();
                if let Some(&real_idx) = indices.get(idx) {
                    if let Some(ref q) = snap.active_queries[real_idx].query {
                        let text = q.clone();
                        self.copy_to_clipboard(&text);
                    }
                }
            }
            BottomPanel::Indexes => {
                let idx = self.panels.indexes.selected().unwrap_or(0);
                let indices = self.sorted_index_indices();
                if let Some(&real_idx) = indices.get(idx) {
                    let text = snap.indexes[real_idx].index_definition.clone();
                    self.copy_to_clipboard(&text);
                }
            }
            BottomPanel::Statements => {
                let idx = self.panels.statements.selected().unwrap_or(0);
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
        self.filter.clear();
        self.view_mode = ViewMode::Normal;
    }

    fn reset_panel_selection(&mut self) {
        self.panels.reset_selection(self.bottom_panel);
    }

    fn handle_queries_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.panels.queries.select_prev();
                self.feedback.status_message = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_query_indices().len();
                self.panels.queries.select_next(max);
                self.feedback.status_message = None;
            }
            KeyCode::Enter | KeyCode::Char('i') => {
                if let Some(pid) = self.selected_query_pid() {
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::Inspect(InspectTarget::Query(pid));
                }
            }
            KeyCode::Char('K') if self.replay.is_none() => {
                if let Some(pid) = self.selected_query_pid() {
                    let filtered_pids = self.get_filtered_pids();
                    if self.filter.active && filtered_pids.len() > 1 {
                        // Multiple matches - show choice dialog
                        self.view_mode = ViewMode::Confirm(ConfirmAction::KillChoice {
                            selected_pid: pid,
                            all_pids: filtered_pids,
                        });
                    } else {
                        // Single query - existing behavior
                        self.view_mode = ViewMode::Confirm(ConfirmAction::Kill(pid));
                    }
                }
            }
            KeyCode::Char('C') if self.replay.is_none() => {
                if let Some(pid) = self.selected_query_pid() {
                    let filtered_pids = self.get_filtered_pids();
                    if self.filter.active && filtered_pids.len() > 1 {
                        // Multiple matches - show choice dialog
                        self.view_mode = ViewMode::Confirm(ConfirmAction::CancelChoice {
                            selected_pid: pid,
                            all_pids: filtered_pids,
                        });
                    } else {
                        // Single query - existing behavior
                        self.view_mode = ViewMode::Confirm(ConfirmAction::Cancel(pid));
                    }
                }
            }
            KeyCode::Char('s') => {
                self.panels.queries.cycle_sort();
                self.panels.queries.select_first();
                self.feedback.status_message = Some(format!(
                    "Sort: {} {}",
                    self.panels.queries.sort_column.label(),
                    if self.panels.queries.sort_ascending {
                        "\u{2191}"
                    } else {
                        "\u{2193}"
                    }
                ));
            }
            _ => {}
        }
    }

    fn handle_indexes_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.panels.indexes.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_index_indices().len();
                self.panels.indexes.select_next(max);
            }
            KeyCode::Enter => {
                if let Some(key) = self.selected_index_key() {
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::Inspect(InspectTarget::Index(key));
                }
            }
            KeyCode::Char('s') => {
                self.panels.indexes.cycle_sort();
                self.panels.indexes.select_first();
                // Default ascending for Name/Scans, descending for others
                self.panels.indexes.sort_ascending = matches!(
                    self.panels.indexes.sort_column,
                    IndexSortColumn::Scans | IndexSortColumn::Name
                );
            }
            KeyCode::Char('b') if self.replay.is_none() => {
                self.feedback.pending_action = Some(AppAction::RefreshBloat);
                self.feedback.status_message = Some("Refreshing bloat estimates...".to_string());
                self.feedback.bloat_loading = true;
            }
            _ => {}
        }
    }

    fn handle_statements_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.panels.statements.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_stmt_indices().len();
                self.panels.statements.select_next(max);
            }
            KeyCode::Enter => {
                if let Some(queryid) = self.selected_statement_queryid() {
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::Inspect(InspectTarget::Statement(queryid));
                }
            }
            KeyCode::Char('s') => {
                self.panels.statements.cycle_sort();
                self.panels.statements.select_first();
                self.feedback.status_message = Some(format!(
                    "Sort: {} {}",
                    self.panels.statements.sort_column.label(),
                    if self.panels.statements.sort_ascending {
                        "\u{2191}"
                    } else {
                        "\u{2193}"
                    }
                ));
            }
            _ => {}
        }
    }

    fn handle_table_stats_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.panels.table_stats.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.sorted_table_stat_indices().len();
                self.panels.table_stats.select_next(max);
            }
            KeyCode::Enter => {
                if let Some(key) = self.selected_table_key() {
                    self.overlay_scroll = 0;
                    self.view_mode = ViewMode::Inspect(InspectTarget::Table(key));
                }
            }
            KeyCode::Char('s') => {
                self.panels.table_stats.cycle_sort();
                self.panels.table_stats.select_first();
                self.feedback.status_message = Some(format!(
                    "Sort: {} {}",
                    self.panels.table_stats.sort_column.label(),
                    if self.panels.table_stats.sort_ascending {
                        "\u{2191}"
                    } else {
                        "\u{2193}"
                    }
                ));
            }
            KeyCode::Char('b') if self.replay.is_none() => {
                self.feedback.pending_action = Some(AppAction::RefreshBloat);
                self.feedback.status_message = Some("Refreshing bloat estimates...".to_string());
                self.feedback.bloat_loading = true;
            }
            _ => {}
        }
    }

    fn handle_replication_key(&mut self, key: KeyEvent) {
        let len = self.snapshot.as_ref().map_or(0, |s| s.replication.len());
        if PanelStates::simple_nav(&mut self.panels.replication, key.code, len) {
            if let Some(pid) = self.selected_replication_pid() {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Inspect(InspectTarget::Replication(pid));
            }
        }
    }

    fn handle_blocking_key(&mut self, key: KeyEvent) {
        let len = self.snapshot.as_ref().map_or(0, |s| s.blocking_info.len());
        if PanelStates::simple_nav(&mut self.panels.blocking, key.code, len) {
            if let Some(pid) = self.selected_blocking_pid() {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Inspect(InspectTarget::Blocking(pid));
            }
        }
    }

    fn handle_vacuum_key(&mut self, key: KeyEvent) {
        let len = self
            .snapshot
            .as_ref()
            .map_or(0, |s| s.vacuum_progress.len());
        if PanelStates::simple_nav(&mut self.panels.vacuum, key.code, len) {
            if let Some(pid) = self.selected_vacuum_pid() {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Inspect(InspectTarget::Vacuum(pid));
            }
        }
    }

    fn handle_wraparound_key(&mut self, key: KeyEvent) {
        let len = self.snapshot.as_ref().map_or(0, |s| s.wraparound.len());
        if PanelStates::simple_nav(&mut self.panels.wraparound, key.code, len) {
            if let Some(datname) = self.selected_wraparound_datname() {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Inspect(InspectTarget::Wraparound(datname));
            }
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        let len = self.sorted_settings_indices().len();
        if PanelStates::simple_nav(&mut self.panels.settings, key.code, len) {
            if let Some(name) = self.selected_setting_name() {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Inspect(InspectTarget::Settings(name));
            }
        }
    }

    fn handle_extensions_key(&mut self, key: KeyEvent) {
        let len = self.sorted_extensions_indices().len();
        if PanelStates::simple_nav(&mut self.panels.extensions, key.code, len) {
            if let Some(name) = self.selected_extension_name() {
                self.overlay_scroll = 0;
                self.view_mode = ViewMode::Inspect(InspectTarget::Extensions(name));
            }
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
            BottomPanel::Extensions => self.handle_extensions_key(key),
            BottomPanel::WalIo | BottomPanel::WaitEvents => {}
        }
    }

    // --- Modal overlay handlers ---

    /// Handle simple yes/no confirmation dialogs.
    /// On 'y'/'Y', executes the action. Any other key aborts with the given message.
    fn handle_yes_no_confirm(&mut self, key: KeyEvent, action: AppAction, abort_msg: &str) {
        if let KeyCode::Char('y' | 'Y') = key.code {
            self.feedback.pending_action = Some(action);
            self.view_mode = ViewMode::Normal;
        } else {
            self.view_mode = ViewMode::Normal;
            self.feedback.status_message = Some(abort_msg.into());
        }
    }

    /// Handle choice confirmation dialogs (single vs batch).
    /// '1'/'o' selects single, 'a' goes to batch confirm, Esc aborts.
    fn handle_choice_confirm(
        &mut self,
        key: KeyEvent,
        single_action: AppAction,
        batch_mode: ViewMode,
        abort_msg: &str,
    ) {
        match key.code {
            KeyCode::Char('1' | 'o') => {
                self.feedback.pending_action = Some(single_action);
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Char('a') => {
                self.view_mode = batch_mode;
            }
            KeyCode::Esc => {
                self.view_mode = ViewMode::Normal;
                self.feedback.status_message = Some(abort_msg.into());
            }
            _ => {}
        }
    }

    /// Handle overlay scroll keys, returns true if handled
    fn handle_overlay_scroll(&mut self, key: KeyEvent) -> bool {
        const PAGE_SIZE: u16 = 10;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.overlay_scroll = self.overlay_scroll.saturating_add(1);
                true
            }
            KeyCode::PageUp => {
                self.overlay_scroll = self.overlay_scroll.saturating_sub(PAGE_SIZE);
                true
            }
            KeyCode::PageDown => {
                self.overlay_scroll = self.overlay_scroll.saturating_add(PAGE_SIZE);
                true
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.overlay_scroll = self.overlay_scroll.saturating_sub(PAGE_SIZE);
                true
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.overlay_scroll = self.overlay_scroll.saturating_add(PAGE_SIZE);
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

    /// Get the text to copy for the current inspect overlay
    fn get_inspect_copy_text(&self) -> Option<String> {
        let ViewMode::Inspect(ref target) = self.view_mode else {
            return None;
        };
        match target {
            InspectTarget::Query(pid) => {
                let snap = self.snapshot.as_ref()?;
                let q = snap.active_queries.iter().find(|q| q.pid == *pid)?;
                q.query.clone()
            }
            InspectTarget::Index(key) => {
                let snap = self.snapshot.as_ref()?;
                let idx = snap.indexes.iter().find(|i| {
                    format!("{}.{}", i.schemaname, i.index_name) == *key
                })?;
                Some(idx.index_definition.clone())
            }
            InspectTarget::Statement(queryid) => {
                let snap = self.snapshot.as_ref()?;
                let stmt = snap.stat_statements.iter().find(|s| s.queryid == *queryid)?;
                Some(stmt.query.clone())
            }
            InspectTarget::Replication(pid) => {
                let snap = self.snapshot.as_ref()?;
                let r = snap.replication.iter().find(|r| r.pid == *pid)?;
                Some(r.application_name.clone().unwrap_or_default())
            }
            InspectTarget::Table(key) => {
                Some(key.clone())
            }
            InspectTarget::Blocking(blocked_pid) => {
                let snap = self.snapshot.as_ref()?;
                let info = snap.blocking_info.iter().find(|b| b.blocked_pid == *blocked_pid)?;
                Some(info.blocked_query.clone().unwrap_or_default())
            }
            InspectTarget::Vacuum(pid) => {
                let snap = self.snapshot.as_ref()?;
                let vac = snap.vacuum_progress.iter().find(|v| v.pid == *pid)?;
                Some(vac.table_name.clone())
            }
            InspectTarget::Wraparound(datname) => {
                Some(datname.clone())
            }
            InspectTarget::Settings(name) => {
                let s = self.server_info.settings.iter().find(|s| s.name == *name)?;
                Some(format!("{} = {}", s.name, s.setting))
            }
            InspectTarget::Extensions(name) => {
                Some(name.clone())
            }
        }
    }

    /// Unified handler for all inspect overlay key events.
    fn handle_inspect_overlay_key(&mut self, key: KeyEvent) {
        // Query inspect allows Enter to close (legacy behavior)
        let query_pid = match &self.view_mode {
            ViewMode::Inspect(InspectTarget::Query(pid)) => Some(*pid),
            _ => None,
        };

        let close = match key.code {
            KeyCode::Esc | KeyCode::Char('q') => true,
            KeyCode::Enter if query_pid.is_some() => true,
            _ => false,
        };

        if close {
            self.overlay_scroll = 0;
            self.view_mode = ViewMode::Normal;
            return;
        }

        if key.code == KeyCode::Char('y') {
            if let Some(text) = self.get_inspect_copy_text() {
                self.copy_to_clipboard(&text);
            }
            return;
        }

        // Kill/Cancel only available for query inspect in live mode
        if let Some(pid) = query_pid {
            if self.replay.is_none() {
                match key.code {
                    KeyCode::Char('K') => {
                        self.view_mode = ViewMode::Confirm(ConfirmAction::Kill(pid));
                        return;
                    }
                    KeyCode::Char('C') => {
                        self.view_mode = ViewMode::Confirm(ConfirmAction::Cancel(pid));
                        return;
                    }
                    _ => {}
                }
            }
        }

        self.handle_overlay_scroll(key);
    }

    fn handle_config_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.feedback.pending_action = Some(AppAction::SaveConfig);
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.config_overlay.selected > 0 {
                    self.config_overlay.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.config_overlay.selected < ConfigItem::ALL.len() - 1 {
                    self.config_overlay.selected += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.config_adjust(-1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.config_adjust(1);
            }
            KeyCode::Enter => {
                // Enter edit mode for RecordingsDir
                if ConfigItem::ALL[self.config_overlay.selected] == ConfigItem::RecordingsDir {
                    self.config_overlay.input_buffer =
                        self.config.recordings_dir.clone().unwrap_or_default();
                    self.view_mode = ViewMode::ConfigEditRecordingsDir;
                }
            }
            _ => {}
        }
    }

    fn handle_config_edit_recordings_dir_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                // Cancel editing
                self.config_overlay.input_buffer.clear();
                self.view_mode = ViewMode::Config;
            }
            KeyCode::Enter => {
                // Save the input
                let input = self.config_overlay.input_buffer.trim();
                if input.is_empty() {
                    self.config.recordings_dir = None;
                } else {
                    self.config.recordings_dir = Some(input.to_string());
                }
                self.config_overlay.input_buffer.clear();
                self.view_mode = ViewMode::Config;
            }
            KeyCode::Backspace => {
                self.config_overlay.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.config_overlay.input_buffer.push(c);
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

    fn handle_recordings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.view_mode = ViewMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.recordings.selected > 0 {
                    self.recordings.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.recordings.list.is_empty()
                    && self.recordings.selected < self.recordings.list.len() - 1
                {
                    self.recordings.selected += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(recording) = self.recordings.current() {
                    self.recordings.pending_path = Some(recording.path.clone());
                    self.running = false;
                }
            }
            KeyCode::Char('d') => {
                if let Some(recording) = self.recordings.current() {
                    self.view_mode =
                        ViewMode::Confirm(ConfirmAction::DeleteRecording(recording.path.clone()));
                }
            }
            _ => {}
        }
    }

    fn handle_confirm_delete_recording_key(&mut self, key: KeyEvent, path: PathBuf) {
        if let KeyCode::Char('y' | 'Y') = key.code {
            if crate::recorder::Recorder::delete_recording(&path).is_ok() {
                self.feedback.status_message = Some("Recording deleted".into());
                // Refresh the list
                self.recordings.list =
                    crate::recorder::Recorder::list_recordings(self.config.recordings_dir.as_deref());
                // Adjust selection if needed
                if self.recordings.selected >= self.recordings.list.len()
                    && !self.recordings.list.is_empty()
                {
                    self.recordings.selected = self.recordings.list.len() - 1;
                }
            } else {
                self.feedback.status_message = Some("Failed to delete recording".into());
            }
            self.view_mode = ViewMode::Recordings;
        } else {
            self.view_mode = ViewMode::Recordings;
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.filter.clear();
                self.view_mode = ViewMode::Normal;
                self.reset_panel_selection();
            }
            KeyCode::Enter => {
                self.filter.active = !self.filter.text.is_empty();
                self.view_mode = ViewMode::Normal;
                self.reset_panel_selection();
            }
            KeyCode::Backspace => {
                self.filter.pop_char();
                self.reset_panel_selection();
            }
            KeyCode::Char(c) => {
                self.filter.push_char(c);
                self.reset_panel_selection();
            }
            _ => {}
        }
    }

    fn handle_normal_global_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
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
            KeyCode::Char('p') if self.replay.is_none() => {
                self.paused = !self.paused;
                true
            }
            KeyCode::Char('r') if self.replay.is_none() => {
                self.feedback.pending_action = Some(AppAction::ForceRefresh);
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
            KeyCode::Char('L') if self.replay.is_none() => {
                // Open recordings browser (live mode only)
                self.recordings.list =
                    crate::recorder::Recorder::list_recordings(self.config.recordings_dir.as_deref());
                self.recordings.selected = 0;
                self.view_mode = ViewMode::Recordings;
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
            KeyCode::Char('E') => {
                self.switch_panel(BottomPanel::Extensions);
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
            ViewMode::Confirm(action) => {
                match action {
                    ConfirmAction::Cancel(pid) => {
                        let action = AppAction::CancelQuery(*pid);
                        self.handle_yes_no_confirm(key, action, "Cancel aborted");
                    }
                    ConfirmAction::Kill(pid) => {
                        let action = AppAction::TerminateBackend(*pid);
                        self.handle_yes_no_confirm(key, action, "Kill aborted");
                    }
                    ConfirmAction::CancelChoice {
                        selected_pid,
                        all_pids,
                    } => {
                        let action = AppAction::CancelQuery(*selected_pid);
                        let batch = ViewMode::Confirm(ConfirmAction::CancelBatch(all_pids.clone()));
                        self.handle_choice_confirm(key, action, batch, "Cancel aborted");
                    }
                    ConfirmAction::KillChoice {
                        selected_pid,
                        all_pids,
                    } => {
                        let action = AppAction::TerminateBackend(*selected_pid);
                        let batch = ViewMode::Confirm(ConfirmAction::KillBatch(all_pids.clone()));
                        self.handle_choice_confirm(key, action, batch, "Kill aborted");
                    }
                    ConfirmAction::CancelBatch(pids) => {
                        let action = AppAction::CancelQueries(pids.clone());
                        self.handle_yes_no_confirm(key, action, "Batch cancel aborted");
                    }
                    ConfirmAction::KillBatch(pids) => {
                        let action = AppAction::TerminateBackends(pids.clone());
                        self.handle_yes_no_confirm(key, action, "Batch kill aborted");
                    }
                    ConfirmAction::DeleteRecording(ref path) => {
                        let path = path.clone();
                        self.handle_confirm_delete_recording_key(key, path);
                    }
                }
                return;
            }
            ViewMode::Inspect(_) => {
                self.handle_inspect_overlay_key(key);
                return;
            }
            ViewMode::Config => {
                self.handle_config_key(key);
                return;
            }
            ViewMode::ConfigEditRecordingsDir => {
                self.handle_config_edit_recordings_dir_key(key);
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
            ViewMode::Recordings => {
                self.handle_recordings_key(key);
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
        let item = ConfigItem::ALL[self.config_overlay.selected];
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
            ConfigItem::ShowEmojis => {
                self.config.show_emojis = !self.config.show_emojis;
            }
            ConfigItem::RefreshInterval => {
                let val = self.config.refresh_interval_secs as i64 + i64::from(direction);
                self.config.refresh_interval_secs = val.clamp(1, 60) as u64;
                self.refresh_interval_secs = self.config.refresh_interval_secs;
                self.feedback.pending_action = Some(AppAction::RefreshIntervalChanged);
            }
            ConfigItem::WarnDuration => {
                let val = f64::from(direction).mul_add(0.5, self.config.warn_duration_secs);
                self.config.warn_duration_secs = val.clamp(0.1, self.config.danger_duration_secs);
                theme::set_duration_thresholds(
                    self.config.warn_duration_secs,
                    self.config.danger_duration_secs,
                );
            }
            ConfigItem::DangerDuration => {
                let val = f64::from(direction).mul_add(1.0, self.config.danger_duration_secs);
                self.config.danger_duration_secs =
                    val.clamp(self.config.warn_duration_secs, 300.0);
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
                let val =
                    self.config.recording_retention_secs as i64 + i64::from(direction) * step;
                self.config.recording_retention_secs = val.clamp(600, 86400) as u64;
            }
            ConfigItem::RecordingsDir => {
                // Path cannot be adjusted with arrows - edit config.toml to change
            }
        }
    }
}

#[cfg(test)]
mod tests;
