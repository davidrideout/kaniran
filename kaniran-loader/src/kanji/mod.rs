//! Kanjidic2 XML loader (the load-time half of `ichiran/kanji`).
//!
//! The runtime kanji layer — DAO rows, readings, matching, stats,
//! JSON — lives in `kaniran_core::kanji`. Only the Postgres-sourcing
//! loader moved here.

pub mod loaders;
