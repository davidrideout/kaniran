//! Port of `ichiran/dict:segfilter-n` (`dict-grammar.lisp:1086`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-n (l r)
//!   (complement (apply 'filter-in-seq-set-simple *noun-particles*))
//!   (filter-in-seq-set 2139720 2849370 2849387) ;;　ん んだ
//!   :allow-first t)
//! ```

use std::sync::Arc;

use super::_star_noun_particles_star_::NOUN_PARTICLES;
use super::classify::classify;
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_in_seq_set_simple::filter_in_seq_set_simple;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;

const N_SEQS: &[i32] = &[2139720, 2849370, 2849387];

pub fn segfilter_n(
    seg_left: Option<&Arc<SegmentList>>,
    seg_right: &Arc<SegmentList>,
) -> Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> {
    let filter_right = filter_in_seq_set(N_SEQS.to_vec());
    let (sat_r, con_r) = classify(filter_right, &seg_right.segments);

    // Cond clause 1: (or (not sat-r) (and allow-first (not l)))
    if sat_r.is_empty() || seg_left.is_none() {
        return vec![(seg_left.cloned(), Arc::clone(seg_right))];
    }

    // Cond clause 2: (or (not l) (/= l.end r.start)) — (not l) absorbed above.
    let l = seg_left.unwrap();
    if l.end != seg_right.start {
        return if con_r.is_empty() {
            Vec::new()
        } else {
            vec![(Some(Arc::clone(l)), Arc::new(make_segment_list_from(seg_right, con_r)))]
        };
    }

    // T branch. Left filter is the complement of
    // (apply 'filter-in-seq-set-simple *noun-particles*).
    let inner = filter_in_seq_set_simple(NOUN_PARTICLES.to_vec());
    let (sat_l, con_l) = classify(|s| !inner(s), &l.segments);

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
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
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

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList { segments, start, end, top: None, matches: 0 }
    }

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn n_a_l_nil_r_all_n_pass_through() {
        // N-A l=NIL r=all-n cnt=1 → pass-through (allow-first)
        let r = Arc::new(sl(0, 1, vec![seg(0, 1, simple_word(2139720), info_with_seq_set(vec![2139720]))]));
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn n_b_l_nil_r_no_match() {
        // N-B l=NIL r=no-match cnt=1 → clause-1
        let r = Arc::new(sl(0, 1, vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))]));
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn n_c_l_nil_r_mixed_pass_through() {
        // N-C l=NIL r=mixed cnt=1 → pass-through (allow-first); both segs preserved
        let r = Arc::new(sl(
            0,
            1,
            vec![
                seg(0, 1, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(0, 1, simple_word(999), info_with_seq_set(vec![999])),
            ],
        ));
        let result = segfilter_n(None, &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 2);
    }

    #[test]
    fn n_d_l_not_noun_r_n() {
        // N-D l-not-noun r-n cnt=1; sat-l full, con-l empty → (l, r)
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))]));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720]))]));
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn n_e_l_is_noun_r_n_empty() {
        // N-E l-is-noun (simple 2028920=は, in *noun-particles*) r-all-n cnt=0
        let l =
            Arc::new(sl(0, 1, vec![seg(0, 1, simple_word(2028920), info_with_seq_set(vec![2028920]))]));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720]))]));
        let result = segfilter_n(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn n_f_l_is_noun_r_mixed() {
        // N-F l-is-noun r-mixed cnt=1 — base pair (l unchanged, mslf r con-r)
        let l =
            Arc::new(sl(0, 1, vec![seg(0, 1, simple_word(2028920), info_with_seq_set(vec![2028920]))]));
        let r = Arc::new(sl(
            1,
            2,
            vec![
                seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(1, 2, simple_word(999), info_with_seq_set(vec![999])),
            ],
        ));
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].info.as_ref().unwrap().seq_set,
            vec![2028920]
        );
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }

    #[test]
    fn n_g_l_mixed_r_all_n() {
        // N-G l-mixed (noun + not-noun) r-all-n cnt=1
        // sat-l = not-noun, con-l = noun; sat-r full, con-r empty
        // → sat-l push only → (mslf l sat-l, mslf r sat-r)
        let l = Arc::new(sl(
            0,
            1,
            vec![
                seg(0, 1, simple_word(2028920), info_with_seq_set(vec![2028920])),
                seg(0, 1, simple_word(999), info_with_seq_set(vec![999])),
            ],
        ));
        let r = Arc::new(sl(1, 2, vec![seg(1, 2, simple_word(2139720), info_with_seq_set(vec![2139720]))]));
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(
            result[0].0.as_ref().unwrap().segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn n_h_gap_r_mixed() {
        // N-H gap r-mixed cnt=1 — clause-2 with con-r non-empty
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))]));
        let r = Arc::new(sl(
            2,
            3,
            vec![
                seg(2, 3, simple_word(2139720), info_with_seq_set(vec![2139720])),
                seg(2, 3, simple_word(999), info_with_seq_set(vec![999])),
            ],
        ));
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        assert_eq!(
            result[0].1.segments[0].info.as_ref().unwrap().seq_set,
            vec![999]
        );
    }

    #[test]
    fn n_i_gap_r_all_n_empty() {
        // N-I gap r-all-n cnt=0 — clause-2 with con-r empty
        let l = Arc::new(sl(0, 1, vec![seg(0, 1, simple_word(999), info_with_seq_set(vec![999]))]));
        let r = Arc::new(sl(2, 3, vec![seg(2, 3, simple_word(2139720), info_with_seq_set(vec![2139720]))]));
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
        let l = Arc::new(sl(0, 2, vec![lseg]));
        let r = Arc::new(sl(2, 3, vec![seg(2, 3, simple_word(2139720), info_with_seq_set(vec![2139720]))]));
        let result = segfilter_n(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }
}
