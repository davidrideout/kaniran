//! Port of `ichiran/dict:process-hints` (`dict-split.lisp:826-827`).
//!
//! Applies the hint-substitution table to a romanizer-ready kana
//! string: collapses each `(*kana-hint-mod* + は|ハ|へ|ヘ)` digram into
//! its rewritten reading and converts standalone sentinels back to
//! user-visible characters (or drops them).

use super::_star_hint_simplify_map_star_::hint_simplify_map;
use crate::characters::simplify_ngrams::simplify_ngrams;

pub fn process_hints(word: &str) -> String {
    simplify_ngrams(word, hint_simplify_map())
}

#[cfg(test)]
mod tests {
    use super::super::_star_kana_hint_mod_star_::KANA_HINT_MOD;
    use super::super::_star_kana_hint_space_star_::KANA_HINT_SPACE;
    use super::*;

    /// `*kana-hint-mod*` + `は` → `わ`. The canonical rewrite the
    /// hint system exists to enable.
    #[test]
    fn mod_plus_ha_becomes_wa() {
        let input = format!("こんにち{}は", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "こんにちわ");
    }

    /// `*kana-hint-mod*` + `へ` → `え`.
    #[test]
    fn mod_plus_he_becomes_e() {
        let input = format!("ところ{}へ", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "ところえ");
    }

    /// Katakana variants: `*kana-hint-mod*` + `ハ` → `ワ`,
    /// `*kana-hint-mod*` + `ヘ` → `エ`.
    #[test]
    fn katakana_variants() {
        let input = format!(
            "{m}ハ{m}ヘ",
            m = KANA_HINT_MOD,
        );
        assert_eq!(process_hints(&input), "ワエ");
    }

    /// `*kana-hint-space*` → ASCII space.
    #[test]
    fn space_sentinel_becomes_ascii_space() {
        let input = format!("ところ{}へ", KANA_HINT_SPACE);
        assert_eq!(process_hints(&input), "ところ へ");
    }

    /// Lone `*kana-hint-mod*` (no following は/ハ/へ/ヘ) drops.
    /// The order in [`hint_simplify_map`] ensures the 2-char rules
    /// fire first so we only fall through to the empty substitution
    /// when no digram matches.
    #[test]
    fn lone_mod_drops() {
        let input = format!("a{}b", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "ab");
    }

    /// No sentinels — pass-through.
    #[test]
    fn no_sentinels_unchanged() {
        assert_eq!(process_hints("こんにちは"), "こんにちは");
    }
}
