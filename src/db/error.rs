use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("connection failed: {0}")]
    Connection(#[from] tokio_postgres::Error),

    #[error("query failed: {context}")]
    Query {
        context: &'static str,
        #[source]
        source: tokio_postgres::Error,
    },

    #[error("permission denied: {message}")]
    PermissionDenied { message: String, hint: Option<String> },

    #[error("unsupported postgres version: {version}")]
    UnsupportedVersion { version: u32 },
}

pub type Result<T> = std::result::Result<T, DbError>;
