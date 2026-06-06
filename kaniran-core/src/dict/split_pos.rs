//! Port of `ichiran/dict:split-pos` (`dict.lisp:1535`).
//!
//! Splits the bracketed part-of-speech string on commas, excluding the
//! enclosing `[` / `]`. Empty subsequences are kept, so `"[]"` yields
//! `[""]`; offsets index by code point.

pub fn split_pos(pos_str: &str) -> Vec<&str> {
    // :start 1 :end (1- (length pos-str)) — code-point offsets.
    let char_count = pos_str.chars().count();
    let start = pos_str
        .char_indices()
        .nth(1)
        .map_or(pos_str.len(), |(byte, _)| byte);
    let end = pos_str
        .char_indices()
        .nth(char_count - 1)
        .map_or(pos_str.len(), |(byte, _)| byte);
    pos_str[start..end].split(',').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL fixtures (.103, `ichiran/dict::split-pos`), 2026-05-24.
    #[test]
    fn split_pos_fixtures() {
        let cases: &[(&str, Vec<&str>)] = &[
            ("[n,adj-no]", vec!["n", "adj-no"]),
            ("[]", vec![""]),
            ("[n]", vec!["n"]),
            ("[adv,adv-to,vs]", vec!["adv", "adv-to", "vs"]),
            ("[ctr]", vec!["ctr"]),
            ("[v5u,vt]", vec!["v5u", "vt"]),
        ];
        for (pos_str, expected) in cases {
            assert_eq!(&split_pos(pos_str), expected, "pos_str={pos_str:?}");
        }
    }
}
