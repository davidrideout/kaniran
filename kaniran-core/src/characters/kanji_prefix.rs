//! Port of `ichiran/characters:kanji-prefix` (`characters.lisp:280-284`).
//!
//! Return the longest prefix of `word` that ends in a kanji-ish
//! character (CJK ideograph plus `々ヶ〆`), or the empty string if
//! `word` contains no kanji at all.

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::_star_kanji_regex_star_::KANJI_REGEX;

fn scanner() -> &'static Regex {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    SCANNER.get_or_init(|| {
        Regex::new(&format!("^.*{KANJI_REGEX}")).expect("kanji-prefix scanner compiles")
    })
}

pub fn kanji_prefix(word: &str) -> String {
    match scanner().find(word) {
        Ok(Some(m)) => m.as_str().to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No kanji → empty string.
    #[test]
    fn returns_empty_when_no_kanji() {
        assert_eq!(kanji_prefix("ひらがな"), "");
        assert_eq!(kanji_prefix(""), "");
    }

    /// Returns up to and including the last kanji; trailing non-kanji dropped.
    #[test]
    fn returns_prefix_up_to_last_kanji() {
        assert_eq!(kanji_prefix("お茶を飲む"), "お茶を飲");
        assert_eq!(kanji_prefix("日本語"), "日本語");
    }
}
