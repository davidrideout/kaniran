//! Rust-only scaffolding for the `ichiran/dict` port. No Lisp FQNs.
//! Three section files mirror the upstream's slot in the port: word /
//! type dispatchers ([`word`]), lite mirrors of the segmentation
//! structs used inside `find-best-path` ([`segment`]), and the runtime
//! engines that interpret the data-row registries written by
//! `def-special-counter` / `def-easy-hint` / `def-simple-split`
//! ([`engines`]).
//!
//! Re-exports flatten every submodule symbol so callers reach them as
//! `crate::dict::kani::<Symbol>`; the per-section module paths are
//! also visible for callers that want to namespace.

pub mod engines;
pub mod segment;
pub mod word;

pub use engines::*;
pub use segment::*;
pub use word::*;
