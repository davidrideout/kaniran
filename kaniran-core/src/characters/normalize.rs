//! Port of `ichiran/characters:normalize` (`characters.lisp:224-232`).
//!
//! Convert abnormal-but-Japanese-rendered ASCII (full-width digits and
//! punctuation, half-width katakana) back to plain ASCII / full-width
//! katakana, then collapse combining-mark sequences (`か゛ → が`) and —
//! outside `:kana` mode — Japanese punctuation runs (`、 → ", "`).

use super::_star_dakuten_join_star_::dakuten_join;
use super::_star_punctuation_marks_star_::PUNCTUATION_MARKS;
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

    /// Default mode normalizes full-width digit, dakuten, and Japanese comma.
    #[test]
    fn default_mode_normalizes_punctuation_and_dakuten() {
        assert_eq!(normalize("０", NormalizationContext::Default), "0");
        assert_eq!(normalize("か゛", NormalizationContext::Default), "が");
        assert_eq!(normalize("、", NormalizationContext::Default), ", ");
    }

    /// Kana mode folds half-width kana and dakuten but leaves ASCII punctuation alone.
    #[test]
    fn kana_mode_only_kana_and_dakuten() {
        assert_eq!(normalize("ｱ", NormalizationContext::Kana), "ア");
        assert_eq!(normalize("か゛", NormalizationContext::Kana), "が");
        assert_eq!(normalize("、", NormalizationContext::Kana), "、");
    }
}
