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
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    const TEST_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIC/zCCAeegAwIBAgIUVB18SrzqagkNTjv+yCGkG2EMGU8wDQYJKoZIhvcNAQEL
BQAwDzENMAsGA1UEAwwEdGVzdDAeFw0yNjAyMTYxNzM5MjVaFw0yNzAyMTYxNzM5
MjVaMA8xDTALBgNVBAMMBHRlc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEK
AoIBAQDSblGewWDEE/zue2M6VTulPikgH7NyjIiHdWxsyoI9AoTicOfVpiDB6BXg
H6+kUwo4vyltJ/tqWHHILy3NwNeb+wpO/ekjzcT3sbgo4tWQu5h5m23FrBv5CDp0
anf7Ul9seOTveprFe2A5stF6lEObx65gmppoBB1h7WDOpnKsk8DyKOhQPM8kPVmL
R7AVfqxXt1puLs0gaUh0jopZLuT0KTwNwYhGknYCF92HBrR5AZCGh62PEdIEXCEk
sC2brakzfjYx/xbhUjYJG2vwbUn+M98zCWtG8BrkyP9hCEaZZaE97/BN5jj+xHZj
Uj+w7yzDFgm4B0CPa3J2W9rFRoyvAgMBAAGjUzBRMB0GA1UdDgQWBBRRJImXIF98
c7AafXvTic/+6zzSWzAfBgNVHSMEGDAWgBRRJImXIF98c7AafXvTic/+6zzSWzAP
BgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3DQEBCwUAA4IBAQC995tLC0XSgXl0T9US
+L7nPxtW5Afcx63AeeTvYkE9PAKPzIsppVO1DFqGsOzAmljmunF7oMmBzkxB7YTC
eEFNyucxZiaPTk5iqlv1YQqIXBWIAex0WCdNSW8dksiopbdLS3CJYp7nBKqXfmE4
XJoYxDIZtwQ5fV3rH4pChm+USchrOVcc0eBLROu3N8BFbVoazsKQJayznuezZfCA
O0qHTkIaWi/ijPXLle5qEXg4b6mZ1sU2UfHZPxtDA2Geoy6269+/OE4qUW/Rlua+
MZ+FZ3+g8qcgpAqOJk2gMPney5Nkr8r3LlSsR8ayt3LbNBZYIejLFAw85G2PxZsf
PeXb
-----END CERTIFICATE-----"#;

    const TEST_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDSblGewWDEE/zu
e2M6VTulPikgH7NyjIiHdWxsyoI9AoTicOfVpiDB6BXgH6+kUwo4vyltJ/tqWHHI
Ly3NwNeb+wpO/ekjzcT3sbgo4tWQu5h5m23FrBv5CDp0anf7Ul9seOTveprFe2A5
stF6lEObx65gmppoBB1h7WDOpnKsk8DyKOhQPM8kPVmLR7AVfqxXt1puLs0gaUh0
jopZLuT0KTwNwYhGknYCF92HBrR5AZCGh62PEdIEXCEksC2brakzfjYx/xbhUjYJ
G2vwbUn+M98zCWtG8BrkyP9hCEaZZaE97/BN5jj+xHZjUj+w7yzDFgm4B0CPa3J2
W9rFRoyvAgMBAAECggEAHEuXOrNNUVr6sg/x/3Wsh9im6683kESTVduVO7+zPtmS
3WafKzGjyqRsJJnozT4ZQW4lojbebftR5BVr9vRiSZK6XSAenvYxRmPQRox8eRyw
hlGUXiB5a8Vjzs7uLo4cpKz7jl68ZvzjAH8p/xiRSrt77XjRywDFTqtTpBXqP4gp
QP4OPxjBOp3gfoYYvAFvWf9DaDYpbX6IDxqHCilwMeYggt4fwfire3ngSZXE5P3T
0MuADC/3MyRAYda5orKH/bzphw58zT7ZofGN5/Kc9OB+D/o2ko83SFT5G8pYWZVG
Xz4mASwRVjAbXFvyN/H8G3lrhzHySFsp4hfO63BFmQKBgQD/KhY9XYbcFZR5aQpD
fwLIHIe17qxLQAog/1TPz9fir0K1zn6cN3pWdUvXjaUAYHRzX9aD2o0IPYbfzYrl
AtgoF8CvG+FHdKRVClD6tTOf7frsuOHF9A7ItfxAO0BHnLEV21DEzdZpe/Y+A+Zl
/gA2EckTzf5LMiJnC+DJFndREwKBgQDTHrr3zQDoqnZkiuZ6p6NU0TPrqr1EWTPL
kDmjKPxhmiJTq77VAnvUZw9eWc6qoesWUOIUDW5ohfDvPe/HWaAoaQk5iurK5AmG
ryilkyyVtsiBXIZ9By52rF3eX6y+LaCCxXbmL9UsKfn+Gperm/Bsn381KutaI/JX
mPNdlXZldQKBgQCBQSKO50efSNczQUBPvJD+KWWdhU+FtuTqniyqMFDdpYYXboi4
PWodTcGjaT8CF9olb5DMrfLvD6u4xvfq1iwE8zNKAMd3WOC9q0ImHZAPHZAURfso
OV8b0QP8zYbcP8V8muIpL1PDj2XHOFaHp8kXmp7PB3QfR0AiDuRJOLYsPQKBgHnr
ej/WlNrIbly71kQpAWre8aP8UxbgiMfa/14ZMj7PO1mkii0LJSXRao+rP21M2q1l
glngM82K5EvVMd6nBJWxqtEfR15p+JJeHxQXfRzslLgYDdawSgXgnsjn6aNeSB6d
GH/wSaQajbNP+hzxjhO8vEKhCY9hyPcLbieyQ9BtAoGBAPXrCXtXtbieLAF45l30
S2ml35ntv1jy+p1SW+Q30nYBploay7Xjwp6Jc6AlGPzGgcVggXdVl2/rNvXceBfz
SafSWJU/hAgYCWdwseKe8g7sVEaAoFT2hI5bj5FC0dzc9ODVgXb2/vdpJBaROQ2y
wsTNKrqMPDgSZdAoJaRCiXWW
-----END PRIVATE KEY-----"#;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
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
        let cert_path = create_test_file(&tmp_dir, "client.crt", TEST_CERT_PEM);
        let key_path = create_test_file(&tmp_dir, "client.key", TEST_KEY_PEM);

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
        let root_cert_path = create_test_file(&tmp_dir, "root.crt", TEST_CERT_PEM);

        let config = SslCertConfig::new().with_root_cert(root_cert_path);

        let result = build_tls_config(true, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tls_config_with_invalid_root_cert() {
        let tmp_dir = TempDir::new().unwrap();
        let root_cert_path = create_test_file(&tmp_dir, "root.crt", "not a certificate");

        let config = SslCertConfig::new().with_root_cert(root_cert_path);

        let result = build_tls_config(true, &config);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConnectionError::Certificate(_)));
    }
}
