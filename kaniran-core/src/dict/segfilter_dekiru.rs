//! Port of `ichiran/dict:segfilter-dekiru` (`dict-grammar.lisp:1150`).
//!
//! Keeps a 来る/来てる right segment only when the preceding left
//! segment is not 出.

use std::sync::Arc;

use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani_lite_segment_list::KaniLiteSegmentList;

const DE_SEQS: &[i32] = &[1896380, 2422860];
const KURU_SEQS: &[i32] = &[2830009, 1547720];

pub fn segfilter_dekiru(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let inner = filter_in_seq_set(DE_SEQS.to_vec());
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(KURU_SEQS.to_vec()),
        true,
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

    // REPL probes from `/tmp/probe_dekiru.lisp` (this session).

    #[test]
    fn de_a_l_nil_r_all_match() {
        // De-A l=NIL r-all-match -> {(L=NIL R=[1 seg seq=2830009])} (allow-first)
        let r = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2830009])))]);
        let result = segfilter_dekiru(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn de_b_l_nil_r_mixed() {
        // De-B l=NIL r-mixed -> {(L=NIL R=[r unchanged, 2 segs])}
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
                seg(0, 1, Some(info_with_seq_set(vec![2830009]))),
            ],
        );
        let result = segfilter_dekiru(None, &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 2);
    }

    #[test]
    fn de_c_l_de_r_no_match() {
        // De-C l-de r-no-match -> {(L=l unchanged, R=r unchanged)} (sat-r empty)
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![1896380])))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![999])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn de_d_l_sat_l_r_sat_r() {
        // De-D l-sat-l (not 出) r-sat-r (来る) -> {(L=l unchanged, R=r unchanged)} (con-l empty)
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![123])))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn de_e_l_de_r_mixed() {
        // De-E l-de r-mixed -> {(L=l unchanged 1 seg, R=mslf(r, con_r)=1 seg seq=999)}
        // l = only 出 → sat-l empty (complement of in-de-seqs is false), con-l = l's seg.
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![1896380])))]);
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, Some(info_with_seq_set(vec![2830009]))),
                seg(1, 2, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn de_f_l_mixed_r_mixed() {
        // De-F l-mixed (1896380=出 fails, 888 sat) r-mixed -> two splits:
        //   1st: (L=mslf(l, sat_l)=1 seg [888], R=mslf(r, sat_r)=1 seg [2830009])
        //   2nd: (L=l unchanged 2 segs, R=mslf(r, con_r)=1 seg [999])
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1896380]))),
                seg(0, 1, Some(info_with_seq_set(vec![888]))),
            ],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, Some(info_with_seq_set(vec![2830009]))),
                seg(1, 2, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 2);

        // First pair: sat × sat.
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![888]);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![2830009]);

        // Second pair: l unchanged × con_r.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments.len(), 1);
        assert_eq!(result[1].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn de_g_l_mixed_r_all_sat() {
        // De-G l-mixed r-all-sat -> {(L=mslf(l, sat_l)=1 seg, R=mslf(r, sat_r)=1 seg)} (con_r empty)
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1896380]))),
                seg(0, 1, Some(info_with_seq_set(vec![888]))),
            ],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![888]);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn de_h_l_info_nil_r_sat() {
        // De-H l-info-nil r-sat -> {(L=l unchanged, R=r unchanged)}
        // info=None ⇒ seq-set empty ⇒ inner filter returns false ⇒ complement returns true ⇒
        // sat-l = full, con-l empty, falls through to "(list (list l r))".
        let l = lite_sl(0, 1, vec![seg(0, 1, None)]);
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert!(result[0].0.as_ref().unwrap().segments[0].seq_set.is_empty());
    }
}
