//! Port of `ichiran/dict:segfilter-janai` (`dict-grammar.lisp:1122`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-janai (l r)
//!   (complement (filter-is-compound-end 2028920))
//!   (filter-in-seq-set 1529520 1296400 2139720)
//!   :allow-first t)
//! ```

use std::sync::Arc;

use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_compound_end::filter_is_compound_end;
use super::kani_lite_segment_list::KaniLiteSegmentList;

const HA_SEQ: i32 = 2028920;
const JANAI_SEQS: &[i32] = &[1529520, 1296400, 2139720];

pub fn segfilter_janai(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let inner = filter_is_compound_end(vec![HA_SEQ]);
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(JANAI_SEQS.to_vec()),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_list_struct::SegmentList;
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
    use crate::dict::simple_text_class::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn simple_word(seq: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(kana("x", seq))
    }

    fn compound_word_ending_in(seq: i32) -> KaniWordDispatchEnum {
        let words = vec![
            KaniWordDispatchEnum::Kana(kana("a", 999)),
            KaniWordDispatchEnum::Kana(kana("b", seq)),
        ];
        let primary = Box::new(words[0].clone());
        KaniWordDispatchEnum::Compound(CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
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

    fn seg(
        start: usize,
        end: usize,
        word: KaniWordDispatchEnum,
        info: Option<KaniSegmentInfo>,
    ) -> Segment {
        Segment {
            start,
            end,
            word,
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
    fn j_a_l_nil_r_janai_pass_through() {
        // J-A l=NIL r=janai cnt=1 r-segs=1
        // allow-first → (list (list nil r))
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, simple_word(1529520), Some(info_with_seq_set(vec![1529520])))],
        );
        let result = segfilter_janai(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn j_b_l_simple_r_janai_pass_through() {
        // J-B l-simple r-janai cnt=1 l-segs=1
        // simple-l → (filter-is-compound-end 2028920) returns NIL → complement → T
        // sat-l full, con-l empty → (list (list l r))
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, simple_word(999), Some(info_with_seq_set(vec![999])))],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, simple_word(1529520), Some(info_with_seq_set(vec![1529520])))],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn j_c_l_compound_ending_ha_r_janai_empty() {
        // J-C l-compound-end-wa r-janai => NIL
        // compound ending in 2028920 → filter-is-compound-end T → complement NIL
        // sat-l empty, con-l full; sat-r full, con-r empty → empty result
        let l = lite_sl(
            0,
            2,
            vec![seg(
                0,
                2,
                compound_word_ending_in(2028920),
                Some(info_with_seq_set(vec![2028920])),
            )],
        );
        let r = lite_sl(
            2,
            3,
            vec![seg(2, 3, simple_word(1529520), Some(info_with_seq_set(vec![1529520])))],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn j_d_l_mixed_compound_r_janai() {
        // J-D l-mixed-comp r-janai cnt=1 l-segs=1 l0-info=(999)
        // sat-l = simple (not compound-end-ha), con-l = compound-end-ha
        // sat-r full, con-r empty → base skipped; sat-l push → 1 pair
        let l = lite_sl(
            0,
            2,
            vec![
                seg(
                    0,
                    2,
                    compound_word_ending_in(2028920),
                    Some(info_with_seq_set(vec![2028920])),
                ),
                seg(0, 2, simple_word(999), Some(info_with_seq_set(vec![999]))),
            ],
        );
        let r = lite_sl(
            2,
            3,
            vec![seg(2, 3, simple_word(1529520), Some(info_with_seq_set(vec![1529520])))],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
    }

    #[test]
    fn j_e_gap_r_janai_mixed() {
        // J-E gap janai-mixed cnt=1 r-segs=1 r-info=(999)
        // clause-2 (l.end=1 != r.start=2) with con-r non-empty
        // → (list (list l (mslf r con-r)))
        let l = lite_sl(
            0,
            1,
            vec![seg(0, 1, simple_word(999), Some(info_with_seq_set(vec![999])))],
        );
        let r = lite_sl(
            2,
            3,
            vec![
                seg(2, 3, simple_word(1296400), Some(info_with_seq_set(vec![1296400]))),
                seg(2, 3, simple_word(999), Some(info_with_seq_set(vec![999]))),
            ],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }
}
