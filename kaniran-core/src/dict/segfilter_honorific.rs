//! Port of `ichiran/dict:segfilter-honorific` (`dict-grammar.lisp:1160`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-honorific (l r)
//!   (complement (apply 'filter-in-seq-set *noun-particles*))
//!   (apply 'filter-in-seq-set *honorifics*))
//! ```
//!
//! Body shared with the other `segfilter_*` callsites via
//! [`super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body`].
//! No `:allow-first` — l=nil falls through to clause-2 of the cond.

use std::sync::Arc;

use super::_star_honorifics_star_::HONORIFICS;
use super::_star_noun_particles_star_::NOUN_PARTICLES;
use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_in_seq_set::filter_in_seq_set;
use super::kani::KaniLiteSegmentList;

pub fn segfilter_honorific(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let inner = filter_in_seq_set(NOUN_PARTICLES.to_vec());
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(HONORIFICS.to_vec()),
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
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
    fn h_a_l_nil_r_all_honor_empty() {
        // H-A l=NIL r=all-honorific => NIL
        // sat-r full, allow-first=nil, clause-2 (not l)=t, con-r empty → ()
        let r = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![1247260])))]);
        let result = segfilter_honorific(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn h_b_l_nil_r_mixed() {
        // H-B l=NIL r=mixed cnt=1
        // clause-2 with (not l)=t, con-r non-empty → (list (list nil (mslf r con-r)))
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![1247260]))),
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_honorific(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn h_c_l_nil_r_no_match() {
        // H-C l=NIL r=no-match cnt=1 l0=NIL r0-segs=1
        // sat-r empty → clause-1 → (list (list l r))
        let r = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let result = segfilter_honorific(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_d_l_not_noun_r_honor() {
        // H-D l-not-noun r-honor cnt=1 l-segs=1 r-segs=1
        // sat-l = full (not in noun particles), con-l empty → (list (list l r))
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![1247260])))]);
        let result = segfilter_honorific(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_e_l_is_noun_r_honor_empty() {
        // H-E l-is-noun r-honor cnt=0
        // sat-l empty, con-l full, sat-r full, con-r empty → empty result
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2028920])))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![1247260])))]);
        let result = segfilter_honorific(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn h_f_l_is_noun_r_mixed() {
        // H-F l-is-noun r-mixed cnt=1 r0-segs=1 r0-seq=(999)
        // sat-l empty, con-l full, sat-r=honor, con-r=other
        // → base pair (l unchanged, mslf r con-r); no sat-l prepend
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![2028920])))]);
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, Some(info_with_seq_set(vec![1247260]))),
                seg(1, 2, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_honorific(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn h_g_l_mixed_r_honor() {
        // H-G l-mixed r-honor cnt=1 l-segs=1 r-segs=1
        // sat-l=non-noun, con-l=noun, sat-r=full, con-r=empty
        // → base skipped; sat-l prepend → 1 pair (mslf l sat-l, mslf r sat-r)
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, Some(info_with_seq_set(vec![2028920]))),
                seg(0, 1, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, Some(info_with_seq_set(vec![1247260])))]);
        let result = segfilter_honorific(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_h_gap_r_mixed() {
        // H-H gap mixed cnt=1 r-segs=1
        // clause-2 (l.end != r.start) with con-r non-empty
        // → (list (list l (mslf r con-r)))
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let r = lite_sl(
            2,
            3,
            vec![
                seg(2, 3, Some(info_with_seq_set(vec![1247260]))),
                seg(2, 3, Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_honorific(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn h_i_gap_r_all_honor_empty() {
        // H-I gap all-honor => NIL
        // clause-2 gap with con-r empty → ()
        let l = lite_sl(0, 1, vec![seg(0, 1, Some(info_with_seq_set(vec![999])))]);
        let r = lite_sl(2, 3, vec![seg(2, 3, Some(info_with_seq_set(vec![1247260])))]);
        let result = segfilter_honorific(Some(&l), &r);
        assert!(result.is_empty());
    }
}
