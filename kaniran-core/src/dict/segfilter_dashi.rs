//! Port of `ichiran/dict:segfilter-dashi` (`dict-grammar.lisp:1143`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-dashi (l r)
//!   (lambda (segment &aux (seq-set (getf (segment-info segment) :seq-set)))
//!     (or (not (find 2089020 seq-set)) ;; だ
//!         (find 2028980 seq-set))) ;; で
//!   (filter-in-seq-set 1157170 2424740 1305070) ;; する　して
//!   :allow-first t)
//! ```

use std::sync::Arc;

use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;
use super::segment_struct::Segment;

const SEQ_DA: i32 = 2089020;
const SEQ_DE: i32 = 2028980;
const SURU_SETE_SEQS: &[i32] = &[1157170, 2424740, 1305070];

fn filter_left(segment: &Segment) -> bool {
    // dict-grammar.lisp:1144 (lambda &aux seq-set ...)
    let seq_set: &[i32] = match &segment.info {
        Some(info) => &info.seq_set,
        None => &[],
    };
    !seq_set.contains(&SEQ_DA) || seq_set.contains(&SEQ_DE)
}

pub fn segfilter_dashi(
    seg_left: Option<&Arc<SegmentList>>,
    seg_right: &Arc<SegmentList>,
) -> Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> {
    let filter_right = filter_in_seq_set(SURU_SETE_SEQS.to_vec());
    let (sat_r, con_r) = classify(filter_right, &seg_right.segments);

    // Cond clause 1: (or (not sat-r) (and allow-first (not l)))
    // allow-first = t, so include the (not l) disjunct.
    if sat_r.is_empty() || seg_left.is_none() {
        return vec![(seg_left.cloned(), Arc::clone(seg_right))];
    }

    // Cond clause 2: (or (not l) (/= l.end r.start)) — (not l) already
    // handled above by the allow-first branch.
    let l = seg_left.unwrap();
    if l.end != seg_right.start {
        return if con_r.is_empty() {
            Vec::new()
        } else {
            vec![(Some(Arc::clone(l)), Arc::new(make_segment_list_from(seg_right, con_r)))]
        };
    }

    // T branch.
    let (sat_l, con_l) = classify(filter_left, &l.segments);

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
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};
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

    // REPL probes from `/tmp/probe_dashi.lisp` (this session).

    #[test]
    fn da_a_l_nil_r_all_match_passes_through_allow_first() {
        // Da-A l=NIL r-all-match -> {(L=NIL R=[1-seg seq=1157170])} (allow-first short-circuit)
        let r = Arc::new(sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![1157170])))]));
        let result = segfilter_dashi(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn da_b_l_nil_r_mixed_passes_through() {
        // Da-B l=NIL r-mixed -> {(L=NIL R=[2-segs unchanged])}
        let r = Arc::new(sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1157170]))),
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
            ],
        ));
        let result = segfilter_dashi(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 2);
    }

    #[test]
    fn da_c_l_da_r_no_match_passes_through() {
        // Da-C l-da r-no-match -> {(L=l unchanged 1 seg, R=r unchanged 1 seg seq=999)}
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2089020])))]));
        let r = Arc::new(sl(1, 3, vec![seg(1, 3, Some(info_with_seq_set(vec![999])))]));
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn da_d_l_sat_l_r_sat_r() {
        // Da-D l-sat-l (no 2089020) r-sat-r (suru) -> {(L=l unchanged, R=r unchanged)}
        // (sat-l = all of l; con-l empty; falls through to "(list (list l r))")
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![123])))]));
        let r = Arc::new(sl(1, 3, vec![seg(1, 3, Some(info_with_seq_set(vec![1157170])))]));
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn da_e_l_da_r_mixed() {
        // Da-E l-da r-mixed -> {(L=l unchanged 1 seg, R=mslf(r, con_r)=1 seg seq=999)}
        // l has only da: sat-l empty (da fails left filter), con-l full.
        // Only the base pair emits (sat-l prepend skipped).
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2089020])))]));
        let r = Arc::new(sl(
            1,
            3,
            vec![
                seg(1, 3, Some(info_with_seq_set(vec![1157170]))),
                seg(1, 3, Some(info_with_seq_set(vec![999]))),
            ],
        ));
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }

    #[test]
    fn da_f_l_de_r_suru() {
        // Da-F l-de (sat-l: has 2028980) r-suru -> {(L=l unchanged, R=r unchanged)} (con-l empty)
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2028980])))]));
        let r = Arc::new(sl(1, 3, vec![seg(1, 3, Some(info_with_seq_set(vec![1157170])))]));
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn da_g_l_da_then_da_de_r_suru() {
        // Da-G l-da-then-da+de (sat-l = 2nd seg has で; con-l = 1st seg da-only) r-suru ->
        // {(L=mslf(l, sat_l)=1 seg [da+de], R=mslf(r, sat_r)=1 seg)} (no base pair: con_r empty)
        let l = Arc::new(sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![2089020]))),
                seg(0, 1, Some(info_with_seq_set(vec![2089020, 2028980]))),
            ],
        ));
        let r = Arc::new(sl(1, 3, vec![seg(1, 3, Some(info_with_seq_set(vec![1157170])))]));
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0]
                .info
                .as_ref()
                .unwrap()
                .seq_set,
            vec![2089020, 2028980]
        );
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn da_h_l_da_r_mixed_gap() {
        // Da-H l-da (l.end=1) r-mixed-gap (r.start=2) -> {(L=l unchanged, R=mslf(r, con_r)=1 seg)}
        // l.end != r.start; con-r non-empty.
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2089020])))]));
        let r = Arc::new(sl(
            2,
            4,
            vec![
                seg(2, 4, Some(info_with_seq_set(vec![1157170]))),
                seg(2, 4, Some(info_with_seq_set(vec![999]))),
            ],
        ));
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }

    #[test]
    fn da_i_l_info_nil_r_suru() {
        // Da-I l-info-nil r-suru -> {(L=l unchanged, R=r unchanged)}
        // info=None ⇒ seq-set empty ⇒ left filter truthy (not (find da empty)=t).
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, None)]));
        let r = Arc::new(sl(1, 3, vec![seg(1, 3, Some(info_with_seq_set(vec![1157170])))]));
        let result = segfilter_dashi(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert!(result[0].0.as_ref().unwrap().segments[0].info.is_none());
    }
}
