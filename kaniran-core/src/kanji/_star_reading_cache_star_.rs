//! Port of `ichiran/kanji:*reading-cache*` (`kanji.lisp:199`).
//!
//! Per-key lazy cache mapping `(text, typeset)` to the list of
//! `(reading-text, reading-type)` pairs that
//! [`crate::kanji::get_readings_cache::get_readings_cache`] returns.
//! The upstream `defparameter` constructs an empty `:test 'equal`
//! hash-table; the cache fills one entry at a time as
//! `get-readings-cache` is called. Owned by
//! [`KaniranContext::reading_cache`], starts empty.
//!
//! Diverges from the upstream defparameter only in storage: a
//! `Mutex<HashMap<...>>` carries the per-key lazy entries instead of
//! the bare hash-table, since the cache is shared across async
//! callers and the upstream `*reading-cache*` is a process-wide
//! global. Last-write-wins on concurrent misses, mirroring the Lisp
//! `(setf (gethash key *reading-cache*) result)` semantics.

use crate::conn::kani_context::KaniranContext;
use std::collections::HashMap;
use std::sync::Mutex;

pub type ReadingCache = Mutex<HashMap<(String, Vec<String>), Vec<(String, String)>>>;

pub fn new_reading_cache() -> ReadingCache {
    Mutex::new(HashMap::new())
}

pub fn reading_cache(ctx: &KaniranContext) -> &ReadingCache {
    &ctx.reading_cache
}
