//! Port of `ichiran/characters:kanji-mask` (`characters.lisp:185-188`).
//!
//! Replace every run of one or more kanji-ish characters in `word`
//! with a single `%`, producing a SQL LIKE-style mask. The underlying
//! pattern is `*kanji-regex*` repeated one-or-more times; the compiled
//! scanner is cached.

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::_star_kanji_regex_star_::KANJI_REGEX;

static SCANNER: OnceLock<Regex> = OnceLock::new();

fn scanner() -> &'static Regex {
    SCANNER.get_or_init(|| {
        Regex::new(&format!("(?:{KANJI_REGEX})+")).expect("kanji-mask scanner compiles")
    })
}

pub fn kanji_mask(word: &str) -> String {
    scanner().replace_all(word, "%").into_owned()
}
