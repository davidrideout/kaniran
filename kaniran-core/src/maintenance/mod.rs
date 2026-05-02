//! Port of `ichiran/maintenance` — utilities for diffing and updating
//! the JMdict-backed entry corpus. Currently only the DB-free leaves
//! are landed; the rest of the package waits on the open DB-layer
//! decision (see `reverse/scripts/HANDOFF.md`).

pub mod diff_content;
