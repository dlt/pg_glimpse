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
use std::path::Path;
use std::time::{Duration, Instant};
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

    let pg_config = cli.pg_config()?;
    let (client, connection) = pg_config.connect(tokio_postgres::NoTls).await?;

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

    // Start recorder
    let mut recorder =
        recorder::Recorder::new(&cli.host, cli.port, &cli.dbname, &cli.user, &server_info).ok();

    let mut app = app::App::new(
        cli.host.clone(),
        cli.port,
        cli.dbname.clone(),
        cli.user.clone(),
        refresh,
        cli.history_length,
        config,
        server_info,
    );

    let mut terminal = ratatui::init();
    let mut events = event::EventHandler::new(Duration::from_millis(50));
    let mut tick_interval = tokio::time::interval(Duration::from_secs(refresh));

    let extensions = app.server_info.extensions;

    // Initial fetch
    match db::queries::fetch_snapshot(&client, &extensions).await {
        Ok(snap) => {
            if let Some(ref mut rec) = recorder {
                let _ = rec.record(&snap);
            }
            app.update(snap);
        }
        Err(e) => app.update_error(e.to_string()),
    }

    while app.running {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        tokio::select! {
            _ = tick_interval.tick() => {
                if !app.paused {
                    match db::queries::fetch_snapshot(&client, &extensions).await {
                        Ok(snap) => {
                            if let Some(ref mut rec) = recorder {
                                let _ = rec.record(&snap);
                            }
                            app.update(snap);
                        }
                        Err(e) => app.update_error(e.to_string()),
                    }
                }
            }
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
        }

        // Process pending actions
        if let Some(action) = app.pending_action.take() {
            match action {
                AppAction::ForceRefresh => {
                    match db::queries::fetch_snapshot(&client, &extensions).await {
                        Ok(snap) => {
                            if let Some(ref mut rec) = recorder {
                                let _ = rec.record(&snap);
                            }
                            app.update(snap);
                        }
                        Err(e) => app.update_error(e.to_string()),
                    }
                }
                AppAction::CancelQuery(pid) => {
                    match db::queries::cancel_backend(&client, pid).await {
                        Ok(true) => {
                            app.status_message =
                                Some(format!("Cancelled query on PID {}", pid));
                        }
                        Ok(false) => {
                            app.status_message =
                                Some(format!("PID {} not found or already finished", pid));
                        }
                        Err(e) => {
                            app.status_message =
                                Some(format!("Cancel failed: {}", e));
                        }
                    }
                    // Refresh after action
                    if let Ok(snap) = db::queries::fetch_snapshot(&client, &extensions).await {
                        if let Some(ref mut rec) = recorder {
                            let _ = rec.record(&snap);
                        }
                        app.update(snap);
                    }
                }
                AppAction::TerminateBackend(pid) => {
                    match db::queries::terminate_backend(&client, pid).await {
                        Ok(true) => {
                            app.status_message =
                                Some(format!("Terminated backend PID {}", pid));
                        }
                        Ok(false) => {
                            app.status_message =
                                Some(format!("PID {} not found or already finished", pid));
                        }
                        Err(e) => {
                            app.status_message =
                                Some(format!("Terminate failed: {}", e));
                        }
                    }
                    // Refresh after action
                    if let Ok(snap) = db::queries::fetch_snapshot(&client, &extensions).await {
                        if let Some(ref mut rec) = recorder {
                            let _ = rec.record(&snap);
                        }
                        app.update(snap);
                    }
                }
                AppAction::SaveConfig => {
                    app.config.save();
                }
                AppAction::RefreshIntervalChanged => {
                    tick_interval = tokio::time::interval(Duration::from_secs(
                        app.config.refresh_interval_secs,
                    ));
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
    let mut events = event::EventHandler::new(Duration::from_millis(50));

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
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
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
