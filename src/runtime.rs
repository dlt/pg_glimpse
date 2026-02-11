//! Main application runtime - live mode event loop.

use crate::app::AppAction;
use crate::cli::Cli;
use crate::config::AppConfig;
use crate::connection::{try_connect, SslMode};
use crate::db::models::PgSnapshot;
use crate::replay::run_replay;
use crate::ui::theme;
use crate::{app, db, event, recorder, ui};
use color_eyre::eyre::{bail, Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Run the main application in live mode.
pub async fn run(cli: Cli) -> Result<()> {
    if let Some(ref replay_path) = cli.replay {
        let config = AppConfig::load();
        theme::set_theme(config.color_theme.colors());
        theme::set_duration_thresholds(config.warn_duration_secs, config.danger_duration_secs);
        return run_replay(replay_path, config).await;
    }

    let pg_config = cli
        .pg_config()
        .context("invalid connection config\n\nTry: pg_glimpse -H localhost -p 5432 -d mydb -U postgres -W mypassword\nSee: pg_glimpse --help")?;

    // Determine connection mode: explicit flags or auto-detect
    let (client, ssl_mode) = if cli.ssl || cli.ssl_insecure {
        // User explicitly specified SSL mode - use it directly
        let mode = if cli.ssl_insecure {
            SslMode::Insecure
        } else {
            SslMode::Verified
        };
        let info = cli.connection_info();
        let client = try_connect(&pg_config, mode).await.with_context(|| {
            format!(
                "could not connect to PostgreSQL ({})\n\nConnection: {}:{}/{}\n\nTry: pg_glimpse -H localhost -p 5432 -d mydb -U postgres -W mypassword\nSee: pg_glimpse --help",
                mode.label(),
                info.host,
                info.port,
                info.dbname
            )
        })?;
        (client, mode)
    } else {
        // Auto-detect: try connection modes in order
        let modes = [SslMode::None, SslMode::Verified, SslMode::Insecure];
        let mut last_error = None;
        let mut result = None;

        for mode in modes {
            match try_connect(&pg_config, mode).await {
                Ok(client) => {
                    result = Some((client, mode));
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        match result {
            Some(r) => r,
            None => {
                let info = cli.connection_info();
                bail!(
                    "could not connect to PostgreSQL with any SSL mode: {:?}\n\nConnection: {}:{}/{}\n\nTried: No TLS, SSL (verified), SSL (insecure)\nTry: pg_glimpse -H localhost -p 5432 -d mydb -U postgres -W mypassword\nSee: pg_glimpse --help",
                    last_error.unwrap(),
                    info.host,
                    info.port,
                    info.dbname
                );
            }
        }
    };

    let config = AppConfig::load();

    // Apply theme and thresholds from config
    theme::set_theme(config.color_theme.colors());
    theme::set_duration_thresholds(config.warn_duration_secs, config.danger_duration_secs);

    let refresh = cli.refresh.unwrap_or(config.refresh_interval_secs);

    // Clean up old recordings on startup
    recorder::Recorder::cleanup_old(config.recording_retention_secs, config.recordings_dir.as_deref());

    // Fetch server info and extensions at startup
    let server_info = db::queries::fetch_server_info(&client).await?;

    // Get connection info for display
    let conn_info = cli.connection_info();

    // Start recorder
    let mut recorder =
        recorder::Recorder::new(&conn_info.host, conn_info.port, &conn_info.dbname, &conn_info.user, &server_info, config.recordings_dir.as_deref()).ok();

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
    app.set_ssl_mode_label(ssl_mode.label());

    let extensions = app.server_info.extensions.clone();
    let pg_major_version = app.server_info.major_version();

    // Channel for DB commands and results
    enum DbCommand {
        FetchSnapshot,
        CancelQuery(i32),
        TerminateBackend(i32),
        CancelQueries(Vec<i32>),
        TerminateBackends(Vec<i32>),
        RefreshBloat,
    }
    type BloatResult = (
        std::collections::HashMap<String, db::queries::TableBloat>,
        std::collections::HashMap<String, db::queries::IndexBloat>,
    );

    enum DbResult {
        Snapshot(Box<Result<PgSnapshot, String>>),
        CancelQuery(i32, Result<bool, String>),
        TerminateBackend(i32, Result<bool, String>),
        CancelQueries(Vec<(i32, bool)>),
        TerminateBackends(Vec<(i32, bool)>),
        BloatData(Result<BloatResult, String>),
    }

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<DbCommand>(16);
    let (result_tx, mut result_rx) = mpsc::unbounded_channel::<DbResult>();
    let client = Arc::new(client);
    let db_client = Arc::clone(&client);

    // Background task for DB operations
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            let result = match cmd {
                DbCommand::FetchSnapshot => {
                    DbResult::Snapshot(Box::new(
                        db::queries::fetch_snapshot(&db_client, &extensions, pg_major_version)
                            .await
                            .map_err(|e| e.to_string()),
                    ))
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
                DbCommand::CancelQueries(pids) => {
                    DbResult::CancelQueries(
                        db::queries::cancel_backends(&db_client, &pids).await,
                    )
                }
                DbCommand::TerminateBackends(pids) => {
                    DbResult::TerminateBackends(
                        db::queries::terminate_backends(&db_client, &pids).await,
                    )
                }
                DbCommand::RefreshBloat => {
                    let table_bloat = db::queries::fetch_table_bloat(&db_client, &extensions).await;
                    let index_bloat = db::queries::fetch_index_bloat(&db_client, &extensions).await;
                    match (table_bloat, index_bloat) {
                        (Ok(tb), Ok(ib)) => DbResult::BloatData(Ok((tb, ib))),
                        (Err(e), Ok(_)) => DbResult::BloatData(Err(format!("Table bloat query failed: {e}"))),
                        (Ok(_), Err(e)) => DbResult::BloatData(Err(format!("Index bloat query failed: {e}"))),
                        (Err(e1), Err(_)) => DbResult::BloatData(Err(format!("Bloat queries failed: {e1}"))),
                    }
                }
            };
            if result_tx.send(result).is_err() {
                break;
            }
        }
    });

    // Initial fetch
    let _ = cmd_tx.try_send(DbCommand::FetchSnapshot);

    let mut terminal = ratatui::init();
    let mut events = event::EventHandler::new(Duration::from_millis(10));
    let mut tick_interval = tokio::time::interval(Duration::from_secs(refresh));
    let mut spinner_interval = tokio::time::interval(Duration::from_millis(80));
    let mut refresh_interval_secs = refresh;

    loop {
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
                        DbResult::Snapshot(result) => match *result {
                            Ok(snap) => {
                                if let Some(ref mut rec) = recorder {
                                    if let Err(e) = rec.record(&snap) {
                                        app.status_message = Some(format!("Recording failed: {e}"));
                                    }
                                }
                                app.update(snap);
                            }
                            Err(e) => {
                                app.update_error(e);
                            }
                        }
                        DbResult::CancelQuery(pid, Ok(true)) => {
                            app.status_message = Some(format!("Cancelled query on PID {pid}"));
                            let _ = cmd_tx.try_send(DbCommand::FetchSnapshot);
                        }
                        DbResult::CancelQuery(pid, Ok(false))
                        | DbResult::TerminateBackend(pid, Ok(false)) => {
                            app.status_message = Some(format!("PID {pid} not found or already finished"));
                        }
                        DbResult::CancelQuery(_, Err(e)) => {
                            app.status_message = Some(format!("Cancel failed: {e}"));
                        }
                        DbResult::TerminateBackend(pid, Ok(true)) => {
                            app.status_message = Some(format!("Terminated backend PID {pid}"));
                            let _ = cmd_tx.try_send(DbCommand::FetchSnapshot);
                        }
                        DbResult::TerminateBackend(_, Err(e)) => {
                            app.status_message = Some(format!("Terminate failed: {e}"));
                        }
                        DbResult::CancelQueries(results) => {
                            let total = results.len();
                            let succeeded = results.iter().filter(|(_, ok)| *ok).count();
                            if succeeded == total {
                                app.status_message = Some(format!("Cancelled {succeeded}/{total} queries"));
                            } else {
                                app.status_message = Some(format!("Cancelled {}/{} queries ({} already finished)", succeeded, total, total - succeeded));
                            }
                            let _ = cmd_tx.try_send(DbCommand::FetchSnapshot);
                        }
                        DbResult::TerminateBackends(results) => {
                            let total = results.len();
                            let succeeded = results.iter().filter(|(_, ok)| *ok).count();
                            if succeeded == total {
                                app.status_message = Some(format!("Terminated {succeeded}/{total} backends"));
                            } else {
                                app.status_message = Some(format!("Terminated {}/{} backends ({} already finished)", succeeded, total, total - succeeded));
                            }
                            let _ = cmd_tx.try_send(DbCommand::FetchSnapshot);
                        }
                        DbResult::BloatData(Ok((table_bloat, index_bloat))) => {
                            app.bloat_loading = false;
                            app.apply_bloat_data(&table_bloat, &index_bloat);
                            let table_count = table_bloat.len();
                            let index_count = index_bloat.len();
                            app.status_message = Some(format!(
                                "Bloat estimates refreshed ({table_count} tables, {index_count} indexes)"
                            ));
                        }
                        DbResult::BloatData(Err(e)) => {
                            app.bloat_loading = false;
                            app.status_message = Some(format!("Bloat estimation failed: {e}"));
                        }
                    }
                }
            }
            _ = tick_interval.tick() => {
                if !app.paused {
                    let _ = cmd_tx.try_send(DbCommand::FetchSnapshot);
                }
            }
            _ = spinner_interval.tick() => {
                if app.bloat_loading {
                    app.spinner_frame = app.spinner_frame.wrapping_add(1);
                }
            }
        }

        // Process pending actions
        if let Some(action) = app.pending_action.take() {
            match action {
                AppAction::ForceRefresh => {
                    let _ = cmd_tx.try_send(DbCommand::FetchSnapshot);
                }
                AppAction::CancelQuery(pid) => {
                    let _ = cmd_tx.try_send(DbCommand::CancelQuery(pid));
                }
                AppAction::TerminateBackend(pid) => {
                    let _ = cmd_tx.try_send(DbCommand::TerminateBackend(pid));
                }
                AppAction::CancelQueries(pids) => {
                    let _ = cmd_tx.try_send(DbCommand::CancelQueries(pids));
                }
                AppAction::TerminateBackends(pids) => {
                    let _ = cmd_tx.try_send(DbCommand::TerminateBackends(pids));
                }
                AppAction::RefreshBloat => {
                    let _ = cmd_tx.try_send(DbCommand::RefreshBloat);
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

        // Check if user selected a recording to replay
        if let Some(replay_path) = app.pending_replay_path.take() {
            // Run replay, then return to live mode
            run_replay(&replay_path, app.config.clone()).await?;

            // Reset app state for live mode
            app.running = true;
            app.bottom_panel = app::BottomPanel::Queries;
            app.view_mode = app::ViewMode::Normal;
            app.replay = None;

            // Trigger immediate refresh
            let _ = cmd_tx.try_send(DbCommand::FetchSnapshot);

            // Continue outer loop to resume live mode
            continue;
        }

        // No replay requested, exit
        break;
    }

    ratatui::restore();
    Ok(())
}
