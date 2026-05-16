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
//! ## Divergence: keyword-string shape
//!
//! Upstream stores Common Lisp keywords (`:TEIRU`, `:NEG`, `:TAI` —
//! `:`-prefixed, SCREAMING-CASE symbol-equivalents) as the first slot
//! of each cache tuple. The Rust port stores **lowercase, no-colon
//! strings** (`"teiru"`, `"neg"`, `"tai"`) — the shape produced by
//! the populator's literal `&str` arguments to `load_kf` / `load_conjs`
//! / `load_abbr` in [`super::init_suffixes_thread`].
//!
//! Captured fixtures encode the upstream `:TEIRU` shape verbatim. The
//! `get_suffixes` audit runners (`audit/dict/get_suffixes_test.rs`,
//! `audit/dict/get_suffixes_dump.rs`) bridge by stripping the leading
//! `:` and lowercasing the captured value before comparison. This is
//! a runner-local adapter, **not** a runtime conversion in the port —
//! Rust never sees `:TEIRU`-shaped strings.
//!
//! The choice pins the eventual enum collapse: when the
//! `def-simple-suffix` callsites land and `class_keyword` becomes a
//! closed enum (e.g. `enum SuffixClass { Teiru, Neg, Tai, … }`), the
//! variants will derive from these lowercase strings, not the Lisp
//! `:UPPER` form. If parity rules later require preserving the
//! `:UPPER` representation through to the cache (e.g. for symbol-
//! identity downstream comparisons), this divergence must be reverted
//! and the audit normalization removed.
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
