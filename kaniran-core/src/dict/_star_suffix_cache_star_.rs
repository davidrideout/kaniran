//! Port of `ichiran/dict:*suffix-cache*` (`dict-grammar.lisp:5`).
//!
//! Per-text registry of grammatical-suffix matches. Keys are the
//! surface text of a suffix (Lisp `equal` test); each value is the
//! list of `(class_keyword, optional kana-form row)` matches the
//! populator has loaded for that text. A length-1 vec is the single-
//! match case (Lisp `(list key kf)`); longer vecs are the join case
//! (Lisp `(cons new old)` / `(list new old)` chains assembled inside
//! `update-suffix-cache`).
//!
//! Populated eagerly during [`KaniranContext::from_url`] by
//! [`super::init_suffixes_thread::build_suffix_caches`] (wave 126).
//! Owned as a plain field on [`KaniranContext`]; accessor returns
//! a borrowed reference for read-only callers.
//!
//! ## Value-list shape
//!
//! - `class_keyword` is the Lisp suffix class (`:teiru`, `:te`,
//!   `:iru`, `:ha`, `:chau`, …). Carried as `String`; the closed enum
//!   collapse waits on the `def-simple-suffix` callsite ports.
//! - The optional kana-form row references a [`KanaText`] DAO row.
//!   `None` corresponds to the `load-abbr` callsite shape `(list key
//!   nil)` (abbreviated forms with no source row).
//!
//! [`KaniranContext`]: crate::conn::kani_context::KaniranContext
//! [`KaniranContext::from_url`]: crate::conn::kani_context::KaniranContext::from_url

use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use std::collections::HashMap;

pub type SuffixCache = HashMap<String, Vec<(String, Option<KanaText>)>>;

pub fn suffix_cache(ctx: &KaniranContext) -> &SuffixCache {
    &ctx.suffix_cache
}
