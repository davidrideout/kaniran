//! Port of `ichiran/dict:*kana-hint-space*` (`dict-split.lisp:814`).
//!
//! Sentinel character marking hint-injected spaces in kana strings.
//! Used by `*hint-char-map*` and `*hint-simplify-map*` to distinguish
//! hint-introduced separators from real spaces in the source text.

pub const KANA_HINT_SPACE: char = '\u{200b}';
