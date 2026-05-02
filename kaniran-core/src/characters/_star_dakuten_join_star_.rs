//! Port of `ichiran/characters:*dakuten-join*` (`characters.lisp:103-104`).
//!
//! Pairs of `(input-with-combining-mark, single-precomposed-char)` for
//! both dakuten (゛) and handakuten (゜) — used by `simplify-ngrams`
//! to normalize decomposed forms (`"か゛"` → `"が"`) into single code
//! points before downstream matching.
//!
//! Derived at first use by calling
//! [`super::dakuten_join::dakuten_join`] on the dakuten and handakuten
//! hashes and concatenating the results — exact upstream construction.
//! Order of pair emission is implementation-defined (depends on hash
//! iteration order); the regression test below sorts both sides before
//! comparing against the Lisp introspector's captured value.

use std::sync::OnceLock;

use super::_star_dakuten_hash_star_::dakuten_hash;
use super::_star_handakuten_hash_star_::handakuten_hash;
use super::dakuten_join::dakuten_join as build;

static CACHE: OnceLock<Vec<(String, String)>> = OnceLock::new();

pub fn dakuten_join() -> &'static Vec<(String, String)> {
    CACHE.get_or_init(|| {
        let mut v = build(dakuten_hash(), '゛');
        v.extend(build(handakuten_hash(), '゜'));
        v
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured by the introspector running
    /// `(append (dakuten-join *dakuten-hash* #\゛) (dakuten-join *handakuten-hash* #\゜))`
    /// against the upstream image. SBCL's `hash-table-alist` order is
    /// implementation-defined, so the upstream pair order itself is
    /// not stable — the comparison sorts both sides.
    static INTROSPECTED: &[(&str, &str)] = &[
        ("う゛", "ゔ"),
        ("ウ゛", "ヴ"),
        ("ほ゛", "ぼ"),
        ("ホ゛", "ボ"),
        ("へ゛", "べ"),
        ("ヘ゛", "ベ"),
        ("ふ゛", "ぶ"),
        ("フ゛", "ブ"),
        ("ひ゛", "び"),
        ("ヒ゛", "ビ"),
        ("は゛", "ば"),
        ("ハ゛", "バ"),
        ("と゛", "ど"),
        ("ト゛", "ド"),
        ("て゛", "で"),
        ("テ゛", "デ"),
        ("つ゛", "づ"),
        ("ツ゛", "ヅ"),
        ("ち゛", "ぢ"),
        ("チ゛", "ヂ"),
        ("た゛", "だ"),
        ("タ゛", "ダ"),
        ("そ゛", "ぞ"),
        ("ソ゛", "ゾ"),
        ("せ゛", "ぜ"),
        ("セ゛", "ゼ"),
        ("す゛", "ず"),
        ("ス゛", "ズ"),
        ("し゛", "じ"),
        ("シ゛", "ジ"),
        ("さ゛", "ざ"),
        ("サ゛", "ザ"),
        ("こ゛", "ご"),
        ("コ゛", "ゴ"),
        ("け゛", "げ"),
        ("ケ゛", "ゲ"),
        ("く゛", "ぐ"),
        ("ク゛", "グ"),
        ("き゛", "ぎ"),
        ("キ゛", "ギ"),
        ("か゛", "が"),
        ("カ゛", "ガ"),
        ("ほ゜", "ぽ"),
        ("ホ゜", "ポ"),
        ("へ゜", "ぺ"),
        ("ヘ゜", "ペ"),
        ("ふ゜", "ぷ"),
        ("フ゜", "プ"),
        ("ひ゜", "ぴ"),
        ("ヒ゜", "ピ"),
        ("は゜", "ぱ"),
        ("ハ゜", "パ"),
    ];

    #[test]
    fn derived_value_matches_introspected_literal_under_sort() {
        let mut derived: Vec<(&str, &str)> = dakuten_join()
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        derived.sort();
        let mut expected: Vec<(&str, &str)> = INTROSPECTED.to_vec();
        expected.sort();
        assert_eq!(derived, expected);
    }
}
