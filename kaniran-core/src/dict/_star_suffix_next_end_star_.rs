//! Port of `ichiran/dict:*suffix-next-end*` (`dict.lisp:1050`).
//!
//! ```lisp
//! (defvar *suffix-next-end* nil)
//! ```
//!
//! Caller-scoped current end-position used as the lookup key into
//! [`super::_star_suffix_map_temp_star_::SuffixMapTemp`]. While
//! `*suffix-map-temp*` is bound, `find-word-suffix`
//! (`dict-grammar.lisp:696-697`) reads
//! `(gethash *suffix-next-end* *suffix-map-temp*)` to retrieve the
//! suffix candidates ending at this position rather than recomputing
//! them via `get-suffixes`.
//!
//! Rebound (let-bound) at:
//! - `dict.lisp:1091` — `join-substring-words*` sets it to the inner
//!   segment's `end` alongside the suffix map.
//! - `dict.lisp:1852` — `find-word-info` sets it to the full-string
//!   `end` after constructing the suffix map.
//! - `dict-grammar.lisp:706` — `find-word-suffix` recurses with
//!   `(- *suffix-next-end* (length suffix))` (or `nil` when the
//!   current value is `nil`), tracking the unconsumed-prefix end as
//!   suffix peeling proceeds.
//!
//! ## Rust shape — ctx slot, not a dynamic binding
//!
//! Per CONVENTIONS §4.10. Stored on
//! [`crate::conn::kani_context::KaniranContext::suffix_next_end`] as
//! `Option<i32>`; rebinding is
//! [`crate::conn::kani_context::KaniranContext::with_suffix_next_end`]
//! returning a sibling ctx. The upstream `(defvar … nil)` default
//! maps to `None` in
//! [`crate::conn::kani_context::KaniranContext::from_url`].
//!
//! **Signed.** Position semantics are character offsets per
//! CONVENTIONS §4.5, but the `find-word-suffix` recursion at
//! `dict-grammar.lisp:706` evaluates
//! `(- *suffix-next-end* (length suffix))` and produces negative
//! intermediates whenever the unconsumed prefix shortens past zero.
//! CL fixnums hold this without complaint; `usize` panics on
//! underflow. The map lookup (`gethash` upstream) returns nil for any
//! negative key — so the [`SuffixMapTemp`] map keys stay `usize`
//! (always populated by `get-suffix-map` with valid positions) and
//! the consumer port converts the lookup key with
//! `usize::try_from(end).ok().and_then(|k| map.get(&k))`, dropping
//! negative-binding rows the same way `gethash` does.
