//! Port of `ichiran/dict:insert-hints` (`dict-split.lisp:834-848`).
//!
//! Splice hint sentinels into a kana string at the positions named
//! by `hints`. Each `(kind, pos)` entry pushes the character looked
//! up via [`super::_star_hint_char_map_star_::HINT_CHAR_MAP`] into a
//! bucket at character index `pos` (where `pos` may equal the
//! string's character length, meaning "after the last char"). Hints
//! whose position exceeds the string length are silently dropped —
//! the Lisp guard `(<= 0 position len)` covers both polarities; in
//! Rust the lower bound is enforced by the `usize` type and the
//! upper bound is checked explicitly.
//!
//! When two or more hints land at the same position, the resulting
//! sentinels are emitted in the order the hints were supplied
//! (mirroring the upstream `push` + `reverse` pair). Returns the
//! input unchanged when `hints` is empty.

use super::_star_hint_char_map_star_::HINT_CHAR_MAP;
use super::kani::KaniHintKind;

pub fn insert_hints(s: &str, hints: &[(KaniHintKind, usize)]) -> String {
    if hints.is_empty() {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut positions: Vec<Vec<char>> = vec![Vec::new(); len + 1];
    for (kind, pos) in hints {
        if *pos > len {
            continue;
        }
        // dict-split.lisp:840 (getf *hint-char-map* character-kw)
        let ch = HINT_CHAR_MAP
            .iter()
            .find(|(k, _)| k == kind)
            .map(|(_, c)| *c)
            .expect("HINT_CHAR_MAP covers every KaniHintKind variant");
        positions[*pos].push(ch);
    }
    let mut out = String::with_capacity(s.len() + hints.len() * 3);
    for i in 0..=len {
        for ch in &positions[i] {
            out.push(*ch);
        }
        if i < len {
            out.push(chars[i]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::_star_kana_hint_mod_star_::KANA_HINT_MOD;
    use super::super::_star_kana_hint_space_star_::KANA_HINT_SPACE;
    use super::*;

    /// Empty hints is a no-op.
    #[test]
    fn empty_hints_returns_input() {
        assert_eq!(insert_hints("こんにちは", &[]), "こんにちは");
    }

    /// `:mod` at position `len-1` inserts the mod sentinel before
    /// the last character — mirrors the simple-hint rule
    /// `(:mod (- l 1))` for the `(2028920 ;; は)` group.
    #[test]
    fn mod_before_last_char() {
        let out = insert_hints("こんにちは", &[(KaniHintKind::Mod, 4)]);
        assert_eq!(out, format!("こんにち{}は", KANA_HINT_MOD));
    }

    /// Position 0 prefixes the sentinel before the entire string.
    #[test]
    fn position_zero_prefixes() {
        let out = insert_hints("は", &[(KaniHintKind::Mod, 0)]);
        assert_eq!(out, format!("{}は", KANA_HINT_MOD));
    }

    /// Position equal to length suffixes after the entire string.
    #[test]
    fn position_equal_to_length_suffixes() {
        let out = insert_hints("ab", &[(KaniHintKind::Space, 2)]);
        assert_eq!(out, format!("ab{}", KANA_HINT_SPACE));
    }

    /// Out-of-range positions are silently dropped.
    #[test]
    fn out_of_range_position_dropped() {
        assert_eq!(insert_hints("ab", &[(KaniHintKind::Mod, 5)]), "ab");
    }

    /// Multiple hints at the same position emit in the supplied
    /// order — verifies the `push` + `reverse` round-trip.
    #[test]
    fn multiple_hints_same_position_keep_supplied_order() {
        let out = insert_hints(
            "ab",
            &[(KaniHintKind::Space, 1), (KaniHintKind::Mod, 1)],
        );
        assert_eq!(
            out,
            format!("a{}{}b", KANA_HINT_SPACE, KANA_HINT_MOD)
        );
    }

    /// Mixed `:space` + `:mod` at different positions — mirrors a
    /// `def-simple-hint` body with two emits.
    #[test]
    fn mixed_kinds_at_different_positions() {
        let out = insert_hints(
            "ところへ",
            &[(KaniHintKind::Space, 3), (KaniHintKind::Mod, 3)],
        );
        assert_eq!(
            out,
            format!("ところ{}{}へ", KANA_HINT_SPACE, KANA_HINT_MOD)
        );
    }
}
