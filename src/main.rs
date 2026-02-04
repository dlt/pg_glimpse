mod app;
mod cli;
mod db;
mod event;
mod history;
mod ui;

use app::AppAction;
use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use std::time::Duration;

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

    let mut app = app::App::new(
        cli.host.clone(),
        cli.port,
        cli.dbname.clone(),
        cli.user.clone(),
        cli.refresh,
        cli.history_length,
    );

    let mut terminal = ratatui::init();
    let mut events = event::EventHandler::new(Duration::from_millis(50));
    let mut tick_interval = tokio::time::interval(Duration::from_secs(cli.refresh));

    // Initial fetch
    match db::queries::fetch_snapshot(&client).await {
        Ok(snap) => app.update(snap),
        Err(e) => app.update_error(e.to_string()),
    }

    while app.running {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        tokio::select! {
            _ = tick_interval.tick() => {
                if !app.paused {
                    match db::queries::fetch_snapshot(&client).await {
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
                    match db::queries::fetch_snapshot(&client).await {
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
                    if let Ok(snap) = db::queries::fetch_snapshot(&client).await {
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
                    if let Ok(snap) = db::queries::fetch_snapshot(&client).await {
                        app.update(snap);
                    }
                }
            }
        }
    }

    ratatui::restore();
    Ok(())
}
