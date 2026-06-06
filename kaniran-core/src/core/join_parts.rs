//! Port of `ichiran:join-parts` (`romanize.lisp:235-246`).
//!
//! Concatenates `parts`, inserting a single space before a part that
//! begins with an alphanumeric character when the running output did not
//! already end in whitespace. Empty parts are skipped entirely.

use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

pub fn join_parts<S: AsRef<str>>(parts: &[S]) -> String {
    let mut out = String::new();
    let mut last_space = true;
    for part in parts {
        let part = part.as_ref();
        let chars: Vec<char> = part.chars().collect();
        let len = chars.len();
        // romanize.lisp:240-243
        if len != 0 && !last_space && alphanumericp(chars[0]) {
            out.push(' ');
        }
        out.push_str(part);
        // romanize.lisp:245-246
        if len != 0 {
            last_space = chars[len - 1].is_whitespace();
        }
    }
    out
}

/// `(alphanumericp char)` — Lisp `alpha-char-p` (letter categories Lu,
/// Ll, Lt, Lm, Lo) or `digit-char-p` radix 10 (decimal-number category
/// Nd). Std `char::is_alphanumeric` diverges by also counting Nl and No
/// (roman numerals, circled / superscript numbers) that `alphanumericp`
/// rejects, so the general category is consulted directly.
fn alphanumericp(char: char) -> bool {
    char.general_category_group() == GeneralCategoryGroup::Letter
        || char.general_category() == GeneralCategory::DecimalNumber
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_parts_fixtures() {
        // REPL fixtures (.103, ichiran::join-parts), 2026-05-23.
        let cases: &[(&[&str], &str)] = &[
            // spaces inserted between alphanumeric parts
            (&["watashi", "wa", "gakusei", "desu"], "watashi wa gakusei desu"),
            // no space before punctuation
            (&["Tokyo", ",", "desu"], "Tokyo, desu"),
            // a trailing space suppresses the next part's space
            (&["hello ", "world"], "hello world"),
            // leading empty part: last_space stays true, no leading space
            (&["", "abc"], "abc"),
            // empty middle part leaves last_space false, so "def" still spaces
            (&["abc", "", "def"], "abc def"),
            // ideographic period is not alphanumeric
            (&["Tokyo", "。"], "Tokyo。"),
            // ① (category No) is not alphanumeric to CL; std is_alphanumeric
            // would have inserted a space here
            (&["a", "①"], "a①"),
            // Ⅴ (category Nl) is not alphanumeric; the space before "a"
            // comes from "a", not Ⅴ
            (&["Ⅴ", "a"], "Ⅴ a"),
            // ascii digit is alphanumeric
            (&["a", "5"], "a 5"),
            // fullwidth ５ (category Nd) is alphanumeric
            (&["a", "５"], "a ５"),
            // prolonged sound mark ー (category Lm) is alphanumeric
            (&["a", "ー"], "a ー"),
            // trailing U+3000 ideographic space sets the whitespace flag
            (&["foo　", "bar"], "foo　bar"),
        ];
        for (parts, expected) in cases {
            assert_eq!(&join_parts(parts), expected, "parts={parts:?}");
        }
    }
}
