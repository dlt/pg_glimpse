use clap::Parser;
use std::path::PathBuf;

/// pg_glimpse - A terminal-based PostgreSQL monitoring tool
#[derive(Parser, Debug)]
#[command(name = "pg_glimpse", version, about)]
pub struct Cli {
    /// Replay a recorded session instead of connecting to a database
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// PostgreSQL connection string (overrides individual params)
    /// Example: "host=localhost port=5432 dbname=mydb user=postgres password=secret"
    /// Or URI: "postgresql://user:pass@host:port/dbname"
    #[arg(short = 'c', long = "connection", env = "PG_GLIMPSE_CONNECTION")]
    pub connection_string: Option<String>,

    /// PostgreSQL host
    #[arg(short = 'H', long, env = "PGHOST", default_value = "localhost")]
    pub host: String,

    /// PostgreSQL port
    #[arg(short = 'p', long, env = "PGPORT", default_value_t = 5432)]
    pub port: u16,

    /// PostgreSQL database name
    #[arg(short = 'd', long, env = "PGDATABASE", default_value = "postgres")]
    pub dbname: String,

    /// PostgreSQL user
    #[arg(short = 'U', long, env = "PGUSER", default_value = "postgres")]
    pub user: String,

    /// PostgreSQL password
    #[arg(short = 'W', long, env = "PGPASSWORD")]
    pub password: Option<String>,

    /// Refresh interval in seconds (overrides config file)
    #[arg(short = 'r', long)]
    pub refresh: Option<u64>,

    /// Number of data points to keep in sparkline history
    #[arg(long, default_value_t = 120)]
    pub history_length: usize,
}

/// Connection display info for the header
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub user: String,
}

impl Cli {
    pub fn pg_config(&self) -> Result<tokio_postgres::Config, tokio_postgres::Error> {
        if let Some(ref conn_str) = self.connection_string {
            conn_str.parse()
        } else {
            let mut config = tokio_postgres::Config::new();
            config.host(&self.host);
            config.port(self.port);
            config.dbname(&self.dbname);
            config.user(&self.user);
            if let Some(ref pw) = self.password {
                config.password(pw);
            }
            Ok(config)
        }
    }

    /// Extract connection info for display, parsing from connection string if provided
    pub fn connection_info(&self) -> ConnectionInfo {
        if let Some(ref conn_str) = self.connection_string {
            if let Ok(config) = conn_str.parse::<tokio_postgres::Config>() {
                let host = config
                    .get_hosts()
                    .first()
                    .map(|h| match h {
                        tokio_postgres::config::Host::Tcp(s) => s.clone(),
                        #[cfg(unix)]
                        tokio_postgres::config::Host::Unix(p) => {
                            p.to_string_lossy().into_owned()
                        }
                    })
                    .unwrap_or_else(|| self.host.clone());
                let port = config.get_ports().first().copied().unwrap_or(self.port);
                let dbname = config
                    .get_dbname()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| self.dbname.clone());
                let user = config
                    .get_user()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| self.user.clone());
                return ConnectionInfo {
                    host,
                    port,
                    dbname,
                    user,
                };
            }
        }
        ConnectionInfo {
            host: self.host.clone(),
            port: self.port,
            dbname: self.dbname.clone(),
            user: self.user.clone(),
        }
    }
}
