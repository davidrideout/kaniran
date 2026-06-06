//! Port of `ichiran/dict:reading-str*` (`dict.lisp:1580`).
//!
//! Formats a `(kanji, kana)` pair as `"kanji 【kana】"`, or the bare kana
//! when there's no kanji.

pub fn reading_str_star_(kanji: Option<&str>, kana: Option<&str>) -> Option<String> {
    match kanji {
        // ~a of nil prints "NIL"
        Some(kanji) => Some(format!("{} 【{}】", kanji, kana.unwrap_or("NIL"))),
        None => kana.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL fixtures (.103, `ichiran/dict::reading-str*`), 2026-05-24.
    /// Covers kanji+kana, kana-only (kanji nil → bare kana), kanji with
    /// nil kana (`~a` of nil → "NIL"), and both nil → nil.
    #[test]
    fn reading_str_star_fixtures() {
        let cases: &[(Option<&str>, Option<&str>, Option<&str>)] = &[
            (Some("日本"), Some("にほん"), Some("日本 【にほん】")),
            (None, Some("ねこ"), Some("ねこ")),
            (Some("猫"), None, Some("猫 【NIL】")),
            (None, None, None),
        ];
        for (kanji, kana, expected) in cases {
            assert_eq!(
                reading_str_star_(*kanji, *kana).as_deref(),
                *expected,
                "kanji={kanji:?} kana={kana:?}"
            );
        }
    }
}
