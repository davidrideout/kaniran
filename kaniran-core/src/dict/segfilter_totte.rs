//! Port of `ichiran/dict:segfilter-totte` (`dict-grammar.lisp:1138`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-totte (l r)
//!   (complement (filter-in-seq-set 1008490))
//!   (filter-in-seq-set 2086960)
//!   :allow-first t)
//! ```

use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;

const TO_SEQ: i32 = 1008490;
const TOTTE_SEQS: &[i32] = &[2086960];

pub fn segfilter_totte(
    seg_left: Option<&SegmentList>,
    seg_right: &SegmentList,
) -> Vec<(Option<SegmentList>, SegmentList)> {
    let filter_right = filter_in_seq_set(TOTTE_SEQS.to_vec());
    let (sat_r, con_r) = classify(filter_right, &seg_right.segments);

    // Cond clause 1: (or (not sat-r) (and allow-first (not l)))
    if sat_r.is_empty() || seg_left.is_none() {
        return vec![(seg_left.cloned(), seg_right.clone())];
    }

    // Cond clause 2: (or (not l) (/= l.end r.start)) — (not l) absorbed above.
    let l = seg_left.unwrap();
    if l.end != seg_right.start {
        return if con_r.is_empty() {
            Vec::new()
        } else {
            vec![(Some(l.clone()), make_segment_list_from(seg_right, con_r))]
        };
    }

    // T branch. Left filter is the complement of (filter-in-seq-set 1008490).
    let inner = filter_in_seq_set(vec![TO_SEQ]);
    let (sat_l, con_l) = classify(|s| !inner(s), &l.segments);

    if con_l.is_empty() {
        return vec![(Some(l.clone()), seg_right.clone())];
    }

    let mut result: Vec<(Option<SegmentList>, SegmentList)> = Vec::new();
    if !con_r.is_empty() {
        result.push((Some(l.clone()), make_segment_list_from(seg_right, con_r)));
    }
    if !sat_l.is_empty() {
        // dict-grammar.lisp:1064 (push) — prepend the satisfies pair.
        result.insert(
            0,
            (
                Some(make_segment_list_from(l, sat_l)),
                make_segment_list_from(seg_right, sat_r),
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
    fn t_a_l_nil_r_totte_pass_through() {
        let r = sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2086960]))]);
        let result = segfilter_totte(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn t_b_l_nil_r_no_match() {
        let r = sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_totte(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn t_c_l_not_to_r_totte() {
        let l = sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2086960]))]);
        let result = segfilter_totte(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn t_d_l_to_r_totte_empty() {
        let l = sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![1008490]))]);
        let r = sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2086960]))]);
        let result = segfilter_totte(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn t_e_l_mixed_r_totte() {
        let l = sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![1008490])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let r = sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2086960]))]);
        let result = segfilter_totte(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
        assert_eq!(result[0].1.segments.len(), 1);
    }
}
