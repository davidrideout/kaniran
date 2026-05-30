//! Port of the dict-grammar.lisp synergy layer.

pub use synergy_struct_inner::*;
pub use _star_synergy_list_star__inner::*;
pub use make_segment_list_from_inner::*;
pub use def_generic_synergy_macro_inner::*;
pub use synergy_noun_particle_inner::*;
pub use synergy_noun_da_inner::*;
pub use synergy_no_da_inner::*;
pub use synergy_sou_nanda_inner::*;
pub use synergy_no_adjectives_inner::*;
pub use synergy_na_adjectives_inner::*;
pub use synergy_to_adverbs_inner::*;
pub use synergy_suffix_chu_inner::*;
pub use synergy_suffix_tachi_inner::*;
pub use synergy_suffix_buri_inner::*;
pub use synergy_suffix_sei_inner::*;
pub use synergy_o_prefix_inner::*;
pub use synergy_kanji_prefix_inner::*;
pub use synergy_shicha_ikenai_inner::*;
pub use synergy_shika_negative_inner::*;
pub use synergy_no_toori_inner::*;
pub use synergy_oki_inner::*;
pub use get_synergies_inner::*;

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_struct_inner {
// `(defstruct synergy description connector score start end)` has no
// `:initform`s, so every slot defaults to nil. The `description` and
// `connector` slots get bound to strings by most upstream
// `def-generic-synergy` callsites, but a few register synergies that
// leave them nil (encountered in the wi-path bulk corpus). `score`,
// `start`, `end` are always set by the macro expansion to integers.
#[derive(Debug, Clone)]
pub struct Synergy {
    pub description: Option<String>,
    pub connector: Option<String>,
    pub score: i32,
    pub start: usize,
    pub end: usize,
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod _star_synergy_list_star__inner {
use std::sync::Arc;

use crate::dict::kani::KaniLiteSegmentList;
use super::synergy_kanji_prefix;
use super::synergy_na_adjectives;
use super::synergy_no_adjectives;
use super::synergy_no_da;
use super::synergy_no_toori;
use super::synergy_noun_da;
use super::synergy_noun_particle;
use super::synergy_o_prefix;
use super::synergy_oki;
use super::synergy_shicha_ikenai;
use super::synergy_shika_negative;
use super::synergy_sou_nanda;
use super::Synergy;
use super::synergy_suffix_buri;
use super::synergy_suffix_chu;
use super::synergy_suffix_sei;
use super::synergy_suffix_tachi;
use super::synergy_to_adverbs;

pub type SynergyFn = fn(
    &KaniLiteSegmentList,
    &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)>;

pub static SYNERGY_LIST: &[SynergyFn] = &[
    synergy_oki,
    synergy_no_toori,
    synergy_shika_negative,
    synergy_shicha_ikenai,
    synergy_kanji_prefix,
    synergy_o_prefix,
    synergy_suffix_sei,
    synergy_suffix_buri,
    synergy_suffix_tachi,
    synergy_suffix_chu,
    synergy_to_adverbs,
    synergy_na_adjectives,
    synergy_no_adjectives,
    synergy_sou_nanda,
    synergy_no_da,
    synergy_noun_da,
    synergy_noun_particle,
];
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod make_segment_list_from_inner {
use crate::dict::segment::SegmentList;
use crate::dict::segment::Segment;

pub fn make_segment_list_from(old_segment_list: &SegmentList, segments: Vec<Segment>) -> SegmentList {
    // Lisp `copy-segment-list` is a shallow defstruct copy that then
    // gets its segments slot overwritten — the old segments are
    // immediately discarded. Constructing the new struct directly
    // avoids the Rust `Clone` deep-copying the old segments only for
    // them to be replaced on the next line.
    SegmentList {
        segments,
        start: old_segment_list.start,
        end: old_segment_list.end,
        top: old_segment_list.top.clone(),
        matches: old_segment_list.matches,
    }
}

#[cfg(test)]
mod tests {
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::text_classes::SimpleText;
    use super::*;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg_with_score(score: i32) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    #[test]
    fn swaps_segments_preserves_other_slots() {
        // REPL:
        //   src segments len=2, dst segments len=1
        //   dst start=0 end=2 matches=3
        //   src not mutated: src segments len=2
        //   first dst seg score=20
        let seg1 = seg_with_score(10);
        let seg2 = seg_with_score(20);
        let sl = SegmentList {
            segments: vec![seg1.clone(), seg2.clone()],
            start: 0,
            end: 2,
            top: None,
            matches: 3,
        };
        let new_sl = make_segment_list_from(&sl, vec![seg2.clone()]);
        assert_eq!(sl.segments.len(), 2);
        assert_eq!(new_sl.segments.len(), 1);
        assert_eq!(new_sl.start, 0);
        assert_eq!(new_sl.end, 2);
        assert_eq!(new_sl.matches, 3);
        assert_eq!(new_sl.segments[0].score, Some(20));
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod def_generic_synergy_macro_inner {
use std::sync::Arc;

use crate::dict::kani::KaniLiteSegment;
use crate::dict::kani::{make_kani_lite_segment_list_from, KaniLiteSegmentList};
use super::Synergy;

pub struct DefGenericSynergyOpts<'a> {
    pub description: Option<&'a str>,
    pub connector: &'a str,
    pub score: i32,
}

pub fn def_generic_synergy_body(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
    filter_left: impl Fn(&Arc<KaniLiteSegment>) -> bool,
    filter_right: impl Fn(&Arc<KaniLiteSegment>) -> bool,
    opts: &DefGenericSynergyOpts<'_>,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    let start = segment_list_left.end;
    let end = segment_list_right.start;
    // dict-grammar.lisp:737 (when (= start end))
    if start != end {
        return vec![];
    }
    // dict-grammar.lisp:738-739 (remove-if-not filter-left/right over segment-list-segments)
    let left: Vec<Arc<KaniLiteSegment>> = segment_list_left
        .segments
        .iter()
        .filter(|s| filter_left(s))
        .cloned()
        .collect();
    let right: Vec<Arc<KaniLiteSegment>> = segment_list_right
        .segments
        .iter()
        .filter(|s| filter_right(s))
        .cloned()
        .collect();
    // dict-grammar.lisp:740 (when (and left right))
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    // dict-grammar.lisp:741-746 (list (list (make-segment-list-from r right) (make-synergy ...) (make-segment-list-from l left)))
    let syn = Synergy {
        description: opts.description.map(|d| d.to_string()),
        connector: Some(opts.connector.to_string()),
        score: opts.score,
        start,
        end,
    };
    vec![(
        Arc::new(make_kani_lite_segment_list_from(segment_list_right, right)),
        syn,
        Arc::new(make_kani_lite_segment_list_from(segment_list_left, left)),
    )]
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_noun_particle_inner {
use std::sync::Arc;

use crate::dict::grammar::filter::NOUN_PARTICLES;
use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_noun;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_noun_particle(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    // dict-grammar.lisp:831 (:score (+ 10 (* 4 (- (segment-list-end r) (segment-list-start r)))))
    let span = r.end - r.start;
    let score = 10 + 4 * (span as i32);
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(NOUN_PARTICLES.to_vec()),
        &DefGenericSynergyOpts {
            description: Some("noun+prt"),
            connector: " ",
            score,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive_len1() {
        // noun-particle/positive-len1: l noun, r seq 2028920 (は), r.end-r.start=1.
        // SYNERGY desc="noun+prt" conn=" " score=14 start=1 end=1.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![2028920])]);
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("noun+prt"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 14);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_len2() {
        // noun-particle/positive-len2: r seq 2215430 (には), span=2 -> score=18.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![2215430])]);
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 18);
        assert_eq!(got[0].1.start, 1);
        assert_eq!(got[0].1.end, 1);
    }

    #[test]
    fn positive_len4() {
        // noun-particle/positive-len4: r seq 1009600 (にとって), span=4 -> score=26.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 5, vec![seg((false, false, false, false), vec![], vec![1009600])]);
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 26);
    }

    #[test]
    fn not_adjacent_empty() {
        // noun-particle/not-adjacent: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(5, 6, vec![seg((false, false, false, false), vec![], vec![2028920])]);
        assert!(synergy_noun_particle(&l, &r).is_empty());
    }

    #[test]
    fn right_misses_empty() {
        // noun-particle/right-misses: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        assert!(synergy_noun_particle(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_noun_da_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_noun;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_noun_da(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![2089020]),
        &DefGenericSynergyOpts {
            description: Some("noun+da"),
            connector: " ",
            score: 10,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // noun-da/positive: l noun (kpcl k=T posi=("n")), r seq 2089020 (だ).
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="noun+da"
        // conn=" " score=10 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![2089020])]);
        let got = synergy_noun_da(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("noun+da"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 10);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn not_adjacent_empty() {
        // noun-da/not-adjacent: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(5, 6, vec![seg((false, false, false, false), vec![], vec![2089020])]);
        assert!(synergy_noun_da(&l, &r).is_empty());
    }

    #[test]
    fn left_not_noun_empty() {
        // noun-da/left-not-noun: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["v5k"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![2089020])]);
        assert!(synergy_noun_da(&l, &r).is_empty());
    }

    #[test]
    fn right_misses_empty() {
        // noun-da/right-misses: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        assert!(synergy_noun_da(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_no_da_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_no_da(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![1469800, 2139720]),
        filter_in_seq_set(vec![2089020, 1007370, 1928670]),
        &DefGenericSynergyOpts {
            description: Some("no da/desu"),
            connector: " ",
            score: 15,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg_with_seqs(seq_set: Vec<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: vec![],
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl: (false, false, false, false),
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_1469800_2089020() {
        // no-da/positive-1: l ends at 2, r starts at 2.
        // RIGHT-SL start=2 end=3 segs=1, SYNERGY desc="no da/desu"
        // conn=" " score=15 start=2 end=2, LEFT-SL start=0 end=2 segs=1
        let l = lite_sl_owned(0, 2, vec![seg_with_seqs(vec![1469800, 999])]);
        let r = lite_sl_owned(2, 3, vec![seg_with_seqs(vec![2089020])]);
        let got = synergy_no_da(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("no da/desu"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_2139720_1928670() {
        // no-da/positive-2.
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![2139720])]);
        let r = lite_sl_owned(1, 2, vec![seg_with_seqs(vec![1928670])]);
        let got = synergy_no_da(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
        assert_eq!(got[0].1.start, 1);
        assert_eq!(got[0].1.end, 1);
    }

    #[test]
    fn not_adjacent_empty() {
        // no-da/not-adjacent: NIL
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![1469800])]);
        let r = lite_sl_owned(5, 6, vec![seg_with_seqs(vec![2089020])]);
        assert!(synergy_no_da(&l, &r).is_empty());
    }

    #[test]
    fn left_misses_empty() {
        // no-da/left-misses: NIL
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![9999999])]);
        let r = lite_sl_owned(1, 2, vec![seg_with_seqs(vec![2089020])]);
        assert!(synergy_no_da(&l, &r).is_empty());
    }

    #[test]
    fn right_misses_empty() {
        // no-da/right-misses: NIL
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![1469800])]);
        let r = lite_sl_owned(1, 2, vec![seg_with_seqs(vec![9999999])]);
        assert!(synergy_no_da(&l, &r).is_empty());
    }

    #[test]
    fn empty_left_segments() {
        // no-da/empty-left: NIL
        let l = lite_sl_owned(0, 1, vec![]);
        let r = lite_sl_owned(1, 2, vec![seg_with_seqs(vec![2089020])]);
        assert!(synergy_no_da(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_sou_nanda_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_sou_nanda(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![2137720]),
        filter_in_seq_set(vec![2140410]),
        &DefGenericSynergyOpts {
            description: Some("sou na n da"),
            connector: " ",
            score: 50,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(start: usize, end: usize, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: vec![],
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl: (false, false, false, false),
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // sou-nanda/positive: RIGHT-SL start=2 end=5 segs=1,
        // SYN desc="sou na n da" conn=" " score=50 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![2137720])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![2140410])]);
        let got = synergy_sou_nanda(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 5);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("sou na n da"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn right_miss_empty() {
        // sou-nanda/right-miss: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![2137720])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![99])]);
        assert!(synergy_sou_nanda(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // sou-nanda/not-adjacent: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![2137720])]);
        let r = lite_sl_owned(3, 6, vec![seg(3, 6, vec![2140410])]);
        assert!(synergy_sou_nanda(&l, &r).is_empty());
    }

    #[test]
    fn left_miss_empty() {
        // sou-nanda/left-miss: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![99])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![2140410])]);
        assert!(synergy_sou_nanda(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_no_adjectives_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_pos;
use crate::dict::kani::POS_ADJ_NO;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_no_adjectives(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:864 (filter-is-pos ("adj-no") (or k l (and p c)))
        filter_is_pos(POS_ADJ_NO, |k, p, c, l| k || l || (p && c)),
        filter_in_seq_set(vec![1469800]),
        &DefGenericSynergyOpts {
            description: Some("no-adjective"),
            connector: " ",
            score: 15,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_kpcl_k() {
        // no-adj/positive-k: l adj-no with k=T, r seq 1469800.
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="no-adjective"
        // conn=" " score=15 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_kpcl_l() {
        // no-adj/positive-l: l=T satisfies (or k l (and p c)).
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, true), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
    }

    #[test]
    fn positive_kpcl_pc() {
        // no-adj/positive-pc: (and p c) satisfies the test.
        let l = lite_sl_owned(0, 1, vec![seg((false, true, true, false), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        let got = synergy_no_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn neg_kpcl_all_nil() {
        // no-adj/neg-kpcl-all-nil: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }

    #[test]
    fn neg_kpcl_p_only() {
        // no-adj/neg-p-only: p without c, no k, no l -> kpcl-test false.
        let l = lite_sl_owned(0, 1, vec![seg((false, true, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_posi() {
        // no-adj/neg-no-posi: posi=("n"), not adj-no.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        assert!(synergy_no_adjectives(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_na_adjectives_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_pos;
use crate::dict::kani::POS_ADJ_NA;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_na_adjectives(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:871 (filter-is-pos ("adj-na") (or k l (and p c)))
        filter_is_pos(POS_ADJ_NA, |k, p, c, l| k || l || (p && c)),
        filter_in_seq_set(vec![2029110, 2028990]),
        &DefGenericSynergyOpts {
            description: Some("na-adjective"),
            connector: " ",
            score: 15,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_na() {
        // na-adj/positive-na: l adj-na with k=T, r seq 2029110.
        // RIGHT-SL start=2 end=3 segs=1, SYNERGY desc="na-adjective"
        // conn=" " score=15 start=2 end=2, LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(0, 2, vec![seg((true, false, false, false), vec!["adj-na"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![2029110])]);
        let got = synergy_na_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("na-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_ni() {
        // na-adj/positive-ni: l adj-na with l=T, r seq 2028990 (に).
        let l = lite_sl_owned(0, 2, vec![seg((false, false, false, true), vec!["adj-na"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![2028990])]);
        let got = synergy_na_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
        assert_eq!(got[0].1.start, 2);
        assert_eq!(got[0].1.end, 2);
    }

    #[test]
    fn wrong_posi_empty() {
        // na-adj/neg-wrong-posi: l posi=("v5k"), not adj-na -> NIL.
        let l = lite_sl_owned(0, 2, vec![seg((true, false, false, false), vec!["v5k"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![2029110])]);
        assert!(synergy_na_adjectives(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_to_adverbs_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_pos;
use crate::dict::kani::POS_ADV_TO;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_to_adverbs(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    // dict-grammar.lisp:881 (:score (+ 10 (* 10 (- (segment-list-end l) (segment-list-start l)))))
    let span = l.end - l.start;
    let score = 10 + 10 * (span as i32);
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:878 (filter-is-pos ("adv-to") (or k l p))
        filter_is_pos(POS_ADV_TO, |k, p, _c, l| k || l || p),
        filter_in_seq_set(vec![1008490]),
        &DefGenericSynergyOpts {
            description: Some("to-adverb"),
            connector: " ",
            score,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_449_451.lisp on .103, 2026-05-18).

    #[test]
    fn positive_k_span2() {
        // to-adv/positive-k: l adv-to k=T span=2 -> score = 10 + 10*2 = 30.
        // RIGHT-SL start=2 end=3 segs=1, SYNERGY desc="to-adverb"
        // conn=" " score=30 start=2 end=2, LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(0, 2, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("to-adverb"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 30);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_l_span1() {
        // to-adv/positive-l: l=T span=1 -> score = 20.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, true), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
        assert_eq!(got[0].1.start, 1);
        assert_eq!(got[0].1.end, 1);
    }

    #[test]
    fn positive_p_alone_span3() {
        // to-adv/positive-p-alone: p=T c=NIL span=3 -> score = 40. Bare
        // `p` is the divergence vs synergy-no-adjectives / synergy-na-
        // adjectives whose kpcl-test is `(or k l (and p c))`.
        let l = lite_sl_owned(0, 3, vec![seg((false, true, false, false), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(3, 4, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 40);
        assert_eq!(got[0].1.start, 3);
        assert_eq!(got[0].1.end, 3);
    }

    #[test]
    fn positive_p_and_c_span4() {
        // to-adv/positive-p-and-c: p=T c=T span=4 -> score = 50.
        let l = lite_sl_owned(0, 4, vec![seg((false, true, true, false), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(4, 5, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 50);
        assert_eq!(got[0].1.start, 4);
        assert_eq!(got[0].1.end, 4);
    }

    #[test]
    fn positive_k_span1() {
        // to-adv/positive-span1: k=T span=1 -> score = 20.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = synergy_to_adverbs(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
    }

    #[test]
    fn neg_kpcl_all_nil() {
        // to-adv/neg-kpcl-all-nil: NIL.
        let l = lite_sl_owned(0, 2, vec![seg((false, false, false, false), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_c_alone() {
        // to-adv/neg-c-alone: c=T only (no k, no l, no p) — kpcl-test is
        // `(or k l p)` so bare c does not pass.
        let l = lite_sl_owned(0, 2, vec![seg((false, false, true, false), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_posi() {
        // to-adv/neg-wrong-posi: posi=("n"), not adv-to -> NIL.
        let l = lite_sl_owned(0, 2, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_wrong_right_seq() {
        // to-adv/neg-wrong-right: r seq not 1008490.
        let l = lite_sl_owned(0, 2, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![9999])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_non_adjacent() {
        // to-adv/neg-non-adjacent: l.end /= r.start.
        let l = lite_sl_owned(0, 2, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = lite_sl_owned(5, 6, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }

    #[test]
    fn neg_empty_left() {
        // to-adv/neg-empty-left: l segs empty.
        let l = lite_sl_owned(0, 2, vec![]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        assert!(synergy_to_adverbs(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_suffix_chu_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_noun;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_suffix_chu(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![1620400, 2083570]),
        &DefGenericSynergyOpts {
            description: Some("suffix-chu"),
            connector: "-",
            score: 12,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive_1620400() {
        // suffix-chu/positive-1620400: RIGHT-SL start=2 end=3 segs=1,
        // SYN desc="suffix-chu" conn="-" score=12 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1620400])],
        );
        let got = synergy_suffix_chu(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("suffix-chu"));
        assert_eq!(syn.connector.as_deref(), Some("-"));
        assert_eq!(syn.score, 12);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_2083570() {
        // suffix-chu/positive-2083570: same shape as positive_1620400.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![2083570])],
        );
        let got = synergy_suffix_chu(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn right_miss_empty() {
        // suffix-chu/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![99])],
        );
        assert!(synergy_suffix_chu(&l, &r).is_empty());
    }

    #[test]
    fn left_not_noun_empty() {
        // suffix-chu/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1620400])],
        );
        assert!(synergy_suffix_chu(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // suffix-chu/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            4,
            5,
            vec![seg(4, 5, (false, false, false, false), vec![], vec![1620400])],
        );
        assert!(synergy_suffix_chu(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_suffix_tachi_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_noun;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_suffix_tachi(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![1416220]),
        &DefGenericSynergyOpts {
            description: Some("suffix-tachi"),
            connector: "-",
            score: 10,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // suffix-tachi/positive: RIGHT-SL start=2 end=3 segs=1,
        // SYN desc="suffix-tachi" conn="-" score=10 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1416220])],
        );
        let got = synergy_suffix_tachi(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("suffix-tachi"));
        assert_eq!(syn.connector.as_deref(), Some("-"));
        assert_eq!(syn.score, 10);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn right_miss_empty() {
        // suffix-tachi/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![99])],
        );
        assert!(synergy_suffix_tachi(&l, &r).is_empty());
    }

    #[test]
    fn left_not_noun_empty() {
        // suffix-tachi/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1416220])],
        );
        assert!(synergy_suffix_tachi(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // suffix-tachi/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            4,
            5,
            vec![seg(4, 5, (false, false, false, false), vec![], vec![1416220])],
        );
        assert!(synergy_suffix_tachi(&l, &r).is_empty());
    }

    #[test]
    fn multi_segs_partial_filter() {
        // suffix-tachi/multi-segs: l has 2 segs (one noun, one not),
        // r has 2 segs (one matches 1416220, one not). RIGHT-SL segs=1,
        // LEFT-SL segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![
                seg(0, 2, (true, false, false, false), vec!["n"], vec![]),
                seg(0, 2, (true, false, false, false), vec!["v5k"], vec![]),
            ],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![
                seg(2, 3, (false, false, false, false), vec![], vec![1416220]),
                seg(2, 3, (false, false, false, false), vec![], vec![99]),
            ],
        );
        let got = synergy_suffix_tachi(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, _syn, left_sl) = &got[0];
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(left_sl.segments.len(), 1);
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_suffix_buri_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_noun;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_suffix_buri(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![1361140]),
        &DefGenericSynergyOpts {
            description: Some("suffix-buri"),
            connector: "",
            score: 40,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // suffix-buri/positive: RIGHT-SL start=2 end=4 segs=1,
        // SYN desc="suffix-buri" conn="" score=40 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            4,
            vec![seg(2, 4, (false, false, false, false), vec![], vec![1361140])],
        );
        let got = synergy_suffix_buri(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 4);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("suffix-buri"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 40);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn left_not_noun_empty() {
        // suffix-buri/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            4,
            vec![seg(2, 4, (false, false, false, false), vec![], vec![1361140])],
        );
        assert!(synergy_suffix_buri(&l, &r).is_empty());
    }

    #[test]
    fn right_miss_empty() {
        // suffix-buri/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            4,
            vec![seg(2, 4, (false, false, false, false), vec![], vec![99])],
        );
        assert!(synergy_suffix_buri(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // suffix-buri/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            5,
            7,
            vec![seg(5, 7, (false, false, false, false), vec![], vec![1361140])],
        );
        assert!(synergy_suffix_buri(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_suffix_sei_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_noun;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_suffix_sei(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(vec![1375260]),
        &DefGenericSynergyOpts {
            description: Some("suffix-sei"),
            connector: "",
            score: 12,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        start: usize,
        end: usize,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // suffix-sei/positive: RIGHT-SL start=2 end=3 segs=1,
        // SYN desc="suffix-sei" conn="" score=12 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1375260])],
        );
        let got = synergy_suffix_sei(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("suffix-sei"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 12);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn left_not_noun_empty() {
        // suffix-sei/left-not-noun: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["v5k"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![1375260])],
        );
        assert!(synergy_suffix_sei(&l, &r).is_empty());
    }

    #[test]
    fn right_miss_empty() {
        // suffix-sei/right-miss: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            2,
            3,
            vec![seg(2, 3, (false, false, false, false), vec![], vec![99])],
        );
        assert!(synergy_suffix_sei(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // suffix-sei/not-adjacent: NIL.
        let l = lite_sl_owned(
            0,
            2,
            vec![seg(0, 2, (true, false, false, false), vec!["n"], vec![])],
        );
        let r = lite_sl_owned(
            4,
            5,
            vec![seg(4, 5, (false, false, false, false), vec![], vec![1375260])],
        );
        assert!(synergy_suffix_sei(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_o_prefix_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_pos;
use crate::dict::kani::POS_N;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_o_prefix(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![1270190]),
        // dict-grammar.lisp:915 (filter-is-pos ("n") (or k l))
        filter_is_pos(POS_N, |k, _p, _c, l| k || l),
        &DefGenericSynergyOpts {
            description: Some("o+noun"),
            connector: "",
            score: 10,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive_k() {
        // o-prefix/positive-k: l seq 1270190 (お), r kpcl k=T posi=("n").
        // RIGHT-SL start=1 end=2 segs=1, SYNERGY desc="o+noun"
        // conn="" score=10 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![1270190])]);
        let r = lite_sl_owned(1, 2, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_o_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("o+noun"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 10);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_l() {
        // o-prefix/positive-l: r kpcl l=T, kpcl-test (or k l) satisfied.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![1270190])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, true), vec!["n"], vec![])]);
        let got = synergy_o_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 10);
    }

    #[test]
    fn neg_pc_only() {
        // o-prefix/neg-pc-only: kpcl-test is (or k l), NOT (and p c) — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![1270190])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, true, true, false), vec!["n"], vec![])]);
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_no_n_posi() {
        // o-prefix/neg-no-n-posi: posi=("adj-na"), not "n" — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![1270190])]);
        let r = lite_sl_owned(1, 2, vec![seg((true, false, false, false), vec!["adj-na"], vec![])]);
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_left_miss() {
        // o-prefix/neg-left-miss: l seq doesn't match 1270190 — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        let r = lite_sl_owned(1, 2, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        assert!(synergy_o_prefix(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_kanji_prefix_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_pos;
use crate::dict::kani::POS_N;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_kanji_prefix(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![2242840, 1922780, 2423740]),
        // dict-grammar.lisp:922 (filter-is-pos ("n") k)
        filter_is_pos(POS_N, |k, _p, _c, _l| k),
        &DefGenericSynergyOpts {
            description: Some("kanji prefix+noun"),
            connector: "",
            score: 15,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_mi() {
        // kanji-prefix/positive-mi: l seq 2242840, r k=T posi=("n").
        // RIGHT-SL start=1 end=3 segs=1, SYNERGY desc="kanji prefix+noun"
        // conn="" score=15 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("kanji prefix+noun"));
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_fu() {
        // kanji-prefix/positive-fu: l seq 1922780.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![1922780])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
    }

    #[test]
    fn positive_2423740() {
        // kanji-prefix/positive-2423740: l seq 2423740.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![2423740])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let got = synergy_kanji_prefix(&l, &r);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn neg_no_k() {
        // kanji-prefix/neg-no-k: r kpcl k=NIL even with posi=("n") -> NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, true), vec!["n"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_no_n_posi() {
        // kanji-prefix/neg-no-n-posi: r k=T but posi=("v5k") (not "n").
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![2242840])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["v5k"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }

    #[test]
    fn neg_left_miss() {
        // kanji-prefix/neg-left-miss: l seq 9999 doesn't match.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec![], vec![9999])]);
        let r = lite_sl_owned(1, 3, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        assert!(synergy_kanji_prefix(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_shicha_ikenai_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_compound_end;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_shicha_ikenai(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_compound_end(vec![2028920]),
        filter_in_seq_set(vec![1000730, 1612750, 1409110, 2829697, 1587610]),
        &DefGenericSynergyOpts {
            description: Some("shicha ikenai"),
            connector: " ",
            score: 50,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn compound_word(child_seqs: &[i32]) -> KaniWordDispatchEnum {
        let words: Vec<KaniWordDispatchEnum> = child_seqs
            .iter()
            .enumerate()
            .map(|(i, s)| KaniWordDispatchEnum::Kana(kana(&format!("w{i}"), *s)))
            .collect();
        let primary = Box::new(words[0].clone());
        KaniWordDispatchEnum::Compound(CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        })
    }

    fn dummy_kana() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(kana("x", 0))
    }

    fn seg(start: usize, end: usize, word: KaniWordDispatchEnum, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word,
            score: None,
            info: Some(KaniSegmentInfo {
                posi: vec![],
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl: (false, false, false, false),
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_442_448.lisp on .103, 2026-05-18).

    #[test]
    fn positive() {
        // shicha-ikenai/positive: RIGHT-SL start=3 end=7 segs=1,
        // SYN desc="shicha ikenai" conn=" " score=50 start=3 end=3,
        // LEFT-SL start=0 end=3 segs=1.
        let l = lite_sl_owned(0, 3, vec![seg(0, 3, compound_word(&[1234567, 2028920]), vec![])]);
        let r = lite_sl_owned(3, 7, vec![seg(3, 7, dummy_kana(), vec![1612750])]);
        let got = synergy_shicha_ikenai(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 3);
        assert_eq!(right_sl.end, 7);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("shicha ikenai"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 3);
        assert_eq!(syn.end, 3);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 3);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn right_miss_empty() {
        // shicha-ikenai/right-miss: NIL.
        let l = lite_sl_owned(0, 3, vec![seg(0, 3, compound_word(&[1234567, 2028920]), vec![])]);
        let r = lite_sl_owned(3, 7, vec![seg(3, 7, dummy_kana(), vec![99999])]);
        assert!(synergy_shicha_ikenai(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // shicha-ikenai/not-adjacent: NIL.
        let l = lite_sl_owned(0, 3, vec![seg(0, 3, compound_word(&[1234567, 2028920]), vec![])]);
        let r = lite_sl_owned(5, 9, vec![seg(5, 9, dummy_kana(), vec![1612750])]);
        assert!(synergy_shicha_ikenai(&l, &r).is_empty());
    }

    #[test]
    fn left_not_compound_empty() {
        // shicha-ikenai/left-not-compound: NIL.
        // Simple word with seq 2028920 fails filter-is-compound-end.
        let l = lite_sl_owned(0, 1, vec![seg(0, 1, dummy_kana(), vec![2028920])]);
        let r = lite_sl_owned(1, 5, vec![seg(1, 5, dummy_kana(), vec![1612750])]);
        assert!(synergy_shicha_ikenai(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_shika_negative_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_shika_negative(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![1005460]),
        // dict-grammar.lisp:936-939 (lambda (some (conj-neg (conj-data-prop cdata)) :conj))
        |s| s.conj_has_neg,
        &DefGenericSynergyOpts {
            description: Some("shika+neg"),
            connector: " ",
            score: 50,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::ConjProp;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn prop(neg: Option<bool>) -> ConjProp {
        ConjProp {
            id: 0,
            conj_id: 0,
            pos: "v5k".into(),
            conj_type: 1,
            neg,
            fml: None,
        }
    }

    fn cdata(neg: Option<bool>) -> ConjData {
        ConjData {
            seq: Some(1),
            from: Some(2),
            via: None,
            prop: Some(prop(neg)),
            src_map: vec![],
        }
    }

    fn seg(
        start: usize,
        end: usize,
        seq_set: Vec<i32>,
        conj: Vec<ConjData>,
    ) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: vec![],
                seq_set,
                conj,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl: (false, false, false, false),
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes:
    // - /tmp/probe_442_448.lisp on .103, 2026-05-18: t / nil cases.
    // - /tmp/probe_shika_null.lisp on .103: :NULL case (DB-null neg
    //   keyword is truthy in CL, so synergy fires).
    //
    // Rust ↔ Lisp neg mapping (parse_opt_bool, audit/common/mod.rs:1789):
    //   Some(true)  ↔ Lisp t      → FIRE
    //   Some(false) ↔ Lisp nil    → reject
    //   None        ↔ Lisp :NULL  → FIRE (:NULL is a truthy keyword)

    #[test]
    fn positive_neg_t() {
        // shika-negative/positive (neg=t): RIGHT-SL start=2 end=5 segs=1,
        // SYN desc="shika+neg" conn=" " score=50 start=2 end=2,
        // LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(true))])]);
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 5);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("shika+neg"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_neg_null() {
        // shika-negative/neg=:NULL ALONE -- expect FIRE (REPL
        // /tmp/probe_shika_null.lisp: COUNT=1 desc="shika+neg"
        // score=50). :NULL keyword is truthy in CL, so `some` returns
        // truthy and the filter accepts.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![cdata(None)])]);
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.description.as_deref(), Some("shika+neg"));
        assert_eq!(got[0].1.score, 50);
    }

    #[test]
    fn right_neg_nil_empty() {
        // shika-negative/neg=NIL ALONE -- expect NIL (REPL
        // /tmp/probe_shika_null.lisp). Lisp nil is the sole falsy
        // value, so `some` returns nil and the filter rejects.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(false))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn right_empty_conj_empty() {
        // shika-negative/right-empty-conj: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn left_miss_empty() {
        // shika-negative/left-miss: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![99], vec![])]);
        let r = lite_sl_owned(2, 5, vec![seg(2, 5, vec![], vec![cdata(Some(true))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn not_adjacent_empty() {
        // shika-negative/not-adjacent: NIL.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(4, 7, vec![seg(4, 7, vec![], vec![cdata(Some(true))])]);
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn multi_conj_nil_plus_nil_empty() {
        // shika-negative/neg=NIL+NIL -- expect NIL (REPL probe).
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(
            2,
            5,
            vec![seg(2, 5, vec![], vec![cdata(Some(false)), cdata(Some(false))])],
        );
        assert!(synergy_shika_negative(&l, &r).is_empty());
    }

    #[test]
    fn multi_conj_nil_plus_null_fires() {
        // shika-negative/neg=NIL+:NULL -- expect FIRE (REPL probe).
        // The :NULL cdata's truthy neg-value satisfies `some`.
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(
            2,
            5,
            vec![seg(2, 5, vec![], vec![cdata(Some(false)), cdata(None)])],
        );
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 50);
    }

    #[test]
    fn multi_conj_nil_plus_t_fires() {
        // shika-negative/multi-conj-mixed: RIGHT-SL segs=1, LEFT-SL segs=1.
        // Mirrors the original probe_442_448 test but with the
        // corrected nil mapping (Some(false), not None).
        let l = lite_sl_owned(0, 2, vec![seg(0, 2, vec![1005460], vec![])]);
        let r = lite_sl_owned(
            2,
            5,
            vec![seg(
                2,
                5,
                vec![],
                vec![cdata(Some(false)), cdata(Some(true))],
            )],
        );
        let got = synergy_shika_negative(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, _syn, left_sl) = &got[0];
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(left_sl.segments.len(), 1);
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_no_toori_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_no_toori(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(vec![1469800]),
        filter_in_seq_set(vec![1432920]),
        &DefGenericSynergyOpts {
            description: Some("no toori"),
            connector: " ",
            score: 50,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg_with_seqs(seq_set: Vec<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: vec![],
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl: (false, false, false, false),
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_no_toori() {
        // no-toori/positive: RIGHT-SL start=1 end=3 segs=1,
        // SYNERGY desc="no toori" conn=" " score=50 start=1 end=1,
        // LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![1469800])]);
        let r = lite_sl_owned(1, 3, vec![seg_with_seqs(vec![1432920])]);
        let got = synergy_no_toori(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("no toori"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 50);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn left_misses_empty() {
        // no-toori/left-misses: NIL.
        let l = lite_sl_owned(0, 1, vec![seg_with_seqs(vec![12345])]);
        let r = lite_sl_owned(1, 3, vec![seg_with_seqs(vec![1432920])]);
        assert!(synergy_no_toori(&l, &r).is_empty());
    }

    #[test]
    fn multi_segs_partial_filter() {
        // no-toori/multi-segs-partial: l has 2 segs (one matches, one
        // does not), r has 2 segs (both match). Expected RIGHT-SL
        // segs=2, LEFT-SL segs=1.
        let l = lite_sl_owned(
            0,
            1,
            vec![seg_with_seqs(vec![1469800]), seg_with_seqs(vec![99])],
        );
        let r = lite_sl_owned(
            1,
            3,
            vec![
                seg_with_seqs(vec![1432920]),
                seg_with_seqs(vec![1432920, 88]),
            ],
        );
        let got = synergy_no_toori(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, _syn, left_sl) = &got[0];
        assert_eq!(right_sl.segments.len(), 2);
        assert_eq!(left_sl.segments.len(), 1);
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod synergy_oki_inner {
use std::sync::Arc;

use super::{def_generic_synergy_body, DefGenericSynergyOpts};
use crate::dict::grammar::filter::filter_in_seq_set;
use crate::dict::grammar::filter::filter_is_pos;
use crate::dict::kani::POS_CTR;
use crate::dict::kani::KaniLiteSegmentList;
use super::Synergy;

pub fn synergy_oki(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:952 (filter-is-pos ("ctr") t)
        filter_is_pos(POS_CTR, |_k, _p, _c, _l| true),
        filter_in_seq_set(vec![2854117, 2084550]),
        &DefGenericSynergyOpts {
            description: None,
            connector: "",
            score: 20,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive_2854117() {
        // oki/positive-2854117: l posi=("ctr") kpcl all nil, r seq 2854117.
        // RIGHT-SL start=1 end=3 segs=1, SYNERGY desc=NIL conn=""
        // score=20 start=1 end=1, LEFT-SL start=0 end=1 segs=1.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![2854117])]);
        let got = synergy_oki(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert!(syn.description.is_none());
        assert_eq!(syn.connector.as_deref(), Some(""));
        assert_eq!(syn.score, 20);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_2084550() {
        // oki/positive-2084550: r matches second seq in the set.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![2084550])]);
        let got = synergy_oki(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 20);
    }

    #[test]
    fn neg_no_ctr_posi() {
        // oki/neg-no-ctr-posi: l posi=("n"), not "ctr" — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![2854117])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }

    #[test]
    fn neg_right_miss() {
        // oki/neg-right-miss: r seq doesn't match either — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["ctr"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }

    #[test]
    fn neg_not_adjacent() {
        // oki/neg-not-adjacent: l.end != r.start — NIL.
        let l = lite_sl_owned(0, 1, vec![seg((false, false, false, false), vec!["ctr"], vec![])]);
        let r = lite_sl_owned(5, 7, vec![seg((false, false, false, false), vec![], vec![2854117])]);
        assert!(synergy_oki(&l, &r).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_synergies_inner {
use super::SYNERGY_LIST;
use crate::dict::kani::KaniLiteSegmentList;
use crate::dict::kani::KaniLitePathElement;

pub fn get_synergies(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
) -> Vec<Vec<KaniLitePathElement>> {
    let mut out = vec![];
    for synergy_fn in SYNERGY_LIST {
        // dict-grammar.lisp:958-959 (`nconc (funcall fn l r)`)
        for (right_sl, syn, left_sl) in synergy_fn(segment_list_left, segment_list_right) {
            out.push(vec![
                KaniLitePathElement::SegmentList(right_sl),
                KaniLitePathElement::Synergy(syn),
                KaniLitePathElement::SegmentList(left_sl),
            ]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(kpcl: (bool, bool, bool, bool), posi: Vec<&str>, seq_set: Vec<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
                seq_set,
                conj: vec![] as Vec<ConjData>,
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: vec![],
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl,
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    fn unwrap_synergy(path: &[KaniLitePathElement]) -> &crate::dict::grammar::synergy::Synergy {
        match &path[1] {
            KaniLitePathElement::Synergy(s) => s,
            other => panic!("expected Synergy at [1], got {:?}", other),
        }
    }

    fn unwrap_sl(elem: &KaniLitePathElement) -> &KaniLiteSegmentList {
        match elem {
            KaniLitePathElement::SegmentList(sl) => sl,
            other => panic!("expected SegmentList, got {:?}", other),
        }
    }

    // REPL probes (/tmp/probe_449_451.lisp on .103, 2026-05-18).

    #[test]
    fn a_none_fire() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["zzz"], vec![9999])]);
        let r = lite_sl(1, 2, vec![seg((false, false, false, false), vec!["zzz"], vec![8888])]);
        assert!(get_synergies(&l, &r).is_empty());
    }

    #[test]
    fn b_only_no_adjectives() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(unwrap_sl(&got[0][0]).start, 1);
        assert_eq!(unwrap_sl(&got[0][0]).end, 2);
        assert_eq!(unwrap_sl(&got[0][2]).start, 0);
        assert_eq!(unwrap_sl(&got[0][2]).end, 1);
    }

    #[test]
    fn c_only_to_adverbs() {
        let l = lite_sl(0, 2, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = lite_sl(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("to-adverb"));
        assert_eq!(syn.score, 30);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
    }

    #[test]
    fn d_noun_da_only() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl(1, 2, vec![seg((false, false, false, false), vec![], vec![2089020])]);
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("noun+da"));
        assert_eq!(syn.score, 10);
    }

    #[test]
    fn e_noun_particle_only() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl(1, 2, vec![seg((false, false, false, false), vec![], vec![2028920])]);
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("noun+prt"));
        assert_eq!(syn.score, 14);
    }

    #[test]
    fn f_two_synergies_order_mirrors_synergy_list() {
        let l = lite_sl(
            0,
            1,
            vec![seg((true, false, false, false), vec!["adj-no"], vec![1469800])],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg((false, false, false, false), vec![], vec![1469800]),
                seg((false, false, false, false), vec![], vec![2089020]),
            ],
        );
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 2);
        let syn0 = unwrap_synergy(&got[0]);
        assert_eq!(syn0.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn0.score, 15);
        let syn1 = unwrap_synergy(&got[1]);
        assert_eq!(syn1.description.as_deref(), Some("no da/desu"));
        assert_eq!(syn1.score, 15);
    }

    #[test]
    fn g_non_adjacent() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl(5, 6, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        assert!(get_synergies(&l, &r).is_empty());
    }
}
}
