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
        let ai = actual.info.as_ref().unwrap();
        let ei = expected.info.as_ref().unwrap();
        assert_eq!(ai.seq_set, ei.seq_set);
        assert_eq!(ai.posi, ei.posi);
        assert_eq!(ai.common, ei.common);
        assert_eq!(ai.kpcl, ei.kpcl);
        assert_eq!(ai.conj.len(), ei.conj.len());
        assert_eq!(ai.score_info.prop_score, ei.score_info.prop_score);
        assert_eq!(ai.score_info.kanji_break, ei.score_info.kanji_break);
        assert_eq!(ai.score_info.use_length_bonus, ei.score_info.use_length_bonus);
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
        assert_eq!(got[0].matches, 7);
    }
}
