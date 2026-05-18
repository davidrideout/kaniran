//! Port of `ichiran/dict:segfilter-janai` (`dict-grammar.lisp:1122`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-janai (l r)
//!   (complement (filter-is-compound-end 2028920))
//!   (filter-in-seq-set 1529520 1296400 2139720)
//!   :allow-first t)
//! ```

use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_compound_end::filter_is_compound_end;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;

const HA_SEQ: i32 = 2028920;
const JANAI_SEQS: &[i32] = &[1529520, 1296400, 2139720];

pub fn segfilter_janai(
    seg_left: Option<&SegmentList>,
    seg_right: &SegmentList,
) -> Vec<(Option<SegmentList>, SegmentList)> {
    let filter_right = filter_in_seq_set(JANAI_SEQS.to_vec());
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

    // T branch. Left filter is the complement of
    // (filter-is-compound-end 2028920).
    let inner = filter_is_compound_end(vec![HA_SEQ]);
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
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
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

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes from `/tmp/probe_410_414.lisp` (this session).

    #[test]
    fn j_a_l_nil_r_janai_pass_through() {
        // J-A l=NIL r=janai cnt=1 r-segs=1
        // allow-first → (list (list nil r))
        let r = sl(
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
        let l = sl(
            0,
            1,
            vec![seg(0, 1, simple_word(999), Some(info_with_seq_set(vec![999])))],
        );
        let r = sl(
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
        let l = sl(
            0,
            2,
            vec![seg(
                0,
                2,
                compound_word_ending_in(2028920),
                Some(info_with_seq_set(vec![2028920])),
            )],
        );
        let r = sl(
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
        let l = sl(
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
        let r = sl(
            2,
            3,
            vec![seg(2, 3, simple_word(1529520), Some(info_with_seq_set(vec![1529520])))],
        );
        let result = segfilter_janai(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0]
                .info
                .as_ref()
                .unwrap()
                .seq_set,
            vec![999]
        );
    }

    #[test]
    fn j_e_gap_r_janai_mixed() {
        // J-E gap janai-mixed cnt=1 r-segs=1 r-info=(999)
        // clause-2 (l.end=1 != r.start=2) with con-r non-empty
        // → (list (list l (mslf r con-r)))
        let l = sl(
            0,
            1,
            vec![seg(0, 1, simple_word(999), Some(info_with_seq_set(vec![999])))],
        );
        let r = sl(
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
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }
}
