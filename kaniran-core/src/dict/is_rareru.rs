//! Port of `ichiran/dict:is-rareru` (`dict.lisp:1619`).
//!
//! True when `text` ends with one of the passive/potential forms
//! られる / られます / られない / られません.

pub fn is_rareru(text: &str) -> bool {
    text.ends_with("られる")
        || text.ends_with("られます")
        || text.ends_with("られない")
        || text.ends_with("られません")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL fixtures (.103, `ichiran/dict::is-rareru`), 2026-05-24.
    /// Covers each of the four suffixes, a non-rareru form, the empty
    /// string, a rareru substring not at the end (`られるよ` → false), and
    /// a kana-only suffix.
    #[test]
    fn is_rareru_fixtures() {
        let cases: &[(&str, bool)] = &[
            ("食べられる", true),
            ("食べられます", true),
            ("食べられない", true),
            ("食べられません", true),
            ("食べる", false),
            ("", false),
            ("られるよ", false),
            ("られる", true),
        ];
        for (text, expected) in cases {
            assert_eq!(is_rareru(text), *expected, "text={text:?}");
        }
    }
}
