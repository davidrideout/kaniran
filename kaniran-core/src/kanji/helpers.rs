use crate::conn::kani_context::KaniranContext;
use std::collections::HashMap;
use std::sync::Mutex;

/// Port of `ichiran/kanji:*reading-cache*` (`kanji.lisp:199`).
///
/// Per-key lazy cache mapping `(text, typeset)` to the list of
/// `(reading-text, reading-type)` pairs `get-readings-cache` returns;
/// starts empty and fills one entry at a time.
pub type ReadingCache = Mutex<HashMap<(String, Vec<String>), Vec<(String, String)>>>;

pub fn new_reading_cache() -> ReadingCache {
    Mutex::new(HashMap::new())
}

pub fn reading_cache(ctx: &KaniranContext) -> &ReadingCache {
    &ctx.reading_cache
}
