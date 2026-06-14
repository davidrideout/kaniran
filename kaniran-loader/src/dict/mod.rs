//! The load-time half of `ichiran/dict`.
//!
//! The runtime dictionary layer (DAO, readings, segmentation, scoring,
//! and the runtime errata constants/hooks) lives in
//! `kaniran_core::dict`. This crate carries the Postgres-sourcing
//! loaders (`load::jmdict`, `load::readings`, `load::conjugate`) and the
//! load-time errata pipeline (`errata`).

pub mod errata;
pub mod load;
