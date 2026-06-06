//! Port of `ichiran/dict:segfilter-wokarasu` (`dict-grammar.lisp:1091`).
//!
//! Keeps a からす (2087020) right segment only when the preceding left
//! segment is を (2029010) — the only must-follow segfilter whose left
//! filter is not complemented.

use std::sync::Arc;

use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani_lite_segment_list::KaniLiteSegmentList;

const WO_SEQ: i32 = 2029010;
const KARASU_SEQS: &[i32] = &[2087020];

pub fn segfilter_wokarasu(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    // Left filter is (filter-in-seq-set 2029010) — no complement here
    // unlike most segfilters; sat-l = matches を, con-l = does not.
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        filter_in_seq_set(vec![WO_SEQ]),
        filter_in_seq_set(KARASU_SEQS.to_vec()),
        false,
    )
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

    fn seg(start: usize, end: usize, info: KaniSegmentInfo) -> Segment {
        Segment { start, end, word: dummy_word(), score: None, info: Some(info), top: None, text: None }
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

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn w_a_l_nil_r_karasu_empty() {
        // W-A l=NIL r=karasu cnt=0 — no allow-first, clause-2 (not l)=T, con-r empty → ()
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn w_b_l_nil_r_mixed() {
        // W-B l=NIL r=mixed cnt=1 — clause-2 (not l)=T, con-r non-empty
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![2087020])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_wokarasu(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn w_c_l_nil_r_no_match() {
        // W-C l=NIL r=no-match cnt=1 — clause-1
        let r = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let result = segfilter_wokarasu(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn w_d_l_wo_r_karasu_pass_through() {
        // W-D l-wo r-karasu cnt=1 — sat-l full, con-l empty → (l, r)
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2029010]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn w_e_l_not_wo_r_karasu_empty() {
        // W-E l-not-wo r-karasu cnt=0 — sat-l empty, con-l full; con-r empty → ()
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn w_f_l_mixed_r_mixed_two_pairs() {
        // W-F l-mixed (wo + other) r-mixed (karasu + other) cnt=2
        //   [0] sat-l × sat-r: L=(wo, 1 seg) × R=(karasu, 1 seg)
        //   [1] l unchanged × con-r: L=(2 segs) × R=(other, 1 seg seq=888)
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, info_with_seq_set(vec![2029010])),
                seg(0, 1, info_with_seq_set(vec![999])),
            ],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, info_with_seq_set(vec![2087020])),
                seg(1, 2, info_with_seq_set(vec![888])),
            ],
        );
        let result = segfilter_wokarasu(Some(&l), &r);
        assert_eq!(result.len(), 2);

        // First pair: sat-l × sat-r.
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![2029010]);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![2087020]);

        // Second pair: l unchanged × con-r.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments.len(), 1);
        assert_eq!(result[1].1.segments[0].seq_set, vec![888]);
    }

    #[test]
    fn w_g_gap_r_mixed() {
        // W-G gap (l.end=1, r.start=2) r-mixed cnt=1 — clause-2 with con-r non-empty
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2029010]))]);
        let r = lite_sl(
            2,
            3,
            vec![
                seg(2, 3, info_with_seq_set(vec![2087020])),
                seg(2, 3, info_with_seq_set(vec![888])),
            ],
        );
        let result = segfilter_wokarasu(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![888]);
    }

    #[test]
    fn w_h_gap_r_all_karasu_empty() {
        // W-H gap r-all-karasu cnt=0 — clause-2 with con-r empty → ()
        let l = lite_sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2029010]))]);
        let r = lite_sl(2, 3, vec![seg(2, 3, info_with_seq_set(vec![2087020]))]);
        let result = segfilter_wokarasu(Some(&l), &r);
        assert!(result.is_empty());
    }
}
