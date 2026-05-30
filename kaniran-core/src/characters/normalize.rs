//! Character/string canonicalization: half-/full-width swap, dakuten
//! folding, punctuation rewriting, and the hiragana↔katakana view
//! converters. From `characters.lisp:210-232` (simplify / to-normal /
//! normalize) and `:251-271` (as-hiragana / as-katakana).

use fancy_regex::{Captures, Regex};

use super::char_classes::{
    ABNORMAL_CHARS, FULL_WIDTH_KANA, HALF_WIDTH_KANA, NORMAL_CHARS, PUNCTUATION_MARKS,
};
use super::kana_class::{all_characters, char_class_hash, KanaClass};
use super::voicing::dakuten_join;

/// Selector for the abnormal→normal substitution table used by
/// [`to_normal_char`] / [`normalize`]. Replaces the upstream `&key
/// context` keyword (only ever `:kana` or absent); per CONVENTIONS §4.4
/// an enum is preferred to a `bool` so call sites read clearly without
/// consulting the function signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationContext {
    Default,
    Kana,
}

/// `to-normal-char` (`characters.lisp:219-222`). Map a single character
/// through the abnormal→normal substitution tables. With
/// [`NormalizationContext::Default`], the source/target pair is
/// `*abnormal-chars*` → `*normal-chars*` (full-width ASCII / half-width
/// katakana → standard ASCII / full-width katakana). With
/// [`NormalizationContext::Kana`], it's `*half-width-kana*` →
/// `*full-width-kana*` only — used by callers that want to normalize
/// half-width katakana but leave ASCII decorations alone.
///
/// Returns `None` when the input is not in the relevant source table,
/// mirroring the Lisp's `(when pos ...)` semantics.
pub fn to_normal_char(c: char, context: NormalizationContext) -> Option<char> {
    let (src, dst): (&str, &str) = match context {
        NormalizationContext::Kana => (HALF_WIDTH_KANA, FULL_WIDTH_KANA),
        NormalizationContext::Default => (ABNORMAL_CHARS, NORMAL_CHARS),
    };
    let pos = src.chars().position(|x| x == c)?;
    dst.chars().nth(pos)
}

/// `normalize` (`characters.lisp:224-232`). Convert abnormal-but-
/// Japanese-rendered ASCII (full-width digits and punctuation,
/// half-width katakana) back to plain ASCII / full-width katakana via
/// [`to_normal_char`], then collapse combining-mark sequences
/// (`か゛ → が`) and — outside `:kana` mode — Japanese punctuation runs
/// (`、 → ", "`).
///
/// With [`NormalizationContext::Kana`] only the half-width-kana
/// substitution and the dakuten-join folding run, leaving ASCII
/// punctuation alone. With [`NormalizationContext::Default`] everything
/// gets normalized.
///
/// Per CONVENTIONS §4.6 the upstream's in-place `setf` mutation is
/// replaced by always allocating a new `String`. The keyword `&key
/// context` becomes a required [`NormalizationContext`] argument; pass
/// `Default` at call sites that previously omitted the keyword.
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

/// `simplify-ngrams` (`characters.lisp:210-217`). Replace every
/// occurrence of a `from` key in `s` with its `to` value, choosing the
/// leftmost-first match when several keys could match at the same
/// position. Used to fold combining-mark sequences into single
/// precomposed glyphs (`"か゛" → "が"`) and to ASCII-ize Japanese
/// punctuation.
///
/// The Lisp builds the matcher from a flat plist of alternating
/// `from`/`to`; the Rust port takes a paired `&[(S, T)]` slice generic
/// over `AsRef<str>` so both static `&[(&str, &str)]` (e.g.
/// [`super::char_classes::PUNCTUATION_MARKS`]) and the runtime
/// `Vec<(String, String)>` from [`super::voicing::dakuten_join`] work
/// without conversion. Keys are passed through `fancy_regex::escape`
/// before alternation — cl-ppcre's parse-tree DSL already treats string
/// elements as literal character sequences, so escaping is how Rust
/// preserves that semantics across the engine boundary, not a safety
/// margin. Both implementations match keys literally regardless of
/// regex metacharacters in the data.
///
/// A new regex is compiled per call — caller-driven maps have unbounded
/// cardinality, so caching by map identity wouldn't be sound.
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

/// `as-hiragana` (`characters.lisp:251-260`). Convert any katakana in
/// `s` to its hiragana counterpart, leaving non-kana characters as-is.
/// Each char is first run through [`to_normal_char`] (default context)
/// to normalize half/full-width forms; the result is then looked up in
/// `*char-class-hash*` and replaced with the hiragana glyph for that
/// class (the first char of the class's `*all-characters*` string).
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

/// `as-katakana` (`characters.lisp:262-271`). Mirror of [`as_hiragana`]:
/// each kana char is replaced with the *last* glyph of its class string
/// in `*all-characters*` (each pair is hiragana-then-katakana, so the
/// last char is katakana). Non-kana characters pass through unchanged.
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

    /// The runtime-derived `dakuten_join()` map feeds into
    /// `simplify_ngrams` correctly — `"か゛"` (ka + combining dakuten)
    /// folds to `"が"`. Pins the *integration* between the two ports,
    /// not the contents of either.
    #[test]
    fn folds_combining_dakuten_via_runtime_map() {
        assert_eq!(simplify_ngrams("か゛", dakuten_join()), "が");
        assert_eq!(simplify_ngrams("ハ゜", dakuten_join()), "パ");
    }

    /// Empty map is a no-op.
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
