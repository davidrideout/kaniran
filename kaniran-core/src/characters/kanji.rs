//! Kanji-specific predicates and helpers. From `characters.lisp:179-208`
//! (sequential / mask / regex / match / cross-match) and `:280-284`
//! (prefix).

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::char_classes::KANJI_REGEX;

/// `sequential-kanji-positions` (`characters.lisp:179-183`). For each
/// adjacent pair of kanji-ish characters in `word`, return the
/// *character* index of the second kanji of the pair, plus `offset`.
/// Used to find ambiguity points in compound words: a stretch of N
/// kanji yields N-1 results.
///
/// The upstream's `&optional (offset 0)` becomes a required `usize` —
/// the single caller passes an explicit offset (CONVENTIONS §4.4 only
/// makes the parameter optional when keyword polarity is involved;
/// here the parameter has a single natural meaning).
///
/// The zero-width lookahead `(?=[々一-龯][々一-龯])` is matched
/// non-overlapping by both cl-ppcre and fancy-regex — each iteration
/// advances by one character past a zero-width hit, so consecutive
/// pairs do all surface.
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

/// `kanji-mask` (`characters.lisp:185-188`). Replace every run of one or
/// more kanji-ish characters in `word` with a single `%`, producing a
/// SQL LIKE-style mask. The underlying pattern is `*kanji-regex*`
/// repeated one-or-more times; the compiled scanner is cached.
pub fn kanji_mask(word: &str) -> String {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    let re = SCANNER.get_or_init(|| {
        Regex::new(&format!("(?:{KANJI_REGEX})+")).expect("kanji-mask scanner compiles")
    });
    re.replace_all(word, "%").into_owned()
}

/// `kanji-regex` (`characters.lisp:190-198`). Build a per-word [`Regex`]
/// for matching candidate readings of `word`: every run of kanji becomes
/// a `.+` (greedy), every non-kanji character is matched literally, and
/// the whole expression is anchored at both ends. Internally walks the
/// output of [`kanji_mask`].
///
/// Compiles a fresh regex each call — the Lisp does the same. Caching
/// by input word would have unbounded keys.
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

/// `kanji-match` (`characters.lisp:200-201`). True iff `reading` matches
/// the per-word regex built by [`kanji_regex`]. The Lisp returns the
/// match position (truthy) or `nil`; every caller uses it as a
/// predicate, so the Rust signature is `bool` per CONVENTIONS §4.1.
pub fn kanji_match(word: &str, reading: &str) -> bool {
    kanji_regex(word).is_match(reading).unwrap_or(false)
}

/// `kanji-cross-match` (`characters.lisp:203-208`). Given an original
/// `word`, its `reading`, and a `new_word`, return the reading of
/// `new_word` derived by replacing the diverging tail of `word` (and the
/// corresponding tail of `reading`) with the diverging tail of
/// `new_word`. Returns `None` when `word` and `new_word` are identical,
/// share no prefix, or when the implied cut position falls outside
/// `reading`.
///
/// Char-position semantics throughout (CONVENTIONS §4.5). The Lisp's
/// latent crash when `mismatch` returns `nil` (arithmetic on `nil`) is
/// not propagated — equal inputs simply yield `None`.
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

/// `kanji-prefix` (`characters.lisp:280-284`). Return the longest
/// prefix of `word` that ends in a kanji-ish character (CJK ideograph
/// plus `々ヶ〆`), or the empty string if `word` contains no kanji at
/// all.
///
/// The Lisp uses `scan-to-strings` against `"^.*<kanji-regex>"`, which
/// falls back to `nil` and is `or`'d with `""`; the Rust port returns
/// `String` directly to mirror the always-string upstream contract.
/// The compiled scanner is cached.
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

    /// Lookahead semantics: a run of N kanji yields N-1 positions, each
    /// pointing to the *second* kanji of an adjacent pair (char index).
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

    /// Pure-kanji word collapses to `^.+$` and accepts any non-empty
    /// reading. The empty reading is rejected.
    #[test]
    fn kanji_regex_pure_kanji_word_accepts_any_nonempty_reading() {
        let re = kanji_regex("日本語");
        assert!(re.is_match("にほんご").unwrap());
        assert!(!re.is_match("").unwrap());
    }

    /// Non-kanji characters in the word stay literal in the regex —
    /// the leading hiragana of `お茶` must appear in the reading too.
    #[test]
    fn kanji_regex_non_kanji_characters_stay_literal() {
        let re = kanji_regex("お茶");
        assert!(re.is_match("おちゃ").unwrap());
        assert!(!re.is_match("にちゃ").unwrap());
    }

    /// No kanji → empty string, mirroring the Lisp `(or scan "")`.
    #[test]
    fn kanji_prefix_returns_empty_when_no_kanji() {
        assert_eq!(kanji_prefix("ひらがな"), "");
        assert_eq!(kanji_prefix(""), "");
    }

    /// Returns up to and including the *last* kanji — the `.*` is
    /// greedy. Trailing non-kanji are dropped.
    #[test]
    fn kanji_prefix_returns_prefix_up_to_last_kanji() {
        assert_eq!(kanji_prefix("お茶を飲む"), "お茶を飲");
        assert_eq!(kanji_prefix("日本語"), "日本語");
    }
}
