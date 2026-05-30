//! Kanji-specific predicates and helpers. From `characters.lisp:179-208`
//! and `:280-284`.

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::char_classes::KANJI_REGEX;

/// `sequential-kanji-positions` (`characters.lisp:179-183`). For each
/// adjacent kanji-ish pair in `word`, the *character* index of the
/// second kanji of the pair, plus `offset`. A run of N kanji yields
/// N-1 results.
pub fn sequential_kanji_positions(word: &str, offset: usize) -> Vec<usize> {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    let re = SCANNER.get_or_init(|| {
        Regex::new("(?=[々一-龯][々一-龯])").expect("sequential-kanji lookahead compiles")
    });
    let mut out = Vec::new();
    for m in re.find_iter(word) {
        let m = m.expect("regex iteration");
        let char_pos = word[..m.start()].chars().count();
        out.push(char_pos + 1 + offset);
    }
    out
}

/// `kanji-mask` (`characters.lisp:185-188`). Replace every run of
/// kanji-ish characters in `word` with a single `%` (SQL LIKE-style).
pub fn kanji_mask(word: &str) -> String {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    let re = SCANNER.get_or_init(|| {
        Regex::new(&format!("(?:{KANJI_REGEX})+")).expect("kanji-mask scanner compiles")
    });
    re.replace_all(word, "%").into_owned()
}

/// `kanji-regex` (`characters.lisp:190-198`). Per-word [`Regex`] for
/// matching candidate readings: every kanji run becomes `.+`, every
/// non-kanji character matches literally, anchored at both ends.
/// Compiles fresh per call — caching by input would have unbounded keys.
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

/// `kanji-match` (`characters.lisp:200-201`). True iff `reading`
/// matches [`kanji_regex`] for `word`.
pub fn kanji_match(word: &str, reading: &str) -> bool {
    kanji_regex(word).is_match(reading).unwrap_or(false)
}

/// `kanji-cross-match` (`characters.lisp:203-208`). Derive the reading
/// of `new_word` by replacing the diverging tail of `word`'s reading
/// with the diverging tail of `new_word`. Returns `None` for identical
/// inputs, no shared prefix, or implied cut outside `reading`.
pub fn kanji_cross_match(word: &str, reading: &str, new_word: &str) -> Option<String> {
    let m = first_mismatch_chars(word, new_word)?;
    let reading_len = reading.chars().count();
    let word_len = word.chars().count();
    let r_cut = (m as isize) + (reading_len as isize) - (word_len as isize);
    if m == 0 || r_cut < 0 || r_cut > reading_len as isize {
        return None;
    }
    let r_cut = r_cut as usize;
    let mut out: String = reading.chars().take(r_cut).collect();
    out.extend(new_word.chars().skip(m));
    Some(out)
}

fn first_mismatch_chars(a: &str, b: &str) -> Option<usize> {
    let mut ai = a.chars();
    let mut bi = b.chars();
    let mut idx = 0usize;
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return None,
            (Some(x), Some(y)) if x == y => idx += 1,
            _ => return Some(idx),
        }
    }
}

/// `kanji-prefix` (`characters.lisp:280-284`). Longest prefix of `word`
/// ending in a kanji-ish character, or `""` if `word` has no kanji.
pub fn kanji_prefix(word: &str) -> String {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    let re = SCANNER.get_or_init(|| {
        Regex::new(&format!("^.*{KANJI_REGEX}")).expect("kanji-prefix scanner compiles")
    });
    match re.find(word) {
        Ok(Some(m)) => m.as_str().to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_kanji_positions_returns_char_position_of_second_in_each_pair() {
        assert_eq!(sequential_kanji_positions("日本語", 0), vec![1, 2]);
        assert_eq!(sequential_kanji_positions("日本語", 5), vec![6, 7]);
    }

    #[test]
    fn sequential_kanji_positions_non_kanji_breaks_adjacency() {
        assert_eq!(sequential_kanji_positions("日の本", 0), Vec::<usize>::new());
        assert_eq!(sequential_kanji_positions("ひらがな", 0), Vec::<usize>::new());
    }

    #[test]
    fn kanji_regex_pure_kanji_word_accepts_any_nonempty_reading() {
        let re = kanji_regex("日本語");
        assert!(re.is_match("にほんご").unwrap());
        assert!(!re.is_match("").unwrap());
    }

    #[test]
    fn kanji_regex_non_kanji_characters_stay_literal() {
        let re = kanji_regex("お茶");
        assert!(re.is_match("おちゃ").unwrap());
        assert!(!re.is_match("にちゃ").unwrap());
    }

    #[test]
    fn kanji_prefix_returns_empty_when_no_kanji() {
        assert_eq!(kanji_prefix("ひらがな"), "");
        assert_eq!(kanji_prefix(""), "");
    }

    #[test]
    fn kanji_prefix_returns_prefix_up_to_last_kanji() {
        assert_eq!(kanji_prefix("お茶を飲む"), "お茶を飲");
        assert_eq!(kanji_prefix("日本語"), "日本語");
    }
}
