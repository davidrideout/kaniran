//! Port of `ichiran/dict:find-counter` (`dict-counters.lisp:273`).
//!
//! Looks up the recipes registered for `counter` in the counter
//! cache, materializes a [`Counter`] from each recipe with the given
//! `number` text, and keeps the ones [`verify`] accepts. Drops recipes
//! whose [`Counter::new`] raises `NotANumber`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_counter_cache_star_::counter_cache;
use crate::dict::counter_text_class::Counter;
use crate::dict::verify::verify;

pub fn find_counter(
    ctx: &KaniranContext,
    number: &str,
    counter: &str,
    unique: Option<bool>,
) -> Vec<Counter> {
    // dict-counters.lisp:273 — `&key (unique t)`. `None` here means
    // "caller didn't supply :UNIQUE", which Lisp resolves to `t`.
    let unique = unique.unwrap_or(true);
    let Some(args_list) = counter_cache(ctx).get(counter) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(args_list.len());
    for args in args_list {
        match Counter::new(args, number) {
            Ok(c) if verify(&c, unique) => out.push(c),
            _ => {}
        }
    }
    out
}
