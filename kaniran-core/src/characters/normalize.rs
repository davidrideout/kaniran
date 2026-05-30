//! Character/string canonicalization: half-/full-width swap, dakuten
//! folding, punctuation rewriting, hiragana↔katakana converters. From
//! `characters.lisp:210-232` and `:251-271`.

use fancy_regex::{Captures, Regex};

use super::char_classes::{
    ABNORMAL_CHARS, FULL_WIDTH_KANA, HALF_WIDTH_KANA, NORMAL_CHARS, PUNCTUATION_MARKS,
};
use super::kana_class::{all_characters, char_class_hash, KanaClass};
use super::voicing::dakuten_join;

/// `&key context` → 2-variant enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationContext {
    Default,
    Kana,
}

/// `to-normal-char` (`characters.lisp:219-222`). Map `c` through the
/// abnormal→normal table. `Default`: `*abnormal-chars*` → `*normal-chars*`
/// (full-width ASCII / half-width katakana → ASCII / full-width
/// katakana). `Kana`: `*half-width-kana*` → `*full-width-kana*` only.
/// `None` when `c` isn't in the source table.
pub fn to_normal_char(c: char, context: NormalizationContext) -> Option<char> {
    let (src, dst): (&str, &str) = match context {
        NormalizationContext::Kana => (HALF_WIDTH_KANA, FULL_WIDTH_KANA),
        NormalizationContext::Default => (ABNORMAL_CHARS, NORMAL_CHARS),
    };
    let pos = src.chars().position(|x| x == c)?;
    dst.chars().nth(pos)
}

/// `normalize` (`characters.lisp:224-232`). [`to_normal_char`] over every
/// char, then collapse combining marks (`か゛ → が`) and — in `Default`
/// only — Japanese punctuation runs (`、 → ", "`).
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

/// `simplify-ngrams` (`characters.lisp:210-217`). Replace every `from`
/// key in `s` with its `to` value, leftmost-first on ties. Lisp builds
/// the matcher from a flat plist; Rust takes paired slices generic over
/// `AsRef<str>`. Keys are `fancy_regex::escape`'d to match cl-ppcre's
/// literal-string semantics. Fresh regex per call — caller-driven maps
/// have unbounded cardinality.
pub fn simplify_ngrams<S, T>(s: &str, map: &[(S, T)]) -> String
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    if map.is_empty() {
        return s.to_string();
    }
    let pattern: String = map
        .iter()
        .map(|(k, _)| fancy_regex::escape(k.as_ref()).into_owned())
        .collect::<Vec<_>>()
        .join("|");
    let re = Regex::new(&pattern).expect("simplify-ngrams alternation compiles");
    re.replace_all(s, |caps: &Captures| -> String {
        let m = caps.get(0).expect("alternation always has group 0").as_str();
        map.iter()
            .find(|(k, _)| k.as_ref() == m)
            .map(|(_, v)| v.as_ref().to_string())
            .unwrap_or_default()
    })
    .into_owned()
}

/// `as-hiragana` (`characters.lisp:251-260`). Katakana → hiragana,
/// non-kana passes through. Each char goes through [`to_normal_char`]
/// first (default context), then `*char-class-hash*`, then back to the
/// first glyph (hiragana side) of its class's `*all-characters*` entry.
pub fn as_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| {
            let c = to_normal_char(c, NormalizationContext::Default).unwrap_or(c);
            match char_class_hash().get(&c) {
                Some(&class) => glyph_for(class, GlyphSide::First),
                None => c,
            }
        })
        .collect()
}

/// `as-katakana` (`characters.lisp:262-271`). Mirror of [`as_hiragana`]
/// using the last glyph (katakana side) of each class entry.
pub fn as_katakana(s: &str) -> String {
    s.chars()
        .map(|c| {
            let c = to_normal_char(c, NormalizationContext::Default).unwrap_or(c);
            match char_class_hash().get(&c) {
                Some(&class) => glyph_for(class, GlyphSide::Last),
                None => c,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
enum GlyphSide {
    First,
    Last,
}

fn glyph_for(class: KanaClass, side: GlyphSide) -> char {
    let s = all_characters()
        .iter()
        .find(|(k, _)| *k == class)
        .expect("class from char-class-hash must be in all-characters")
        .1;
    match side {
        GlyphSide::First => s.chars().next(),
        GlyphSide::Last => s.chars().last(),
    }
    .expect("class string is non-empty")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_normalizes_punctuation_and_dakuten() {
        assert_eq!(normalize("０", NormalizationContext::Default), "0");
        assert_eq!(normalize("か゛", NormalizationContext::Default), "が");
        assert_eq!(normalize("、", NormalizationContext::Default), ", ");
    }

    #[test]
    fn kana_mode_only_kana_and_dakuten() {
        assert_eq!(normalize("ｱ", NormalizationContext::Kana), "ア");
        assert_eq!(normalize("か゛", NormalizationContext::Kana), "が");
        assert_eq!(normalize("、", NormalizationContext::Kana), "、");
    }

    #[test]
    fn folds_combining_dakuten_via_runtime_map() {
        assert_eq!(simplify_ngrams("か゛", dakuten_join()), "が");
        assert_eq!(simplify_ngrams("ハ゜", dakuten_join()), "パ");
    }

    #[test]
    fn empty_map_returns_input_unchanged() {
        let map: &[(&str, &str)] = &[];
        assert_eq!(simplify_ngrams("hello", map), "hello");
    }

    #[test]
    fn katakana_becomes_hiragana_kanji_passes_through() {
        assert_eq!(as_hiragana("ア"), "あ");
        assert_eq!(as_hiragana("カタカナ"), "かたかな");
        assert_eq!(as_hiragana("日本ア"), "日本あ");
    }
}
