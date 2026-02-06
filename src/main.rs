mod app;
mod cli;
mod config;
mod db;
mod event;
mod history;
mod recorder;
mod replay;
mod ui;

use app::AppAction;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use config::AppConfig;
use crossterm::event::KeyCode;
use db::models::PgSnapshot;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use ui::theme;

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(run(cli))
}

async fn run(cli: Cli) -> Result<()> {
    if let Some(ref replay_path) = cli.replay {
        let config = AppConfig::load();
        theme::set_theme(config.color_theme.colors());
        theme::set_duration_thresholds(config.warn_duration_secs, config.danger_duration_secs);
        return run_replay(replay_path, config).await;
    }

    let pg_config = match cli.pg_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: invalid connection config: {e}\n");
            eprintln!("Try: pg_glimpse -H localhost -p 5432 -d mydb -U postgres -W mypassword");
            eprintln!("See: pg_glimpse --help");
            std::process::exit(1);
        }
    };
    let (client, connection) = match pg_config.connect(tokio_postgres::NoTls).await {
        Ok(c) => c,
        Err(e) => {
            let info = cli.connection_info();
            eprintln!("Error: could not connect to PostgreSQL: {e}\n");
            eprintln!(
                "Connection: {}:{}/{}",
                info.host, info.port, info.dbname
            );
            eprintln!("\nTry: pg_glimpse -H localhost -p 5432 -d mydb -U postgres -W mypassword");
            eprintln!("See: pg_glimpse --help");
            std::process::exit(1);
        }
    };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("PostgreSQL connection error: {}", e);
        }
    });

    let config = AppConfig::load();

    // Apply theme and thresholds from config
    theme::set_theme(config.color_theme.colors());
    theme::set_duration_thresholds(config.warn_duration_secs, config.danger_duration_secs);

    let refresh = cli.refresh.unwrap_or(config.refresh_interval_secs);

    // Clean up old recordings on startup
    recorder::Recorder::cleanup_old(config.recording_retention_secs);

    // Fetch server info and extensions at startup
    let server_info = db::queries::fetch_server_info(&client).await?;

    // Get connection info for display
    let conn_info = cli.connection_info();

    // Start recorder
    let mut recorder =
        recorder::Recorder::new(&conn_info.host, conn_info.port, &conn_info.dbname, &conn_info.user, &server_info).ok();

    let mut app = app::App::new(
        conn_info.host,
        conn_info.port,
        conn_info.dbname,
        conn_info.user,
        refresh,
        cli.history_length,
        config,
        server_info,
    );

    let extensions = app.server_info.extensions.clone();
    let pg_major_version = app.server_info.major_version();

    // Channel for DB commands and results
    enum DbCommand {
        FetchSnapshot,
        CancelQuery(i32),
        TerminateBackend(i32),
    }
    #[allow(clippy::large_enum_variant)]
    enum DbResult {
        Snapshot(Result<PgSnapshot, String>),
        CancelQuery(i32, Result<bool, String>),
        TerminateBackend(i32, Result<bool, String>),
    }

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<DbCommand>();
    let (result_tx, mut result_rx) = mpsc::unbounded_channel::<DbResult>();
    let client = Arc::new(client);
    let db_client = Arc::clone(&client);

    // Background task for DB operations
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let result = match cmd {
                DbCommand::FetchSnapshot => {
                    DbResult::Snapshot(
                        db::queries::fetch_snapshot(&db_client, &extensions, pg_major_version)
                            .await
                            .map_err(|e| e.to_string()),
                    )
                }
                DbCommand::CancelQuery(pid) => {
                    DbResult::CancelQuery(
                        pid,
                        db::queries::cancel_backend(&db_client, pid)
                            .await
                            .map_err(|e| e.to_string()),
                    )
                }
                DbCommand::TerminateBackend(pid) => {
                    DbResult::TerminateBackend(
                        pid,
                        db::queries::terminate_backend(&db_client, pid)
                            .await
                            .map_err(|e| e.to_string()),
                    )
                }
            };
            if result_tx.send(result).is_err() {
                break;
            }
        }
    });

    // Initial fetch
    let _ = cmd_tx.send(DbCommand::FetchSnapshot);

    let mut terminal = ratatui::init();
    let mut events = event::EventHandler::new(Duration::from_millis(10));
    let mut tick_interval = tokio::time::interval(Duration::from_secs(refresh));
    let mut refresh_interval_secs = refresh;

    while app.running {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        tokio::select! {
            biased;

            event = events.next() => {
                if let Some(evt) = event {
                    match evt {
                        event::AppEvent::Key(key) => {
                            app.handle_key(key);
                        }
                        event::AppEvent::Resize(_, _) => {}
                    }
                }
            }
            result = result_rx.recv() => {
                if let Some(res) = result {
                    match res {
                        DbResult::Snapshot(Ok(snap)) => {
                            if let Some(ref mut rec) = recorder {
                                let _ = rec.record(&snap);
                            }
                            app.update(snap);
                        }
                        DbResult::Snapshot(Err(e)) => {
                            app.update_error(e);
                        }
                        DbResult::CancelQuery(pid, Ok(true)) => {
                            app.status_message = Some(format!("Cancelled query on PID {}", pid));
                            let _ = cmd_tx.send(DbCommand::FetchSnapshot);
                        }
                        DbResult::CancelQuery(pid, Ok(false)) => {
                            app.status_message = Some(format!("PID {} not found or already finished", pid));
                        }
                        DbResult::CancelQuery(_, Err(e)) => {
                            app.status_message = Some(format!("Cancel failed: {}", e));
                        }
                        DbResult::TerminateBackend(pid, Ok(true)) => {
                            app.status_message = Some(format!("Terminated backend PID {}", pid));
                            let _ = cmd_tx.send(DbCommand::FetchSnapshot);
                        }
                        DbResult::TerminateBackend(pid, Ok(false)) => {
                            app.status_message = Some(format!("PID {} not found or already finished", pid));
                        }
                        DbResult::TerminateBackend(_, Err(e)) => {
                            app.status_message = Some(format!("Terminate failed: {}", e));
                        }
                    }
                }
            }
            _ = tick_interval.tick() => {
                if !app.paused {
                    let _ = cmd_tx.send(DbCommand::FetchSnapshot);
                }
            }
        }

        // Process pending actions
        if let Some(action) = app.pending_action.take() {
            match action {
                AppAction::ForceRefresh => {
                    let _ = cmd_tx.send(DbCommand::FetchSnapshot);
                }
                AppAction::CancelQuery(pid) => {
                    let _ = cmd_tx.send(DbCommand::CancelQuery(pid));
                }
                AppAction::TerminateBackend(pid) => {
                    let _ = cmd_tx.send(DbCommand::TerminateBackend(pid));
                }
                AppAction::SaveConfig => {
                    app.config.save();
                }
                AppAction::RefreshIntervalChanged => {
                    if app.config.refresh_interval_secs != refresh_interval_secs {
                        refresh_interval_secs = app.config.refresh_interval_secs;
                        tick_interval = tokio::time::interval(Duration::from_secs(refresh_interval_secs));
                    }
                }
            }
        }
    }

    ratatui::restore();
    Ok(())
}

async fn run_replay(path: &Path, config: AppConfig) -> Result<()> {
    let mut session = replay::ReplaySession::load(path)?;

    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut app = app::App::new_replay(
        session.host.clone(),
        session.port,
        session.dbname.clone(),
        session.user.clone(),
        120,
        config,
        session.server_info.clone(),
        filename,
        session.len(),
    );

    // Feed first snapshot
    if let Some(snap) = session.current() {
        app.update(snap.clone());
        app.replay_position = 1;
    }

    let mut terminal = ratatui::init();
    let mut events = event::EventHandler::new(Duration::from_millis(10));

    let mut last_advance = Instant::now();

    while app.running {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        // Auto-advance when playing
        if app.replay_playing && !session.at_end() {
            let interval = compute_replay_interval(&session, app.replay_speed);
            if last_advance.elapsed() >= interval {
                if session.step_forward() {
                    if let Some(snap) = session.current() {
                        app.update(snap.clone());
                        app.replay_position = session.position + 1;
                    }
                }
                last_advance = Instant::now();
                if session.at_end() {
                    app.replay_playing = false;
                }
            }
        }

        // Handle events with a short timeout so auto-advance works
        tokio::select! {
            biased;

            event = events.next() => {
                if let Some(evt) = event {
                    match evt {
                        event::AppEvent::Key(key) => {
                            // Replay-specific keys first
                            let handled = handle_replay_key(&mut app, &mut session, key.code, &mut last_advance);
                            if !handled {
                                app.handle_key(key);
                            }
                        }
                        event::AppEvent::Resize(_, _) => {}
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(10)) => {}
        }

        // Process pending actions (only SaveConfig matters in replay)
        if let Some(AppAction::SaveConfig) = app.pending_action.take() {
            app.config.save();
        }
    }

    ratatui::restore();
    Ok(())
}

fn handle_replay_key(
    app: &mut app::App,
    session: &mut replay::ReplaySession,
    code: KeyCode,
    last_advance: &mut Instant,
) -> bool {
    match code {
        KeyCode::Char(' ') => {
            app.replay_playing = !app.replay_playing;
            *last_advance = Instant::now();
            true
        }
        KeyCode::Right | KeyCode::Char('l')
            if app.view_mode == app::ViewMode::Normal =>
        {
            if session.step_forward() {
                if let Some(snap) = session.current() {
                    app.update(snap.clone());
                    app.replay_position = session.position + 1;
                }
            }
            true
        }
        KeyCode::Left | KeyCode::Char('h')
            if app.view_mode == app::ViewMode::Normal =>
        {
            if session.step_back() {
                if let Some(snap) = session.current() {
                    app.update(snap.clone());
                    app.replay_position = session.position + 1;
                }
            }
            true
        }
        KeyCode::Char('>') => {
            app.replay_speed = next_speed(app.replay_speed);
            true
        }
        KeyCode::Char('<') => {
            app.replay_speed = prev_speed(app.replay_speed);
            true
        }
        KeyCode::Char('g') if app.view_mode == app::ViewMode::Normal => {
            session.jump_start();
            if let Some(snap) = session.current() {
                app.update(snap.clone());
                app.replay_position = session.position + 1;
            }
            true
        }
        KeyCode::Char('G') if app.view_mode == app::ViewMode::Normal => {
            session.jump_end();
            if let Some(snap) = session.current() {
                app.update(snap.clone());
                app.replay_position = session.position + 1;
            }
            app.replay_playing = false;
            true
        }
        _ => false,
    }
}

fn compute_replay_interval(session: &replay::ReplaySession, speed: f64) -> Duration {
    // Try to use timestamps from adjacent snapshots
    let pos = session.position;
    if pos + 1 < session.len() {
        let current_ts = session.snapshots[pos].timestamp;
        let next_ts = session.snapshots[pos + 1].timestamp;
        let diff = (next_ts - current_ts).num_milliseconds().unsigned_abs();
        if diff > 0 {
            let adjusted = (diff as f64 / speed) as u64;
            return Duration::from_millis(adjusted.max(50));
        }
    }
    // Fallback: 2 seconds / speed
    let ms = (2000.0 / speed) as u64;
    Duration::from_millis(ms.max(50))
}

const SPEEDS: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];

fn next_speed(current: f64) -> f64 {
    for &s in &SPEEDS {
        if s > current + 0.01 {
            return s;
        }
    }
    *SPEEDS.last().unwrap()
}

fn prev_speed(current: f64) -> f64 {
    for &s in SPEEDS.iter().rev() {
        if s < current - 0.01 {
            return s;
        }
    }
    SPEEDS[0]
}
