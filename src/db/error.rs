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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_denied_display() {
        let err = DbError::PermissionDenied {
            message: "cannot read pg_stat_statements".to_string(),
            hint: Some("GRANT pg_read_all_stats TO user".to_string()),
        };
        assert_eq!(
            err.to_string(),
            "permission denied: cannot read pg_stat_statements"
        );
    }

    #[test]
    fn permission_denied_without_hint() {
        let err = DbError::PermissionDenied {
            message: "access denied".to_string(),
            hint: None,
        };
        assert_eq!(err.to_string(), "permission denied: access denied");
    }

    #[test]
    fn unsupported_version_display() {
        let err = DbError::UnsupportedVersion { version: 9 };
        assert_eq!(err.to_string(), "unsupported postgres version: 9");
    }

    #[test]
    fn unsupported_version_display_modern() {
        let err = DbError::UnsupportedVersion { version: 17 };
        assert_eq!(err.to_string(), "unsupported postgres version: 17");
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DbError>();
    }

    #[test]
    fn result_type_works() {
        let err: Result<i32> = Err(DbError::UnsupportedVersion { version: 8 });
        assert!(err.is_err());
    }

    #[test]
    fn debug_format_includes_variant() {
        let err = DbError::PermissionDenied {
            message: "test".to_string(),
            hint: None,
        };
        let debug = format!("{err:?}");
        assert!(debug.contains("PermissionDenied"));
        assert!(debug.contains("test"));
    }
}
