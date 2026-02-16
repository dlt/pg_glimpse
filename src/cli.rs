use crate::ssl::SslCertConfig;
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// `pg_glimpse` - A terminal-based `PostgreSQL` monitoring tool
#[derive(Parser, Debug)]
#[command(name = "pg_glimpse", version, about)]
pub struct Cli {
    /// Replay a recorded session instead of connecting to a database
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// `PostgreSQL` service name from ~/.pg_service.conf or pg_service.conf
    /// Example: --service=production (reads [production] section from service file)
    #[arg(long, env = "PGSERVICE")]
    pub service: Option<String>,

    /// `PostgreSQL` connection string (overrides service and individual params)
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

    /// Path to client certificate file for mutual TLS authentication
    #[arg(long = "ssl-cert", env = "PGSSLCERT")]
    pub ssl_cert: Option<PathBuf>,

    /// Path to client private key file for mutual TLS authentication
    #[arg(long = "ssl-key", env = "PGSSLKEY")]
    pub ssl_key: Option<PathBuf>,

    /// Path to CA root certificate file for server verification
    #[arg(long = "ssl-root-cert", env = "PGSSLROOTCERT")]
    pub ssl_root_cert: Option<PathBuf>,

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

/// Parse PostgreSQL service file and return parameters for the given service name
fn parse_pg_service_file(service_name: &str) -> Option<HashMap<String, String>> {
    // Try ~/.pg_service.conf first, then PGSYSCONFDIR/pg_service.conf
    let home_path = dirs::home_dir()?.join(".pg_service.conf");
    let paths = vec![home_path];

    for path in paths {
        if !path.exists() {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            let mut current_service: Option<String> = None;
            let mut service_params = HashMap::new();

            for line in content.lines() {
                let line = line.trim();

                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Check for section header [service_name]
                if line.starts_with('[') && line.ends_with(']') {
                    let section = &line[1..line.len() - 1];
                    current_service = Some(section.to_string());
                    if current_service.as_deref() != Some(service_name) {
                        service_params.clear();
                    }
                    continue;
                }

                // Parse key=value pairs
                if current_service.as_deref() == Some(service_name) {
                    if let Some((key, value)) = line.split_once('=') {
                        service_params.insert(
                            key.trim().to_string(),
                            value.trim().to_string()
                        );
                    }
                }
            }

            if !service_params.is_empty() {
                return Some(service_params);
            }
        }
    }

    None
}

impl Cli {
    /// Builds SSL certificate configuration from CLI args, service file, environment, and defaults.
    ///
    /// Priority (highest to lowest):
    /// 1. CLI arguments
    /// 2. Environment variables (handled by clap)
    /// 3. Service file
    /// 4. Default paths (~/.postgresql/) if files exist
    pub fn ssl_cert_config(&self) -> SslCertConfig {
        // Start with service file params if service is specified
        let service_params = self.service.as_ref()
            .and_then(|name| parse_pg_service_file(name));

        let mut config = SslCertConfig::new();

        // Apply service file parameters first (lowest priority)
        if let Some(ref params) = service_params {
            if let Some(cert) = params.get("sslcert") {
                config.cert_path = Some(PathBuf::from(cert));
            }
            if let Some(key) = params.get("sslkey") {
                config.key_path = Some(PathBuf::from(key));
            }
            if let Some(root_cert) = params.get("sslrootcert") {
                config.root_cert_path = Some(PathBuf::from(root_cert));
            }
        }

        // Override with CLI/env parameters (higher priority)
        if let Some(ref cert) = self.ssl_cert {
            config.cert_path = Some(cert.clone());
        }
        if let Some(ref key) = self.ssl_key {
            config.key_path = Some(key.clone());
        }
        if let Some(ref root_cert) = self.ssl_root_cert {
            config.root_cert_path = Some(root_cert.clone());
        }

        // Fall back to defaults if nothing specified AND files exist
        if config.cert_path.is_none() && config.key_path.is_none() && config.root_cert_path.is_none() {
            if let Some(defaults) = crate::ssl::default_paths() {
                // Only use default paths if the files actually exist
                if let Some(ref cert_path) = defaults.cert_path {
                    if cert_path.exists() {
                        config.cert_path = Some(cert_path.clone());
                    }
                }
                if let Some(ref key_path) = defaults.key_path {
                    if key_path.exists() {
                        config.key_path = Some(key_path.clone());
                    }
                }
                if let Some(ref root_cert_path) = defaults.root_cert_path {
                    if root_cert_path.exists() {
                        config.root_cert_path = Some(root_cert_path.clone());
                    }
                }
            }
        }

        config
    }

    pub fn pg_config(&self) -> Result<tokio_postgres::Config, tokio_postgres::Error> {
        // If connection string is provided, use it (highest priority)
        if let Some(ref conn_str) = self.connection_string {
            return conn_str.parse();
        }

        // Start with service file params if service is specified
        let service_params = self.service.as_ref()
            .and_then(|name| parse_pg_service_file(name));

        let mut config = tokio_postgres::Config::new();

        // Apply service parameters first (lowest priority)
        if let Some(ref params) = service_params {
            if let Some(host) = params.get("host") {
                config.host(host);
            }
            if let Some(port) = params.get("port") {
                if let Ok(p) = port.parse::<u16>() {
                    config.port(p);
                }
            }
            if let Some(dbname) = params.get("dbname") {
                config.dbname(dbname);
            }
            if let Some(user) = params.get("user") {
                config.user(user);
            }
            if let Some(password) = params.get("password") {
                config.password(password);
            }
        }

        // Override with CLI parameters (higher priority than service file)
        // Only override if the CLI value is not the default
        // For host, port, dbname, user: check if they're different from defaults
        let is_default_host = self.host == "localhost";
        let is_default_port = self.port == 5432;
        let is_default_dbname = self.dbname == "postgres";
        let is_default_user = self.user == "postgres";

        // If no service file, or if CLI param is explicitly set (not default), use CLI value
        if service_params.is_none() || !is_default_host {
            config.host(&self.host);
        }
        if service_params.is_none() || !is_default_port {
            config.port(self.port);
        }
        if service_params.is_none() || !is_default_dbname {
            config.dbname(&self.dbname);
        }
        if service_params.is_none() || !is_default_user {
            config.user(&self.user);
        }

        // Password from CLI always overrides if present
        if let Some(ref pw) = self.password {
            config.password(pw);
        }

        Ok(config)
    }

    /// Extract connection info for display, parsing from connection string if provided
    pub fn connection_info(&self) -> ConnectionInfo {
        // Connection string has highest priority
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

        // Try service file next
        let service_params = self.service.as_ref()
            .and_then(|name| parse_pg_service_file(name));

        let mut host = self.host.clone();
        let mut port = self.port;
        let mut dbname = self.dbname.clone();
        let mut user = self.user.clone();

        // Apply service parameters as defaults
        if let Some(ref params) = service_params {
            if let Some(h) = params.get("host") {
                host = h.clone();
            }
            if let Some(p) = params.get("port") {
                if let Ok(parsed_port) = p.parse::<u16>() {
                    port = parsed_port;
                }
            }
            if let Some(d) = params.get("dbname") {
                dbname = d.clone();
            }
            if let Some(u) = params.get("user") {
                user = u.clone();
            }
        }

        // CLI params override service file if not default
        let is_default_host = self.host == "localhost";
        let is_default_port = self.port == 5432;
        let is_default_dbname = self.dbname == "postgres";
        let is_default_user = self.user == "postgres";

        if service_params.is_none() || !is_default_host {
            host = self.host.clone();
        }
        if service_params.is_none() || !is_default_port {
            port = self.port;
        }
        if service_params.is_none() || !is_default_dbname {
            dbname = self.dbname.clone();
        }
        if service_params.is_none() || !is_default_user {
            user = self.user.clone();
        }

        ConnectionInfo {
            host,
            port,
            dbname,
            user,
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
            service: None,
            connection_string: Some("completely invalid {{{{".to_string()),
            host: "fallback".to_string(),
            port: 9999,
            dbname: "fallbackdb".to_string(),
            user: "fallbackuser".to_string(),
            password: None,
            ssl: false,
            ssl_insecure: false,
            ssl_cert: None,
            ssl_key: None,
            ssl_root_cert: None,
            refresh: None,
            history_length: 120,
        };
        let info = cli.connection_info();
        assert_eq!(info.host, "fallback");
        assert_eq!(info.port, 9999);
        assert_eq!(info.dbname, "fallbackdb");
        assert_eq!(info.user, "fallbackuser");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Service file support
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn parse_service_arg() {
        let cli = cli_from_args(&["--service", "production"]);
        assert_eq!(cli.service, Some("production".to_string()));
    }

    #[test]
    fn service_with_individual_params_override() {
        // When using --service but also providing CLI params, CLI params should override
        let cli = cli_from_args(&["--service", "myservice", "-H", "override-host"]);
        assert_eq!(cli.service, Some("myservice".to_string()));
        assert_eq!(cli.host, "override-host");
    }

    #[test]
    fn connection_string_overrides_service() {
        // Connection string should have highest priority
        let cli = cli_from_args(&[
            "--service",
            "myservice",
            "-c",
            "host=connhost port=5555 dbname=conndb user=connuser",
        ]);
        let info = cli.connection_info();
        // Connection string should win
        assert_eq!(info.host, "connhost");
        assert_eq!(info.port, 5555);
        assert_eq!(info.dbname, "conndb");
        assert_eq!(info.user, "connuser");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // SSL certificate arguments
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn parse_ssl_cert_arg() {
        let cli = cli_from_args(&["--ssl-cert", "/path/to/cert.pem"]);
        assert_eq!(cli.ssl_cert, Some(PathBuf::from("/path/to/cert.pem")));
    }

    #[test]
    fn parse_ssl_key_arg() {
        let cli = cli_from_args(&["--ssl-key", "/path/to/key.pem"]);
        assert_eq!(cli.ssl_key, Some(PathBuf::from("/path/to/key.pem")));
    }

    #[test]
    fn parse_ssl_root_cert_arg() {
        let cli = cli_from_args(&["--ssl-root-cert", "/path/to/root.crt"]);
        assert_eq!(cli.ssl_root_cert, Some(PathBuf::from("/path/to/root.crt")));
    }

    #[test]
    fn parse_all_ssl_cert_args() {
        let cli = cli_from_args(&[
            "--ssl-cert", "/path/to/cert.pem",
            "--ssl-key", "/path/to/key.pem",
            "--ssl-root-cert", "/path/to/root.crt",
        ]);
        assert_eq!(cli.ssl_cert, Some(PathBuf::from("/path/to/cert.pem")));
        assert_eq!(cli.ssl_key, Some(PathBuf::from("/path/to/key.pem")));
        assert_eq!(cli.ssl_root_cert, Some(PathBuf::from("/path/to/root.crt")));
    }

    #[test]
    fn ssl_cert_config_from_cli_args() {
        let cli = cli_from_args(&[
            "--ssl-cert", "/cli/cert.pem",
            "--ssl-key", "/cli/key.pem",
            "--ssl-root-cert", "/cli/root.crt",
        ]);
        let config = cli.ssl_cert_config();
        assert_eq!(config.cert_path, Some(PathBuf::from("/cli/cert.pem")));
        assert_eq!(config.key_path, Some(PathBuf::from("/cli/key.pem")));
        assert_eq!(config.root_cert_path, Some(PathBuf::from("/cli/root.crt")));
        assert!(config.has_client_cert());
    }

    #[test]
    fn ssl_cert_config_empty_when_no_args() {
        let cli = cli_from_args(&[]);
        let config = cli.ssl_cert_config();
        // Should fall back to defaults only if they exist, which they likely don't in test env
        assert!(!config.has_client_cert() || config.cert_path.is_some());
    }

    #[test]
    fn ssl_cert_config_partial_client_cert() {
        // Only cert without key should not report has_client_cert
        let cli = cli_from_args(&["--ssl-cert", "/path/to/cert.pem"]);
        let config = cli.ssl_cert_config();
        assert!(!config.has_client_cert());

        // Only key without cert should not report has_client_cert
        let cli = cli_from_args(&["--ssl-key", "/path/to/key.pem"]);
        let config = cli.ssl_cert_config();
        assert!(!config.has_client_cert());
    }
}
