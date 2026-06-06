//! Port of `ichiran/characters:kanji-regex` (`characters.lisp:190-198`).
//!
//! Build a per-word [`fancy_regex::Regex`] for matching candidate
//! readings of `word`: every run of kanji becomes a `.+` (greedy),
//! every non-kanji character is matched literally, and the whole
//! expression is anchored at both ends.

use fancy_regex::Regex;

use super::kanji_mask::kanji_mask;

pub fn kanji_regex(word: &str) -> Regex {
    let masked = kanji_mask(word);
    let mut pattern = String::from("^");
    for c in masked.chars() {
        if c == '%' {
            pattern.push_str(".+");
        } else {
            pattern.push_str(&fancy_regex::escape(&c.to_string()));
        }
    }
    pattern.push('$');
    Regex::new(&pattern).expect("kanji_regex pattern compiles")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure-kanji word collapses to `^.+$`: any non-empty reading accepted, empty rejected.
    #[test]
    fn pure_kanji_word_accepts_any_nonempty_reading() {
        let re = kanji_regex("日本語");
        assert!(re.is_match("にほんご").unwrap());
        assert!(!re.is_match("").unwrap());
    }

    /// Non-kanji characters in the word stay literal in the regex.
    #[test]
    fn non_kanji_characters_stay_literal() {
        let re = kanji_regex("お茶");
        assert!(re.is_match("おちゃ").unwrap());
        assert!(!re.is_match("にちゃ").unwrap());
    }
}
