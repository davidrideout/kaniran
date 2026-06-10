//! Port of the `ichiran/conn` package — database connection handling.
//!
//! Most of the upstream package is subsumed by
//! [`kani_context::KaniranContext`]; the files here are the survivors
//! with a 1:1 counterpart, plus the `KaniranContext` sidecar itself.
//! Connection URLs are Postgres URLs read via the [`config`] crate from
//! the [`_star_connection_env_var_star_::DATABASE_URL`] env var.

pub mod _star_connection_env_var_star_;
pub mod get_ichiran_connection_env;
pub mod kani_context;
pub mod kani_postgres_backend;
#[cfg(feature = "rkyv")]
pub mod kani_snapshot;
