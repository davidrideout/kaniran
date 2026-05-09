//! Port of `ichiran/dict:find-counter` (`dict-counters.lisp:273`).
//!
//! Looks up the recipes registered for `counter` in the counter
//! cache, materializes a [`Counter`] from each recipe with the given
//! `number` text, and keeps the ones [`verify`] accepts. Drops
//! recipes whose [`Counter::new`] raises `NotANumber` (mirrors the
//! upstream `(handler-case ... (not-a-number () nil))` at line 279).
//!
//! Diverges from the upstream lambda list
//! `(number counter &key (unique t))` only by taking
//! `&KaniranContext` as the first parameter, replacing Lisp's
//! dynamic `*connection*` and the `(ensure :counters)` cache
//! lookup with explicit field access on the context per
//! [`crate::conn::kani_context`] (§4.8).
//!
//! `unique` is preserved as `Option<bool>` so the upstream
//! "absent → defaults to `t`" shape lives at the function boundary,
//! not at every call site: `find_counter(ctx, num, ctr, None)`
//! mirrors `(find-counter num ctr)`, while `Some(true)` / `Some(false)`
//! mirror `:unique t` / `:unique nil`. Without this, callers ported
//! against the bare `bool` form would silently lose the default-true
//! contract on omission.
//!
//! The cache itself is populated eagerly during context construction;
//! at call time this fn is a pure HashMap lookup + per-recipe
//! construct + verify.

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
