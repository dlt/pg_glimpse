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

/// Current view/interaction mode.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    SettingsInspect,
    ExtensionsInspect,
    ConfirmCancel(i32),
    ConfirmKill(i32),
    ConfirmCancelChoice { selected_pid: i32, all_pids: Vec<i32> },
    ConfirmKillChoice { selected_pid: i32, all_pids: Vec<i32> },
    ConfirmCancelBatch(Vec<i32>),
    ConfirmKillBatch(Vec<i32>),
    Config,
    ConfigEditRecordingsDir,
    Help,
    Recordings,
    ConfirmDeleteRecording(PathBuf),
}
