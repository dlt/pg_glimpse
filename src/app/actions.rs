//! Application actions (side effects requested by App).

/// Actions that require the runtime to perform side effects.
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
    ResetStatStatements,
}
