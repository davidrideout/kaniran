//! Port of `ichiran/characters:normalize` (`characters.lisp:224-232`).
//!
//! Convert abnormal-but-Japanese-rendered ASCII (full-width digits and
//! punctuation, half-width katakana) back to plain ASCII / full-width
//! katakana via [`super::to_normal_char::to_normal_char`], then collapse
//! combining-mark sequences (`か゛ → が`) and — outside `:kana` mode —
//! Japanese punctuation runs (`、 → ", "`).
//!
//! With [`NormalizationContext::Kana`] only the half-width-kana
//! substitution and the dakuten-join folding run, leaving ASCII
//! punctuation alone. With [`NormalizationContext::Default`] everything
//! gets normalized.
//!
//! Per CONVENTIONS §4.6 the upstream's in-place `setf` mutation is
//! replaced by always allocating a new `String`. The keyword `&key
//! context` becomes a required [`NormalizationContext`] argument; pass
//! `Default` at call sites that previously omitted the keyword.

use super::voicing_tables::dakuten_join;
use super::constants::PUNCTUATION_MARKS;
use super::simplify_ngrams::simplify_ngrams;
use super::to_normal_char::{to_normal_char, NormalizationContext};

pub fn normalize(s: &str, context: NormalizationContext) -> String {
    let phase1: String = s
        .chars()
        .map(|c| to_normal_char(c, context).unwrap_or(c))
        .collect();
    match context {
        NormalizationContext::Kana => simplify_ngrams(&phase1, dakuten_join()),
        NormalizationContext::Default => {
            let combined: Vec<(&str, &str)> = PUNCTUATION_MARKS
                .iter()
                .copied()
                .chain(dakuten_join().iter().map(|(a, b)| (a.as_str(), b.as_str())))
                .collect();
            simplify_ngrams(&phase1, &combined)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default mode: full-width digit normalizes to ASCII; combining
    /// dakuten folds into a precomposed glyph; Japanese comma rewrites.
    #[test]
    fn default_mode_normalizes_punctuation_and_dakuten() {
        assert_eq!(normalize("０", NormalizationContext::Default), "0");
        assert_eq!(normalize("か゛", NormalizationContext::Default), "が");
        assert_eq!(normalize("、", NormalizationContext::Default), ", ");
    }

    /// Kana mode: half-width kana → full-width, dakuten still folds,
    /// but ASCII-style punctuation is left alone.
    #[test]
    fn kana_mode_only_kana_and_dakuten() {
        assert_eq!(normalize("ｱ", NormalizationContext::Kana), "ア");
        assert_eq!(normalize("か゛", NormalizationContext::Kana), "が");
        assert_eq!(normalize("、", NormalizationContext::Kana), "、");
    }
}
