//! Port of `ichiran/dict:*suffix-class*` (`dict-grammar.lisp:6`).
//!
//! Per-seq registry of which suffix class a JMdict entry belongs to —
//! "seq → class" per the upstream comment. Keys are JMdict seqs (Lisp
//! `eql` test); values are the class keyword carried by the entry's
//! suffix-cache rows (`:teiru`, `:te`, `:iru`, `:ha`, …).
//!
//! Consumed by `find-word-with-suffix` /
//! `find-word-suffix` / `get-suffix-description` to resolve a word's
//! class without re-scanning the suffix table, and populated by
//! [`super::init_suffixes_thread::build_suffix_caches`] (wave 126)
//! via the `load-kf` helper (`(setf (gethash (seq kf) *suffix-class*)
//! (or class key))`).
//!
//! Class values are carried as `String`; collapse to a closed enum
//! waits on the `def-simple-suffix` callsite ports. Owned as a plain
//! field on [`KaniranContext`]; accessor returns a borrowed reference.
//!
//! [`KaniranContext`]: crate::conn::kani_context::KaniranContext

use crate::conn::kani_context::KaniranContext;
use std::collections::HashMap;

pub type SuffixClass = HashMap<i32, String>;

pub fn suffix_class(ctx: &KaniranContext) -> &SuffixClass {
    &ctx.suffix_class
}
