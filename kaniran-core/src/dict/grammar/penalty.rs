use crate::dict::grammar::segfilter::{penalty_semi_final, penalty_short};
use crate::dict::grammar::synergy::{
    def_generic_synergy_body, filter_in_seq_set, filter_is_pos, DefGenericSynergyOpts, Synergy,
    SYNERGY_LIST,
};
use crate::dict::kani_lite_segment::{KPCL_K, POS_CTR};
use crate::dict::kani_lite_segment_list::KaniLiteSegmentList;
use crate::dict::kani_seg_split_enum::KaniSegSplitEnum;
use std::sync::Arc;

/// Port of `ichiran/dict:*penalty-list*` (`dict-grammar.lisp:964`).
///
/// Registry of penalty functions applied to adjacent segments during
/// scoring.
pub type Penalty = fn(&KaniLiteSegmentList, &KaniLiteSegmentList) -> Option<Synergy>;

pub static PENALTY_LIST: &[Penalty] = &[penalty_semi_final, penalty_short];

/// Port of `ichiran/dict:synergy-no-toori` (`dict-grammar.lisp:944`).
///
/// "no toori" synergy: binds の (seq 1469800) on the left to 通り
/// (seq 1432920) on the right.
pub fn synergy_no_toori(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(&[1469800]),
        filter_in_seq_set(&[1432920]),
        &DefGenericSynergyOpts {
            description: Some("no toori"),
            connector: " ",
            score: 50,
        },
    )
}

/// Port of `ichiran/dict:synergy-oki` (`dict-grammar.lisp:951`).
///
/// Synergy binding a counter (pos "ctr") on the left to おき
/// (seqs 2854117/2084550) on the right.
pub fn synergy_oki(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:952 (filter-is-pos ("ctr") t)
        filter_is_pos(POS_CTR, |_k, _p, _c, _l| true),
        filter_in_seq_set(&[2854117, 2084550]),
        &DefGenericSynergyOpts {
            description: None,
            connector: "",
            score: 20,
        },
    )
}

/// Port of `ichiran/dict:get-synergies` (`dict-grammar.lisp:957`).
///
/// Gathers every synergy that fires between `segment_list_left` and
/// `segment_list_right`, each as a 3-element path
/// `(seg_right, synergy, seg_left)`.
pub fn get_synergies(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
) -> Vec<KaniSegSplitEnum> {
    // Every synergy in SYNERGY_LIST only fires when the two segment-lists
    // abut: each one bails unless left.end == right.start. When they don't
    // touch, the whole list is empty — decide that once here instead of
    // calling all 17 rules to each rediscover it.
    if segment_list_left.end != segment_list_right.start {
        return vec![];
    }
    let mut out = vec![];
    for synergy_fn in SYNERGY_LIST {
        // dict-grammar.lisp:958-959 (`nconc (funcall fn l r)`)
        for (right_sl, syn, left_sl) in synergy_fn(segment_list_left, segment_list_right) {
            out.push(KaniSegSplitEnum::WithSynergy {
                right: right_sl,
                synergy: syn,
                left: left_sl,
            });
        }
    }
    out
}

/// Port of `ichiran/dict:def-generic-penalty` (`dict-grammar.lisp:972-984`).
///
/// Shared body for the generic penalty definers: when the two adjacent
/// segment-lists pass their left/right tests (and abut, if `serial`),
/// emit a [`Synergy`] carrying the penalty's description, connector,
/// and score.
pub struct DefGenericPenaltyOpts<'a> {
    pub serial: bool,
    pub description: &'a str,
    pub score: i32,
    pub connector: &'a str,
}

pub fn def_generic_penalty_body(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
    test_left: impl Fn(&KaniLiteSegmentList) -> bool,
    test_right: impl Fn(&KaniLiteSegmentList) -> bool,
    opts: &DefGenericPenaltyOpts<'_>,
) -> Option<Synergy> {
    let start = segment_list_left.end;
    let end = segment_list_right.start;
    // dict-grammar.lisp:978-980 (and (if serial (= start end) t) (funcall test-left ...) (funcall test-right ...))
    if (!opts.serial || start == end)
        && test_left(segment_list_left)
        && test_right(segment_list_right)
    {
        // dict-grammar.lisp:981-984 (make-synergy :start :end :description :connector :score)
        Some(Synergy {
            description: Some(opts.description.to_string()),
            connector: Some(opts.connector.to_string()),
            score: opts.score,
            start,
            end,
        })
    } else {
        None
    }
}

/// Port of `ichiran/dict:filter-short-kana` (`dict-grammar.lisp:986-994`).
///
/// Returns a predicate matching a segment-list at most `len` long whose
/// first segment is non-kanji and not in the `except` list.
pub fn filter_short_kana(
    len: usize,
    except: &'static [&'static str],
) -> impl Fn(&KaniLiteSegmentList) -> bool {
    move |segment_list| -> bool {
        let seg = match segment_list.segments.first() {
            Some(s) => s,
            None => return false,
        };
        if segment_list.end - segment_list.start > len {
            return false;
        }
        if seg.kpcl & KPCL_K != 0 {
            return false;
        }
        if !except.is_empty() && except.iter().any(|e| *e == seg.text.as_ref()) {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests;
