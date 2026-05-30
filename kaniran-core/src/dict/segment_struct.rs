//! Port of `ichiran/dict:segment` (`dict.lisp:674`).
//!
//! In-memory record for one candidate word match at a fixed
//! `(start, end)` slice — the unit `join-substring-words` produces
//! and `gen-score` decorates with score and info plist before the
//! find-best-path scoring loop runs. One [`Segment`] per
//! `(slice, word)` pair: each substring may yield several segments
//! (one per dictionary reading) which `cull-segments` then filters
//! by score cutoff.
//!
//! Slot shape (`(defstruct segment start end word (score nil) (info nil) (top nil) (text nil))`):
//! - `start` / `end` — character offsets of the slice in the source
//!   string; always set by `make-segment` callers (the bare
//!   `defstruct` defaults are `nil`, but every callsite passes both).
//! - `word` — the matched dictionary reading; per `slot_types.csv`
//!   this resolves to the [`KaniWordDispatchEnum`] union the
//!   segmenter dispatches over.
//! - `score` — populated by `gen-score` (`dict.lisp:986`) once
//!   `calc-score` runs; [`None`] until then.
//! - `info` — the property-list value `calc-score` returns alongside
//!   the score (`dict.lisp:976-980`). Modeled as a typed sidecar
//!   [`KaniSegmentInfo`] capturing the six plist keys calc-score
//!   writes (`:posi`, `:seq-set`, `:conj`, `:common`, `:score-info`,
//!   `:kpcl`) per CONVENTIONS §4.3. [`None`] until `gen-score` runs.
//! - `top` — backing [`TopArray`] populated during `find-best-path`'s
//!   inner loop (`dict.lisp:1192-1230`); per `slot_types.csv`. Cleared
//!   to [`None`] when the path search completes.
//! - `text` — lazy cache for `(text word)` produced by `get-text`
//!   (`dict.lisp:677-679`).
//!
//! Divergences from Lisp:
//! - `start` / `end` are [`usize`] rather than `Option<usize>`. Every
//!   `make-segment` call passes both; modeling as `Option` would
//!   force every reader to unwrap a value that's never absent in
//!   practice. Documented because the bare `defstruct` slot defaults
//!   to `nil`.
//! - The `info` plist's `:common` key carries the Lisp sentinel
//!   `:null` for "explicitly not common"; the Rust port collapses
//!   that to `None`. The distinction between "info plist absent" and
//!   "info plist present, common unspecified" is preserved by the
//!   outer `Option<KaniSegmentInfo>`.

use super::conj_data_struct::ConjData;
use super::kani::KaniWordDispatchEnum;
use super::top_array_class::TopArray;

#[derive(Debug, Clone)]
pub struct Segment {
    pub start: usize,
    pub end: usize,
    pub word: KaniWordDispatchEnum,
    pub score: Option<i32>,
    pub info: Option<KaniSegmentInfo>,
    pub top: Option<TopArray>,
    pub text: Option<String>,
}

impl Segment {
    /// `get-text` override — `dict.lisp:677-679`:
    ///
    /// ```lisp
    /// (defmethod get-text ((segment segment))
    ///   (or (segment-text segment)
    ///       (setf (segment-text segment) (text (segment-word segment)))))
    /// ```
    ///
    /// Lazy memoization: returns the cached [`Segment::text`] when
    /// already populated, otherwise computes
    /// `text(segment.word)` (via [`crate::dict::text::text`]),
    /// stores it, and returns a borrow. Mirrors upstream's `setf`
    /// — repeated calls are O(1) after the first.
    pub fn get_text(&mut self) -> &str {
        if self.text.is_none() {
            let t = crate::dict::text::text(&self.word).into_owned();
            self.text = Some(t);
        }
        self.text.as_deref().unwrap()
    }
}

/// Sidecar (no Lisp FQN). Typed model of the property-list value
/// `calc-score` returns and `gen-score` stores in [`Segment::info`]
/// (`dict.lisp:976-980`). The Lisp slot type is `t`; the port pins
/// the six plist keys calc-score actually writes so downstream
/// consumers (the `def-generic-penalty` / `defsynergy` machinery in
/// `dict-grammar.lisp`, `cull-segments` in `dict.lisp:1024`) get a
/// checked field rather than a `getf` against an untyped list.
///
/// Closed-variant note (CONVENTIONS §4.3): the field set is
/// exhaustive against the current `calc-score` terminal `let`. If a
/// future calc-score change adds a `(setf (getf info :NEW-KEY) …)`
/// or extends the terminal `let`, this struct must grow a matching
/// field — silent drop would corrupt the segment.info contract that
/// every penalty / synergy rule reads.
#[derive(Debug, Clone)]
pub struct KaniSegmentInfo {
    /// `:posi` — part-of-speech tags. `("ctr",)` for counter
    /// readings; otherwise `(get-non-arch-posi seq-set)`.
    pub posi: Vec<String>,
    /// `:seq-set` — JMdict entry-seq plus every `from`-seq the
    /// reading conjugates from. `(cons seq conj-of)` in the
    /// non-counter branch.
    pub seq_set: Vec<i32>,
    /// `:conj` — conjugation records attached to this reading; one
    /// [`ConjData`] per `conj-prop` row. Empty when the reading is
    /// not a conjugated form.
    pub conj: Vec<ConjData>,
    /// `:common` — commonness rank. Lisp value is
    /// `(and common-p common-of)` where `common-of` may be `:null`
    /// (explicit "not common") or an integer. The port collapses
    /// `nil` (no common-p) and `:null` to [`None`]; integer
    /// commonness is [`Some`].
    pub common: Option<i32>,
    /// `:score-info` — `(prop-score, kanji-break, use-length-bonus,
    /// split-info)` telemetry tuple emitted by calc-score. Modeled
    /// as the typed sidecar [`KaniScoreInfo`].
    pub score_info: KaniScoreInfo,
    /// `:kpcl` — `(kanji-or-katakana, primary, common, long)`
    /// destructured by `def-generic-penalty` macro callsites in
    /// `dict-grammar.lisp:759`.
    pub kpcl: (bool, bool, bool, bool),
}

/// Sidecar (no Lisp FQN). The four-element list `calc-score` builds
/// at `dict.lisp:979` and stores under [`KaniSegmentInfo::score_info`].
/// Each field corresponds to one position in the Lisp
/// `(list prop-score kanji-break use-length-bonus split-info)`.
///
/// No `Default` derive — every instance must be the result of a real
/// `calc-score` call. A zero-valued default would let a careless
/// caller fabricate score-info that doesn't correspond to any
/// upstream Lisp state.
#[derive(Debug, Clone)]
pub struct KaniScoreInfo {
    pub prop_score: i32,
    /// `kanji-break` argument to calc-score; a list of character
    /// positions where a kanji boundary falls inside the slice, or
    /// empty when no break applies.
    pub kanji_break: Vec<usize>,
    pub use_length_bonus: i32,
    /// `split-info` — populated by the split-handling branch of
    /// calc-score (`dict.lisp:939-974`): a single integer
    /// (`:score` split), [`None`] (`:pscore` / no split), or the
    /// `(score-mod-split . part-scores)` list (multi-part split).
    pub split_info: KaniSplitInfo,
}

#[derive(Debug, Clone)]
pub enum KaniSplitInfo {
    None,
    Score(i32),
    Parts {
        score_mod: i32,
        part_scores: Vec<i32>,
    },
}
