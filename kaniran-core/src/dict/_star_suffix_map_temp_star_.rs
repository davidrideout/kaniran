//! Port of `ichiran/dict:*suffix-map-temp*` (`dict.lisp:1049`).
//!
//! ```lisp
//! (defvar *suffix-map-temp* nil)
//! ```
//!
//! Caller-scoped suffix lookup cache. When bound (non-nil), it holds
//! the result of `get-suffix-map` (`dict-grammar.lisp:671`) — a
//! hash-table keyed by an end-position in the input string, valued
//! with the `(substr keyword kf)` triples that
//! [`super::find_word::find_word`]-style suffix expansion should
//! consider at that end position. While bound, `find-word-suffix`
//! (`dict-grammar.lisp:695`) short-circuits its own `get-suffixes`
//! call and reads precomputed candidates from this map instead.
//!
//! Rebound (let-bound) at:
//! - `dict.lisp:1090` — `join-substring-words*` threads the prebuilt
//!   `suffix-map` through the inner `find-word-full` call.
//! - `dict.lisp:1851` — `find-word-info` builds the map once with
//!   `(get-suffix-map text)` and threads it through `find-word-full`.
//! - `dict-grammar.lisp:348` — `def-simple-suffix` macro body: keeps
//!   the current binding when `stem = 0`, clears to nil otherwise.
//! - `dict-grammar.lisp:442`, `dict-grammar.lisp:501`,
//!   `dict-grammar.lisp:555` — explicit `nil` rebinds in
//!   `suffix-sou-base`, `suffix-garu`, and `def-abbr-suffix`.
//!
//! ## Rust shape — ctx slot, not a dynamic binding
//!
//! Per CONVENTIONS §4.10. The value carries on
//! [`crate::conn::kani_context::KaniranContext::suffix_map_temp`] as
//! `Option<Arc<SuffixMapTemp>>`; rebinding is
//! [`crate::conn::kani_context::KaniranContext::with_suffix_map_temp`]
//! returning a sibling ctx. `Arc` keeps the rebind clone cheap. The
//! upstream `(defvar … nil)` default maps to `None` in
//! [`crate::conn::kani_context::KaniranContext::from_url`].
//!
//! ## Value shape
//!
//! Hash keyed by **character** end-position (CONVENTIONS §4.5).
//! Values mirror the upstream `(substr keyword kf)` triples: the
//! suffix surface text, the suffix-class keyword, and the optional
//! [`super::kana_text_dao::KanaText`] DAO row carrying it. This is
//! [`super::_star_suffix_cache_star_::SuffixCache`]'s
//! `(class, row)` pair prefixed with the substring key, threaded into
//! per-end-position buckets by
//! [`super::_star_suffix_cache_star_::parse_suffix_val`]'s eventual
//! port (`dict-grammar.lisp:665`).

use crate::dict::kana_text_dao::KanaText;
use std::collections::HashMap;

pub type SuffixMapTemp = HashMap<usize, Vec<(String, String, Option<KanaText>)>>;
