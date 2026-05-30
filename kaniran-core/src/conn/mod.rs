//! Port of the `ichiran/conn` package — database connection handling.
//!
//! Most of the upstream package is subsumed by
//! [`kani_context::KaniranContext`] and is marked `skip` in the port
//! plan with reasons; see `reverse/scripts/symbols.csv`. The files in
//! this directory are the survivors that still have a 1:1 counterpart,
//! plus the `KaniranContext` and `KaniConfig` sidecars (no Lisp
//! counterparts).

pub mod kani_config;
pub mod kani_context;
