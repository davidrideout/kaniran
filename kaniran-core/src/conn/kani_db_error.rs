//! Rust-only sidecar: the dictionary-lookup error type.
//!
//! Replaces `crate::conn::KaniDbError` after the Postgres backend was removed in the
//! async-removal proof-of-concept. The rkyv snapshot backend serves
//! every lookup synchronously from a memory-mapped archive, so the only
//! genuine failure modes left are a missing single-row lookup, a
//! caller-synthesized always-failing probe, and a regex compile/match
//! failure. The three variants below carry exactly those, mapping 1:1
//! onto the `crate::conn::KaniDbError` variants the port previously constructed
//! (`RowNotFound`, `Protocol`, `Database`).

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
