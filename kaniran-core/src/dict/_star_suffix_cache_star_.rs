//! Port of `ichiran/dict:*suffix-cache*` (`dict-grammar.lisp:5`).
//!
//! Suffix surface text → list of `(class, optional kana-form row)`
//! grammatical-suffix matches loaded for that text.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use std::collections::HashMap;

pub type SuffixCache = HashMap<String, Vec<(String, Option<KanaText>)>>;

pub fn suffix_cache(ctx: &KaniranContext) -> &SuffixCache {
    &ctx.suffix_cache
}
