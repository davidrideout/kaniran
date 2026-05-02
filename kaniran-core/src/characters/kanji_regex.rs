//! Port of `ichiran/characters:kanji-regex` (`characters.lisp:190-198`).
//!
//! Build a per-word [`fancy_regex::Regex`] for matching candidate
//! readings of `word`: every run of kanji becomes a `.+` (greedy),
//! every non-kanji character is matched literally, and the whole
//! expression is anchored at both ends. Internally walks the output
//! of [`super::kanji_mask::kanji_mask`].
//!
//! Compiles a fresh regex each call — the Lisp does the same. Caching
//! by input word would have unbounded keys.

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

    /// Pure-kanji word collapses to `^.+$` and accepts any non-empty
    /// reading. The empty reading is rejected.
    #[test]
    fn pure_kanji_word_accepts_any_nonempty_reading() {
        let re = kanji_regex("日本語");
        assert!(re.is_match("にほんご").unwrap());
        assert!(!re.is_match("").unwrap());
    }

    /// Non-kanji characters in the word stay literal in the regex —
    /// the leading hiragana of `お茶` must appear in the reading too.
    #[test]
    fn non_kanji_characters_stay_literal() {
        let re = kanji_regex("お茶");
        assert!(re.is_match("おちゃ").unwrap());
        assert!(!re.is_match("にちゃ").unwrap());
    }
}
