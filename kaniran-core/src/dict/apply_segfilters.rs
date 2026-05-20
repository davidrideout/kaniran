//! Port of `ichiran/dict:apply-segfilters` (`dict-grammar.lisp:1170`).
//!
//! Threads `(seg-left, seg-right)` through each filter in
//! [`SEGFILTER_LIST`] in order. Each filter returns a list of
//! `(seg-left, seg-right)` candidates; the union (Lisp `nconc`) of
//! those candidates becomes the input to the next filter.
//!
//! Divergences from Lisp:
//! - `seg-left` is [`Option<&SegmentList>`] at the public boundary —
//!   matching the call sites — and the rolling `splits` Vec stores
//!   `(Option<Arc<SegmentList>>, Arc<SegmentList>)` so the 16
//!   pass-through filter clones reduce to refcount bumps. The Lisp
//!   `(list seg-left seg-right)` is itself pointer-shared. The return
//!   shape is unchanged: `Arc::unwrap_or_clone` collapses each pair
//!   back to owned `SegmentList`s on the way out (cheap when refcount
//!   is 1, which is typical because each split lands in exactly one
//!   downstream pair).
//!
//! [`SEGFILTER_LIST`]: super::_star_segfilter_list_star_::SEGFILTER_LIST

use std::sync::Arc;

use super::_star_segfilter_list_star_::SEGFILTER_LIST;
use super::segment_list_struct::SegmentList;

pub fn apply_segfilters(
    seg_left: Option<&SegmentList>,
    seg_right: &SegmentList,
) -> Vec<(Option<SegmentList>, SegmentList)> {
    // dict-grammar.lisp:1171 (`with splits = (list (list seg-left seg-right))`)
    let mut splits: Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> = vec![(
        seg_left.cloned().map(Arc::new),
        Arc::new(seg_right.clone()),
    )];
    for segfilter in SEGFILTER_LIST {
        // dict-grammar.lisp:1173-1175 (inner loop nconc-ing each
        // filter's output across the current splits)
        let mut next: Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> = Vec::new();
        for (left, right) in &splits {
            next.extend(segfilter(left.as_ref(), right));
        }
        splits = next;
    }
    splits
        .into_iter()
        .map(|(left, right)| (left.map(Arc::unwrap_or_clone), Arc::unwrap_or_clone(right)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::conj_prop_dao::ConjProp;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_struct::{
        KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment,
    };
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

    fn seg_with_text(start: usize, end: usize, info: KaniSegmentInfo, text: &str) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: Some(text.to_string()),
        }
    }

    fn seg(start: usize, end: usize, info: KaniSegmentInfo) -> Segment {
        seg_with_text(start, end, info, "")
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

    // REPL probes (`/tmp/probe_apply_segfilters.lisp` on .103). Each
    // case pins a literal split count and structural shape that the
    // Lisp `apply-segfilters` returned for the same inputs.

    fn assert_seq_set_seg(seg: &Segment, start: usize, end: usize, seq_set: &[i32]) {
        assert_eq!(seg.start, start);
        assert_eq!(seg.end, end);
        let info = seg.info.as_ref().unwrap();
        assert_eq!(info.seq_set, seq_set);
        assert!(info.conj.is_empty());
        assert!(info.posi.is_empty());
        assert!(info.common.is_none());
        assert_eq!(info.kpcl, (false, false, false, false));
    }

    fn assert_conj_seg(seg: &Segment, start: usize, end: usize, conj_type: i32) {
        assert_eq!(seg.start, start);
        assert_eq!(seg.end, end);
        let info = seg.info.as_ref().unwrap();
        assert!(info.seq_set.is_empty());
        assert_eq!(info.conj.len(), 1);
        assert_eq!(info.conj[0].prop.as_ref().unwrap().conj_type, conj_type);
        assert!(info.posi.is_empty());
        assert!(info.common.is_none());
        assert_eq!(info.kpcl, (false, false, false, false));
    }

    fn assert_sl(sl: &SegmentList, start: usize, end: usize, matches: usize, n_segs: usize) {
        assert_eq!(sl.start, start);
        assert_eq!(sl.end, end);
        assert_eq!(sl.matches, matches);
        assert!(sl.top.is_none());
        assert_eq!(sl.segments.len(), n_segs);
    }

    #[test]
    fn a_nil_left_unmatched_right_identity() {
        // l = nil, r = single segment seq=999 — no segfilter matches.
        // REPL: len=1, lp=NIL, rp: start=0 end=2 matches=0 n-segs=1,
        // rp.seg[0] start=0 end=2 seq-set=(999) (all other slots default).
        let r = sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![999]))]);
        let result = apply_segfilters(None, &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        assert!(lp.is_none());
        assert_sl(rp, 0, 2, 0, 1);
        assert_seq_set_seg(&rp.segments[0], 0, 2, &[999]);
    }

    #[test]
    fn b_nil_left_aux_verb_only_right_filtered_to_empty() {
        // l = nil, r = single segment seq=1342560 (aux-verb member).
        // segfilter-aux-verb's allow-first=nil → no fallback when
        // l is nil; everything else is no-op → 0 splits.
        // REPL: len=0.
        let r = sl(0, 2, vec![seg(0, 2, info_with_seq_set(vec![1342560]))]);
        let result = apply_segfilters(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn c_adjacent_l_conj13_r_aux_verb_full_pair() {
        // l = adjacent (l.end == r.start), single conj13 seg.
        // r = single seq=1342560 (aux-verb member).
        // All other 15 segfilters: con-r empty → clause-1 returns
        // (l, r) unchanged. segfilter-aux-verb: con-l empty → also
        // (l, r) unchanged.
        // REPL: len=1, lp: start=0 end=2 matches=0 n-segs=1,
        // rp: start=2 end=4 matches=0 n-segs=1,
        // lp.seg[0]: start=0 end=2 conj-types=(13),
        // rp.seg[0]: start=2 end=4 seq-set=(1342560).
        let l = sl(0, 2, vec![seg(0, 2, info_with_conj(vec![cdata(13)]))]);
        let r = sl(2, 4, vec![seg(2, 4, info_with_seq_set(vec![1342560]))]);
        let result = apply_segfilters(Some(&l), &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        let lp_ref = lp.as_ref().unwrap();
        assert_sl(lp_ref, 0, 2, 0, 1);
        assert_sl(rp, 2, 4, 0, 1);
        assert_conj_seg(&lp_ref.segments[0], 0, 2, 13);
        assert_seq_set_seg(&rp.segments[0], 2, 4, &[1342560]);
    }

    #[test]
    fn d_adjacent_l_mixed_conj_r_mixed_aux_two_splits() {
        // l = two segs: conj-type=13 and conj-type=3.
        // r = two segs: seq=1342560 (aux) and seq=999 (non-aux).
        // Most segfilters pass-through. segfilter-aux-verb:
        //   pair[0] = (mslf(l, sat-l=[conj13]), mslf(r, sat-r=[1342560]))
        //   pair[1] = (l unchanged, mslf(r, con-r=[999]))
        // REPL: len=2.
        //   pair[0]: lp 0/2/matches=0/n=1 (conj13), rp 2/4/0/1 (seq=1342560)
        //   pair[1]: lp 0/2/0/2 (conj13, conj3), rp 2/4/0/1 (seq=999)
        let l = sl(
            0,
            2,
            vec![
                seg(0, 2, info_with_conj(vec![cdata(13)])),
                seg(0, 2, info_with_conj(vec![cdata(3)])),
            ],
        );
        let r = sl(
            2,
            4,
            vec![
                seg(2, 4, info_with_seq_set(vec![1342560])),
                seg(2, 4, info_with_seq_set(vec![999])),
            ],
        );
        let result = apply_segfilters(Some(&l), &r);
        assert_eq!(result.len(), 2);

        // Pair[0]: sat-l × sat-r.
        let (lp0, rp0) = &result[0];
        let lp0_ref = lp0.as_ref().unwrap();
        assert_sl(lp0_ref, 0, 2, 0, 1);
        assert_sl(rp0, 2, 4, 0, 1);
        assert_conj_seg(&lp0_ref.segments[0], 0, 2, 13);
        assert_seq_set_seg(&rp0.segments[0], 2, 4, &[1342560]);

        // Pair[1]: l unchanged × con-r.
        let (lp1, rp1) = &result[1];
        let lp1_ref = lp1.as_ref().unwrap();
        assert_sl(lp1_ref, 0, 2, 0, 2);
        assert_sl(rp1, 2, 4, 0, 1);
        assert_conj_seg(&lp1_ref.segments[0], 0, 2, 13);
        assert_conj_seg(&lp1_ref.segments[1], 0, 2, 3);
        assert_seq_set_seg(&rp1.segments[0], 2, 4, &[999]);
    }

    #[test]
    fn e_nil_left_n_only_right_filtered() {
        // l = nil, r = single seq=2139720 (segfilter-n's sat-r set,
        // allow-first = t).
        // REPL: len=1, lp=NIL, rp: start=0 end=1 matches=0 n-segs=1,
        // rp.seg[0]: start=0 end=1 seq-set=(2139720).
        let r = sl(0, 1, vec![seg(0, 1, info_with_seq_set(vec![2139720]))]);
        let result = apply_segfilters(None, &r);
        assert_eq!(result.len(), 1);
        let (lp, rp) = &result[0];
        assert!(lp.is_none());
        assert_sl(rp, 0, 1, 0, 1);
        assert_seq_set_seg(&rp.segments[0], 0, 1, &[2139720]);
    }
}
