//! Kanji↔integer conversion. From `numbers.lisp:35-79`.

use thiserror::Error;

use super::num_class::{char_number_class_hash, NumClass};

/// `number-to-kanji` (`numbers.lisp:35`). Render a non-negative integer
/// as a kanji number string — `1234` → `千二百三十四`, `0` → `〇`.
/// `one_sen = true` suppresses a leading `一` only before `百`;
/// `one_sen = false` extends suppression to `千`. Recursion-only;
/// top-level callers pass `false`.
pub fn number_to_kanji(n: u64, digits: &str, powers: &str, one_sen: bool) -> String {
    let digit_chars: Vec<char> = digits.chars().collect();
    let power_chars: Vec<char> = powers.chars().collect();
    if n == 0 {
        return digit_chars[0].to_string();
    }
    let mut mp: u64 = 1;
    let mut mc: char = power_chars[0];
    let mut p: u64 = 1;
    for &c in &power_chars {
        if p > n {
            break;
        }
        if c != ' ' {
            mp = p;
            mc = c;
        }
        match p.checked_mul(10) {
            Some(np) => p = np,
            None => break,
        }
    }
    if mp == 1 {
        return digit_chars[n as usize].to_string();
    }
    let qt = n / mp;
    let rem = n % mp;
    let head_threshold: u64 = if one_sen { 100 } else { 1000 };
    let head = if qt == 1 && mp <= head_threshold {
        String::new()
    } else {
        number_to_kanji(qt, digits, powers, true)
    };
    let tail = if rem == 0 {
        String::new()
    } else {
        number_to_kanji(rem, digits, powers, one_sen)
    };
    format!("{head}{mc}{tail}")
}

/// `parse-number*` (`numbers.lisp:57`). Recursive parser over a slice
/// of classified atoms. Finds the largest `(P, exponent)` and splits:
/// `left * 10^exp + right`. With no power token, reduces digits
/// left-to-right (`a, b, c → a*100 + b*10 + c`). Lisp `&key start end`
/// becomes a `&[]` slice.
pub fn parse_number_star_(na: &[(NumClass, u8)]) -> u64 {
    let mut mp: u8 = 0;
    let mut mi: Option<usize> = None;
    for (i, &(class, val)) in na.iter().enumerate() {
        if class == NumClass::P && val > mp {
            mp = val;
            mi = Some(i);
        }
    }
    match mi {
        None => na
            .iter()
            .fold(0u64, |a, &(_class, v)| a * 10 + v as u64),
        Some(idx) if idx == 0 => {
            let head = 10u64.pow(mp as u32);
            let tail = if na.len() > 1 {
                parse_number_star_(&na[1..])
            } else {
                0
            };
            head + tail
        }
        Some(idx) => {
            let left = parse_number_star_(&na[..idx]);
            let right = if idx + 1 < na.len() {
                parse_number_star_(&na[idx + 1..])
            } else {
                0
            };
            left * 10u64.pow(mp as u32) + right
        }
    }
}

/// `not-a-number` condition (`numbers.lisp:67`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{text:?} is not a number: {reason}")]
pub struct NotANumber {
    pub text: String,
    pub reason: String,
}

/// `parse-number` (`numbers.lisp:74`). Parse a string of digit/power
/// glyphs (kanji, ASCII, or full-width) into a `u64`. Returns
/// [`NotANumber`] on the first unclassifiable character.
pub fn parse_number(s: &str) -> Result<u64, NotANumber> {
    let h = char_number_class_hash();
    let mut na = Vec::with_capacity(s.chars().count());
    for c in s.chars() {
        match h.get(&c) {
            Some(&pair) => na.push(pair),
            None => {
                return Err(NotANumber {
                    text: s.to_string(),
                    reason: format!("Invalid character: {c}"),
                });
            }
        }
    }
    Ok(parse_number_star_(&na))
}

#[cfg(test)]
mod tests {
    use super::super::num_class::{DIGIT_KANJI_DEFAULT, DIGIT_KANJI_LEGAL, POWER_KANJI};
    use super::*;
    use NumClass::*;

    fn n2k(n: u64) -> String {
        number_to_kanji(n, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
    }

    #[test]
    fn number_to_kanji_zero_is_single_glyph() {
        assert_eq!(n2k(0), "〇");
    }

    #[test]
    fn number_to_kanji_single_digit_passes_through() {
        assert_eq!(n2k(7), "七");
    }

    #[test]
    fn number_to_kanji_ten_omits_leading_one() {
        assert_eq!(n2k(10), "十");
    }

    #[test]
    fn number_to_kanji_one_thousand_omits_leading_one_at_top_level() {
        assert_eq!(n2k(1000), "千");
    }

    #[test]
    fn number_to_kanji_ten_thousand_keeps_leading_one() {
        assert_eq!(n2k(10000), "一万");
    }

    #[test]
    fn number_to_kanji_nested_one_sen_keeps_leading_one_for_sen() {
        assert_eq!(n2k(11000), "一万千");
    }

    #[test]
    fn number_to_kanji_full_decomposition_1234() {
        assert_eq!(n2k(1234), "千二百三十四");
    }

    /// Pinned against `(ichiran/numbers:number-to-kanji N)` on the
    /// reference Lisp install.
    #[test]
    fn number_to_kanji_matches_lisp_for_multi_kanji_inputs() {
        assert_eq!(n2k(10001), "一万一");
        assert_eq!(n2k(20020001), "二千二万一");
        assert_eq!(n2k(12_423_000_430), "百二十四億二千三百万四百三十");
    }

    /// `十` comes from `POWER_KANJI`, not from the legal-digit swap.
    #[test]
    fn number_to_kanji_legal_digits_swap_in() {
        assert_eq!(
            number_to_kanji(123, DIGIT_KANJI_LEGAL, POWER_KANJI, false),
            "百弐十参"
        );
    }

    #[test]
    fn parse_number_star_pure_digits_reduce_left_to_right() {
        let na = &[(Jd, 2), (Jd, 0), (Jd, 2), (Jd, 0)];
        assert_eq!(parse_number_star_(na), 2020);
    }

    #[test]
    fn parse_number_star_lone_power_at_start_yields_power_value() {
        let na = &[(P, 4)];
        assert_eq!(parse_number_star_(na), 10000);
    }

    #[test]
    fn parse_number_star_power_with_following_remainder_adds() {
        // 千五百: split on 千 → ∅ + 1000 + 五百(500) = 1500.
        let na = &[(P, 3), (Jd, 5), (P, 2)];
        assert_eq!(parse_number_star_(na), 1500);
    }

    #[test]
    fn parse_number_star_nested_split_with_left_and_right() {
        // 千二百三十四: 千 largest → ∅+1000 + 二百三十四(234) = 1234.
        let na = &[(P, 3), (Jd, 2), (P, 2), (Jd, 3), (P, 1), (Jd, 4)];
        assert_eq!(parse_number_star_(na), 1234);
    }

    #[test]
    fn parse_number_pure_kanji_digits() {
        assert_eq!(parse_number("二〇二〇").unwrap(), 2020);
    }

    #[test]
    fn parse_number_power_kanji_alone() {
        assert_eq!(parse_number("百万").unwrap(), 1000000);
    }

    #[test]
    fn parse_number_power_with_remainder() {
        assert_eq!(parse_number("100万500").unwrap(), 1_000_500);
    }

    #[test]
    fn parse_number_full_width_ascii_digits() {
        assert_eq!(parse_number("１２３").unwrap(), 123);
    }

    /// Mirrors `parse-number-test` in `tests.lisp:640-644`.
    #[test]
    fn parse_number_roundtrips_against_number_to_kanji() {
        for &n in &[0u64, 10001, 20020001, 12_423_000_430] {
            let s = number_to_kanji(n, DIGIT_KANJI_DEFAULT, POWER_KANJI, false);
            assert_eq!(parse_number(&s).unwrap(), n, "roundtrip failed for {n} → {s}");
        }
    }

    #[test]
    fn parse_number_invalid_char_raises_not_a_number() {
        let err = parse_number("一X").unwrap_err();
        assert_eq!(err.text, "一X");
        assert!(err.reason.contains('X'), "reason was: {}", err.reason);
    }
}
