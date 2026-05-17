//! Port of `ichiran/dict:segfilter-dekiru` (`dict-grammar.lisp:1150`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-dekiru (l r)
//!   ;; 出 followed by 来る or 来てる
//!   (complement (filter-in-seq-set 1896380 2422860))
//!   (filter-in-seq-set 2830009 1547720)
//!   :allow-first t)
//! ```

use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;

const DE_SEQS: &[i32] = &[1896380, 2422860];
const KURU_SEQS: &[i32] = &[2830009, 1547720];

pub fn segfilter_dekiru(
    seg_left: Option<&SegmentList>,
    seg_right: &SegmentList,
) -> Vec<(Option<SegmentList>, SegmentList)> {
    let filter_right = filter_in_seq_set(KURU_SEQS.to_vec());
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

    // T branch. Left filter is the complement of (filter-in-seq-set 1896380 2422860).
    let inner = filter_in_seq_set(DE_SEQS.to_vec());
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

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes from `/tmp/probe_dekiru.lisp` (this session).

    #[test]
    fn de_a_l_nil_r_all_match() {
        // De-A l=NIL r-all-match -> {(L=NIL R=[1 seg seq=2830009])} (allow-first)
        let r = sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2830009])))]);
        let result = segfilter_dekiru(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn de_b_l_nil_r_mixed() {
        // De-B l=NIL r-mixed -> {(L=NIL R=[r unchanged, 2 segs])}
        let r = sl(
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
        let l = sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![1896380])))]);
        let r = sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![999])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn de_d_l_sat_l_r_sat_r() {
        // De-D l-sat-l (not 出) r-sat-r (来る) -> {(L=l unchanged, R=r unchanged)} (con-l empty)
        let l = sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![123])))]);
        let r = sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn de_e_l_de_r_mixed() {
        // De-E l-de r-mixed -> {(L=l unchanged 1 seg, R=mslf(r, con_r)=1 seg seq=999)}
        // l = only 出 → sat-l empty (complement of in-de-seqs is false), con-l = l's seg.
        let l = sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![1896380])))]);
        let r = sl(
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
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }

    #[test]
    fn de_f_l_mixed_r_mixed() {
        // De-F l-mixed (1896380=出 fails, 888 sat) r-mixed -> two splits:
        //   1st: (L=mslf(l, sat_l)=1 seg [888], R=mslf(r, sat_r)=1 seg [2830009])
        //   2nd: (L=l unchanged 2 segs, R=mslf(r, con_r)=1 seg [999])
        let l = sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1896380]))),
                seg(0, 1, Some(info_with_seq_set(vec![888]))),
            ],
        );
        let r = sl(
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
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0]
                .info
                .as_ref()
                .unwrap()
                .seq_set,
            vec![888]
        );
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![2830009]
        );

        // Second pair: l unchanged × con_r.
        assert_eq!(result[1].0.as_ref().unwrap().segments.len(), 2);
        assert_eq!(result[1].1.segments.len(), 1);
        assert_eq!(
            result[1].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }

    #[test]
    fn de_g_l_mixed_r_all_sat() {
        // De-G l-mixed r-all-sat -> {(L=mslf(l, sat_l)=1 seg, R=mslf(r, sat_r)=1 seg)} (con_r empty)
        let l = sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1896380]))),
                seg(0, 1, Some(info_with_seq_set(vec![888]))),
            ],
        );
        let r = sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0]
                .info
                .as_ref()
                .unwrap()
                .seq_set,
            vec![888]
        );
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn de_h_l_info_nil_r_sat() {
        // De-H l-info-nil r-sat -> {(L=l unchanged, R=r unchanged)}
        // info=None ⇒ seq-set empty ⇒ inner filter returns false ⇒ complement returns true ⇒
        // sat-l = full, con-l empty, falls through to "(list (list l r))".
        let l = sl(0, 1, vec![seg(0, 1, None)]);
        let r = sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![2830009])))]);
        let result = segfilter_dekiru(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert!(result[0].0.as_ref().unwrap().segments[0].info.is_none());
    }
}
