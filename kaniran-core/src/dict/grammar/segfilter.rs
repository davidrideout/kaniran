use crate::dict::errata::semi_final_prt;
use crate::dict::grammar::penalty::{
    def_generic_penalty_body, filter_short_kana, DefGenericPenaltyOpts, PENALTY_LIST,
};
use crate::dict::grammar::synergy::{
    filter_in_seq_set, filter_in_seq_set_simple, filter_is_compound_end,
    filter_is_compound_end_text, filter_is_conjugation, Synergy, NOUN_PARTICLES,
};
use crate::dict::kani_lite_segment::KaniLiteSegment;
use crate::dict::kani_lite_segment_list::{make_kani_lite_segment_list_from, KaniLiteSegmentList};
use crate::dict::kani_seg_split_enum::KaniSegSplitEnum;
use smallvec::SmallVec;
use std::sync::Arc;

/// A `(seg-left, seg-right)` candidate pair threaded through the
/// segfilters. Inlined at capacity 1 because the overwhelmingly common
/// outcome is a single pass-through pair that never spills to the heap.
pub type SegfilterSplits =
    SmallVec<[(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>); 1]>;

/// Port of `ichiran/dict:penalty-short` (`dict-grammar.lisp:996-1001`).
///
/// Scoring penalty (-9) applied when both adjacent segments are short
/// single-kana words (the right side excepting と).
pub fn penalty_short(l: &KaniLiteSegmentList, r: &KaniLiteSegmentList) -> Option<Synergy> {
    def_generic_penalty_body(
        l,
        r,
        filter_short_kana(1, &[]),
        filter_short_kana(1, &["と"]),
        &DefGenericPenaltyOpts {
            serial: false,
            description: "short",
            score: -9,
            connector: " ",
        },
    )
}

/// Port of `ichiran/dict:penalty-semi-final` (`dict-grammar.lisp:1003-1009`).
///
/// Scoring penalty (-15) applied when the left segment is a semi-final
/// particle that isn't actually in final position.
pub fn penalty_semi_final(l: &KaniLiteSegmentList, r: &KaniLiteSegmentList) -> Option<Synergy> {
    let f = filter_in_seq_set(semi_final_prt());
    def_generic_penalty_body(
        l,
        r,
        // dict-grammar.lisp:1004-1006 (test-left lambda over (apply 'filter-in-seq-set *semi-final-prt*))
        |sl| sl.segments.iter().any(&f),
        // dict-grammar.lisp:1007 (test-right = (constantly t))
        |_| true,
        &DefGenericPenaltyOpts {
            serial: true,
            description: "semi-final not final",
            score: -15,
            connector: " ",
        },
    )
}

/// Port of `ichiran/dict:*segfilter-list*` (`dict-grammar.lisp:1024`).
///
/// Registry of segfilter functions applied to adjacent segment pairs.
/// `None` means pass-through — the input pair flows on unchanged
/// (upstream's `(list (list seg-left seg-right))` identity result,
/// returned without allocating it); `Some` carries the rewritten pairs.
pub type SegFilter = fn(
    Option<&Arc<KaniLiteSegmentList>>,
    &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>>;

pub static SEGFILTER_LIST: &[SegFilter] = &[
    segfilter_mononi,
    segfilter_honorific,
    segfilter_dekiru,
    segfilter_dashi,
    segfilter_totte,
    segfilter_toomou,
    segfilter_nohayamete,
    segfilter_janai,
    segfilter_sae,
    segfilter_roku,
    segfilter_sukiyoki,
    segfilter_badend,
    segfilter_wokarasu,
    segfilter_n,
    segfilter_tsu_iru,
    segfilter_aux_verb,
];

/// Port of `ichiran/dict:get-penalties` (`dict-grammar.lisp:1011-1016`).
///
/// Walks [`PENALTY_LIST`] in order, returning the first penalty that
/// fires between `seg_left` and `seg_right`. Result is the
/// `(seg_right, penalty, seg_left)` shape when a penalty matched, else
/// the plain `(seg_right, seg_left)` shape — both inline in
/// [`KaniSegSplitEnum`], so the dominant no-penalty case stays off the
/// heap.
///
/// [`PENALTY_LIST`]: crate::dict::grammar::penalty::PENALTY_LIST
pub fn get_penalties(
    seg_left: &Arc<KaniLiteSegmentList>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> KaniSegSplitEnum {
    for penalty_fn in PENALTY_LIST {
        if let Some(penalty) = penalty_fn(seg_left, seg_right) {
            // dict-grammar.lisp:1015 (`(return (list seg-right penalty seg-left))`)
            return KaniSegSplitEnum::WithSynergy {
                right: Arc::clone(seg_right),
                synergy: penalty,
                left: Arc::clone(seg_left),
            };
        }
    }
    // dict-grammar.lisp:1016 (`(finally (return (list seg-right seg-left)))`)
    KaniSegSplitEnum::Plain {
        right: Arc::clone(seg_right),
        left: Arc::clone(seg_left),
    }
}

/// Port of `ichiran/dict:classify` (`dict-grammar.lisp:1032`).
///
/// Partitions a list into elements that satisfy `filter` and elements
/// that do not, preserving the original order in each output.
pub fn classify<T, F>(filter: F, list: &[T]) -> (Vec<T>, Vec<T>)
where
    T: Clone,
    F: Fn(&T) -> bool,
{
    let mut yep: Vec<T> = Vec::new();
    let mut nope: Vec<T> = Vec::new();
    for element in list {
        if filter(element) {
            yep.push(element.clone());
        } else {
            nope.push(element.clone());
        }
    }
    (yep, nope)
}

/// Port of `ichiran/dict:def-segfilter-must-follow`
/// (`dict-grammar.lisp:1039-1069`).
///
/// Shared body for the "must-follow" segment filters: partitions the
/// right segment-list by `filter_right` and recombines it with the
/// left list so that segments matching `filter_right` are kept only
/// when preceded by a left segment matching `filter_left`. Returns
/// `None` for the pass-through outcomes (the [`SegFilter`] contract) so
/// the dominant nothing-matched case allocates nothing.
pub fn def_segfilter_must_follow_body<FL, FR>(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
    filter_left: FL,
    filter_right: FR,
    allow_first: bool,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>>
where
    FL: Fn(&Arc<KaniLiteSegment>) -> bool,
    FR: Fn(&Arc<KaniLiteSegment>) -> bool,
{
    // dict-grammar.lisp:1048-1049 (cond clause 1) — pass through when
    // nothing on the right matches, or when allow-first and l=nil.
    // Tested BEFORE partitioning: this is the overwhelmingly common
    // outcome, and classify clones every segment it walks.
    if (allow_first && seg_left.is_none())
        || !seg_right.segments.iter().any(&filter_right)
    {
        return None;
    }
    let (sat_r, con_r) = classify(filter_right, &seg_right.segments);

    // dict-grammar.lisp:1050-1054 (cond clause 2) — l absent or
    // non-adjacent: keep only the non-matching right segments.
    let l = match seg_left {
        None => {
            return Some(if con_r.is_empty() {
                Vec::new()
            } else {
                vec![(
                    None,
                    Arc::new(make_kani_lite_segment_list_from(seg_right, con_r)),
                )]
            });
        }
        Some(l) if l.end != seg_right.start => {
            return Some(if con_r.is_empty() {
                Vec::new()
            } else {
                vec![(
                    Some(Arc::clone(l)),
                    Arc::new(make_kani_lite_segment_list_from(seg_right, con_r)),
                )]
            });
        }
        Some(l) => l,
    };

    // dict-grammar.lisp:1055-1069 (t branch) — l adjacent to r:
    // classify l and emit the satisfies × satisfies pair (prepended)
    // alongside the unchanged-l × contradicts-r pair. The all-satisfy
    // case returns before partitioning, same as the right side above.
    if l.segments.iter().all(&filter_left) {
        return None;
    }
    let sat_l: Vec<Arc<KaniLiteSegment>> = l
        .segments
        .iter()
        .filter(|segment| filter_left(segment))
        .cloned()
        .collect();

    let mut result: Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> = Vec::new();
    if !con_r.is_empty() {
        result.push((
            Some(Arc::clone(l)),
            Arc::new(make_kani_lite_segment_list_from(seg_right, con_r)),
        ));
    }
    if !sat_l.is_empty() {
        // dict-grammar.lisp:1064 (push) — prepend the satisfies pair.
        result.insert(
            0,
            (
                Some(Arc::new(make_kani_lite_segment_list_from(l, sat_l))),
                Arc::new(make_kani_lite_segment_list_from(seg_right, sat_r)),
            ),
        );
    }
    Some(result)
}

/// Port of `ichiran/dict:*aux-verbs*` (`dict-grammar.lisp:1072`).
pub static AUX_VERBS: &[i32] = &[
    1342560, // 初める/そめる
];

/// Port of `ichiran/dict:segfilter-aux-verb` (`dict-grammar.lisp:1077`).
///
/// Keeps a left/right segment pair when the right segment is a
/// conjugation type 13 verb that follows one of the auxiliary verbs.
pub fn segfilter_aux_verb(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        filter_is_conjugation(13),
        filter_in_seq_set(AUX_VERBS),
        false,
    )
}

/// Port of `ichiran/dict:segfilter-tsu-iru` (`dict-grammar.lisp:1081`).
///
/// Keeps an いる (1577980) right segment only when the preceding left
/// segment is not つ (2221640).
const TSU_SEQ: i32 = 2221640;
const IRU_SEQS: &[i32] = &[1577980];

pub fn segfilter_tsu_iru(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_in_seq_set(&[TSU_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(IRU_SEQS),
        true,
    )
}

/// Port of `ichiran/dict:segfilter-n` (`dict-grammar.lisp:1086`).
///
/// Keeps a ん/んだ right segment only when the preceding left segment
/// is not a noun particle.
const N_SEQS: &[i32] = &[2139720, 2849370, 2849387];

pub fn segfilter_n(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_in_seq_set_simple(NOUN_PARTICLES);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(N_SEQS),
        true,
    )
}

/// Port of `ichiran/dict:segfilter-wokarasu` (`dict-grammar.lisp:1091`).
///
/// Keeps a からす (2087020) right segment only when the preceding left
/// segment is を (2029010) — the only must-follow segfilter whose left
/// filter is not complemented.
const WO_SEQ: i32 = 2029010;
const KARASU_SEQS: &[i32] = &[2087020];

pub fn segfilter_wokarasu(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    // Left filter is (filter-in-seq-set 2029010) — no complement here
    // unlike most segfilters; sat-l = matches を, con-l = does not.
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        filter_in_seq_set(&[WO_SEQ]),
        filter_in_seq_set(KARASU_SEQS),
        false,
    )
}

/// Port of `ichiran/dict:segfilter-badend` (`dict-grammar.lisp:1095`).
///
/// Drops right segments whose compound ends in one of the spurious
/// tails ちゃい/いか/とか/とき/い (the left filter is always false).
static BADEND_TEXTS: &[&str] = &["ちゃい", "いか", "とか", "とき", "い"];

pub fn segfilter_badend(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    // Left filter (constantly nil) — sat-l is always empty for this
    // segfilter so the prepended sat-pair branch in the macro
    // expansion is unreachable in practice.
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |_: &Arc<KaniLiteSegment>| false,
        filter_is_compound_end_text(BADEND_TEXTS),
        false,
    )
}

/// Port of `ichiran/dict:segfilter-sukiyoki` (`dict-grammar.lisp:1101`).
///
/// Drops a spurious 好き literary-adjective conjugation that some
/// adj-ix words ending in 好い produce (the left filter is always false).
/// - `+conj-adjective-literary+` (`dict-errata.lisp:1240`) is a plain
///   `defconstant` with value `54`; no standalone Rust port file
///   exists for it (`_star_weak_conj_forms_star_.rs` references the
///   bare literal). Inlined as `54` with the same comment annotation.
const SUKI_SUFFIX: &str = "好き";

pub fn segfilter_sukiyoki(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let conj_filter = filter_is_conjugation(54); // +conj-adjective-literary+
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |_: &Arc<KaniLiteSegment>| false,
        // dict-grammar.lisp:1103 (lambda) — and conj-type=54 ends-with "好き".
        |s| conj_filter(s) && s.text.ends_with(SUKI_SUFFIX),
        false,
    )
}

/// Port of `ichiran/dict:segfilter-roku` (`dict-grammar.lisp:1112`).
///
/// Keeps a right segment whose text starts with く only when the
/// preceding left segment does not end in いろ.
const IRO_TEXTS: &[&str] = &["いろ"];
const KU_CHAR: char = 'く';

pub fn segfilter_roku(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_is_compound_end_text(IRO_TEXTS);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        // dict-grammar.lisp:1114 (lambda) — (starts-with #\く (get-text segment)).
        |s| s.text.starts_with(KU_CHAR),
        true,
    )
}

/// Port of `ichiran/dict:segfilter-sae` (`dict-grammar.lisp:1117`).
///
/// Keeps a right segment whose text starts with え only when the
/// preceding left segment is not a さえ (2029120) compound end.
const SAE_SEQ: i32 = 2029120;
const E_CHAR: char = 'え';

pub fn segfilter_sae(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_is_compound_end(&[SAE_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        // dict-grammar.lisp:1119 (lambda) — (starts-with #\え (get-text segment)).
        |s| s.text.starts_with(E_CHAR),
        true,
    )
}

/// Port of `ichiran/dict:segfilter-janai` (`dict-grammar.lisp:1122`).
///
/// Keeps a じゃない/ではない right segment only when the preceding
/// left segment is not a は compound end.
const HA_SEQ: i32 = 2028920;
const JANAI_SEQS: &[i32] = &[1529520, 1296400, 2139720];

pub fn segfilter_janai(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_is_compound_end(&[HA_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(JANAI_SEQS),
        true,
    )
}

/// Port of `ichiran/dict:segfilter-nohayamete` (`dict-grammar.lisp:1127`).
///
/// Keeps a 早めて (1601080) right segment only when the preceding left
/// segment is not の (1469800).
const NO_SEQ: i32 = 1469800;
const HAYAMETE_SEQS: &[i32] = &[1601080];

pub fn segfilter_nohayamete(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_in_seq_set(&[NO_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(HAYAMETE_SEQS),
        true,
    )
}

/// Port of `ichiran/dict:*honorifics*` (`dict-grammar.lisp:1156`).
pub static HONORIFICS: &[i32] = &[
    1247260, // 君
];

/// Port of `ichiran/dict:segfilter-toomou` (`dict-grammar.lisp:1132`).
///
/// Splits と before 思う/言う: keeps a 思う/言う right segment only when
/// the preceding left segment is not 何だと (2837117).
const NANDATO_SEQ: i32 = 2837117;
const OMOU_IU_SEQS: &[i32] = &[1589350, 1587040];

pub fn segfilter_toomou(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_in_seq_set(&[NANDATO_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(OMOU_IU_SEQS),
        true,
    )
}

/// Port of `ichiran/dict:segfilter-totte` (`dict-grammar.lisp:1138`).
///
/// Keeps a とって (2086960) right segment only when the preceding left
/// segment is not と (1008490).
const TO_SEQ: i32 = 1008490;
const TOTTE_SEQS: &[i32] = &[2086960];

pub fn segfilter_totte(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_in_seq_set(&[TO_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(TOTTE_SEQS),
        true,
    )
}

/// Port of `ichiran/dict:segfilter-dashi` (`dict-grammar.lisp:1143`).
///
/// Keeps a する/して right segment only when the preceding left
/// segment is not だ (or is で).
const SEQ_DA: i32 = 2089020;
const SEQ_DE: i32 = 2028980;
const SURU_SETE_SEQS: &[i32] = &[1157170, 2424740, 1305070];

fn filter_left(segment: &Arc<KaniLiteSegment>) -> bool {
    // dict-grammar.lisp:1144 (lambda &aux seq-set ...)
    !segment.seq_set.contains(&SEQ_DA) || segment.seq_set.contains(&SEQ_DE)
}

pub fn segfilter_dashi(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        filter_left,
        filter_in_seq_set(SURU_SETE_SEQS),
        true,
    )
}

/// Port of `ichiran/dict:segfilter-dekiru` (`dict-grammar.lisp:1150`).
///
/// Keeps a 来る/来てる right segment only when the preceding left
/// segment is not 出.
const DE_SEQS: &[i32] = &[1896380, 2422860];
const KURU_SEQS: &[i32] = &[2830009, 1547720];

pub fn segfilter_dekiru(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_in_seq_set(DE_SEQS);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(KURU_SEQS),
        true,
    )
}

/// Conjugation type the sukiyoki filter keys on (`+conj-adjective-literary+`,
/// `dict-errata.lisp:1240`); also used as a gate trigger by
/// [`seg_right_may_fire`].
const SUKIYOKI_CONJ_TYPE: i32 = 54;

/// Sorted union of every segfilter's right-side trigger seq, built once
/// from the per-filter constants themselves — not a re-typed literal
/// list — so the gate can't drift from the seqs the filters actually
/// test. Each constant is the `filter_right` seq set of the segfilter
/// named alongside it; the four non-seq filters (sukiyoki conj-type,
/// roku/sae text prefix, badend compound-end) are handled directly in
/// [`seg_right_may_fire`]. Add a filter's constant here when adding a
/// seq-based segfilter to [`SEGFILTER_LIST`].
fn segfilter_trigger_seqs() -> &'static [i32] {
    use std::sync::OnceLock;
    static SEQS: OnceLock<Vec<i32>> = OnceLock::new();
    SEQS.get_or_init(|| {
        let mut seqs: Vec<i32> = Vec::new();
        for set in [
            MONONI_SEQS,
            HONORIFICS,
            KURU_SEQS,
            SURU_SETE_SEQS,
            TOTTE_SEQS,
            OMOU_IU_SEQS,
            HAYAMETE_SEQS,
            JANAI_SEQS,
            KARASU_SEQS,
            N_SEQS,
            IRU_SEQS,
            AUX_VERBS,
        ] {
            seqs.extend_from_slice(set);
        }
        seqs.sort_unstable();
        seqs.dedup();
        seqs
    })
}

/// True when some segment of `seg_right` could satisfy at least one
/// segfilter's right-side test. When false, every filter short-circuits
/// to `None` (the `!any(filter_right)` arm of
/// [`def_segfilter_must_follow_body`]), so [`apply_segfilters`] can
/// return the identity split without running the loop. Conservative: the
/// conj-type arm over-approximates sukiyoki (which also wants a 好き
/// suffix), and the text arm covers roku (く) and sae (え) by first char.
fn seg_right_may_fire(seg_right: &KaniLiteSegmentList) -> bool {
    let triggers = segfilter_trigger_seqs();
    seg_right.segments.iter().any(|segment| {
        segment
            .seq_set
            .iter()
            .any(|seq| triggers.binary_search(seq).is_ok())
            || segment.conj_types.contains(&SUKIYOKI_CONJ_TYPE)
            || segment.text.starts_with(E_CHAR)
            || segment.text.starts_with(KU_CHAR)
            || segment
                .compound_end_text
                .as_deref()
                .is_some_and(|end| BADEND_TEXTS.contains(&end))
    })
}

/// Port of `ichiran/dict:apply-segfilters` (`dict-grammar.lisp:1170`).
///
/// Threads `(seg-left, seg-right)` through each filter in
/// [`SEGFILTER_LIST`] in order. Each filter returns a list of
/// `(seg-left, seg-right)` candidates; the union of those candidates
/// becomes the input to the next filter.
///
/// [`SEGFILTER_LIST`]: crate::dict::grammar::segfilter::SEGFILTER_LIST
pub fn apply_segfilters(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> SegfilterSplits {
    // Fast path: if no segment of seg_right can satisfy any segfilter's
    // right-side test, every filter returns None and the pair passes
    // through unchanged. Decide that in one scan instead of running all
    // 16 filters. Byte-identical to the loop below (see seg_right_may_fire).
    if !seg_right_may_fire(seg_right) {
        return SmallVec::from_buf([(seg_left.cloned(), Arc::clone(seg_right))]);
    }
    // dict-grammar.lisp:1171 (`with splits = (list (list seg-left seg-right))`)
    let mut splits: SegfilterSplits =
        SmallVec::from_buf([(seg_left.cloned(), Arc::clone(seg_right))]);
    for segfilter in SEGFILTER_LIST {
        // dict-grammar.lisp:1173-1175 (inner loop nconc-ing each
        // filter's output across the current splits). The next
        // generation is materialized only once some pair is actually
        // rewritten; until then pass-through (None) results leave
        // `splits` untouched.
        let mut next: Option<SegfilterSplits> = None;
        for (index, (left, right)) in splits.iter().enumerate() {
            match segfilter(left.as_ref(), right) {
                None => {
                    if let Some(next_splits) = next.as_mut() {
                        next_splits.push((left.clone(), Arc::clone(right)));
                    }
                }
                Some(rewritten) => {
                    // First rewrite: start the new generation with the
                    // pass-through pairs already walked.
                    next.get_or_insert_with(|| splits[..index].iter().cloned().collect())
                        .extend(rewritten);
                }
            }
        }
        if let Some(next_splits) = next {
            splits = next_splits;
        }
    }
    splits
}

/// Port of `ichiran/dict:segfilter-honorific` (`dict-grammar.lisp:1160`).
///
/// Keeps an honorific (君) right segment only when the preceding left
/// segment is not a noun particle.
pub fn segfilter_honorific(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_in_seq_set(NOUN_PARTICLES);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(HONORIFICS),
        false,
    )
}

/// Port of `ichiran/dict:segfilter-mononi` (`dict-grammar.lisp:1165`).
///
/// Keeps a に (1009980) right segment only when the preceding left
/// segment is not もの (2028940).
const MO_SEQ: i32 = 2028940;
const MONONI_SEQS: &[i32] = &[1009980];

pub fn segfilter_mononi(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Option<Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>> {
    let inner = filter_in_seq_set(&[MO_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(MONONI_SEQS),
        true,
    )
}

#[cfg(test)]
mod tests;
