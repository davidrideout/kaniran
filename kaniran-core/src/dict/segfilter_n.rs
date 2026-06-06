//! Port of `ichiran/dict:segfilter-n` (`dict-grammar.lisp:1086`).
//!
//! Keeps a ん/んだ right segment only when the preceding left segment
//! is not a noun particle.

use std::sync::Arc;

use super::_star_noun_particles_star_::NOUN_PARTICLES;
use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_in_seq_set_simple::filter_in_seq_set_simple;
use super::kani_lite_segment_list::KaniLiteSegmentList;

const N_SEQS: &[i32] = &[2139720, 2849370, 2849387];

pub fn segfilter_n(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let inner = filter_in_seq_set_simple(NOUN_PARTICLES.to_vec());
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        filter_in_seq_set(N_SEQS.to_vec()),
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

    fn kana(seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: "x".into(),
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
        KaniWordDispatchEnum::Kana(kana(seq))
    }

    fn compound_word(child_seqs: &[i32]) -> KaniWordDispatchEnum {
        let words: Vec<KaniWordDispatchEnum> =
            child_seqs.iter().map(|s| KaniWordDispatchEnum::Kana(kana(*s))).collect();
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

    fn seg(start: usize, end: usize, word: KaniWordDispatchEnum, info: KaniSegmentInfo) -> Segment {
        Segment { start, end, word, score: None, info: Some(info), top: None, text: None }
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

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn n_a_l_nil_r_all_n_pass_through() {
        // N-A l=NIL r=all-n cnt=1 → pass-through (allow-first)
        let r = lite_sl(0, 1, vec![seg(0, 1, simple_word(2139720), info_with_seq_set(vec![2139720]))]);
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn n_b_l_nil_r_no_match() {
        // N-B l=NIL r=no-match cnt=1 → clause-1
        let r = lite_sl(0, 1, vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))]);
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn n_c_l_nil_r_mixed_pass_through() {
        // N-C l=NIL r=mixed cnt=1 → pass-through (allow-first); both segs preserved
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(0, 1, simple_word(999), info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 2);
    }

    #[test]
    fn n_d_l_not_noun_r_n() {
        // N-D l-not-noun r-n cnt=1; sat-l full, con-l empty → (l, r)
        let l = lite_sl(0, 1, vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720]))]);
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn n_e_l_is_noun_r_n_empty() {
        // N-E l-is-noun (simple 2028920=は, in *noun-particles*) r-all-n cnt=0
        let l =
            lite_sl(0, 1, vec![seg(0, 1, simple_word(2028920), info_with_seq_set(vec![2028920]))]);
        let r = lite_sl(1, 2, vec![seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720]))]);
        let result = segfilter_n(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn n_f_l_is_noun_r_mixed() {
        // N-F l-is-noun r-mixed cnt=1 — base pair (l unchanged, mslf r con-r)
        let l =
            lite_sl(0, 1, vec![seg(0, 1, simple_word(2028920), info_with_seq_set(vec![2028920]))]);
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(1, 2, simple_word(999), info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![2028920]);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn n_g_l_mixed_r_all_n() {
        // N-G l-mixed (noun + not-noun) r-all-n cnt=1
        // sat-l = not-noun, con-l = noun; sat-r full, con-r empty
        // → sat-l push only → (mslf l sat-l, mslf r sat-r)
        let l = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, simple_word(2028920), info_with_seq_set(vec![2028920])),
                seg(0, 1, simple_word(999), info_with_seq_set(vec![999])),
            ],
        );
        let r = lite_sl(1, 2, vec![seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720]))]);
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments[0].seq_set, vec![999]);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn n_h_gap_r_mixed() {
        // N-H gap r-mixed cnt=1 — clause-2 with con-r non-empty
        let l = lite_sl(0, 1, vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))]);
        let r = lite_sl(
            2,
            3,
            vec![
                seg(2, 3, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(2, 3, simple_word(999), info_with_seq_set(vec![999])),
            ],
        );
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(result[0].1.segments[0].seq_set, vec![999]);
    }

    #[test]
    fn n_i_gap_r_all_n_empty() {
        // N-I gap r-all-n cnt=0 — clause-2 with con-r empty
        let l = lite_sl(0, 1, vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))]);
        let r = lite_sl(2, 3, vec![seg(2, 3, simple_word(2139720), info_with_seq_set(vec![2139720]))]);
        let result = segfilter_n(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn n_j_l_compound_r_all_n() {
        // N-J l-compound r-all-n cnt=1
        // compound seq is Multi → filter-in-seq-set-simple returns false → complement true
        // → sat-l full, con-l empty → pass through
        let lseg = Segment {
            start: 0,
            end: 2,
            word: compound_word(&[2028920, 999]),
            score: None,
            info: Some(info_with_seq_set(vec![2028920, 999])),
            top: None,
            text: None,
        };
        let l = lite_sl(0, 2, vec![lseg]);
        let r = lite_sl(2, 3, vec![seg(2, 3, simple_word(2139720), info_with_seq_set(vec![2139720]))]);
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }
}
