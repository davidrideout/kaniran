//! Port of `ichiran/numbers:parse-number` (`numbers.lisp:74`).
//!
//! Parse a string of digit / power glyphs (kanji, ASCII, or full-width)
//! into a `u64`. Classifies each character via
//! [`super::_star_char_number_class_hash_star_::char_number_class_hash`]
//! and delegates the structural arithmetic to
//! [`super::parse_number_star__::parse_number_star_`].
//!
//! Returns [`Err`] with a [`NotANumber`] carrying the offending input
//! and a per-character reason when any glyph is unclassifiable, matching
//! the upstream `error 'not-a-number :text :reason` raise site.

use super::_star_char_number_class_hash_star_::char_number_class_hash;
use super::not_a_number_condition::NotANumber;
use super::parse_number_star_::parse_number_star_;

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
    use super::*;

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
        use super::super::_star_digit_kanji_default_star_::DIGIT_KANJI_DEFAULT;
        use super::super::_star_power_kanji_star_::POWER_KANJI;
        use super::super::number_to_kanji::number_to_kanji;
        for &n in &[0u64, 10001, 20020001, 12_423_000_430] {
            let s = number_to_kanji(n, DIGIT_KANJI_DEFAULT, POWER_KANJI, false);
            assert_eq!(parse_number(&s).unwrap(), n, "roundtrip failed for {n} → {s}");
        }
    }

    #[test]
    fn invalid_char_raises_not_a_number() {
        let err = parse_number("一X").unwrap_err();
        assert_eq!(err.text, "一X");
        assert!(err.reason.contains('X'), "reason was: {}", err.reason);
    }
}
