use clap::Parser;
use std::path::PathBuf;

/// `pg_glimpse` - A terminal-based `PostgreSQL` monitoring tool
#[derive(Parser, Debug)]
#[command(name = "pg_glimpse", version, about)]
pub struct Cli {
    /// Replay a recorded session instead of connecting to a database
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// `PostgreSQL` connection string (overrides individual params)
    /// Example: "host=localhost port=5432 dbname=mydb user=postgres password=secret"
    /// Or URI: "postgresql://user:pass@host:port/dbname"
    #[arg(short = 'c', long = "connection", env = "PG_GLIMPSE_CONNECTION")]
    pub connection_string: Option<String>,

    /// `PostgreSQL` host
    #[arg(short = 'H', long, env = "PGHOST", default_value = "localhost")]
    pub host: String,

    /// `PostgreSQL` port
    #[arg(short = 'p', long, env = "PGPORT", default_value_t = 5432)]
    pub port: u16,

    /// `PostgreSQL` database name
    #[arg(short = 'd', long, env = "PGDATABASE", default_value = "postgres")]
    pub dbname: String,

    /// `PostgreSQL` user
    #[arg(short = 'U', long, env = "PGUSER", default_value = "postgres")]
    pub user: String,

    /// `PostgreSQL` password
    #[arg(short = 'W', long, env = "PGPASSWORD")]
    pub password: Option<String>,

    /// Enable SSL/TLS connection (required for most cloud databases like AWS RDS)
    #[arg(short = 's', long, env = "PGSSLMODE")]
    pub ssl: bool,

    /// Skip SSL certificate verification (use with --ssl for self-signed or cloud certs)
    #[arg(long)]
    pub ssl_insecure: bool,

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
        self.connection_string.as_ref().map_or_else(
            || {
                let mut config = tokio_postgres::Config::new();
                config.host(&self.host);
                config.port(self.port);
                config.dbname(&self.dbname);
                config.user(&self.user);
                if let Some(ref pw) = self.password {
                    config.password(pw);
                }
                Ok(config)
            },
            |conn_str| conn_str.parse(),
        )
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
                    .get_dbname().map_or_else(|| self.dbname.clone(), std::string::ToString::to_string);
                let user = config
                    .get_user().map_or_else(|| self.user.clone(), std::string::ToString::to_string);
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn cli_from_args(args: &[&str]) -> Cli {
        let mut full_args = vec!["pg_glimpse"];
        full_args.extend(args);
        Cli::parse_from(full_args)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Default values
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn default_values() {
        let cli = cli_from_args(&[]);
        assert_eq!(cli.host, "localhost");
        assert_eq!(cli.port, 5432);
        assert_eq!(cli.dbname, "postgres");
        assert_eq!(cli.user, "postgres");
        assert!(!cli.ssl);
        assert!(!cli.ssl_insecure);
        assert!(cli.password.is_none());
        assert!(cli.connection_string.is_none());
        assert!(cli.replay.is_none());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Individual parameter parsing
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn parse_host_short() {
        let cli = cli_from_args(&["-H", "myhost.example.com"]);
        assert_eq!(cli.host, "myhost.example.com");
    }

    #[test]
    fn parse_host_long() {
        let cli = cli_from_args(&["--host", "db.example.com"]);
        assert_eq!(cli.host, "db.example.com");
    }

    #[test]
    fn parse_port_short() {
        let cli = cli_from_args(&["-p", "5433"]);
        assert_eq!(cli.port, 5433);
    }

    #[test]
    fn parse_port_long() {
        let cli = cli_from_args(&["--port", "15432"]);
        assert_eq!(cli.port, 15432);
    }

    #[test]
    fn parse_dbname_short() {
        let cli = cli_from_args(&["-d", "mydb"]);
        assert_eq!(cli.dbname, "mydb");
    }

    #[test]
    fn parse_user_short() {
        let cli = cli_from_args(&["-U", "admin"]);
        assert_eq!(cli.user, "admin");
    }

    #[test]
    fn parse_password_short() {
        let cli = cli_from_args(&["-W", "secret123"]);
        assert_eq!(cli.password, Some("secret123".to_string()));
    }

    #[test]
    fn parse_ssl_flag() {
        let cli = cli_from_args(&["-s"]);
        assert!(cli.ssl);
        assert!(!cli.ssl_insecure);
    }

    #[test]
    fn parse_ssl_insecure_flag() {
        let cli = cli_from_args(&["--ssl-insecure"]);
        assert!(cli.ssl_insecure);
    }

    #[test]
    fn parse_ssl_and_ssl_insecure_together() {
        let cli = cli_from_args(&["-s", "--ssl-insecure"]);
        assert!(cli.ssl);
        assert!(cli.ssl_insecure);
    }

    #[test]
    fn parse_refresh_interval() {
        let cli = cli_from_args(&["-r", "5"]);
        assert_eq!(cli.refresh, Some(5));
    }

    #[test]
    fn parse_history_length() {
        let cli = cli_from_args(&["--history-length", "240"]);
        assert_eq!(cli.history_length, 240);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Connection string parsing
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn connection_string_key_value() {
        let cli = cli_from_args(&["-c", "host=myhost port=5433 dbname=mydb user=myuser"]);
        let info = cli.connection_info();
        assert_eq!(info.host, "myhost");
        assert_eq!(info.port, 5433);
        assert_eq!(info.dbname, "mydb");
        assert_eq!(info.user, "myuser");
    }

    #[test]
    fn connection_string_uri() {
        let cli = cli_from_args(&["-c", "postgresql://testuser@testhost:5434/testdb"]);
        let info = cli.connection_info();
        assert_eq!(info.host, "testhost");
        assert_eq!(info.port, 5434);
        assert_eq!(info.dbname, "testdb");
        assert_eq!(info.user, "testuser");
    }

    #[test]
    fn connection_string_uri_with_password() {
        let cli = cli_from_args(&["-c", "postgresql://user:pass%40word@host:5432/db"]);
        let info = cli.connection_info();
        assert_eq!(info.host, "host");
        assert_eq!(info.user, "user");
        assert_eq!(info.dbname, "db");
    }

    #[test]
    fn connection_string_partial_uses_defaults() {
        let cli = cli_from_args(&["-c", "host=customhost"]);
        let info = cli.connection_info();
        assert_eq!(info.host, "customhost");
        // Port should fall back to CLI default
        assert_eq!(info.port, 5432);
    }

    #[test]
    fn connection_string_overrides_individual_params() {
        let cli = cli_from_args(&[
            "-c",
            "host=connhost port=5555 dbname=conndb user=connuser",
            "-H",
            "clihost",
            "-p",
            "6666",
        ]);
        let info = cli.connection_info();
        // Connection string should win
        assert_eq!(info.host, "connhost");
        assert_eq!(info.port, 5555);
        assert_eq!(info.dbname, "conndb");
        assert_eq!(info.user, "connuser");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // pg_config generation
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn pg_config_from_individual_params() {
        let cli = cli_from_args(&["-H", "testhost", "-p", "5433", "-d", "testdb", "-U", "testuser"]);
        let config = cli.pg_config().unwrap();

        assert_eq!(config.get_hosts().len(), 1);
        assert_eq!(config.get_ports(), &[5433]);
        assert_eq!(config.get_dbname(), Some("testdb"));
        assert_eq!(config.get_user(), Some("testuser"));
    }

    #[test]
    fn pg_config_from_connection_string() {
        let cli = cli_from_args(&["-c", "host=connhost port=5555 dbname=conndb user=connuser"]);
        let config = cli.pg_config().unwrap();

        assert_eq!(config.get_dbname(), Some("conndb"));
        assert_eq!(config.get_user(), Some("connuser"));
        assert_eq!(config.get_ports(), &[5555]);
    }

    #[test]
    fn pg_config_with_password() {
        let cli = cli_from_args(&["-H", "host", "-W", "secret"]);
        let config = cli.pg_config().unwrap();
        // Password is set but not directly queryable from Config
        assert!(config.get_user().is_some());
    }

    #[test]
    fn pg_config_invalid_connection_string() {
        let cli = cli_from_args(&["-c", "not a valid connection string with = but bad"]);
        // tokio-postgres is lenient, but completely malformed strings may error
        // This test documents current behavior
        let _ = cli.pg_config();
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Replay mode
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn parse_replay_path() {
        let cli = cli_from_args(&["--replay", "/path/to/recording.jsonl"]);
        assert_eq!(
            cli.replay,
            Some(std::path::PathBuf::from("/path/to/recording.jsonl"))
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Edge cases
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn host_with_special_chars() {
        let cli = cli_from_args(&["-H", "my-host.example.com"]);
        assert_eq!(cli.host, "my-host.example.com");
    }

    #[test]
    fn dbname_with_underscore() {
        let cli = cli_from_args(&["-d", "my_database_name"]);
        assert_eq!(cli.dbname, "my_database_name");
    }

    #[test]
    fn all_params_together() {
        let cli = cli_from_args(&[
            "-H",
            "prodhost",
            "-p",
            "5433",
            "-d",
            "production",
            "-U",
            "admin",
            "-W",
            "secret",
            "-s",
            "-r",
            "5",
            "--history-length",
            "200",
        ]);

        assert_eq!(cli.host, "prodhost");
        assert_eq!(cli.port, 5433);
        assert_eq!(cli.dbname, "production");
        assert_eq!(cli.user, "admin");
        assert_eq!(cli.password, Some("secret".to_string()));
        assert!(cli.ssl);
        assert_eq!(cli.refresh, Some(5));
        assert_eq!(cli.history_length, 200);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Connection info fallback behavior
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn connection_info_without_connection_string() {
        let cli = cli_from_args(&["-H", "myhost", "-p", "5433", "-d", "mydb", "-U", "myuser"]);
        let info = cli.connection_info();
        assert_eq!(info.host, "myhost");
        assert_eq!(info.port, 5433);
        assert_eq!(info.dbname, "mydb");
        assert_eq!(info.user, "myuser");
    }

    #[test]
    fn connection_info_invalid_string_falls_back() {
        // If connection string can't be parsed, should fall back to individual params
        let cli = Cli {
            replay: None,
            connection_string: Some("completely invalid {{{{".to_string()),
            host: "fallback".to_string(),
            port: 9999,
            dbname: "fallbackdb".to_string(),
            user: "fallbackuser".to_string(),
            password: None,
            ssl: false,
            ssl_insecure: false,
            refresh: None,
            history_length: 120,
        };
        let info = cli.connection_info();
        assert_eq!(info.host, "fallback");
        assert_eq!(info.port, 9999);
        assert_eq!(info.dbname, "fallbackdb");
        assert_eq!(info.user, "fallbackuser");
    }
}
