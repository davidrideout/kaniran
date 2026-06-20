//! Build-the-archive data loaders for kaniran.
//!
//! These loaders source a kaniran Postgres database from the upstream
//! data files (JMdict, kanjidic2) plus the custom dictionary and the
//! load-time errata. They do real Postgres I/O, so they stay async
//! internally (sqlx + tokio); the synchronous runtime lookups live in
//! `kaniran-core`. This crate was split out of `kaniran-core` so the
//! default runtime build never links the loader stack.
//!
//! Module layout mirrors the ichiran packages the loaders came from:
//! `custom::` (`ichiran/custom`), `maintenance::` (`ichiran/maintenance`),
//! `dict::load::` and `dict::errata` (`ichiran/dict`'s load-time half),
//! and `kanji::loaders` (the Kanjidic2 XML loader).

pub mod custom;
pub mod dict;
pub mod kanji;
pub mod maintenance;

/// Map the backend-neutral lookup error from `kaniran-core`'s sync
/// lookup layer back into `sqlx::Error`, so loader functions that mix
/// direct sqlx writes with core lookups keep a single `sqlx::Error`
/// return type. A missing-row lookup round-trips to
/// `sqlx::Error::RowNotFound`; anything else is wrapped.
fn kani_db_to_sqlx(err: kaniran_core::conn::KaniDbError) -> sqlx::Error {
    match err {
        kaniran_core::conn::KaniDbError::RowNotFound => sqlx::Error::RowNotFound,
        other => sqlx::Error::Configuration(Box::new(other)),
    }
}
