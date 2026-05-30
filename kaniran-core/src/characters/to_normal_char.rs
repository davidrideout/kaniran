//! Port of `ichiran/characters:to-normal-char` (`characters.lisp:219-222`).
//!
//! Map a single character through the abnormal→normal substitution
//! tables. With [`NormalizationContext::Default`], the source/target
//! pair is `*abnormal-chars*` → `*normal-chars*` (full-width ASCII /
//! half-width katakana → standard ASCII / full-width katakana). With
//! [`NormalizationContext::Kana`], it's `*half-width-kana*` →
//! `*full-width-kana*` only — used by callers that want to normalize
//! half-width katakana but leave ASCII decorations alone.
//!
//! Returns `None` when the input is not in the relevant source table,
//! mirroring the Lisp's `(when pos ...)` semantics. The `context`
//! parameter replaces the upstream `&key context` keyword (only ever
//! `:kana` or absent); per §4.4 of `CONVENTIONS.md`, an enum is
//! preferred to a `bool` so call sites read clearly without consulting
//! the function signature.

use super::_star_normal_chars_star_::normal_chars;
use super::constants::{ABNORMAL_CHARS, FULL_WIDTH_KANA, HALF_WIDTH_KANA};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationContext {
    Default,
    Kana,
}

pub fn to_normal_char(c: char, context: NormalizationContext) -> Option<char> {
    let (src, dst): (&str, &str) = match context {
        NormalizationContext::Kana => (HALF_WIDTH_KANA, FULL_WIDTH_KANA),
        NormalizationContext::Default => (ABNORMAL_CHARS, normal_chars()),
    };
    let pos = src.chars().position(|x| x == c)?;
    dst.chars().nth(pos)
}
