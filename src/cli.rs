use clap::Parser;

/// pg_glimpse - A terminal-based PostgreSQL monitoring tool
#[derive(Parser, Debug)]
#[command(name = "pg_glimpse", version, about)]
pub struct Cli {
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

    /// Refresh interval in seconds
    #[arg(short = 'r', long, default_value_t = 2)]
    pub refresh: u64,

    /// Number of data points to keep in sparkline history
    #[arg(long, default_value_t = 120)]
    pub history_length: usize,
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
}
