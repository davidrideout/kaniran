//! Port of `ichiran/dict:segfilter-mononi` (`dict-grammar.lisp:1165`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-mononi (l r)
//!                            (complement (filter-in-seq-set 2028940))
//!                            (filter-in-seq-set 1009980)
//!                            :allow-first t)
//! ```

use std::sync::Arc;

use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani_lite_segment_list::{make_kani_lite_segment_list_from, KaniLiteSegmentList};

const MO_SEQ: i32 = 2028940;
const MONONI_SEQS: &[i32] = &[1009980];

pub fn segfilter_mononi(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let filter_right = filter_in_seq_set(MONONI_SEQS.to_vec());
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
            vec![(Some(Arc::clone(l)), Arc::new(make_kani_lite_segment_list_from(seg_right, con_r)))]
        };
    }

    // T branch. Left filter is the complement of
    // (filter-in-seq-set 2028940).
    let inner = filter_in_seq_set(vec![MO_SEQ]);
    let (sat_l, con_l) = classify(|s| !inner(s), &l.segments);

    if con_l.is_empty() {
        return vec![(Some(Arc::clone(l)), Arc::clone(seg_right))];
    }

    let mut result: Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> = Vec::new();
    if !con_r.is_empty() {
        result.push((Some(Arc::clone(l)), Arc::new(make_kani_lite_segment_list_from(seg_right, con_r))));
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
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_list_struct::SegmentList;
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

    fn seg(start: usize, end: usize, info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    // REPL probes from `/tmp/probe_410_414.lisp` (this session).

    #[test]
    fn m_a_l_nil_r_mononi_pass_through() {
        // M-A l=NIL r=mononi cnt=1
        // allow-first → clause-1 → (list (list nil r))
        let r = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![1009980])))]);
        let result = segfilter_mononi(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn m_b_l_not_mo_r_mononi() {
        // M-B l-not-mo r-mononi cnt=1 l-segs=1
        // sat-l full (not mo), con-l empty → (list (list l r))
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![1009980])))]);
        let result = segfilter_mononi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn m_c_l_mo_r_mononi_empty() {
        // M-C l-mo r-mononi => NIL
        // sat-l empty, con-l full, sat-r full, con-r empty → empty result
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2028940])))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![1009980])))]);
        let result = segfilter_mononi(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn m_d_l_mixed_mo_r_mononi() {
        // M-D l-mixed-mo r-mononi cnt=1 l-segs=1 l0-info=(999)
        // sat-l = non-mo, con-l = mo, sat-r full, con-r empty
        // → base skipped; sat-l push → 1 pair (mslf l sat-l, mslf r sat-r)
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![2028940]))),
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![1009980])))]);
        let result = segfilter_mononi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }
}
