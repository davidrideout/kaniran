use super::super::constants::DIGIT_KANJI_DEFAULT;
use super::super::constants::DIGIT_TO_KANA;
use super::super::constants::POWER_KANJI;
use super::super::constants::POWER_TO_KANA;
use super::super::kanji_form::number_to_kanji;
use super::*;
use NumClass::*;

// --- num_sandhi ---
#[test]
fn no_prev_just_concatenates() {
    assert_eq!(num_sandhi(None, None, Jd, 5, "", "ご"), "ご");
}

#[test]
fn ichi_sen_geminates_s1() {
    // (Jd 1) + (P 3) → 1+千 → s1 "いち" → "いっ", s2 "せん" unchanged.
    assert_eq!(
        num_sandhi(Some(Jd), Some(1), P, 3, "いち", "せん"),
        "いっせん"
    );
}

#[test]
fn san_byaku_voices_s2() {
    // (Jd 3) + (P 2) → 3+百 → s2 "ひゃく" → "びゃく".
    assert_eq!(
        num_sandhi(Some(Jd), Some(3), P, 2, "さん", "ひゃく"),
        "さんびゃく"
    );
}

#[test]
fn roku_pyaku_geminates_and_handakuten() {
    // (Jd 6) + (P 2) → 6+百 → "ろく" → "ろっ", "ひゃく" → "ぴゃく".
    assert_eq!(
        num_sandhi(Some(Jd), Some(6), P, 2, "ろく", "ひゃく"),
        "ろっぴゃく"
    );
}

#[test]
fn unmatched_pair_just_concatenates() {
    assert_eq!(
        num_sandhi(Some(Jd), Some(2), P, 2, "に", "ひゃく"),
        "にひゃく"
    );
}

// --- group_to_kana ---
fn g(group: &[(NumClass, u8)]) -> String {
    group_to_kana(group, DIGIT_TO_KANA, POWER_TO_KANA)
}

#[test]
fn single_digit_group() {
    assert_eq!(g(&[(Jd, 4)]), "よん");
}

#[test]
fn lone_power_group() {
    assert_eq!(g(&[(P, 3)]), "せん");
}

#[test]
fn digit_then_power_with_sandhi() {
    // [(Jd 3)(P 2)] = 三百 → "さん" + rendaku("ひゃく") = "さんびゃく"
    assert_eq!(g(&[(Jd, 3), (P, 2)]), "さんびゃく");
}

#[test]
fn digit_then_power_no_sandhi() {
    // [(Jd 2)(P 2)] = 二百 → "に" + "ひゃく" (no transformation)
    assert_eq!(g(&[(Jd, 2), (P, 2)]), "にひゃく");
}

#[test]
fn caller_can_swap_digit_table() {
    // Stand-in alt table with different readings — exercises the
    // `class-to-kana` swap path that the Lisp keyword enables.
    const ALT: &[&str] = &[
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    assert_eq!(group_to_kana(&[(Jd, 5)], ALT, POWER_TO_KANA), "five");
}

// --- number_to_kana ---
fn k(n: u128) -> String {
    number_to_kanji(n, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
}

fn joined(n: u128, sep: char) -> String {
    match number_to_kana(n, Some(sep), k) {
        NumberToKanaOutput::Joined(s) => s,
        NumberToKanaOutput::Groups(_) => panic!("expected Joined"),
    }
}

fn groups(n: u128) -> Vec<String> {
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

// --- values above u64 ---
// Corpus row idxs=[664844]: 「…その半減期は…(≒ 1700京〜2100京年)である。」
#[test]
fn kana_above_u64() {
    // (ichiran/numbers:number-to-kana 21000000000000000000)
    //   => にせん ひゃっけい  — the 2100京 of the corpus sentence,
    // romanized "nisen hyakkeinen" with the 年 counter appended.
    assert_eq!(joined(21_000_000_000_000_000_000, ' '), "にせん ひゃっけい");
}

#[test]
fn kana_stacked_kei_above_ten_to_the_32() {
    // (ichiran/numbers:number-to-kana 1234567890123456780000000000000000)
    // Values ≥ 10^32 read 京 twice; reachable through the counter gate
    // at 17-19 digits + 京.
    assert_eq!(
        joined(1_234_567_890_123_456_780_000_000_000_000_000, ' '),
        "じゅう にけい さんぜん よんひゃく ごじゅう ろくちょう ななせん はっぴゃく きゅうじゅうおく いっせん にひゃく さんじゅう よんまん ごせん ろっぴゃく ななじゅう はっけい"
    );
}

#[test]
fn kana_largest_gated_value() {
    // (ichiran/numbers:number-to-kana 99999999999999999990000000000000000)
    // = 9999999999999999999京, the largest value the segmenter's
    // 20-char counter gate admits.
    assert_eq!(
        joined(99_999_999_999_999_999_990_000_000_000_000_000, ' '),
        "きゅうひゃく きゅうじゅう きゅうけい きゅうせん きゅうひゃく きゅうじゅう きゅうちょう きゅうせん きゅうひゃく きゅうじゅう きゅうおく きゅうせん きゅうひゃく きゅうじゅう きゅうまん きゅうせん きゅうひゃく きゅうじゅう きゅうけい"
    );
}
