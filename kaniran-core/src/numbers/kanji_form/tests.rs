use super::super::constants::DIGIT_KANJI_DEFAULT;
use super::super::constants::POWER_KANJI;
use super::*;
use NumClass::*;

// --- number_to_kanji ---
fn n2k(n: u128) -> String {
    number_to_kanji(n, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
}

#[test]
fn zero_is_single_glyph() {
    assert_eq!(n2k(0), "〇");
}

#[test]
fn single_digit_passes_through() {
    assert_eq!(n2k(7), "七");
}

#[test]
fn ten_omits_leading_one() {
    assert_eq!(n2k(10), "十");
}

#[test]
fn one_thousand_omits_leading_one_at_top_level() {
    // mp=1000=千, qt=1, threshold (one_sen=false) = 1000, mp ≤ threshold → head="".
    assert_eq!(n2k(1000), "千");
}

#[test]
fn ten_thousand_keeps_leading_one() {
    // mp=10000=万 > threshold 1000 → head = recurse(1, one_sen=true) = "一".
    assert_eq!(n2k(10000), "一万");
}

#[test]
fn nested_one_sen_keeps_leading_one_for_sen() {
    // 11000 → mp=万, qt=1, head=recurse(1,one_sen=true)="一"; rem=1000;
    // tail=recurse(1000,one_sen=false)="千" (threshold 1000, mp=1000 ≤ threshold).
    assert_eq!(n2k(11000), "一万千");
}

#[test]
fn full_decomposition_1234() {
    assert_eq!(n2k(1234), "千二百三十四");
}

#[test]
fn matches_lisp_for_multi_kanji_inputs() {
    // Pinned against `(ichiran/numbers:number-to-kanji N)` on the
    // reference Lisp install — same inputs as `parse-number-test`
    // (`tests.lisp:640-644`), but pinning the kanji form, not just
    // the roundtrip.
    assert_eq!(n2k(10001), "一万一");
    assert_eq!(n2k(20020001), "二千二万一");
    assert_eq!(n2k(12_423_000_430), "百二十四億二千三百万四百三十");
}

#[test]
fn legal_digits_swap_in() {
    // 123 → 百 + 弐 + 十 + 参. The "十" comes from the powers
    // table (POWER_KANJI), which we deliberately don't swap for
    // the legal form; only digit glyphs are substituted.
    use super::super::constants::DIGIT_KANJI_LEGAL;
    assert_eq!(
        number_to_kanji(123, DIGIT_KANJI_LEGAL, POWER_KANJI, false),
        "百弐十参"
    );
}

// --- parse_number_star_ ---
#[test]
fn pure_digits_reduce_left_to_right() {
    // 二〇二〇 — four jd digits, no power → 2020.
    let na = &[(Jd, 2), (Jd, 0), (Jd, 2), (Jd, 0)];
    assert_eq!(parse_number_star_(na), Some(2020));
}

#[test]
fn lone_power_at_start_yields_power_value() {
    // 万 alone → 10000.
    let na = &[(P, 4)];
    assert_eq!(parse_number_star_(na), Some(10000));
}

#[test]
fn power_with_following_remainder_adds() {
    // 千五百 — split on 千 (3): left=∅ → +1000; right=五百 → 500. = 1500.
    let na = &[(P, 3), (Jd, 5), (P, 2)];
    assert_eq!(parse_number_star_(na), Some(1500));
}

#[test]
fn nested_split_with_left_and_right() {
    // 千二百三十四 = 1234. 千(3) is largest power → left=∅+1000, right=二百三十四=234. → 1234.
    let na = &[(P, 3), (Jd, 2), (P, 2), (Jd, 3), (P, 1), (Jd, 4)];
    assert_eq!(parse_number_star_(na), Some(1234));
}

// --- parse_number ---
#[test]
fn pure_kanji_digits() {
    assert_eq!(parse_number("二〇二〇").unwrap(), 2020);
}

#[test]
fn power_kanji_alone() {
    assert_eq!(parse_number("百万").unwrap(), 1000000);
}

#[test]
fn power_with_remainder() {
    assert_eq!(parse_number("100万500").unwrap(), 1_000_500);
}

#[test]
fn full_width_ascii_digits() {
    assert_eq!(parse_number("１２３").unwrap(), 123);
}

#[test]
fn roundtrips_against_number_to_kanji() {
    // Mirrors `parse-number-test` in `tests.lisp:640-644`.
    use super::super::constants::DIGIT_KANJI_DEFAULT;
    use super::super::constants::POWER_KANJI;
    use super::super::kanji_form::number_to_kanji;
    for &n in &[0u128, 10001, 20020001, 12_423_000_430] {
        let s = number_to_kanji(n, DIGIT_KANJI_DEFAULT, POWER_KANJI, false);
        assert_eq!(
            parse_number(&s).unwrap(),
            n,
            "roundtrip failed for {n} → {s}"
        );
    }
}

#[test]
fn invalid_char_raises_not_a_number() {
    let err = parse_number("一X").unwrap_err();
    assert_eq!(err.text, "一X");
    assert!(err.reason.contains('X'), "reason was: {}", err.reason);
}

// --- values above u64 ---
// Corpus row idxs=[664844]: 「…その半減期は…(≒ 1700京〜2100京年)である。」
// 2100京 = 2.1×10^19 exceeds u64::MAX; the Lisp parses it as a bignum.
#[test]
fn kei_value_above_u64_parses_exactly() {
    // (ichiran/numbers:parse-number "2100京") => 21000000000000000000
    assert_eq!(
        parse_number("2100京").unwrap().to_string(),
        "21000000000000000000"
    );
}

#[test]
fn largest_gated_number_text_parses_exactly() {
    // 19 digits + 京 = 20 chars, the longest number text the
    // segmenter's counter gate admits (dict.lisp:1095-1098).
    // (ichiran/numbers:parse-number "9999999999999999999京")
    //   => 99999999999999999990000000000000000
    assert_eq!(
        parse_number("9999999999999999999京").unwrap().to_string(),
        "99999999999999999990000000000000000"
    );
    // (ichiran/numbers:parse-number "123456789012345678京")
    //   => 1234567890123456780000000000000000
    assert_eq!(
        parse_number("123456789012345678京").unwrap().to_string(),
        "1234567890123456780000000000000000"
    );
}

#[test]
fn kanji_form_above_u64() {
    // (ichiran/numbers:number-to-kanji 21000000000000000000) => 二千百京
    assert_eq!(n2k(21_000_000_000_000_000_000), "二千百京");
    // Values ≥ 10^32 stack 京 recursively — pinned against
    // (ichiran/numbers:number-to-kanji 1234567890123456780000000000000000).
    assert_eq!(
        n2k(1_234_567_890_123_456_780_000_000_000_000_000),
        "十二京三千四百五十六兆七千八百九十億一千二百三十四万五千六百七十八京"
    );
}

#[test]
fn overflow_past_u128_raises_not_a_number() {
    // 40 digits ≈ 10^39 > u128::MAX. The Lisp parses this as a bignum;
    // it can only reach kaniran through the ungated direct-lookup
    // paths, where the parse failure drops the counter candidate.
    let text = "1".repeat(40);
    let err = parse_number(&text).unwrap_err();
    assert_eq!(err.text, text);
    assert!(err.reason.contains("u128"), "reason was: {}", err.reason);
}
