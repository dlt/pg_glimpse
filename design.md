# pg_glimpse Architecture

## Overview

pg_glimpse is a terminal-based PostgreSQL monitoring tool built with Rust. It follows a **unidirectional data flow** architecture similar to The Elm Architecture (TEA) / Redux pattern, adapted for a TUI application.

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Event Loop (runtime.rs)                    │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐      │
│  │  Events  │───▶│   App    │───▶│  Render  │───▶│ Terminal │      │
│  │ (input)  │    │ (state)  │    │   (ui)   │    │ (output) │      │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘      │
│       ▲               │                                              │
│       │               ▼                                              │
│  ┌──────────┐    ┌──────────┐                                       │
│  │  Timer   │    │ Actions  │──▶ DB Commands (async)                │
│  │  Ticks   │    │ (effects)│                                       │
│  └──────────┘    └──────────┘                                       │
└─────────────────────────────────────────────────────────────────────┘
```

## Core Pattern: Unidirectional Data Flow

1. **Events** arrive (keyboard input, timer ticks, DB results)
2. **App.handle_key()** processes input and updates state
3. **App.update()** receives new snapshots and updates metrics
4. **ui::render()** reads App state and draws the UI
5. **AppAction** enum carries side effects back to the runtime

This pattern ensures:
- UI is always a pure function of state
- State mutations are centralized in `App`
- Side effects (DB calls) are explicit via `AppAction`

## Module Structure

```
src/
├── lib.rs              # Module exports, run_cli() entry point
├── main.rs             # Binary entry point (calls run_cli)
├── connection.rs       # SSL/TLS connection handling
├── runtime.rs          # Live mode event loop (tokio select!)
├── replay.rs           # Replay session loading + replay runtime
├── app.rs              # App state + key handling (the "Model")
├── cli.rs              # CLI argument parsing (clap)
├── config.rs           # User configuration (persisted to disk)
├── event.rs            # Keyboard/terminal event handling
├── history.rs          # RingBuffer for sparkline data
├── recorder.rs         # JSONL recording writer
├── db/
│   ├── mod.rs
│   ├── models.rs       # Data structs (PgSnapshot, ServerInfo, etc.)
│   ├── queries.rs      # SQL queries with version-aware variants
│   └── error.rs        # Database error types
└── ui/
    ├── mod.rs          # Main render() dispatcher
    ├── layout.rs       # Screen area computation
    ├── header.rs       # Top bar (connection info, status)
    ├── footer.rs       # Bottom bar (keybindings, mode)
    ├── panels.rs       # Bottom panel renderers (tables, replication, etc.)
    ├── active_queries.rs # Queries panel (special handling)
    ├── overlay.rs      # Modal popups (help, config, inspect)
    ├── stats_panel.rs  # Server stats sidebar with sparklines
    ├── graph.rs        # Line/ratio chart widgets
    ├── sparkline.rs    # Sparkline widget
    ├── theme.rs        # Color themes
    └── util.rs         # Formatting helpers
```

## Key Components

### App (app.rs) - The Model

The `App` struct holds all application state:

```rust
pub struct App {
    // Core state
    pub running: bool,
    pub paused: bool,
    pub snapshot: Option<PgSnapshot>,      // Latest DB snapshot
    pub view_mode: ViewMode,               // Current UI mode
    pub bottom_panel: BottomPanel,         // Active panel

    // Table view states (selection, sort)
    pub queries: TableViewState<SortColumn>,
    pub indexes: TableViewState<IndexSortColumn>,
    // ...

    // Metrics history for sparklines
    pub metrics: MetricsHistory,

    // Side effect output
    pub pending_action: Option<AppAction>,
    // ...
}
```

**Key handling is layered:**
1. Modal overlays (consume all input when active)
2. Global keys (q, Ctrl+C, ?, etc.)
3. Panel switch keys (Tab, I, S, etc.)
4. Panel-specific keys (j/k navigation, s for sort, etc.)

### Runtime (runtime.rs) - The Event Loop

The runtime orchestrates the main loop using `tokio::select!`:

```rust
loop {
    terminal.draw(|f| ui::render(f, &mut app))?;

    tokio::select! {
        event = events.next() => { /* handle input */ }
        result = result_rx.recv() => { /* handle DB results */ }
        _ = tick_interval.tick() => { /* periodic refresh */ }
    }

    // Process pending actions (side effects)
    if let Some(action) = app.pending_action.take() {
        match action {
            AppAction::CancelQuery(pid) => { /* send to DB task */ }
            // ...
        }
    }
}
```

**Communication with DB:**
- `DbCommand` enum sent via channel to background task
- `DbResult` enum received back with query results
- Decouples UI thread from potentially slow DB operations

### UI Layer (ui/)

The UI is **stateless** - it reads from `App` and produces widgets:

```rust
pub fn render(frame: &mut Frame, app: &mut App) {
    let areas = layout::compute_layout(frame.area());

    header::render(frame, app, areas.header);

    // Dispatch to active panel
    match app.bottom_panel {
        BottomPanel::Queries => active_queries::render(frame, app, areas.queries),
        BottomPanel::Indexes => panels::render_indexes(frame, app, areas.queries),
        // ...
    }

    footer::render(frame, app, areas.footer);

    // Overlays on top
    match &app.view_mode {
        ViewMode::Help => overlay::render_help(frame, app, frame.area()),
        ViewMode::Inspect => overlay::render_inspect(frame, app, frame.area()),
        // ...
    }
}
```

### Database Layer (db/)

- **models.rs**: Data structs that mirror PostgreSQL system views
- **queries.rs**: SQL with version-aware variants (PG11 vs PG17 differences)
- All models derive `Serialize, Deserialize` for recording/replay

### Recording/Replay

Sessions are recorded as JSONL files:
```
{"type":"header","host":"localhost","port":5432,...}
{"type":"snapshot","data":{...}}
{"type":"snapshot","data":{...}}
```

Replay mode reuses the same `App` and UI, just with a different runtime loop that reads from the file instead of the database.

## Data Flow Example: Cancel Query

1. User presses `C` on a query
2. `handle_queries_key()` sets `view_mode = ConfirmCancel(pid)`
3. Confirmation overlay renders
4. User presses `y`
5. `handle_yes_no_confirm()` sets `pending_action = Some(CancelQuery(pid))`
6. Runtime processes action, sends `DbCommand::CancelQuery(pid)`
7. Background task executes `pg_cancel_backend()`
8. `DbResult::CancelQuery` arrives, runtime updates `app.status_message`
9. Next render shows the status message

## Adding a New Feature

### Adding a New Panel

1. **Add enum variant** in `app.rs`:
   ```rust
   pub enum BottomPanel {
       // ...
       NewPanel,
   }
   ```

2. **Add to label()** in `BottomPanel`:
   ```rust
   Self::NewPanel => "New Panel",
   ```

3. **Add panel switch key** in `handle_panel_switch_key()`:
   ```rust
   KeyCode::Char('N') => {
       self.switch_panel(BottomPanel::NewPanel);
       true
   }
   ```

4. **Create render function** in `ui/panels.rs`:
   ```rust
   pub fn render_new_panel(frame: &mut Frame, app: &App, area: Rect) {
       // ...
   }
   ```

5. **Add to render dispatch** in `ui/mod.rs`:
   ```rust
   BottomPanel::NewPanel => panels::render_new_panel(frame, app, areas.queries),
   ```

6. **Add key handler** if panel needs special keys:
   ```rust
   fn handle_new_panel_key(&mut self, key: KeyEvent) { ... }
   ```

7. **Update handle_panel_key()** dispatch

### Adding a Config Option

1. **Add to ConfigItem enum** in `config.rs`:
   ```rust
   pub enum ConfigItem {
       // ...
       NewOption,
   }
   ```

2. **Add to ConfigItem::ALL array**

3. **Add label()** match arm

4. **Add to config_adjust()** in `app.rs`

5. **Add to overlay render** in `ui/overlay.rs`

6. **Add field to AppConfig** with serde default

### Adding a Database Metric

1. **Add field to model** in `db/models.rs`
2. **Add to SQL query** in `db/queries.rs`
3. **Add to MetricsHistory** if sparkline needed
4. **Update UI** to display the new metric

## Testing Strategy

- **Unit tests**: Individual functions, sorting, filtering logic
- **Snapshot tests**: UI rendering via `insta` crate (`cargo insta review`)
- **Integration tests**: Real PostgreSQL via Docker (PG11, PG14, PG17)
- **Fuzz tests**: JSONL parsing robustness via `proptest`

Run tests:
```bash
cargo test                                    # Unit tests
cargo insta review                            # Review UI snapshots
docker compose -f tests/docker-compose.yml up -d
cargo test --features integration --test integration
```

## Design Decisions

**Why not Elm/Redux exactly?**
- Rust ownership makes pure message passing verbose
- `App.handle_key()` mutates directly for simplicity
- Side effects via `pending_action` instead of Cmd type

**Why tokio::select! instead of threads?**
- Single UI thread avoids synchronization complexity
- Async DB operations don't block rendering
- Timer-based refresh integrates naturally

**Why JSONL for recordings?**
- Streamable (can write incrementally)
- Human-readable for debugging
- Easy to parse with serde

**Why separate runtime.rs and replay.rs?**
- Different event sources (DB vs file)
- Replay needs speed control, live needs pause/refresh
- Shared App/UI code via composition
