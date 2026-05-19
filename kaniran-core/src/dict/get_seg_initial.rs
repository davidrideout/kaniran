//! Port of `ichiran/dict:get-seg-initial` (`dict.lisp:1171-1173`).
//!
//! ```lisp
//! (defun get-seg-initial (seg)
//!   (loop for split in (apply-segfilters nil seg)
//!      collect (cadr split)))
//! ```

use super::apply_segfilters::apply_segfilters;
use super::segment_list_struct::SegmentList;

pub fn get_seg_initial(seg: &SegmentList) -> Vec<SegmentList> {
    apply_segfilters(None, seg)
        .into_iter()
        .map(|(_left, right)| right)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
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
            conj: vec![] as Vec<ConjData>,
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

    fn seg(start: usize, end: usize, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info_with_seq_set(seq_set)),
            top: None,
            text: Some(String::new()),
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

    // REPL probes (/tmp/probe_a.lisp on .103, 2026-05-18) used
    // `join-substring-words` to build real segment-lists; the cases
    // below pin the same shapes via synthetic segments to avoid the
    // database dependency at unit-test time. apply_segfilters'
    // own tests at apply_segfilters.rs:175-283 already pin the
    // outputs that feed in.

    fn assert_segment_eq(actual: &Segment, expected: &Segment) {
        assert_eq!(actual.start, expected.start);
        assert_eq!(actual.end, expected.end);
        assert_eq!(actual.score, expected.score);
        assert_eq!(actual.text, expected.text);
        assert!(actual.top.is_none() && expected.top.is_none());
        assert_eq!(
            format!("{:?}", actual.word),
            format!("{:?}", expected.word),
            "word"
        );
        let ai = actual.info.as_ref().unwrap();
        let ei = expected.info.as_ref().unwrap();
        assert_eq!(ai.seq_set, ei.seq_set);
        assert_eq!(ai.posi, ei.posi);
        assert_eq!(ai.common, ei.common);
        assert_eq!(ai.kpcl, ei.kpcl);
        assert_eq!(ai.conj.len(), ei.conj.len());
        for (i, (ac, ec)) in ai.conj.iter().zip(ei.conj.iter()).enumerate() {
            assert_eq!(ac.seq, ec.seq, "conj[{}].seq", i);
            assert_eq!(ac.from, ec.from, "conj[{}].from", i);
            assert_eq!(ac.via, ec.via, "conj[{}].via", i);
            assert_eq!(ac.src_map, ec.src_map, "conj[{}].src_map", i);
            match (&ac.prop, &ec.prop) {
                (None, None) => {}
                (Some(ap), Some(ep)) => {
                    assert_eq!(ap.id, ep.id, "conj[{}].prop.id", i);
                    assert_eq!(ap.conj_id, ep.conj_id, "conj[{}].prop.conj_id", i);
                    assert_eq!(ap.conj_type, ep.conj_type, "conj[{}].prop.conj_type", i);
                    assert_eq!(ap.pos, ep.pos, "conj[{}].prop.pos", i);
                    assert_eq!(ap.neg, ep.neg, "conj[{}].prop.neg", i);
                    assert_eq!(ap.fml, ep.fml, "conj[{}].prop.fml", i);
                }
                (a, e) => panic!(
                    "conj[{}].prop: actual={} expected={}",
                    i,
                    if a.is_some() { "Some" } else { "None" },
                    if e.is_some() { "Some" } else { "None" }
                ),
            }
        }
        assert_eq!(ai.score_info.prop_score, ei.score_info.prop_score);
        assert_eq!(ai.score_info.kanji_break, ei.score_info.kanji_break);
        assert_eq!(ai.score_info.use_length_bonus, ei.score_info.use_length_bonus);
        match (&ai.score_info.split_info, &ei.score_info.split_info) {
            (KaniSplitInfo::None, KaniSplitInfo::None) => {}
            (KaniSplitInfo::Score(a), KaniSplitInfo::Score(e)) => assert_eq!(a, e),
            (
                KaniSplitInfo::Parts { score_mod: ams, part_scores: aps },
                KaniSplitInfo::Parts { score_mod: ems, part_scores: eps },
            ) => {
                assert_eq!(ams, ems, "split_info.score_mod");
                assert_eq!(aps, eps, "split_info.part_scores");
            }
            (a, e) => panic!("split_info: actual={:?} expected={:?}", a, e),
        }
    }

    fn assert_segment_list_eq(actual: &SegmentList, expected: &SegmentList) {
        assert_eq!(actual.start, expected.start);
        assert_eq!(actual.end, expected.end);
        assert_eq!(actual.matches, expected.matches);
        assert!(actual.top.is_none() && expected.top.is_none());
        assert_eq!(actual.segments.len(), expected.segments.len());
        for (a, e) in actual.segments.iter().zip(expected.segments.iter()) {
            assert_segment_eq(a, e);
        }
    }

    #[test]
    fn a1_empty_segment_list_returns_passthrough() {
        let r = sl(0, 0, vec![]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_segment_list_eq(&got[0], &r);
    }

    #[test]
    fn a2_seq_not_in_any_segfilter_returns_one_unchanged() {
        let r = sl(0, 2, vec![seg(0, 2, vec![999])]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_segment_list_eq(&got[0], &r);
    }

    #[test]
    fn a3_aux_verb_only_seg_yields_zero_splits() {
        let r = sl(0, 2, vec![seg(0, 2, vec![1342560])]);
        let got = get_seg_initial(&r);
        assert!(got.is_empty());
    }

    #[test]
    fn a4_matches_field_carries_through_unchanged() {
        let mut r = sl(0, 2, vec![seg(0, 2, vec![999])]);
        r.matches = 7;
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_segment_list_eq(&got[0], &r);
    }

    #[test]
    fn a5_mixed_aux_and_normal_yields_filtered_subset() {
        // dict-grammar.lisp:1047-1054 — when seg-left is nil and
        // satisfies-right is non-empty, def-segfilter-must-follow falls
        // into clause 2: pushes (nil, mslf(r, contradicts-right)) when
        // contradicts-right is non-empty. For seg-left=nil:
        //   r.segments = [seg(seq=1342560 aux-verb), seg(seq=999 normal)]
        //   satisfies-right = [aux],  contradicts-right = [normal]
        //   → one pair: (nil, SL{segments=[normal], start/end/matches kept})
        // get_seg_initial then collects right halves → [SL with just normal].
        let aux_seg = seg(0, 2, vec![1342560]);
        let normal_seg = seg(0, 2, vec![999]);
        let r = sl(0, 2, vec![aux_seg, normal_seg.clone()]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        let expected = sl(0, 2, vec![normal_seg]);
        assert_segment_list_eq(&got[0], &expected);
        // explicit: the aux segment must NOT be in the output.
        assert_eq!(got[0].segments.len(), 1);
        assert!(got[0].segments[0]
            .info
            .as_ref()
            .unwrap()
            .seq_set
            .contains(&999));
        assert!(!got[0].segments[0]
            .info
            .as_ref()
            .unwrap()
            .seq_set
            .contains(&1342560));
    }
}
