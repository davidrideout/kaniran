//! Port of `ichiran/characters:basic-split` (`characters.lisp:238-243`).
//!
//! Segment Japanese-mixed text into alternating misc / word runs.
//! Splits with `*basic-split-regex*` (compiled lazily here), tags the
//! first segment via `test_word(.., :nonword)`, and alternates from
//! there — that's how upstream encodes "consecutive segments returned
//! by `:with-registers-p t` alternate between between-match misc text
//! and matched word groups."

use std::sync::OnceLock;

use fancy_regex::Regex;

use super::char_classes::basic_split_regex;
use super::char_classes::CharClass;
use super::split_by_regex::split_by_regex;
use super::test_word::test_word;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Misc,
    Word,
}

static SCANNER: OnceLock<Regex> = OnceLock::new();

fn scanner() -> &'static Regex {
    SCANNER.get_or_init(|| {
        Regex::new(basic_split_regex()).expect("basic-split-regex must compile")
    })
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
                if misc { SegmentKind::Misc } else { SegmentKind::Word },
                seg,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use SegmentKind::*;

    #[test]
    fn alternates_misc_and_word_segments() {
        assert_eq!(
            basic_split("hello 日本 world"),
            vec![
                (Misc, "hello ".to_string()),
                (Word, "日本".to_string()),
                (Misc, " world".to_string()),
            ]
        );
    }

    #[test]
    fn pure_japanese_is_one_word_segment() {
        assert_eq!(basic_split("日本語"), vec![(Word, "日本語".to_string())]);
    }
}
