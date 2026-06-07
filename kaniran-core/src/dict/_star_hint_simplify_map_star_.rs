//! Port of `ichiran/dict:*hint-simplify-map*` (`dict-split.lisp:818-824`).
//!
//! Ordered (from, to) substitution table that folds the hint
//! sentinels back into reader-facing characters:
//!
//! - `*kana-hint-space*` → ASCII space `" "`
//! - `*kana-hint-mod*` + `は` → `わ`  (and `ハ` → `ワ`)
//! - `*kana-hint-mod*` + `へ` → `え`  (and `ヘ` → `エ`)
//! - lone `*kana-hint-mod*` → empty string (drop)
//!
//! Order matters: the 2-char sentinel+kana entries must precede the
//! lone-sentinel entry so the longer match wins at the same offset.

use std::sync::OnceLock;

use super::_star_kana_hint_mod_star_::KANA_HINT_MOD;
use super::_star_kana_hint_space_star_::KANA_HINT_SPACE;

pub fn hint_simplify_map() -> &'static [(String, &'static str)] {
    static CACHE: OnceLock<Vec<(String, &'static str)>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut map: Vec<(String, &'static str)> = Vec::with_capacity(6);
            map.push((KANA_HINT_SPACE.to_string(), " "));
            map.push(([KANA_HINT_MOD, 'は'].iter().collect(), "わ"));
            map.push(([KANA_HINT_MOD, 'ハ'].iter().collect(), "ワ"));
            map.push(([KANA_HINT_MOD, 'へ'].iter().collect(), "え"));
            map.push(([KANA_HINT_MOD, 'ヘ'].iter().collect(), "エ"));
            map.push((KANA_HINT_MOD.to_string(), ""));
            map
        })
        .as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the build output against the introspected upstream value —
    /// catches drift in the source character constants.
    #[test]
    fn matches_introspected_value() {
        let map = hint_simplify_map();
        assert_eq!(map.len(), 6);
        assert_eq!(map[0], ("\u{200b}".to_string(), " "));
        assert_eq!(map[1], ("\u{200c}は".to_string(), "わ"));
        assert_eq!(map[2], ("\u{200c}ハ".to_string(), "ワ"));
        assert_eq!(map[3], ("\u{200c}へ".to_string(), "え"));
        assert_eq!(map[4], ("\u{200c}ヘ".to_string(), "エ"));
        assert_eq!(map[5], ("\u{200c}".to_string(), ""));
    }
}
