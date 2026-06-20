//! Port of the `ichiran/conn` package — database connection handling.
//!
//! Most of the upstream package is subsumed by
//! [`kani_context::KaniranContext`]; the files here are the survivors
//! with a 1:1 counterpart, plus the `KaniranContext` sidecar itself.
//!
//! Lookups are synchronous and fail with [`kani_db_error::KaniDbError`].
//! The default build's only backend is the memory-mapped rkyv snapshot
//! ([`kani_rkyv_backend`]), selected via `DATABASE_URL=memory://<path>`.
//! The `postgres` feature adds the runtime-swappable Postgres backend
//! ([`kani_postgres_backend`]), selected via `DATABASE_URL=postgres://...`.

pub mod _star_connection_env_var_star_;
pub mod get_ichiran_connection_env;
pub mod kani_backend;
pub mod kani_context;
pub mod kani_db_error;
#[cfg(feature = "postgres")]
pub mod kani_postgres_backend;
#[cfg(feature = "rkyv")]
pub mod kani_rkyv_backend;
#[cfg(feature = "rkyv")]
pub mod kani_snapshot;

pub use kani_db_error::KaniDbError;
