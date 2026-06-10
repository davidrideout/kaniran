use super::helpers::{basic_split_regex, char_class_hash, char_scanners, char_scanners_inner};
use super::kani_char_class_bare_scanners::char_class_bare_scanners;
use super::kani_kana_class::KanaClass;
use super::kani_ngram_scanner::KaniNgramScanner;
use fancy_regex::Regex;
use std::sync::OnceLock;

/// Port of `ichiran/characters:get-char-class` (`characters.lisp:44-45`).
///
/// Look up the [`KanaClass`] of a single character.
pub fn get_char_class(c: char) -> Option<KanaClass> {
    char_class_hash().get(&c).copied()
}

/// Port of `ichiran/characters:char-class` (`characters.lisp:147`).
///
/// Closed enumeration of the broad character-class tags used to drive
/// regex-based scanning, segmentation, and counting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CharClass {
    Katakana,
    KatakanaUniq,
    Hiragana,
    Kanji,
    KanjiChar,
    Kana,
    Traditional,
    Nonword,
    Number,
}

/// Port of `ichiran/characters:test-word` (`characters.lisp:160-163`).
///
/// True iff every character of `word` belongs to `char_class` — the
/// scanner from `*char-scanners*` is anchored as `^pat+$`, so any
/// non-class character makes the match fail.
pub fn test_word(word: &str, char_class: CharClass) -> bool {
    char_scanners()
        .get(&char_class)
        .expect("char_class is in *char-scanners*")
        .is_match(word)
        .unwrap_or(false)
}

/// Port of `ichiran/characters:count-char-class` (`characters.lisp:165-170`).
///
/// Count non-overlapping matches of `char_class`'s pattern from
/// `*char-class-regex-mapping*` in `word`.
pub fn count_char_class(word: &str, char_class: CharClass) -> usize {
    char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*")
        .find_iter(word)
        .count()
}

/// Port of `ichiran/characters:collect-char-class` (`characters.lisp:172-177`).
///
/// Collect every non-overlapping match of `char_class`'s pattern from
/// `*char-class-regex-mapping*` in `word`, in left-to-right order.
pub fn collect_char_class(word: &str, char_class: CharClass) -> Vec<String> {
    char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*")
        .find_iter(word)
        .map(|m| m.expect("regex iteration").as_str().to_string())
        .collect()
}

/// Port of `ichiran/characters:simplify-ngrams` (`characters.lisp:210-217`).
///
/// Replace every occurrence of a `from` key in `s` with its `to` value,
/// choosing the leftmost-first match when several keys could match at
/// the same position. Used to fold combining-mark sequences into single
/// precomposed glyphs (`"か゛" → "が"`) and to ASCII-ize Japanese
/// punctuation. Keys are matched literally regardless of any regex
/// metacharacters in the data.
pub fn simplify_ngrams<S, T>(s: &str, map: &[(S, T)]) -> String
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    // One-shot form. Hot-path callers with a fixed table hold a cached
    // [`KaniNgramScanner`] instead of recompiling the alternation here.
    KaniNgramScanner::new(map).simplify(s)
}

/// Port of `ichiran/characters:split-by-regex` (`characters.lisp:234-236`).
///
/// Split `s` by `regex`, interleaving the captured groups with the
/// between-match text and dropping any empty pieces. With a regex that
/// has a single outer capture group, the result alternates
/// "between-match text" with "the matched text".
pub fn split_by_regex(regex: &Regex, s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut last = 0;
    for cap in regex.captures_iter(s) {
        let cap = cap.expect("regex iteration error");
        let m = cap.get(0).expect("capture 0 is the whole match");
        let before = &s[last..m.start()];
        if !before.is_empty() {
            out.push(before.to_string());
        }
        for i in 1..cap.len() {
            if let Some(g) = cap.get(i) {
                let g = g.as_str();
                if !g.is_empty() {
                    out.push(g.to_string());
                }
            }
        }
        last = m.end();
    }
    let after = &s[last..];
    if !after.is_empty() {
        out.push(after.to_string());
    }
    out
}

/// Port of `ichiran/characters:basic-split` (`characters.lisp:238-243`).
///
/// Segment Japanese-mixed text into alternating misc / word runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Misc,
    Word,
}

fn scanner() -> &'static Regex {
    static SCANNER: OnceLock<Regex> = OnceLock::new();
    SCANNER.get_or_init(|| Regex::new(basic_split_regex()).expect("basic-split-regex must compile"))
}

pub fn basic_split(s: &str) -> Vec<(SegmentKind, String)> {
    let segments = split_by_regex(scanner(), s);
    let mut prev: Option<bool> = None;
    segments
        .into_iter()
        .map(|seg| {
            let misc = match prev {
                None => test_word(&seg, CharClass::Nonword),
                Some(p) => !p,
            };
            prev = Some(misc);
            (
                if misc {
                    SegmentKind::Misc
                } else {
                    SegmentKind::Word
                },
                seg,
            )
        })
        .collect()
}

/// Port of `ichiran/characters:consecutive-char-groups` (`characters.lisp:273-278`).
///
/// Find every run of consecutive characters in `char_class` within
/// `s[start..end]` and return each as a `(start, end)` pair.
///
/// Positions are *character* offsets, not byte offsets.
pub fn consecutive_char_groups(
    char_class: CharClass,
    s: &str,
    start: usize,
    end: usize,
) -> Vec<(usize, usize)> {
    let re = char_scanners_inner()
        .get(&char_class)
        .expect("char_class is in *char-scanners-inner*");
    let start_byte = nth_char_byte(s, start);
    let end_byte = nth_char_byte(s, end);
    let slice = &s[start_byte..end_byte];
    re.find_iter(slice)
        .map(|m| m.expect("regex iteration"))
        .map(|m| {
            let s_char = start + slice[..m.start()].chars().count();
            let e_char = start + slice[..m.end()].chars().count();
            (s_char, e_char)
        })
        .collect()
}

fn nth_char_byte(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests;
