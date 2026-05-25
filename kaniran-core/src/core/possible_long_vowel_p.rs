//! Port of `ichiran:possible-long-vowel-p` (`deromanize.lisp:32`).
//!
//! Returns the trailing `o` or `u` of `text`, or `None` when `text`
//! is empty or ends in any other character. Upstream returns the
//! matched character (`#\o` / `#\u`) or `nil`; the port keeps that as
//! `Option<char>`.

pub fn possible_long_vowel_p(text: &str) -> Option<char> {
    if text.is_empty() {
        return None;
    }
    let ch = text.chars().next_back().expect("text is non-empty here");
    ['o', 'u'].into_iter().find(|vowel| *vowel == ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn possible_long_vowel_p_fixtures() {
        // REPL fixtures (.103, ichiran::possible-long-vowel-p), 2026-05-25.
        let cases: &[(&str, Option<char>)] = &[
            ("", None),                // empty
            ("ko", Some('o')),         // ends o
            ("ku", Some('u')),         // ends u
            ("ka", None),              // ends a
            ("o", Some('o')),          // single o
            ("u", Some('u')),          // single u
            ("shinbun", None),         // ends n
            ("kyou", Some('u')),
            ("toukyou", Some('u')),
            ("sapporo", Some('o')),
            ("gakkou", Some('u')),
            ("arigatou", Some('u')),
            ("tomodachi", None),       // ends i
            ("fujisan", None),         // ends n
            ("おう", None),            // last char う (not ASCII o/u)
            ("あo", Some('o')),        // multibyte prefix, last char o
            ("katsu", Some('u')),
        ];
        for (text, expected) in cases {
            assert_eq!(possible_long_vowel_p(text), *expected, "text={text:?}");
        }
    }
}
