mod app;
mod cli;
mod config;
mod db;
mod event;
mod history;
mod ui;

use app::AppAction;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use config::AppConfig;
use std::time::Duration;
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

    // Fetch server info and extensions at startup
    let server_info = db::queries::fetch_server_info(&client).await?;

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
        Ok(snap) => app.update(snap),
        Err(e) => app.update_error(e.to_string()),
    }

    while app.running {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        tokio::select! {
            _ = tick_interval.tick() => {
                if !app.paused {
                    match db::queries::fetch_snapshot(&client, &extensions).await {
                        Ok(snap) => app.update(snap),
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
                        Ok(snap) => app.update(snap),
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
