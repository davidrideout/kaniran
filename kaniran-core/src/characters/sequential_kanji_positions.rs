//! Port of `ichiran/characters:sequential-kanji-positions` (`characters.lisp:179-183`).
//!
//! For each adjacent pair of kanji-ish characters in `word`, return the
//! *character* index of the second kanji of the pair, plus `offset`.
//! Used to find ambiguity points in compound words: a stretch of N
//! kanji yields N-1 results.
//!
//! The upstream's `&optional (offset 0)` becomes a required `usize` —
//! the single caller passes an explicit offset (CONVENTIONS §4.4 only
//! makes the parameter optional when keyword polarity is involved;
//! here the parameter has a single natural meaning).
//!
//! The zero-width lookahead `(?=[々一-龯][々一-龯])` is matched
//! non-overlapping by both cl-ppcre and fancy-regex — each iteration
//! advances by one character past a zero-width hit, so consecutive
//! pairs do all surface.

use std::sync::OnceLock;

use fancy_regex::Regex;

static SCANNER: OnceLock<Regex> = OnceLock::new();

fn scanner() -> &'static Regex {
    SCANNER.get_or_init(|| {
        Regex::new("(?=[々一-龯][々一-龯])").expect("sequential-kanji lookahead compiles")
    })
}

pub fn sequential_kanji_positions(word: &str, offset: usize) -> Vec<usize> {
    let re = scanner();
    let mut out = Vec::new();
    for m in re.find_iter(word) {
        let m = m.expect("regex iteration");
        let char_pos = word[..m.start()].chars().count();
        out.push(char_pos + 1 + offset);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lookahead semantics: a run of N kanji yields N-1 positions, each
    /// pointing to the *second* kanji of an adjacent pair (char index).
    /// Worth pinning because zero-width-match iteration is the kind of
    /// thing that silently regresses across regex-engine versions.
    #[test]
    fn returns_char_position_of_second_in_each_adjacent_pair() {
        assert_eq!(sequential_kanji_positions("日本語", 0), vec![1, 2]);
        assert_eq!(sequential_kanji_positions("日本語", 5), vec![6, 7]);
    }

    /// Non-kanji separators break adjacency.
    #[test]
    fn non_kanji_breaks_adjacency() {
        assert_eq!(sequential_kanji_positions("日の本", 0), Vec::<usize>::new());
        assert_eq!(sequential_kanji_positions("ひらがな", 0), Vec::<usize>::new());
    }
}
