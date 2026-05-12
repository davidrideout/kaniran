//! Port of `ichiran/dict:strip-hints` (`dict-split.lisp:829-830`).
//!
//! Drops every occurrence of a hint sentinel character (i.e. the
//! values held in [`super::_star_hint_char_map_star_::HINT_CHAR_MAP`])
//! from `word`. The Lisp `(remove-if (lambda (c) (find c
//! *hint-char-map*)) word)` treats the plist as a flat sequence and
//! tests `c` against every element; only the value side ever
//! collides because the keys are symbols, not characters.

use super::_star_hint_char_map_star_::HINT_CHAR_MAP;

pub fn strip_hints(word: &str) -> String {
    word.chars()
        .filter(|c| !HINT_CHAR_MAP.iter().any(|(_, hc)| hc == c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::_star_kana_hint_mod_star_::KANA_HINT_MOD;
    use super::super::_star_kana_hint_space_star_::KANA_HINT_SPACE;
    use super::*;

    /// Round-trip with [`super::super::insert_hints`]: inserting a
    /// `:mod` sentinel then stripping yields the original kana.
    #[test]
    fn strips_inserted_mod() {
        let with_hint = format!("こんにち{}は", KANA_HINT_MOD);
        assert_eq!(strip_hints(&with_hint), "こんにちは");
    }

    /// Strips both sentinels in one pass.
    #[test]
    fn strips_space_and_mod() {
        let mixed = format!("a{}b{}c{}d", KANA_HINT_SPACE, KANA_HINT_MOD, KANA_HINT_SPACE);
        assert_eq!(strip_hints(&mixed), "abcd");
    }

    /// Regular ASCII space (U+0020) is not a sentinel and stays.
    #[test]
    fn preserves_regular_space() {
        assert_eq!(strip_hints("a b c"), "a b c");
    }

    /// No sentinels — input passes through unchanged.
    #[test]
    fn no_sentinels_unchanged() {
        assert_eq!(strip_hints("こんにちは"), "こんにちは");
    }

    /// Empty input.
    #[test]
    fn empty_input() {
        assert_eq!(strip_hints(""), "");
    }
}
