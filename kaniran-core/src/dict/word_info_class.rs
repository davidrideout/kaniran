//! Port of `ichiran/dict:word-info` (`dict.lisp:1245`).
//!
//! Plain CLOS class — the runtime descriptor produced by the
//! segmenter for each word in a tokenized sentence. Not a DAO; built
//! by `word-info-from-segment` / `word-info-from-segment-list` and
//! by `fill-segment-path` (`dict.lisp:1327`, `dict.lisp:1353`,
//! `dict.lisp:1389`).
//!
//! Slot shape (`dict.lisp:1245-1260`):
//! - `type` — `:kanji` / `:kana` / `:gap` ([`WordInfoType`]).
//! - `text` — surface text.
//! - `true-text` — slot value or nil.
//! - `kana` — single string, list (heterogeneous, may contain
//!   nil-entries or nested lists), or nil. Modeled as
//!   [`Option<WordInfoKana>`] where [`WordInfoKana::Multi`] is
//!   `Vec<Option<WordInfoKana>>` to preserve every position-aligned
//!   element the upstream `(collect (word-info-kana wi))` produces,
//!   including nested kana lists from compound-text children and
//!   nil-entries from children with absent kana.
//! - `seq` — single int, list, or nil. Same recursive
//!   [`Option<WordInfoSeq>`] shape as `kana` — `Multi(Vec<Option<WordInfoSeq>>)`
//!   preserves nil and nested-list elements without flattening.
//! - `conjugations` — none / `:root` / list of conjugation row ids
//!   (`Option<WordConjugations>`).
//! - `score` — integer or nil. Modeled as `Option<i32>` because the
//!   `:score` initarg is set from `(segment-score segment)` which
//!   can be nil; `:initform 0` only fires when the initarg is absent.
//! - `components` — recursive list of word-infos (`Vec<WordInfo>`,
//!   empty for the slot-value nil).
//! - `alternative` — `bool`.
//! - `primary` — `bool` (slot `:initform t`).
//! - `start` / `end` — `Option<usize>` (character offsets).
//! - `counter` — `Option<(String, bool)>` (the `(value-string, ordinalp)`
//!   pair counter-text segments populate).
//! - `skipped` — `i32` (slot `:initform 0`).

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
