//! Port of `ichiran/dict:segfilter-aux-verb` (`dict-grammar.lisp:1077`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-aux-verb (l r)
//!   (filter-is-conjugation 13)
//!   (apply #'filter-in-seq-set *aux-verbs*))
//! ```
//!
//! `def-segfilter-must-follow` template (`dict-grammar.lisp:1039`)
//! inlined per §4.6. `seg-left` is nil-able (`dict.lisp:1172` passes
//! nil); `seg-right` is always non-nil.

use std::sync::Arc;

use super::_star_aux_verbs_star_::AUX_VERBS;
use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_conjugation::filter_is_conjugation;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;

pub fn segfilter_aux_verb(
    seg_left: Option<&Arc<SegmentList>>,
    seg_right: &Arc<SegmentList>,
) -> Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> {
    // dict-grammar.lisp:1043 (def-segfilter-must-follow expansion)
    // — classify right by filter-right first.
    let filter_right = filter_in_seq_set(AUX_VERBS.to_vec());
    let (sat_r, con_r) = classify(filter_right, &seg_right.segments);

    // Cond clause 1: (or (not sat-r) (and allow-first (not l))).
    // allow-first = nil here, so just (not sat-r).
    if sat_r.is_empty() {
        return vec![(seg_left.cloned(), Arc::clone(seg_right))];
    }

    // Cond clause 2: (or (not l) (/= l.end r.start)).
    let l = match seg_left {
        None => {
            return if con_r.is_empty() {
                Vec::new()
            } else {
                vec![(None, Arc::new(make_segment_list_from(seg_right, con_r)))]
            };
        }
        Some(l) if l.end != seg_right.start => {
            return if con_r.is_empty() {
                Vec::new()
            } else {
                vec![(
                    Some(Arc::clone(l)),
                    Arc::new(make_segment_list_from(seg_right, con_r)),
                )]
            };
        }
        Some(l) => l,
    };

    // T branch: classify left by filter-left, recombine.
    let filter_left = filter_is_conjugation(13);
    let (sat_l, con_l) = classify(filter_left, &l.segments);

    if con_l.is_empty() {
        return vec![(Some(Arc::clone(l)), Arc::clone(seg_right))];
    }

    let mut result: Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> = Vec::new();
    if !con_r.is_empty() {
        // Base pair carries the ORIGINAL left segment-list unchanged,
        // mirroring the Lisp `(list ,segment-list-left
        // (make-segment-list-from ,segment-list-right ,contradicts-right))`.
        result.push((
            Some(Arc::clone(l)),
            Arc::new(make_segment_list_from(seg_right, con_r)),
        ));
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
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::conj_prop_dao::ConjProp;
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

    fn cdata(conj_type: i32) -> ConjData {
        ConjData {
            seq: None,
            from: None,
            via: None,
            prop: Some(ConjProp {
                id: 0,
                conj_id: 0,
                conj_type,
                pos: String::new(),
                neg: None,
                fml: None,
            }),
            src_map: vec![],
        }
    }

    fn info_with_conj(conj: Vec<ConjData>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set: vec![],
            conj,
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
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes from `/tmp/probe_aux_verb.lisp` (this session); each
    // assertion below pins a Lisp result line.

    #[test]
    fn a_l_nil_r_no_match() {
        // A l=NIL r=no-match -> {(L=NIL R=[r unchanged 1 seg seq=999])}
        let r = Arc::new(sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![999]))]));
        let result = segfilter_aux_verb(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].info.as_ref().unwrap().seq_set, vec![999]);
    }

    #[test]
    fn b_l_nil_r_all_match() {
        // B l=NIL r=all-match -> {} (empty — sat-r is full, con-r is empty)
        let r = Arc::new(sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![1342560]))]));
        let result = segfilter_aux_verb(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn c_l_nil_r_mixed() {
        // C l=NIL r=mixed -> {(L=NIL R=[1-seg seq=999])}
        let r = Arc::new(sl(
            0,
            2,
            vec![
                seg(0, 2, info_with_seq_set(vec![1342560])),
                seg(0, 2, info_with_seq_set(vec![999])),
            ],
        ));
        let result = segfilter_aux_verb(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].info.as_ref().unwrap().seq_set, vec![999]);
    }

    #[test]
    fn d_l_adj_gap_r_mixed() {
        // D l=adj-gap (l.end != r.start), r=mixed -> {(L=l unchanged, R=r-reduced-to-non-aux 1 seg)}
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, info_with_conj(vec![]))]));
        let r = Arc::new(sl(
            2,
            4,
            vec![
                seg(2, 4, info_with_seq_set(vec![1342560])),
                seg(2, 4, info_with_seq_set(vec![999])),
            ],
        ));
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        let lp_ref = lp.as_ref().unwrap();
        assert_eq!(lp_ref.segments.len(), 1);
        assert_eq!(rp.segments.len(), 1);
        assert_eq!(rp.segments[0].info.as_ref().unwrap().seq_set, vec![999]);
    }

    #[test]
    fn e_l_no_sat_r_mixed() {
        // E l-no-sat (cd missing conj-type=13), r=mixed -> {(L=l unchanged 1 seg, R=r-reduced 1 seg)}
        let l = Arc::new(sl(0, 2, vec![seg(0, 2, info_with_conj(vec![]))]));
        let r = Arc::new(sl(
            2,
            4,
            vec![
                seg(2, 4, info_with_seq_set(vec![1342560])),
                seg(2, 4, info_with_seq_set(vec![999])),
            ],
        ));
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        assert_eq!(lp.as_ref().unwrap().segments.len(), 1);
        assert_eq!(rp.segments.len(), 1);
        assert_eq!(rp.segments[0].info.as_ref().unwrap().seq_set, vec![999]);
    }

    #[test]
    fn f_l_mixed_r_mixed() {
        // F l-mixed (conj13 + conj3) r-mixed -> two splits:
        //   1st: (L=mslf(l, sat_l)=1 seg, R=mslf(r, sat_r)=1 seg seq=1342560)
        //   2nd: (L=l unchanged 2 segs, R=mslf(r, con_r)=1 seg seq=999)
        let l = Arc::new(sl(
            0,
            2,
            vec![
                seg(0, 2, info_with_conj(vec![cdata(13)])),
                seg(0, 2, info_with_conj(vec![cdata(3)])),
            ],
        ));
        let r = Arc::new(sl(
            2,
            4,
            vec![
                seg(2, 4, info_with_seq_set(vec![1342560])),
                seg(2, 4, info_with_seq_set(vec![999])),
            ],
        ));
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 2);

        // First pair: sat-l × sat-r.
        let (lp0, rp0) = &result[0];
        let lp0_ref = lp0.as_ref().unwrap();
        assert_eq!(lp0_ref.segments.len(), 1);
        assert_eq!(rp0.segments.len(), 1);
        assert_eq!(rp0.segments[0].info.as_ref().unwrap().seq_set, vec![1342560]);

        // Second pair: l unchanged × con-r.
        let (lp1, rp1) = &result[1];
        let lp1_ref = lp1.as_ref().unwrap();
        assert_eq!(lp1_ref.segments.len(), 2);
        assert_eq!(rp1.segments.len(), 1);
        assert_eq!(rp1.segments[0].info.as_ref().unwrap().seq_set, vec![999]);
    }

    #[test]
    fn g_l_all_sat_r_all_sat() {
        // G l-all-sat r-all-sat -> {(L=l unchanged, R=r unchanged)} (con-l empty path)
        let l = Arc::new(sl(0, 2, vec![seg(0, 2, info_with_conj(vec![cdata(13)]))]));
        let r = Arc::new(sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![1342560]))]));
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn i_l_all_sat_r_no_match() {
        // I l-all-sat r-no-match -> {(L=l unchanged, R=r unchanged)} (clause-1 path, sat-r empty)
        let l = Arc::new(sl(0, 2, vec![seg(0, 2, info_with_conj(vec![cdata(13)]))]));
        let r = Arc::new(sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![999]))]));
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }

    #[test]
    fn j_l_mixed_r_all_sat() {
        // J l-mixed r-all-sat -> {(L=mslf(l, sat_l), R=mslf(r, sat_r))}  (con-r empty; no base pair)
        let l = Arc::new(sl(
            0,
            2,
            vec![
                seg(0, 2, info_with_conj(vec![cdata(13)])),
                seg(0, 2, info_with_conj(vec![cdata(3)])),
            ],
        ));
        let r = Arc::new(sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![1342560]))]));
        let result = segfilter_aux_verb(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![1342560]
        );
    }
}
