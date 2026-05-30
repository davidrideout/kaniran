//! Port of `ichiran/numbers:number-to-kana` (`numbers.lisp:122`).
//!
//! Render an integer as its Japanese reading. Pipeline:
//! integer → kanji string (via `kanji_method`) → split into number
//! groups (each ending in a strictly-larger power-of-ten than its
//! interior) → render each group via [`super::group_to_kana::group_to_kana`]
//! → either join with `separator` or return the per-group readings.
//!
//! Diverges from the Lisp on two cosmetic axes (full surface preserved):
//!
//! - **Return type is an enum.** The Lisp's `&key separator` accepts
//!   `nil` (return the un-joined list of group readings) or a char
//!   (return a single joined string). Rust encodes the same dual-shape
//!   return as [`NumberToKanaOutput`] — `Joined` for the char case,
//!   `Groups` for the `nil` case.
//! - **`kanji_method` is `impl Fn(u64) -> String`.** The Lisp
//!   `&key (kanji-method 'number-to-kanji)` accepts any 1-arg callable.
//!   `impl Fn` lets Rust callers pass either a bare fn pointer or a
//!   closure that wraps [`super::number_to_kanji::number_to_kanji`]
//!   with its three table / flag args (`DIGIT_KANJI_DEFAULT`,
//!   `POWER_KANJI`, `false`).

use crate::characters::join::join;

use super::_star_char_number_class_hash_star_::char_number_class_hash;
use super::constants::{DIGIT_TO_KANA, POWER_TO_KANA};
use super::group_to_kana::group_to_kana;
use super::kani_num_class::NumClass;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberToKanaOutput {
    Joined(String),
    Groups(Vec<String>),
}

pub fn number_to_kana(
    n: u64,
    separator: Option<char>,
    kanji_method: impl Fn(u64) -> String,
) -> NumberToKanaOutput {
    let h = char_number_class_hash();
    let mut groups: Vec<Vec<(NumClass, u8)>> = Vec::new();
    let mut cur: Vec<(NumClass, u8)> = Vec::new();
    let mut last: Option<(NumClass, u8)> = None;

    for kanji in kanji_method(n).chars() {
        let &(class, val) = h
            .get(&kanji)
            .expect("kanji_method emitted a character not in *char-number-class-hash*");
        let extend = match last {
            None => true,
            Some((last_class, last_val)) => {
                class == NumClass::P
                    && (last_class == NumClass::Jd
                        || (last_class == NumClass::P && val > last_val))
            }
        };
        if !extend {
            groups.push(std::mem::take(&mut cur));
        }
        cur.push((class, val));
        last = Some((class, val));
    }
    groups.push(cur);

    let parts: Vec<String> = groups
        .iter()
        .map(|g| group_to_kana(g, DIGIT_TO_KANA, POWER_TO_KANA))
        .collect();
    match separator {
        Some(sep) => {
            let mut sep_buf = [0u8; 4];
            let sep_str = sep.encode_utf8(&mut sep_buf);
            NumberToKanaOutput::Joined(join(sep_str, &parts))
        }
        None => NumberToKanaOutput::Groups(parts),
    }
}

#[cfg(test)]
mod tests {
    use super::super::constants::{DIGIT_KANJI_DEFAULT, POWER_KANJI};
    use super::super::number_to_kanji::number_to_kanji;
    use super::*;

    fn k(n: u64) -> String {
        number_to_kanji(n, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
    }

    fn joined(n: u64, sep: char) -> String {
        match number_to_kana(n, Some(sep), k) {
            NumberToKanaOutput::Joined(s) => s,
            NumberToKanaOutput::Groups(_) => panic!("expected Joined"),
        }
    }

    fn groups(n: u64) -> Vec<String> {
        match number_to_kana(n, None, k) {
            NumberToKanaOutput::Groups(v) => v,
            NumberToKanaOutput::Joined(_) => panic!("expected Groups"),
        }
    }

    const HINT: char = '\u{200b}'; // *kana-hint-space*

    #[test]
    fn zero() {
        assert_eq!(joined(0, ' '), "れい");
    }

    #[test]
    fn single_digit() {
        assert_eq!(joined(7, ' '), "なな");
    }

    #[test]
    fn ten_groups_to_a_single_power() {
        assert_eq!(joined(10, ' '), "じゅう");
    }

    #[test]
    fn one_two_three_four_groups() {
        // 1234 → 千 / 二百 / 三十 / 四 → せん にひゃく さんじゅう よん
        assert_eq!(joined(1234, ' '), "せん にひゃく さんじゅう よん");
    }

    #[test]
    fn applies_sandhi_inside_group() {
        // 300 → 三百 → "さんびゃく" (rendaku on ひゃく).
        assert_eq!(joined(300, ' '), "さんびゃく");
    }

    #[test]
    fn hint_separator_for_dict_counters_callsite() {
        // The upstream `dict-counters` callers pass `*kana-hint-space*`
        // (U+200B). Pin the byte sequence to guard the encoding path.
        assert_eq!(
            joined(1234, HINT),
            "せん\u{200b}にひゃく\u{200b}さんじゅう\u{200b}よん"
        );
    }

    #[test]
    fn separator_none_returns_unjoined_groups() {
        assert_eq!(
            groups(1234),
            vec![
                "せん".to_string(),
                "にひゃく".to_string(),
                "さんじゅう".to_string(),
                "よん".to_string(),
            ]
        );
    }
}
