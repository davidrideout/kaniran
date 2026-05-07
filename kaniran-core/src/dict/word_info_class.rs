//! Port of `ichiran/dict:word-info` (`dict.lisp:1245`).
//!
//! Plain CLOS class — the runtime descriptor produced by the
//! segmenter for each word in a tokenized sentence. Not a DAO; built
//! by `word-info-from-segment` / `word-info-from-segment-list` and
//! by `fill-segment-path` (`dict.lisp:1327`, `dict.lisp:1353`,
//! `dict.lisp:1389`). Two consumers populate two distinct shapes:
//! ordinary readings carry single-valued `kana` / `seq`; the
//! "alternative" shape produced by `word-info-from-segment-list`
//! collects `kana` / `seq` across multiple segments into lists.
//!
//! Slot shape (faithful to `dict.lisp:1245-1260`):
//! - `type` — one of `:kanji` / `:kana` / `:gap`. The first two are
//!   propagated from `word-type` on a `simple-text` / `compound-text`
//!   / `counter-text`; `:gap` is set by `fill-segment-path` for the
//!   character runs between matched segments. Modeled as a closed
//!   3-variant enum per CONVENTIONS §4.3.
//! - `text` — the surface text of the word.
//! - `true-text` — the `simple-text`'s `true-text` accessor return
//!   when the source was a `simple-text`; `nil` otherwise.
//! - `kana` — kana reading. Single string for a normal reading; list
//!   of strings for the alternative-segment shape (one per merged
//!   wi). Modeled as [`WordInfoKana`] per CONVENTIONS §4.3.
//! - `seq` — JMdict entry sequence id. Single integer for a normal
//!   reading; list of integers for the alternative-segment shape.
//!   Modeled as [`WordInfoSeq`] per CONVENTIONS §4.3.
//! - `conjugations` — none / `:root` / list of conjugation row ids.
//!   Reuses [`super::simple_text_class::WordConjugations`].
//! - `score` — segment score, integer.
//! - `components` — recursive list of nested word-infos for compound
//!   readings or alternative groups.
//! - `alternative` — flag set by `word-info-from-segment-list` when
//!   the wi merges multiple segments under one `text`.
//! - `primary` — flag distinguishing the head of a compound from its
//!   adjoined parts; defaults to `t`.
//! - `start`, `end` — character offsets of the segment in the source
//!   string; `None` outside `fill-segment-path` flows.
//! - `counter` — when the source was a `counter-text`, a pair
//!   `(value-string, ordinalp)`; otherwise `nil`. Modeled as
//!   `Option<(String, bool)>`.
//! - `skipped` — count of suppressed alternative segments.
//!
//! [`Default`] mirrors the upstream `:initform`s so future construction
//! sites can use `..Default::default()` and inherit the correct
//! `primary: true`, `score: 0`, `skipped: 0` defaults without
//! restating them. The three slots without an initform (`type`,
//! `text`, `kana`) default to inert "no value" placeholders
//! (`Gap` / empty string / empty `Single`); every real callsite
//! overrides them.

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
    Multi(Vec<String>),
}

impl Default for WordInfoKana {
    fn default() -> Self {
        WordInfoKana::Single(String::new())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordInfoSeq {
    Single(i32),
    Multi(Vec<i32>),
}

#[derive(Debug, Clone)]
pub struct WordInfo {
    pub kind: WordInfoType,
    pub text: String,
    pub true_text: Option<String>,
    pub kana: WordInfoKana,
    pub seq: Option<WordInfoSeq>,
    pub conjugations: Option<WordConjugations>,
    pub score: i32,
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
        Self {
            kind: WordInfoType::default(),
            text: String::new(),
            true_text: None,
            kana: WordInfoKana::default(),
            seq: None,
            conjugations: None,
            score: 0,
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
