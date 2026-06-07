use super::constants::KANJI_REGEX;
use fancy_regex::Regex;
use std::sync::OnceLock;

/// Port of `ichiran/characters:sequential-kanji-positions` (`characters.lisp:179-183`).
///
/// For each adjacent pair of kanji-ish characters in `word`, return the
/// *character* index of the second kanji of the pair, plus `offset`.
/// Used to find ambiguity points in compound words: a stretch of N
/// kanji yields N-1 results.
fn sequential_kanji_positions_scanner() -> &'static Regex {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    SCANNER.get_or_init(|| {
        Regex::new("(?=[々一-龯][々一-龯])").expect("sequential-kanji lookahead compiles")
    })
}

pub fn sequential_kanji_positions(word: &str, offset: usize) -> Vec<usize> {
    let re = sequential_kanji_positions_scanner();
    let mut out = Vec::new();
    for m in re.find_iter(word) {
        let m = m.expect("regex iteration");
        let char_pos = word[..m.start()].chars().count();
        out.push(char_pos + 1 + offset);
    }
    out
}

/// Port of `ichiran/characters:kanji-mask` (`characters.lisp:185-188`).
///
/// Replace every run of one or more kanji-ish characters in `word`
/// with a single `%`, producing a SQL LIKE-style mask.
fn kanji_mask_scanner() -> &'static Regex {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    SCANNER.get_or_init(|| {
        Regex::new(&format!("(?:{KANJI_REGEX})+")).expect("kanji-mask kanji_mask_scanner compiles")
    })
}

pub fn kanji_mask(word: &str) -> String {
    kanji_mask_scanner().replace_all(word, "%").into_owned()
}

/// Port of `ichiran/characters:kanji-regex` (`characters.lisp:190-198`).
///
/// Build a per-word [`fancy_regex::Regex`] for matching candidate
/// readings of `word`: every run of kanji becomes a `.+` (greedy),
/// every non-kanji character is matched literally, and the whole
/// expression is anchored at both ends.
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

/// Port of `ichiran/characters:kanji-match` (`characters.lisp:200-201`).
///
/// True iff `reading` matches the per-word regex built by
/// [`super::kanji::kanji_regex`].
pub fn kanji_match(word: &str, reading: &str) -> bool {
    kanji_regex(word).is_match(reading).unwrap_or(false)
}

/// Port of `ichiran/characters:kanji-cross-match` (`characters.lisp:203-208`).
///
/// Given an original `word`, its `reading`, and a `new_word`, return
/// the reading of `new_word` derived by replacing the diverging tail
/// of `word` (and the corresponding tail of `reading`) with the
/// diverging tail of `new_word`. Returns `None` when `word` and
/// `new_word` are identical, share no prefix, or when the implied cut
/// position falls outside `reading`.
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

/// Port of `ichiran/characters:kanji-prefix` (`characters.lisp:280-284`).
///
/// Return the longest prefix of `word` that ends in a kanji-ish
/// character (CJK ideograph plus `々ヶ〆`), or the empty string if
/// `word` contains no kanji at all.
fn kanji_prefix_scanner() -> &'static Regex {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    SCANNER.get_or_init(|| {
        Regex::new(&format!("^.*{KANJI_REGEX}"))
            .expect("kanji-prefix kanji_prefix_scanner compiles")
    })
}

pub fn kanji_prefix(word: &str) -> String {
    match kanji_prefix_scanner().find(word) {
        Ok(Some(m)) => m.as_str().to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests;
