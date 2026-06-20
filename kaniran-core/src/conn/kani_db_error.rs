//! Rust-only sidecar: the backend-neutral dictionary-lookup error type.
//!
//! Returned by every [`KaniBackend`](crate::conn::kani_backend::KaniBackend)
//! method. The rkyv snapshot backend serves lookups synchronously from a
//! memory-mapped archive, so it only ever produces a missing single-row
//! lookup (`RowNotFound`), a bad regex or a Postgres-only entry point
//! reached on rkyv (`Protocol`), or a wrapped lower-level error
//! (`Database`, e.g. the synthesized `exists-reading` probe in
//! `find_word_info`). With the `postgres` feature, the Postgres backend
//! maps every `sqlx::Error` into these three via the `From` impl below.

/// Dictionary-store lookup error.
#[derive(Debug)]
pub enum KaniDbError {
    /// A single-row lookup found nothing. Was `crate::conn::KaniDbError::RowNotFound`.
    RowNotFound,
    /// A usage/protocol error carrying a message — a bad regex, or a
    /// Postgres-only entry point reached on the rkyv backend. Was
    /// `crate::conn::KaniDbError::Protocol`.
    Protocol(String),
    /// A wrapped lower-level error (the synthesized compound-seq
    /// `exists-reading` probe in `find_word_info`). Was
    /// `crate::conn::KaniDbError::Database`.
    Database(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for KaniDbError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KaniDbError::RowNotFound => write!(formatter, "no rows returned"),
            KaniDbError::Protocol(message) => write!(formatter, "protocol error: {message}"),
            KaniDbError::Database(source) => write!(formatter, "database error: {source}"),
        }
    }
}

impl std::error::Error for KaniDbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KaniDbError::Database(source) => Some(source.as_ref()),
            _ => None,
        }
    }
}

/// Maps the Postgres backend's `sqlx::Error` into the backend-neutral
/// error: a missing row becomes [`KaniDbError::RowNotFound`]; everything
/// else is wrapped as [`KaniDbError::Database`].
#[cfg(feature = "postgres")]
impl From<sqlx::Error> for KaniDbError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => KaniDbError::RowNotFound,
            other => KaniDbError::Database(Box::new(other)),
        }
    }
}
