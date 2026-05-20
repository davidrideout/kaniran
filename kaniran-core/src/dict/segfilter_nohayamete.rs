//! Port of `ichiran/dict:segfilter-nohayamete` (`dict-grammar.lisp:1127`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-nohayamete (l r)
//!   (complement (filter-in-seq-set 1469800))
//!   (filter-in-seq-set 1601080)
//!   :allow-first t)
//! ```

use std::sync::Arc;

use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;

const NO_SEQ: i32 = 1469800;
const HAYAMETE_SEQS: &[i32] = &[1601080];

pub fn segfilter_nohayamete(
    seg_left: Option<&Arc<SegmentList>>,
    seg_right: &Arc<SegmentList>,
) -> Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> {
    let filter_right = filter_in_seq_set(HAYAMETE_SEQS.to_vec());
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

    // T branch. Left filter is the complement of (filter-in-seq-set 1469800).
    let inner = filter_in_seq_set(vec![NO_SEQ]);
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
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: None,
        }
    }

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList { segments, start, end, top: None, matches: 0 }
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn nh_a_l_nil_r_match() {
        // NH-A l=NIL r=match cnt=1 — pass-through (allow-first)
        let r = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1601080]))]));
        let result = segfilter_nohayamete(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn nh_b_l_nil_r_no_match() {
        // NH-B l=NIL r=no-match cnt=1 — clause-1
        let r = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]));
        let result = segfilter_nohayamete(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn nh_c_l_not_no_r_hayamete() {
        // NH-C l-not-no r-hayamete cnt=1
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1601080]))]));
        let result = segfilter_nohayamete(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn nh_d_l_is_no_r_hayamete_empty() {
        // NH-D l-is-no r-hayamete cnt=0
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1469800]))]));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1601080]))]));
        let result = segfilter_nohayamete(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn nh_e_l_mixed_r_hayamete() {
        // NH-E l-mixed r-hayamete cnt=1 — sat-l push (con-r empty)
        let l = Arc::new(sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![1469800])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        ));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![1601080]))]));
        let result = segfilter_nohayamete(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn nh_f_gap_r_mixed() {
        // NH-F gap r-mixed cnt=1 — clause-2 with con-r non-empty
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]));
        let r = Arc::new(sl(
            2,
            3,
            vec![
                seg(2, 3, info_with_seq_set(vec![1601080])),
                seg(2, 3, info_with_seq_set(vec![999])),
            ],
        ));
        let result = segfilter_nohayamete(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }
}
