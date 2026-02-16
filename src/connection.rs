//! PostgreSQL connection handling with SSL/TLS support.

use crate::ssl::{self, CertError, SslCertConfig};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::DigitallySignedStruct;
use std::io;
use std::sync::Arc;
use thiserror::Error;

/// Connection error types
#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("PostgreSQL connection failed: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    #[error("Certificate error: {0}")]
    Certificate(#[from] CertError),

    #[error("TLS configuration error: {0}")]
    Tls(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Certificate verifier that accepts any certificate (for --ssl-insecure)
#[derive(Debug)]
struct NoVerifier;

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// SSL connection mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SslMode {
    None,
    Verified,
    Insecure,
}

impl SslMode {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::None => "No TLS",
            Self::Verified => "SSL",
            Self::Insecure => "SSL (unverified)",
        }
    }
}

/// Spawn the connection handler task
fn spawn_connection<S, T>(connection: tokio_postgres::Connection<S, T>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("PostgreSQL connection error: {e}");
        }
    });
}

/// Build TLS configuration with optional client certificate authentication.
///
/// # Arguments
/// * `verify_server` - Whether to verify server certificates (false for --ssl-insecure)
/// * `cert_config` - Optional client certificate configuration
///
/// # Returns
/// Configured rustls ClientConfig
///
/// # Errors
/// Returns `ConnectionError` if certificate loading fails
fn build_tls_config(
    verify_server: bool,
    cert_config: &SslCertConfig,
) -> Result<rustls::ClientConfig, ConnectionError> {
    // Build root certificate store
    let mut root_store = rustls::RootCertStore::empty();

    if verify_server {
        // Use custom root cert if provided, otherwise use system roots
        if let Some(ref root_cert_path) = cert_config.root_cert_path {
            let certs = ssl::load_certs(root_cert_path)?;
            for cert in certs {
                root_store
                    .add(cert)
                    .map_err(|e| ConnectionError::Tls(format!("Failed to add root certificate: {}", e)))?;
            }
        } else {
            // Use webpki system roots
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
    }

    // Build TLS config with appropriate verification
    let config_builder = if verify_server {
        rustls::ClientConfig::builder().with_root_certificates(root_store)
    } else {
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
    };

    // Add client certificate authentication if configured
    let tls_config = if cert_config.has_client_cert() {
        let cert_path = cert_config.cert_path.as_ref().unwrap();
        let key_path = cert_config.key_path.as_ref().unwrap();

        let certs = ssl::load_certs(cert_path)?;
        let key = ssl::load_private_key(key_path)?;

        config_builder
            .with_client_auth_cert(certs, key)
            .map_err(|e| ConnectionError::Tls(format!("Failed to configure client certificate: {}", e)))?
    } else {
        config_builder.with_no_client_auth()
    };

    Ok(tls_config)
}

/// Try to connect with a specific SSL mode
pub async fn try_connect(
    pg_config: &tokio_postgres::Config,
    ssl_mode: SslMode,
    cert_config: &SslCertConfig,
) -> Result<tokio_postgres::Client, ConnectionError> {
    match ssl_mode {
        SslMode::None => {
            let (client, connection) = pg_config.connect(tokio_postgres::NoTls).await?;
            spawn_connection(connection);
            Ok(client)
        }
        SslMode::Verified => {
            let tls_config = build_tls_config(true, cert_config)?;
            let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);
            let (client, connection) = pg_config.connect(tls).await?;
            spawn_connection(connection);
            Ok(client)
        }
        SslMode::Insecure => {
            let tls_config = build_tls_config(false, cert_config)?;
            let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);
            let (client, connection) = pg_config.connect(tls).await?;
            spawn_connection(connection);
            Ok(client)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Generate a test certificate and key pair using OpenSSL.
    /// Returns (cert_path, key_path).
    fn generate_test_cert(dir: &TempDir) -> (PathBuf, PathBuf) {
        use std::process::Command;

        let cert_path = dir.path().join("test.crt");
        let key_path = dir.path().join("test.key");

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

    #[test]
    fn test_ssl_mode_labels() {
        assert_eq!(SslMode::None.label(), "No TLS");
        assert_eq!(SslMode::Verified.label(), "SSL");
        assert_eq!(SslMode::Insecure.label(), "SSL (unverified)");
    }

    #[test]
    fn test_build_tls_config_no_client_cert_verified() {
        let config = SslCertConfig::new();
        let result = build_tls_config(true, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tls_config_no_client_cert_insecure() {
        let config = SslCertConfig::new();
        let result = build_tls_config(false, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tls_config_with_valid_client_cert() {
        let tmp_dir = TempDir::new().unwrap();
        let (cert_path, key_path) = generate_test_cert(&tmp_dir);

        let config = SslCertConfig::new()
            .with_cert(cert_path)
            .with_key(key_path);

        let result = build_tls_config(false, &config);
        if let Err(e) = &result {
            eprintln!("Error: {}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tls_config_with_missing_cert_file() {
        let config = SslCertConfig::new()
            .with_cert(PathBuf::from("/nonexistent/cert.pem"))
            .with_key(PathBuf::from("/nonexistent/key.pem"));

        let result = build_tls_config(false, &config);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConnectionError::Certificate(_)));
    }

    #[test]
    fn test_build_tls_config_with_custom_root_cert() {
        let tmp_dir = TempDir::new().unwrap();
        let (root_cert_path, _key_path) = generate_test_cert(&tmp_dir);

        let config = SslCertConfig::new().with_root_cert(root_cert_path);

        let result = build_tls_config(true, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tls_config_with_invalid_root_cert() {
        let tmp_dir = TempDir::new().unwrap();
        let root_cert_path = tmp_dir.path().join("root.crt");
        fs::write(&root_cert_path, "not a certificate").unwrap();

        let config = SslCertConfig::new().with_root_cert(root_cert_path);

        let result = build_tls_config(true, &config);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConnectionError::Certificate(_)));
    }
}
