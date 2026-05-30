//! Kaniran sidecars — Rust-only lite mirrors of the segmentation
//! structs used inside the `find-best-path` inner loop. No Lisp FQNs.
//!
//! Folds four per-symbol sidecars into one module:
//!
//! - [`KaniLiteSegment`] — pre-computed predicate inputs for a
//!   [`crate::dict::segment::Segment`], read by the
//!   `find-best-path` segfilter / synergy / penalty predicates. One
//!   `KaniLiteSegment` is built per source `Segment` at `find-best-path`
//!   entry and shared via [`Arc`] across every filtered sub-list; the
//!   `source` back-ref lets `find-best-path` materialize a full
//!   `Segment` at exit for each surviving path. Field set is the union
//!   of everything the 16 segfilter + 17 synergy + 2 penalty bodies
//!   actually read off a segment (see session-2 discovery pass in
//!   `find_best_path.md`).
//!
//! - [`KaniLiteSegmentList`] — lite mirror of
//!   [`crate::dict::segment::SegmentList`]; carries
//!   `Arc<KaniLiteSegment>`s and a per-list lite [`KaniLiteTopArray`].
//!   Full materialization happens at `find-best-path` exit, by
//!   deep-cloning each `KaniLiteSegment.source` for the surviving top-K
//!   paths.
//!
//! - [`KaniLiteTopArray`] — lite mirror of
//!   [`crate::dict::segment::TopArray`] +
//!   [`crate::dict::word_info::register_item`] +
//!   [`crate::dict::best_text::get_array`]; logic identical, only the
//!   item type changes.
//!
//! - [`KaniLiteTopArrayItem`] / [`KaniLitePathElement`] — lite mirror
//!   of [`crate::dict::segment::TopArrayItem`]. Payload
//!   is `Arc<[_]>` so the `(register-item ...) / (register-item ...)`
//!   pair at `dict.lisp:1226-1227` becomes a refcount bump instead of a
//!   `Vec` clone (the "Option C" fold-in from `find_best_path.md`
//!   session-2).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::word::KaniWordDispatchEnum;
use crate::dict::segment::SegmentList;
use crate::dict::segment::Segment;
use crate::dict::counters::dispatchers::seq;
use crate::dict::grammar::synergy::Synergy;
use crate::dict::counters::dispatchers::text;
use crate::dict::word_info::WordInfoSeq;

// =========================================================================
// KaniLiteSegment + field-coverage telemetry
// =========================================================================

/// Per-field "ever observed non-default value" counters, bumped in
/// [`KaniLiteSegment::from_segment`]. Used by the find_best_path
/// audit binary to assert every lite field is exercised by the
/// corpus — a zero count after a long run means either (a) the field
/// extraction is broken and always returns default, or (b) the
/// corpus has no rows that should produce a non-default value. Both
/// warrant investigation.
pub static EXERCISED_SEQ_SET: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_CONJ_TYPES: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_CONJ_HAS_NEG: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_KPCL: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_POS: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_HAS_SIMPLE_SEQ: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_NOT_SIMPLE_SEQ: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_IS_COUNTER: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_COMPOUND_END_SEQ: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_COMPOUND_END_TEXT: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_TEXT_NONEMPTY: AtomicU64 = AtomicU64::new(0);
pub static EXERCISED_SCORE_SOME: AtomicU64 = AtomicU64::new(0);

/// Panic if any [`KaniLiteSegment`] field was never exercised by a
/// non-default value during the process lifetime. Call this after a
/// large audit run to prove the corpus-coverage of every predicate
/// input. See the `EXERCISED_*` statics for the per-field counters.
pub fn assert_field_coverage() {
    let probes: &[(&str, &AtomicU64)] = &[
        ("seq_set non-empty", &EXERCISED_SEQ_SET),
        ("conj_types non-empty", &EXERCISED_CONJ_TYPES),
        ("conj_has_neg=true", &EXERCISED_CONJ_HAS_NEG),
        ("kpcl != 0", &EXERCISED_KPCL),
        ("pos != 0", &EXERCISED_POS),
        ("has_simple_seq=true", &EXERCISED_HAS_SIMPLE_SEQ),
        ("has_simple_seq=false", &EXERCISED_NOT_SIMPLE_SEQ),
        ("is_counter=true", &EXERCISED_IS_COUNTER),
        ("compound_end_seq=Some", &EXERCISED_COMPOUND_END_SEQ),
        ("compound_end_text=Some", &EXERCISED_COMPOUND_END_TEXT),
        ("text non-empty", &EXERCISED_TEXT_NONEMPTY),
        ("score=Some", &EXERCISED_SCORE_SOME),
    ];
    let mut report_lines: Vec<String> = Vec::with_capacity(probes.len());
    let mut unexercised: Vec<&str> = Vec::new();
    for (label, counter) in probes {
        let n = counter.load(Ordering::Relaxed);
        report_lines.push(format!("  {label}: {n}"));
        if n == 0 {
            unexercised.push(label);
        }
    }
    eprintln!("KaniLiteSegment field-coverage report:");
    for line in &report_lines {
        eprintln!("{line}");
    }
    if !unexercised.is_empty() {
        panic!(
            "KaniLiteSegment field coverage gap — these fields were never observed non-default: {:?}. \
             Either the field extraction is broken (always returns the default value) or the corpus \
             has no rows that exercise the predicate using this field. Investigate before trusting \
             the audit's pass count.",
            unexercised
        );
    }
}

/// kpcl bits — the `(kanji-or-katakana, primary, common, long)`
/// destructuring used by `def-generic-penalty` callsites
/// (`dict-grammar.lisp:759`). Packed so the noun-kpcl predicates
/// (`(k || l || (p && c))` etc.) become 2-3 bit ops.
pub const KPCL_K: u8 = 1 << 0;
pub const KPCL_P: u8 = 1 << 1;
pub const KPCL_C: u8 = 1 << 2;
pub const KPCL_L: u8 = 1 << 3;

/// POS bitmask — the 9 strings actually tested in any predicate body
/// (`n`, `n-adv`, `n-t`, `adj-na`, `n-suf`, `pn`, `adj-no`, `ctr`,
/// `adv-to`). [`POS_NOUN`] is the precomputed union of the 6 tags
/// filter_is_noun checks (per `NOUN_POS_TAGS` in
/// [`crate::dict::filter_is_noun`]); test it with a single `&` instead
/// of six string compares.
pub const POS_N: u16 = 1 << 0;
pub const POS_N_ADV: u16 = 1 << 1;
pub const POS_N_T: u16 = 1 << 2;
pub const POS_ADJ_NA: u16 = 1 << 3;
pub const POS_N_SUF: u16 = 1 << 4;
pub const POS_PN: u16 = 1 << 5;
pub const POS_ADJ_NO: u16 = 1 << 6;
pub const POS_CTR: u16 = 1 << 7;
pub const POS_ADV_TO: u16 = 1 << 8;
pub const POS_NOUN: u16 =
    POS_N | POS_N_ADV | POS_N_T | POS_ADJ_NA | POS_N_SUF | POS_PN;

#[derive(Debug, Clone)]
pub struct KaniLiteSegment {
    /// Original [`Segment`], retained for reconstruction at
    /// `find-best-path` exit. `Arc` so building a lite slot is O(1)
    /// per segment regardless of `Segment`'s internal size.
    pub source: Arc<Segment>,

    /// Score read by [`crate::dict::segment::get_segment_score`]
    /// off `segments[0]` of a [`KaniLiteSegmentList`].
    pub score: Option<i32>,

    /// `info.seq_set` — used by `filter_in_seq_set`,
    /// `filter_in_seq_set_simple`, `segfilter_dashi`'s inline
    /// predicate, and `filter_is_noun` branch 2 (non-empty check).
    pub seq_set: Vec<i32>,

    /// `info.conj[*].prop.conj_type` — used by
    /// `filter_is_conjugation`.
    pub conj_types: Vec<i32>,

    /// `info.conj` has any cdata with `prop.neg != Some(false)` —
    /// the truthy-not-nil polarity from
    /// `synergy_shika_negative`'s `right_filter`.
    pub conj_has_neg: bool,

    /// kpcl quad packed as [`KPCL_K`] | [`KPCL_P`] | [`KPCL_C`] |
    /// [`KPCL_L`].
    pub kpcl: u8,

    /// POS bitmask over the 9 distinct strings tested by any
    /// predicate body; see the `POS_*` constants. [`POS_NOUN`] is
    /// the precomputed `filter_is_noun` composite.
    pub pos: u16,

    /// `seq(word)` is `Single(_)` — the gate
    /// `filter_in_seq_set_simple` applies before its intersection
    /// test (excludes compound `Multi` and counter `None`).
    pub has_simple_seq: bool,

    /// `word` is the `Counter` variant — branch 2 of
    /// `filter_is_noun`.
    pub is_counter: bool,

    /// Seq of `word.words.last()` when `word` is `Compound` AND the
    /// last child has `seq` = `Single(_)`. `None` otherwise — covers
    /// `filter_is_compound_end`'s gate.
    pub compound_end_seq: Option<i32>,

    /// `get_text(word.words.last())` when `word` is `Compound`.
    /// `None` otherwise — covers `filter_is_compound_end_text`'s
    /// last-child-text test. `Arc<str>` so the cached value survives
    /// per-segment cloning of the lite list.
    pub compound_end_text: Option<Arc<str>>,

    /// Final display text — `source.text` override if set, else
    /// `text(&source.word)`. Used by `segfilter_roku` /
    /// `segfilter_sae`'s `starts_with(char)`,
    /// `segfilter_sukiyoki`'s `ends_with("好き")`, and
    /// `filter_short_kana`'s literal compare against the `except`
    /// list. `Arc<str>` so first-segment text reads on the lite
    /// list don't allocate.
    pub text: Arc<str>,
}

impl KaniLiteSegment {
    /// Build a [`KaniLiteSegment`] from a source [`Segment`],
    /// precomputing every predicate-input field. The source is
    /// `Arc`-shared so the lite copy is O(1) per segment.
    pub fn from_segment(source: Arc<Segment>) -> Self {
        let score = source.score;

        let (seq_set, conj_types, conj_has_neg, kpcl, pos) = match &source.info {
            Some(info) => {
                let conj_types: Vec<i32> = info
                    .conj
                    .iter()
                    .filter_map(|c| c.prop.as_ref().map(|p| p.conj_type))
                    .collect();
                // shika-negative reads `cd.prop.neg != Some(false)` —
                // Lisp truthy = non-nil; only Some(false) (Lisp nil) rejects.
                let conj_has_neg = info
                    .conj
                    .iter()
                    .any(|cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)));
                let (k, p, c, l) = info.kpcl;
                let kpcl_bits = (k as u8) * KPCL_K
                    | (p as u8) * KPCL_P
                    | (c as u8) * KPCL_C
                    | (l as u8) * KPCL_L;
                let mut pos_bits = 0u16;
                for tag in &info.posi {
                    pos_bits |= match tag.as_str() {
                        "n" => POS_N,
                        "n-adv" => POS_N_ADV,
                        "n-t" => POS_N_T,
                        "adj-na" => POS_ADJ_NA,
                        "n-suf" => POS_N_SUF,
                        "pn" => POS_PN,
                        "adj-no" => POS_ADJ_NO,
                        "ctr" => POS_CTR,
                        "adv-to" => POS_ADV_TO,
                        _ => 0,
                    };
                }
                (info.seq_set.clone(), conj_types, conj_has_neg, kpcl_bits, pos_bits)
            }
            None => (Vec::new(), Vec::new(), false, 0, 0),
        };

        let has_simple_seq = matches!(seq(&source.word), Some(WordInfoSeq::Single(_)));
        let is_counter = matches!(source.word, KaniWordDispatchEnum::Counter(_));

        let (compound_end_seq, compound_end_text) = match &source.word {
            KaniWordDispatchEnum::Compound(c) => match c.words.last() {
                Some(last) => {
                    let end_seq = match seq(last) {
                        Some(WordInfoSeq::Single(s)) => Some(s),
                        _ => None,
                    };
                    let end_text = Arc::<str>::from(text(last).as_ref());
                    (end_seq, Some(end_text))
                }
                None => (None, None),
            },
            _ => (None, None),
        };

        let text: Arc<str> = match &source.text {
            Some(t) => Arc::<str>::from(t.as_str()),
            None => Arc::<str>::from(text(&source.word).as_ref()),
        };

        // Field-coverage telemetry — see EXERCISED_* statics + assert_field_coverage.
        if !seq_set.is_empty() {
            EXERCISED_SEQ_SET.fetch_add(1, Ordering::Relaxed);
        }
        if !conj_types.is_empty() {
            EXERCISED_CONJ_TYPES.fetch_add(1, Ordering::Relaxed);
        }
        if conj_has_neg {
            EXERCISED_CONJ_HAS_NEG.fetch_add(1, Ordering::Relaxed);
        }
        if kpcl != 0 {
            EXERCISED_KPCL.fetch_add(1, Ordering::Relaxed);
        }
        if pos != 0 {
            EXERCISED_POS.fetch_add(1, Ordering::Relaxed);
        }
        if has_simple_seq {
            EXERCISED_HAS_SIMPLE_SEQ.fetch_add(1, Ordering::Relaxed);
        } else {
            EXERCISED_NOT_SIMPLE_SEQ.fetch_add(1, Ordering::Relaxed);
        }
        if is_counter {
            EXERCISED_IS_COUNTER.fetch_add(1, Ordering::Relaxed);
        }
        if compound_end_seq.is_some() {
            EXERCISED_COMPOUND_END_SEQ.fetch_add(1, Ordering::Relaxed);
        }
        if compound_end_text.is_some() {
            EXERCISED_COMPOUND_END_TEXT.fetch_add(1, Ordering::Relaxed);
        }
        if !text.is_empty() {
            EXERCISED_TEXT_NONEMPTY.fetch_add(1, Ordering::Relaxed);
        }
        if score.is_some() {
            EXERCISED_SCORE_SOME.fetch_add(1, Ordering::Relaxed);
        }

        Self {
            source,
            score,
            seq_set,
            conj_types,
            conj_has_neg,
            kpcl,
            pos,
            has_simple_seq,
            is_counter,
            compound_end_seq,
            compound_end_text,
            text,
        }
    }
}

// =========================================================================
// KaniLiteSegmentList
// =========================================================================

#[derive(Debug, Clone)]
pub struct KaniLiteSegmentList {
    pub start: usize,
    pub end: usize,
    pub matches: usize,
    pub segments: Vec<Arc<KaniLiteSegment>>,
    pub top: Option<Arc<KaniLiteTopArray>>,
}

impl KaniLiteSegmentList {
    /// Build a [`KaniLiteSegmentList`] from a source [`SegmentList`].
    /// Each contained [`Segment`] is `Arc`-wrapped once and converted
    /// into a [`KaniLiteSegment`] with precomputed predicate inputs.
    pub fn from_segment_list(source: &SegmentList) -> Self {
        let segments = source
            .segments
            .iter()
            .map(|s| Arc::new(KaniLiteSegment::from_segment(Arc::new(s.clone()))))
            .collect();
        Self {
            start: source.start,
            end: source.end,
            matches: source.matches,
            segments,
            top: None,
        }
    }

    /// Materialize a full [`SegmentList`] for export at
    /// `find-best-path` exit. Deep-clones each
    /// `KaniLiteSegment.source` because the lite layer doesn't own
    /// the full `Segment` — it borrows it via `Arc<Segment>`.
    pub fn to_segment_list(&self) -> SegmentList {
        let segments: Vec<Segment> =
            self.segments.iter().map(|s| (*s.source).clone()).collect();
        SegmentList {
            segments,
            start: self.start,
            end: self.end,
            top: None,
            matches: self.matches,
        }
    }
}

/// Sidecar mirror of [`crate::dict::grammar::synergy::make_segment_list_from`]
/// for the lite layer. Builds a fresh [`KaniLiteSegmentList`] with
/// the given filtered segments, inheriting `start` / `end` /
/// `matches` / `top` from `old`. Matches the upstream
/// `(copy-segment-list source)` + slot-overwrite pattern.
pub fn make_kani_lite_segment_list_from(
    old: &KaniLiteSegmentList,
    segments: Vec<Arc<KaniLiteSegment>>,
) -> KaniLiteSegmentList {
    KaniLiteSegmentList {
        segments,
        start: old.start,
        end: old.end,
        top: old.top.clone(),
        matches: old.matches,
    }
}

// =========================================================================
// KaniLiteTopArray
// =========================================================================

#[derive(Debug, Clone)]
pub struct KaniLiteTopArray {
    pub array: Vec<Option<KaniLiteTopArrayItem>>,
    pub count: usize,
}

impl KaniLiteTopArray {
    pub fn new(limit: usize) -> Self {
        Self {
            array: vec![None; limit],
            count: 0,
        }
    }
}

/// Mirror of `register-item` (`dict.lisp:1148`) for the lite item
/// type — same insertion-sort by score with bounded array.
pub fn kani_lite_register_item(
    obj: &mut KaniLiteTopArray,
    score: i32,
    payload: Arc<[KaniLitePathElement]>,
) {
    let mut item: Option<KaniLiteTopArrayItem> = Some(KaniLiteTopArrayItem { score, payload });
    let len = obj.array.len();
    let start = obj.count.min(len);
    let mut idx = start;
    loop {
        let prev_score = if idx > 0 {
            obj.array[idx - 1].as_ref().map(|prev| prev.score)
        } else {
            None
        };
        let done = match prev_score {
            None => true,
            Some(prev) => prev >= score,
        };
        if idx < len {
            obj.array[idx] = if done {
                item.take()
            } else {
                obj.array[idx - 1].take()
            };
        }
        if done {
            break;
        }
        idx -= 1;
    }
    obj.count += 1;
}

/// Mirror of `get-array` (`dict.lisp:1145`) for the lite array — the
/// first `count` slots filled by insertion.
pub fn kani_lite_get_array(top: &KaniLiteTopArray) -> &[Option<KaniLiteTopArrayItem>] {
    let used = top.count.min(top.array.len());
    &top.array[..used]
}

// =========================================================================
// KaniLiteTopArrayItem / KaniLitePathElement
// =========================================================================

#[derive(Debug, Clone)]
pub struct KaniLiteTopArrayItem {
    pub score: i32,
    pub payload: Arc<[KaniLitePathElement]>,
}

/// Lite mirror of [`crate::dict::segment::PathElement`].
/// The inner loop builds these; `find-best-path` reconstructs full
/// [`PathElement`]s from them at exit.
///
/// [`PathElement`]: crate::dict::segment::PathElement
#[derive(Debug, Clone)]
pub enum KaniLitePathElement {
    SegmentList(Arc<KaniLiteSegmentList>),
    Synergy(Synergy),
}
