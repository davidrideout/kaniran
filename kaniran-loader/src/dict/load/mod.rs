//! Postgres-sourcing loaders for `ichiran/dict`'s load-time half.
//!
//! The sync conjugation helpers (`construct_conjugation`,
//! `conjugate_word`, `lex_compare`) and the rule/POS tables stay in
//! `kaniran_core::dict::load`; here live the async DB writers.

pub mod conjugate;
pub mod jmdict;
pub mod readings;
