//! Port of `ichiran/dict:*suffix-list*` (`dict-grammar.lisp:329`).
//!
//! ```lisp
//! (defparameter *suffix-list* nil)
//! ```
//!
//! Alist mapping a suffix keyword (the `:tag` cached alongside a
//! reading in `*suffix-cache*`) to the function that materializes
//! compound-text candidates for that suffix. Upstream is populated at
//! load time by `(defsuffix ...)` / `(def-simple-suffix ...)` /
//! `(def-abbr-suffix ...)` forms, each appending one
//! `(cons keyword fn-name)` pair via `(pushnew (cons ,key ',name)
//! *suffix-list*)`. The full Lisp table has 43 entries; this port
//! currently carries 2 — `suru` → [`suffix_suru`] and `ra` →
//! [`suffix_ra`] — and grows row-by-row as the remaining suffix bodies
//! port from the CYCLE-484 unit.
//!
//! Consumed by `find-word-suffix` (`dict-grammar.lisp:695-707`):
//! `(cdr (assoc keyword *suffix-list*))` returns the function (or
//! `nil`); the loop's `(when suffix-fn …)` gate silently drops any
//! suffix whose keyword has no entry — matching upstream behavior for
//! a suffix-list missing rows.
//!
//! ## Divergences from Lisp
//!
//! - **Sidecar `fn` pointer table.** The Lisp value is an alist of
//!   `(keyword . symbol-naming-a-fn)` pairs. The Rust mirror uses a
//!   `&'static [(&'static str, SuffixFn)]` slice where each entry's
//!   second element is a `fn` adapter that boxes the suffix-fn's
//!   returned future. Keys are the lowercase keyword strings that
//!   `*suffix-cache*` already stores (e.g. `"suru"`, `"ra"`), matching
//!   [`super::_star_suffix_cache_star_::SuffixCache`]'s value shape.
//! - **Adapters unwrap `kf`.** Both ported suffix-fn bodies — `suffix-suru`
//!   and `suffix-ra` — were ported with non-`Option` `&KanaText` for
//!   `suf`, reflecting their cache-invariant that `kf` is always a
//!   kana-text under `(load-conjs :suru …)` / `(load-kf :ra …)`
//!   (see `suffix_suru.rs` / `suffix_ra.rs` headers). The dispatch
//!   adapters take `Option<&KanaText>` to match the uniform
//!   `(funcall suffix-fn root suf kf)` shape at the find-word-suffix
//!   callsite (`dict-grammar.lisp:707`); `None` would violate the
//!   cache invariant and panics so a future cache-shape regression is
//!   visible at the dispatch boundary rather than silently producing
//!   garbage.
//!
//! ## Future suffix-fn registrations
//!
//! When porting another `defsuffix` / `def-simple-suffix` /
//! `def-abbr-suffix` body, add a row to [`SUFFIX_LIST`] keyed by the
//! lowercase keyword the upstream `defsuffix` form passes as `key`.
//! The adapter signature is fixed by [`SuffixFn`]; the wrapped fn may
//! take its `suf` parameter as `Option<&KanaText>` directly (for
//! `(def-abbr-suffix …)` bodies where the cache routinely emits `nil`
//! kf) or as `&KanaText` with a matching `kf.expect(...)` in the
//! adapter (when the cache row is guaranteed by the upstream populator).

use std::future::Future;
use std::pin::Pin;

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::CompoundText;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::suffix_ra::suffix_ra;
use crate::dict::suffix_suru::suffix_suru;

/// Dispatch signature for one entry in [`SUFFIX_LIST`]. Mirrors the
/// `(funcall suffix-fn root suf kf)` shape at
/// `dict-grammar.lisp:707`: `root` is the prefix substring being
/// treated as a verb / noun stem, `suf` is the suffix surface text
/// from the cache, and `kf` is the optional kana-text row carrying
/// that suffix (`nil` upstream for abbreviated suffixes loaded with
/// `(load-abbr …)`).
pub type SuffixFn = for<'a> fn(
    &'a KaniranContext,
    &'a str,
    &'a str,
    Option<&'a KanaText>,
) -> Pin<Box<dyn Future<Output = Result<Vec<CompoundText>, sqlx::Error>> + Send + 'a>>;

/// dict-grammar.lisp:432 (def-simple-suffix suffix-suru :suru …)
fn suffix_suru_dispatch<'a>(
    ctx: &'a KaniranContext,
    root: &'a str,
    sv: &'a str,
    kf: Option<&'a KanaText>,
) -> Pin<Box<dyn Future<Output = Result<Vec<CompoundText>, sqlx::Error>> + Send + 'a>> {
    Box::pin(async move {
        let kf = kf.expect(
            "suffix-list :suru dispatch: kf is nil; cache invariant (load-conjs :suru) broken",
        );
        suffix_suru(ctx, root, sv, kf).await
    })
}

/// dict-grammar.lisp:504 (def-simple-suffix suffix-ra :ra …)
fn suffix_ra_dispatch<'a>(
    ctx: &'a KaniranContext,
    root: &'a str,
    sv: &'a str,
    kf: Option<&'a KanaText>,
) -> Pin<Box<dyn Future<Output = Result<Vec<CompoundText>, sqlx::Error>> + Send + 'a>> {
    Box::pin(async move {
        let kf = kf.expect(
            "suffix-list :ra dispatch: kf is nil; cache invariant (load-kf :ra) broken",
        );
        suffix_ra(ctx, root, sv, kf).await
    })
}

/// Partial port of `*suffix-list*`: 2 of 43 upstream entries. Keys are
/// the lowercase keyword strings already used by the suffix cache
/// (`super::_star_suffix_cache_star_`). Linear scan via
/// [`lookup_suffix_fn`] mirrors the upstream `(assoc keyword
/// *suffix-list*)`; with N ≤ 43, the constant factor is negligible.
pub static SUFFIX_LIST: &[(&str, SuffixFn)] = &[
    ("suru", suffix_suru_dispatch),
    ("ra", suffix_ra_dispatch),
];

/// `(cdr (assoc keyword *suffix-list*))` — returns the dispatch fn for
/// `keyword`, or `None` when the keyword is absent.
pub fn lookup_suffix_fn(keyword: &str) -> Option<SuffixFn> {
    SUFFIX_LIST
        .iter()
        .find_map(|(k, f)| if *k == keyword { Some(*f) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both currently-registered entries are reachable through the
    /// `lookup_suffix_fn` linear scan.
    #[test]
    fn registered_keys_resolve() {
        assert!(lookup_suffix_fn("suru").is_some());
        assert!(lookup_suffix_fn("ra").is_some());
    }

    /// REPL: `(cdr (assoc :teiru *suffix-list*))` does return a symbol
    /// in upstream (43 entries), but the Rust port has only 2 rows —
    /// every other key is absent until the remaining suffix-fn bodies
    /// land. Pin the absence so a future row addition is visible as a
    /// test edit rather than a silent change.
    #[test]
    fn unregistered_keys_return_none() {
        assert!(lookup_suffix_fn("teiru").is_none());
        assert!(lookup_suffix_fn("te").is_none());
        assert!(lookup_suffix_fn("tai").is_none());
        assert!(lookup_suffix_fn("").is_none());
        assert!(lookup_suffix_fn("unknown").is_none());
    }

    #[test]
    fn entry_count_matches_partial_population() {
        assert_eq!(SUFFIX_LIST.len(), 2);
    }
}
