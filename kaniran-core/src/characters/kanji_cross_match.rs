//! Port of `ichiran/characters:kanji-cross-match` (`characters.lisp:203-208`).
//!
//! Given an original `word`, its `reading`, and a `new_word`, return
//! the reading of `new_word` derived by replacing the diverging tail
//! of `word` (and the corresponding tail of `reading`) with the
//! diverging tail of `new_word`. Returns `None` when `word` and
//! `new_word` are identical, share no prefix, or when the implied cut
//! position falls outside `reading`.
//!
//! Char-position semantics throughout (CONVENTIONS §4.5). The Lisp's
//! latent crash when `mismatch` returns `nil` (arithmetic on `nil`) is
//! not propagated — equal inputs simply yield `None`.

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
