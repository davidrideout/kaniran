//! Integer → kana reading rendering. From `numbers.lisp:82-130`.
//!
//! Pipeline: integer → kanji string (via `kanji_method`, typically
//! [`super::kanji_form::number_to_kanji`]) → split into number groups →
//! [`group_to_kana`] per group → joined or returned as a list.

use crate::characters::text_utils::join;
use crate::characters::voicing::{geminate, rendaku, Voicing};

use super::num_class::{char_number_class_hash, NumClass, DIGIT_TO_KANA, POWER_TO_KANA};

/// `num-sandhi` (`numbers.lisp:82-112`). Apply euphonic changes between
/// two adjacent number kana groups: geminate the tail of `s1`
/// (`いち → いっ`) and/or voice the head of `s2` (`ひゃく → びゃく`)
/// when the `(c1, v1, c2, v2)` tuple matches one of the upstream
/// `defmethod` specializers. Lisp 6-method dispatch collapses to a
/// single `match`; the `_` arm is the default method (plain concat).
/// `c1`/`v1` are `Option` because the upstream allows `nil` for the
/// initial "no previous group" state.
pub fn num_sandhi(
    c1: Option<NumClass>,
    v1: Option<u8>,
    c2: NumClass,
    v2: u8,
    s1: &str,
    s2: &str,
) -> String {
    use NumClass::*;
    let mut s1_buf = s1.to_string();
    let mut s2_buf = s2.to_string();
    match (c1, v1, c2, v2) {
        (Some(Jd), Some(1), P, v) if matches!(v, 3 | 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(Jd), Some(3), P, v) if matches!(v, 2 | 3) => {
            rendaku(&mut s2_buf, Voicing::Dakuten);
        }
        (Some(Jd), Some(6), P, 2) => {
            geminate(&mut s1_buf);
            rendaku(&mut s2_buf, Voicing::Handakuten);
        }
        (Some(Jd), Some(6), P, 16) => {
            geminate(&mut s1_buf);
        }
        (Some(Jd), Some(8), P, 2) => {
            geminate(&mut s1_buf);
            rendaku(&mut s2_buf, Voicing::Handakuten);
        }
        (Some(Jd), Some(8), P, v) if matches!(v, 3 | 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(P), Some(1), P, v) if matches!(v, 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(P), Some(2), P, 16) => {
            geminate(&mut s1_buf);
        }
        _ => {}
    }
    format!("{s1_buf}{s2_buf}")
}

/// `group-to-kana` (`numbers.lisp:114`). Render one number group
/// (`三百` = `(Jd 3)(P 2)`) into its hiragana reading, folding
/// adjacent pairs through [`num_sandhi`]. Lisp `&key class-to-kana`
/// plist → two explicit `&[]` tables. `Ad` is mapped to `digit_table`
/// (same shape as `Jd`); upstream crashes on `Ad` here.
pub fn group_to_kana(
    group: &[(NumClass, u8)],
    digit_table: &[&str],
    power_table: &[(u8, &str)],
) -> String {
    let mut result = String::new();
    let mut prev_class: Option<NumClass> = None;
    let mut prev_val: Option<u8> = None;
    for &(class, val) in group {
        let kana = lookup(class, val, digit_table, power_table);
        result = num_sandhi(prev_class, prev_val, class, val, &result, kana);
        prev_class = Some(class);
        prev_val = Some(val);
    }
    result
}

fn lookup<'a>(
    class: NumClass,
    val: u8,
    digit_table: &'a [&'a str],
    power_table: &'a [(u8, &'a str)],
) -> &'a str {
    match class {
        NumClass::Jd | NumClass::Ad => digit_table[val as usize],
        NumClass::P => power_table
            .iter()
            .find_map(|&(k, s)| if k == val { Some(s) } else { None })
            .expect("power_table missing entry for exponent"),
    }
}

/// `&key separator` accepts a char (joined string) or `nil` (list of
/// group readings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberToKanaOutput {
    Joined(String),
    Groups(Vec<String>),
}

/// `number-to-kana` (`numbers.lisp:122`). Render an integer as its
/// Japanese reading. `kanji_method` is `impl Fn(u64) -> String` to
/// accept either a bare fn pointer or a closure wrapping
/// [`super::kanji_form::number_to_kanji`] with its tables.
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
    use super::super::kanji_form::number_to_kanji;
    use super::super::num_class::{DIGIT_KANJI_DEFAULT, POWER_KANJI};
    use super::*;
    use NumClass::*;

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
    fn num_sandhi_no_prev_just_concatenates() {
        assert_eq!(num_sandhi(None, None, Jd, 5, "", "ご"), "ご");
    }

    #[test]
    fn num_sandhi_ichi_sen_geminates_s1() {
        assert_eq!(
            num_sandhi(Some(Jd), Some(1), P, 3, "いち", "せん"),
            "いっせん"
        );
    }

    #[test]
    fn num_sandhi_san_byaku_voices_s2() {
        assert_eq!(
            num_sandhi(Some(Jd), Some(3), P, 2, "さん", "ひゃく"),
            "さんびゃく"
        );
    }

    #[test]
    fn num_sandhi_roku_pyaku_geminates_and_handakuten() {
        assert_eq!(
            num_sandhi(Some(Jd), Some(6), P, 2, "ろく", "ひゃく"),
            "ろっぴゃく"
        );
    }

    #[test]
    fn num_sandhi_unmatched_pair_just_concatenates() {
        assert_eq!(
            num_sandhi(Some(Jd), Some(2), P, 2, "に", "ひゃく"),
            "にひゃく"
        );
    }

    fn g(group: &[(NumClass, u8)]) -> String {
        group_to_kana(group, DIGIT_TO_KANA, POWER_TO_KANA)
    }

    #[test]
    fn group_to_kana_single_digit_group() {
        assert_eq!(g(&[(Jd, 4)]), "よん");
    }

    #[test]
    fn group_to_kana_lone_power_group() {
        assert_eq!(g(&[(P, 3)]), "せん");
    }

    #[test]
    fn group_to_kana_digit_then_power_with_sandhi() {
        assert_eq!(g(&[(Jd, 3), (P, 2)]), "さんびゃく");
    }

    #[test]
    fn group_to_kana_digit_then_power_no_sandhi() {
        assert_eq!(g(&[(Jd, 2), (P, 2)]), "にひゃく");
    }

    /// Exercises the `class-to-kana` swap path.
    #[test]
    fn group_to_kana_caller_can_swap_digit_table() {
        const ALT: &[&str] = &["zero", "one", "two", "three", "four",
                               "five", "six", "seven", "eight", "nine"];
        assert_eq!(group_to_kana(&[(Jd, 5)], ALT, POWER_TO_KANA), "five");
    }

    #[test]
    fn number_to_kana_zero() {
        assert_eq!(joined(0, ' '), "れい");
    }

    #[test]
    fn number_to_kana_single_digit() {
        assert_eq!(joined(7, ' '), "なな");
    }

    #[test]
    fn number_to_kana_ten_groups_to_a_single_power() {
        assert_eq!(joined(10, ' '), "じゅう");
    }

    #[test]
    fn number_to_kana_one_two_three_four_groups() {
        assert_eq!(joined(1234, ' '), "せん にひゃく さんじゅう よん");
    }

    #[test]
    fn number_to_kana_applies_sandhi_inside_group() {
        assert_eq!(joined(300, ' '), "さんびゃく");
    }

    /// `dict-counters` callers pass `*kana-hint-space*` (U+200B). Pins
    /// the byte sequence.
    #[test]
    fn number_to_kana_hint_separator_for_dict_counters_callsite() {
        assert_eq!(
            joined(1234, HINT),
            "せん\u{200b}にひゃく\u{200b}さんじゅう\u{200b}よん"
        );
    }

    #[test]
    fn number_to_kana_separator_none_returns_unjoined_groups() {
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
