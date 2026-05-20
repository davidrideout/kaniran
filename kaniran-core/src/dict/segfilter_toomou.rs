//! Port of `ichiran/dict:segfilter-toomou` (`dict-grammar.lisp:1132`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-toomou (l r)
//!   ;; split と before 思う 言う
//!   (complement (filter-in-seq-set 2837117)) ;; 何だと
//!   (filter-in-seq-set 1589350 1587040)
//!   :allow-first t)
//! ```

use std::sync::Arc;

use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;

const NANDATO_SEQ: i32 = 2837117;
const OMOU_IU_SEQS: &[i32] = &[1589350, 1587040];

pub fn segfilter_toomou(
    seg_left: Option<&Arc<SegmentList>>,
    seg_right: &Arc<SegmentList>,
) -> Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> {
    let filter_right = filter_in_seq_set(OMOU_IU_SEQS.to_vec());
    let (sat_r, con_r) = classify(filter_right, &seg_right.segments);

    // Cond clause 1: (or (not sat-r) (and allow-first (not l)))
    if sat_r.is_empty() || seg_left.is_none() {
        return vec![(seg_left.cloned(), Arc::clone(seg_right))];
    }

    // Cond clause 2: (or (not l) (/= l.end r.start)) — (not l) absorbed above.
    let l = seg_left.unwrap();
    if l.end != seg_right.start {
        return if con_r.is_empty() {
            Vec::new()
        } else {
            vec![(Some(Arc::clone(l)), Arc::new(make_segment_list_from(seg_right, con_r)))]
        };
    }

    // T branch. Left filter is the complement of (filter-in-seq-set 2837117).
    let inner = filter_in_seq_set(vec![NANDATO_SEQ]);
    let (sat_l, con_l) = classify(|s| !inner(s), &l.segments);

    if con_l.is_empty() {
        return vec![(Some(Arc::clone(l)), Arc::clone(seg_right))];
    }

    let mut result: Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> = Vec::new();
    if !con_r.is_empty() {
        result.push((Some(Arc::clone(l)), Arc::new(make_segment_list_from(seg_right, con_r))));
    }
    if !sat_l.is_empty() {
        // dict-grammar.lisp:1064 (push) — prepend the satisfies pair.
        result.insert(
            0,
            (
                Some(Arc::new(make_segment_list_from(l, sat_l))),
                Arc::new(make_segment_list_from(seg_right, sat_r)),
            ),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::simple_text_class::SimpleText;

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

    fn info_with_seq_set(seq_set: Vec<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set,
            conj: vec![],
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl: (false, false, false, false),
        }
    }

    fn seg(start: usize, end: usize, info: KaniSegmentInfo) -> Segment {
        Segment { start, end, word: dummy_word(), score: None, info: Some(info), top: None, text: None }
    }

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList { segments, start, end, top: None, matches: 0 }
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn tm_a_l_nil_r_omou_pass_through() {
        // TM-A l=NIL r=omou cnt=1 — allow-first
        let r = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1589350]))]));
        let result = segfilter_toomou(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn tm_b_l_nil_r_no_match() {
        // TM-B l=NIL r=no-match cnt=1 — clause-1
        let r = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]));
        let result = segfilter_toomou(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn tm_c_l_not_nandato_r_omou() {
        // TM-C l-not-nandato r-omou cnt=1
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1589350]))]));
        let result = segfilter_toomou(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn tm_d_l_nandato_r_omou_empty() {
        // TM-D l-nandato r-omou cnt=0
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2837117]))]));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1589350]))]));
        let result = segfilter_toomou(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn tm_e_l_mixed_r_omou() {
        // TM-E l-mixed r-omou cnt=1 — sat-l push (con-r empty)
        let l = Arc::new(sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![2837117])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        ));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1589350]))]));
        let result = segfilter_toomou(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
        assert_eq!(result[0].1.segments.len(), 1);
    }
}
