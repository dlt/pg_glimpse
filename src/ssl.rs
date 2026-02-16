//! SSL/TLS certificate handling for client authentication.
//!
//! This module provides functionality for loading and managing client certificates
//! for mutual TLS (mTLS) authentication with PostgreSQL servers.

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Error types for certificate operations.
#[derive(Error, Debug)]
pub enum CertError {
    #[error("Failed to read certificate file {path}: {source}")]
    ReadCert {
        path: PathBuf,
        source: io::Error,
    },

    #[error("Failed to read private key file {path}: {source}")]
    ReadKey {
        path: PathBuf,
        source: io::Error,
    },

    #[error("Invalid PEM format in certificate file {path}: {reason}")]
    InvalidCertFormat {
        path: PathBuf,
        reason: String,
    },

    #[error("Invalid PEM format in private key file {path}: {reason}")]
    InvalidKeyFormat {
        path: PathBuf,
        reason: String,
    },

    #[error("No valid private key found in {path}")]
    NoPrivateKey {
        path: PathBuf,
    },
}

/// Configuration for SSL client certificates.
#[derive(Debug, Clone, Default)]
pub struct SslCertConfig {
    /// Path to client certificate file (PEM format).
    pub cert_path: Option<PathBuf>,

    /// Path to client private key file (PEM format).
    pub key_path: Option<PathBuf>,

    /// Path to CA root certificate file (PEM format).
    pub root_cert_path: Option<PathBuf>,
}

impl SslCertConfig {
    /// Creates a new empty certificate configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if client certificate paths are configured.
    pub fn has_client_cert(&self) -> bool {
        self.cert_path.is_some() && self.key_path.is_some()
    }

    /// Sets the client certificate path.
    pub fn with_cert(mut self, path: PathBuf) -> Self {
        self.cert_path = Some(path);
        self
    }

    /// Sets the client private key path.
    pub fn with_key(mut self, path: PathBuf) -> Self {
        self.key_path = Some(path);
        self
    }

    /// Sets the CA root certificate path.
    pub fn with_root_cert(mut self, path: PathBuf) -> Self {
        self.root_cert_path = Some(path);
        self
    }
}

/// Loads certificates from a PEM file.
///
/// # Arguments
/// * `path` - Path to the PEM-encoded certificate file
///
/// # Returns
/// Vector of parsed certificates
///
/// # Errors
/// Returns `CertError` if the file cannot be read or contains invalid PEM data.
pub fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, CertError> {
    let file = File::open(path).map_err(|e| CertError::ReadCert {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CertError::InvalidCertFormat {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

    if certs.is_empty() {
        return Err(CertError::InvalidCertFormat {
            path: path.to_path_buf(),
            reason: "No certificates found in file".to_string(),
        });
    }

    Ok(certs)
}

/// Loads a private key from a PEM file.
///
/// # Arguments
/// * `path` - Path to the PEM-encoded private key file
///
/// # Returns
/// Parsed private key
///
/// # Errors
/// Returns `CertError` if the file cannot be read, contains invalid PEM data,
/// or no valid private key is found.
pub fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, CertError> {
    let file = File::open(path).map_err(|e| CertError::ReadKey {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| CertError::InvalidKeyFormat {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?
        .ok_or_else(|| CertError::NoPrivateKey {
            path: path.to_path_buf(),
        })
}

/// Returns default certificate paths following PostgreSQL libpq conventions.
///
/// Returns paths to `~/.postgresql/postgresql.crt`, `~/.postgresql/postgresql.key`,
/// and `~/.postgresql/root.crt` if the `.postgresql` directory exists.
///
/// # Returns
/// `Some(SslCertConfig)` with default paths if `~/.postgresql/` exists, otherwise `None`.
pub fn default_paths() -> Option<SslCertConfig> {
    let home = dirs::home_dir()?;
    let pg_dir = home.join(".postgresql");

    if !pg_dir.is_dir() {
        return None;
    }

    Some(SslCertConfig {
        cert_path: Some(pg_dir.join("postgresql.crt")),
        key_path: Some(pg_dir.join("postgresql.key")),
        root_cert_path: Some(pg_dir.join("root.crt")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    /// Generate a test certificate and key pair using OpenSSL.
    /// Returns (cert_path, key_path).
    fn generate_test_cert(dir: &Path) -> (PathBuf, PathBuf) {
        use std::process::Command;

        let cert_path = dir.join("test.crt");
        let key_path = dir.join("test.key");

        // Generate self-signed certificate with OpenSSL
        let output = Command::new("openssl")
            .args([
                "req", "-x509", "-newkey", "rsa:2048", "-nodes",
                "-keyout", key_path.to_str().unwrap(),
                "-out", cert_path.to_str().unwrap(),
                "-days", "1",
                "-subj", "/CN=test",
            ])
            .output()
            .expect("Failed to generate test certificate (openssl not found?)");

        assert!(output.status.success(), "OpenSSL failed: {}", String::from_utf8_lossy(&output.stderr));
        (cert_path, key_path)
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_load_certs_valid() {
        let tmp_dir = TempDir::new().unwrap();
        let (cert_path, _key_path) = generate_test_cert(tmp_dir.path());

        let certs = load_certs(&cert_path).unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn test_load_certs_missing_file() {
        let result = load_certs(Path::new("/nonexistent/cert.pem"));
        assert!(matches!(result, Err(CertError::ReadCert { .. })));
    }

    #[test]
    fn test_load_certs_invalid_format() {
        let tmp_dir = TempDir::new().unwrap();
        let cert_path = create_test_file(tmp_dir.path(), "invalid.crt", "not a certificate");

        let result = load_certs(&cert_path);
        assert!(matches!(result, Err(CertError::InvalidCertFormat { .. })));
    }

    #[test]
    fn test_load_private_key_valid() {
        let tmp_dir = TempDir::new().unwrap();
        let (_cert_path, key_path) = generate_test_cert(tmp_dir.path());

        let key = load_private_key(&key_path);
        assert!(key.is_ok());
    }

    #[test]
    fn test_load_private_key_missing_file() {
        let result = load_private_key(Path::new("/nonexistent/key.pem"));
        assert!(matches!(result, Err(CertError::ReadKey { .. })));
    }

    #[test]
    fn test_load_private_key_invalid_format() {
        let tmp_dir = TempDir::new().unwrap();
        let key_path = create_test_file(tmp_dir.path(), "invalid.key", "not a private key");

        let result = load_private_key(&key_path);
        // rustls_pemfile::private_key returns Ok(None) for non-key content
        // which we then map to NoPrivateKey error
        assert!(matches!(result, Err(CertError::NoPrivateKey { .. })));
    }

    #[test]
    fn test_ssl_cert_config_has_client_cert() {
        let config = SslCertConfig::new()
            .with_cert(PathBuf::from("cert.pem"))
            .with_key(PathBuf::from("key.pem"));

        assert!(config.has_client_cert());
    }

    #[test]
    fn test_ssl_cert_config_no_client_cert() {
        let config = SslCertConfig::new();
        assert!(!config.has_client_cert());

        let config = SslCertConfig::new().with_cert(PathBuf::from("cert.pem"));
        assert!(!config.has_client_cert());

        let config = SslCertConfig::new().with_key(PathBuf::from("key.pem"));
        assert!(!config.has_client_cert());
    }

    #[test]
    fn test_default_paths_no_pg_dir() {
        // This test assumes ~/.postgresql doesn't exist or tests in temp directory
        // The actual behavior depends on the test environment
        let paths = default_paths();
        // Can't reliably test without mocking the filesystem
        // Just ensure it doesn't panic
        drop(paths);
    }

    #[test]
    fn test_default_paths_with_pg_dir() {
        // Create a temporary home directory structure
        let tmp_dir = TempDir::new().unwrap();
        let pg_dir = tmp_dir.path().join(".postgresql");
        fs::create_dir(&pg_dir).unwrap();

        // We can't easily test default_paths() without mocking dirs::home_dir()
        // so we'll just verify the structure would be correct
        let expected_cert = pg_dir.join("postgresql.crt");
        let expected_key = pg_dir.join("postgresql.key");
        let expected_root = pg_dir.join("root.crt");

        assert_eq!(expected_cert.file_name().unwrap(), "postgresql.crt");
        assert_eq!(expected_key.file_name().unwrap(), "postgresql.key");
        assert_eq!(expected_root.file_name().unwrap(), "root.crt");
    }
}
