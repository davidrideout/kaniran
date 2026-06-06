//! Port of `ichiran/dict:word-info` (`dict.lisp:1245`).
//!
//! The runtime descriptor the segmenter produces for each word in a
//! tokenized sentence (a plain CLOS class, not a DAO).

use super::simple_text_class::WordConjugations;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WordInfoType {
    Kanji,
    Kana,
    #[default]
    Gap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordInfoKana {
    Single(String),
    Multi(Vec<Option<WordInfoKana>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordInfoSeq {
    Single(i32),
    Multi(Vec<Option<WordInfoSeq>>),
}

#[derive(Debug, Clone)]
pub struct WordInfo {
    pub kind: WordInfoType,
    pub text: String,
    pub true_text: Option<String>,
    pub kana: Option<WordInfoKana>,
    pub seq: Option<WordInfoSeq>,
    pub conjugations: Option<WordConjugations>,
    pub score: Option<i32>,
    pub components: Vec<WordInfo>,
    pub alternative: bool,
    pub primary: bool,
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub counter: Option<(String, bool)>,
    pub skipped: i32,
}

impl Default for WordInfo {
    fn default() -> Self {
        // Mirrors the upstream slot `:initform`s:
        //   score :initform 0, primary :initform t, skipped :initform 0.
        // The remaining slots (true_text, kana, seq, conjugations,
        // components, alternative, start, end, counter) initform to nil.
        // `:initform 0` only fires when the `:score` initarg is absent;
        // a caller supplying `:score nil` (e.g. word-info-from-segment
        // with a scoreless segment) overrides via `..Default::default()`.
        Self {
            kind: WordInfoType::default(),
            text: String::new(),
            true_text: None,
            kana: None,
            seq: None,
            conjugations: None,
            score: Some(0),
            components: Vec::new(),
            alternative: false,
            primary: true,
            start: None,
            end: None,
            counter: None,
            skipped: 0,
        }
    }
}
