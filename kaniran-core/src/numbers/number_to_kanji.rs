//! Port of `ichiran/numbers:number-to-kanji` (`numbers.lisp:35`).
//!
//! Render a non-negative integer as a kanji number string —
//! `1234` → `千二百三十四`, `0` → `〇`. The `digits` and `powers`
//! parameters are the per-digit and per-power glyph tables; `one_sen`
//! is the recursion flag controlling whether a leading `一` is
//! suppressed before `千` (`one_sen = false`) or only before `百`
//! (`one_sen = true`).

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

#[cfg(test)]
mod tests {
    use super::super::_star_digit_kanji_default_star_::DIGIT_KANJI_DEFAULT;
    use super::super::_star_power_kanji_star_::POWER_KANJI;
    use super::*;

    fn n2k(n: u64) -> String {
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
        use super::super::_star_digit_kanji_legal_star_::DIGIT_KANJI_LEGAL;
        assert_eq!(
            number_to_kanji(123, DIGIT_KANJI_LEGAL, POWER_KANJI, false),
            "百弐十参"
        );
    }
}
