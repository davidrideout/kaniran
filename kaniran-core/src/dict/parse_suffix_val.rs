//! Port of `ichiran/dict:parse-suffix-val` (`dict-grammar.lisp:665`).
//!
//! Pairs `substr` with each cache entry under that substring, yielding
//! a flat list of `(substr, key, kf)` triples for downstream
//! destructuring (`(suffix keyword kf)` in `find-word-suffix` /
//! `get-suffix-map` / `get-suffixes`).
//!
//! ## Divergences from Lisp
//!
//! Upstream dispatches on `(consp (car val))` to handle two physical
//! shapes the cache can hold: a single 2-element entry `(key kf)`
//! versus a list of such entries `((key1 kf1) (key2 kf2) …)`. The Rust
//! [`SuffixCache`] port flattens both shapes into a uniform
//! `Vec<(String, Option<KanaText>)>` (the populator always wraps a new
//! match in a `vec![..]` and prepends on `join=true`), so the runtime
//! `consp` dispatch collapses to a straight iteration over the slice.
//! Both `val = nil` and `val = ()` in Lisp map to either `None` or an
//! empty slice here; both produce an empty `Vec`.
//!
//! [`SuffixCache`]: super::_star_suffix_cache_star_::SuffixCache

use super::kana_text_dao::KanaText;

pub fn parse_suffix_val<'a, 'b>(
    substr: &'a str,
    val: Option<&'b [(String, Option<KanaText>)]>,
) -> Vec<(&'a str, &'b str, Option<&'b KanaText>)> {
    match val {
        None => Vec::new(),
        Some(entries) => entries
            .iter()
            .map(|(key, kf)| (substr, key.as_str(), kf.as_ref()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::simple_text_class::SimpleText;
    use super::*;

    fn kf(seq: i32, text: &str) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.to_string(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    /// REPL: `(parse-suffix-val "abc" nil)` → `NIL`.
    #[test]
    fn nil_val_yields_empty() {
        let out = parse_suffix_val("abc", None);
        assert!(out.is_empty());
    }

    /// REPL: `(parse-suffix-val "abc" '())` → `NIL`.
    /// Empty cache vec maps to empty output.
    #[test]
    fn empty_slice_yields_empty() {
        let v: Vec<(String, Option<KanaText>)> = Vec::new();
        let out = parse_suffix_val("abc", Some(&v));
        assert!(out.is_empty());
    }

    /// REPL: `(parse-suffix-val "ねえ" '(:nai nil))` →
    /// `(("ねえ" :NAI NIL))`.
    /// Mirrors the `load-abbr` cache shape (single entry with no kf).
    #[test]
    fn single_entry_no_kf() {
        let v = vec![("nai".to_string(), None)];
        let out = parse_suffix_val("ねえ", Some(&v));
        assert_eq!(out.len(), 1);
        let (substr, key, kana_form) = out[0];
        assert_eq!(substr, "ねえ");
        assert_eq!(key, "nai");
        assert!(kana_form.is_none());
    }

    /// REPL backing: matches the populator shape from `load_kf` —
    /// single 2-tuple entry `(key, Some(kf))` produces one triple.
    /// REPL parallel: `(parse-suffix-val "abc" '(:foo bar))` →
    /// `(("abc" :FOO BAR))` (Lisp flat-entry case; Rust always wraps
    /// in a 1-elem vec, same observable result).
    #[test]
    fn single_entry_with_kf() {
        let kana_form = kf(12345, "ちゃ");
        let v = vec![("chau".to_string(), Some(kana_form))];
        let out = parse_suffix_val("ちゃ", Some(&v));
        assert_eq!(out.len(), 1);
        let (substr, key, kf_ref) = out[0];
        assert_eq!(substr, "ちゃ");
        assert_eq!(key, "chau");
        assert_eq!(kf_ref.map(|k| k.seq), Some(12345));
    }

    /// REPL: `(parse-suffix-val "abc" '((:foo 1) (:bar 2)))` →
    /// `(("abc" :FOO 1) ("abc" :BAR 2))`.
    /// Order is preserved across the slice.
    #[test]
    fn multi_entry_preserves_order() {
        let v = vec![
            ("foo".to_string(), Some(kf(1, "x"))),
            ("bar".to_string(), Some(kf(2, "y"))),
        ];
        let out = parse_suffix_val("abc", Some(&v));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "abc");
        assert_eq!(out[0].1, "foo");
        assert_eq!(out[0].2.map(|k| k.seq), Some(1));
        assert_eq!(out[1].0, "abc");
        assert_eq!(out[1].1, "bar");
        assert_eq!(out[1].2.map(|k| k.seq), Some(2));
    }
}
