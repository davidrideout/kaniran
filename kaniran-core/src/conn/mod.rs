//! Port of the `ichiran/conn` package — database connection handling.
//!
//! Most of the upstream package is subsumed by
//! [`kani_context::KaniranContext`] and is marked `skip` in the port
//! plan with reasons; see `reverse/scripts/symbols.csv`. The files in
//! this directory are the survivors that still have a 1:1 counterpart,
//! plus the `KaniranContext` sidecar itself (Rust-only, no Lisp
//! counterpart — CONVENTIONS §1, §2).
//!
//! Connection URLs are Postgres URLs read via the [`config`] crate from
//! the [`_star_connection_env_var_star_::DATABASE_URL`] env var (and any
//! future config-file source layered in).

pub mod _star_connection_env_var_star_;
pub mod get_ichiran_connection_env;
pub mod kani_context;
