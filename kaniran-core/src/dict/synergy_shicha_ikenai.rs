//! Port of `ichiran/dict:synergy-shicha-ikenai` (`dict-grammar.lisp:927`).
//!
//! ```lisp
//! (def-generic-synergy synergy-shicha-ikenai (l r)
//!   (filter-is-compound-end 2028920) ;; は
//!   (filter-in-seq-set 1000730 1612750 1409110 2829697 1587610) ;; いけない いけません だめ いかん いや
//!   :description "shicha ikenai"
//!   :score 50
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use std::sync::Arc;

use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_compound_end::filter_is_compound_end;
use super::kani_lite_segment::KaniLiteSegment;
use super::kani_lite_segment_list::{make_kani_lite_segment_list_from, KaniLiteSegmentList};
use super::synergy_struct::Synergy;

pub fn synergy_shicha_ikenai(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    let start = l.end;
    let end = r.start;
    // dict-grammar.lisp:731-746 (def-generic-synergy expansion)
    if start != end {
        return vec![];
    }
    let test_left = filter_is_compound_end(vec![2028920]);
    let test_right = filter_in_seq_set(vec![1000730, 1612750, 1409110, 2829697, 1587610]);
    let left: Vec<Arc<KaniLiteSegment>> =
        l.segments.iter().filter(|s| test_left(s)).cloned().collect();
    let right: Vec<Arc<KaniLiteSegment>> =
        r.segments.iter().filter(|s| test_right(s)).cloned().collect();
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    let syn = Synergy {
        description: Some("shicha ikenai".to_string()),
        connector: Some(" ".to_string()),
        score: 50,
        start,
        end,
    };
    vec![(
        Arc::new(make_kani_lite_segment_list_from(r, right)),
        syn,
        Arc::new(make_kani_lite_segment_list_from(l, left)),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_list_struct::SegmentList;
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::simple_text_class::SimpleText;

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
