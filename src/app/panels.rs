//! Panel and view mode enums.

use std::path::PathBuf;

/// The active bottom panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Extensions,
}

impl BottomPanel {
    pub const fn supports_filter(self) -> bool {
        matches!(
            self,
            Self::Queries | Self::Indexes | Self::Statements | Self::TableStats | Self::Settings | Self::Extensions
        )
    }

    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
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
            Self::Extensions => "Extensions",
        }
    }
}

/// Target for inspect overlays
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectTarget {
    Query,
    Index,
    Statement,
    Replication,
    Table,
    Blocking,
    Vacuum,
    Wraparound,
    Settings,
    Extensions,
}

/// Confirmation action types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    Cancel(i32),
    Kill(i32),
    CancelChoice { selected_pid: i32, all_pids: Vec<i32> },
    KillChoice { selected_pid: i32, all_pids: Vec<i32> },
    CancelBatch(Vec<i32>),
    KillBatch(Vec<i32>),
    DeleteRecording(PathBuf),
}

/// Current view/interaction mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    Normal,
    Filter,
    Inspect(InspectTarget),
    Confirm(ConfirmAction),
    Config,
    ConfigEditRecordingsDir,
    Help,
    Recordings,
}
