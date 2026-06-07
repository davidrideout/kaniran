//! Port of `ichiran/characters:kanji-mask` (`characters.lisp:185-188`).
//!
//! Replace every run of one or more kanji-ish characters in `word`
//! with a single `%`, producing a SQL LIKE-style mask.

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::_star_kanji_regex_star_::KANJI_REGEX;

fn scanner() -> &'static Regex {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    SCANNER.get_or_init(|| {
        Regex::new(&format!("(?:{KANJI_REGEX})+")).expect("kanji-mask scanner compiles")
    })
}

pub fn kanji_mask(word: &str) -> String {
    scanner().replace_all(word, "%").into_owned()
}
