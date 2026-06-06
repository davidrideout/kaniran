//! Port of `ichiran/dict:segfilter-roku` (`dict-grammar.lisp:1112`).
//!
//! Keeps a right segment whose text starts with く only when the
//! preceding left segment does not end in いろ.

use std::sync::Arc;

use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_is_compound_end_text::filter_is_compound_end_text;
use super::kani_lite_segment_list::KaniLiteSegmentList;

const IRO_TEXTS: &[&str] = &["いろ"];
const KU_CHAR: char = 'く';

pub fn segfilter_roku(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let inner = filter_is_compound_end_text(IRO_TEXTS.iter().map(|s| s.to_string()).collect());
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |s| !inner(s),
        // dict-grammar.lisp:1114 (lambda) — (starts-with #\く (get-text segment)).
        |s| s.text.starts_with(KU_CHAR),
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
    use crate::dict::segment_struct::Segment;
    use crate::dict::simple_text_class::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn simple_seg(start: usize, end: usize, t: &str, seq: i32) -> Segment {
        Segment {
            start,
            end,
            word: KaniWordDispatchEnum::Kana(kana(t, seq)),
            score: None,
            info: None,
            top: None,
            text: None,
        }
    }

    fn compound_ending_seg(start: usize, end: usize, inner_text: &str, inner_seq: i32) -> Segment {
        let words = vec![
            KaniWordDispatchEnum::Kana(kana("z", 11111)),
            KaniWordDispatchEnum::Kana(kana(inner_text, inner_seq)),
        ];
        let primary = Box::new(words[0].clone());
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        Segment {
            start,
            end,
            word: KaniWordDispatchEnum::Compound(c),
            score: None,
            info: None,
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

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn r_a_l_nil_r_ku_pass_through() {
        // R-A l=NIL r=ku cnt=1 — allow-first pass-through
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "くる", 100)]);
        let result = segfilter_roku(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
    }

    #[test]
    fn r_b_l_nil_r_not_ku() {
        // R-B l=NIL r=not-ku cnt=1 — clause-1
        let r = lite_sl(0, 1, vec![simple_seg(0, 1, "あさ", 100)]);
        let result = segfilter_roku(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn r_c_l_simple_r_ku_pass_through() {
        // R-C l-simple r-ku cnt=1 — sat-l full, con-l empty → (l, r)
        let l = lite_sl(0, 1, vec![simple_seg(0, 1, "abc", 999)]);
        let r = lite_sl(1, 2, vec![simple_seg(1, 2, "くる", 100)]);
        let result = segfilter_roku(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
    }

    #[test]
    fn r_d_l_iro_r_ku_empty() {
        // R-D l-iro r-ku cnt=0 — sat-l empty, con-l full; con-r empty → ()
        let l = lite_sl(0, 2, vec![compound_ending_seg(0, 2, "いろ", 50)]);
        let r = lite_sl(2, 3, vec![simple_seg(2, 3, "くる", 100)]);
        let result = segfilter_roku(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn r_e_l_mixed_iro_r_ku_sat_push() {
        // R-E l-mixed (compound-iro + simple) r-ku cnt=1
        // sat-l=simple, con-l=compound-iro; con-r empty → sat-l push only
        let l = lite_sl(
            0,
            2,
            vec![
                compound_ending_seg(0, 2, "いろ", 50),
                simple_seg(0, 2, "abc", 999),
            ],
        );
        let r = lite_sl(2, 3, vec![simple_seg(2, 3, "くる", 100)]);
        let result = segfilter_roku(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        // Surviving L seg is the simple one (text="abc", seq=999).
        match &result[0].0.as_ref().unwrap().segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "abc");
                assert_eq!(k.seq, 999);
            }
            _ => panic!("expected Kana variant"),
        }
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn r_f_gap_r_mixed() {
        // R-F gap r-mixed cnt=1 — clause-2 with con-r non-empty (only the non-ku reading survives)
        let l = lite_sl(0, 1, vec![simple_seg(0, 1, "abc", 999)]);
        let r = lite_sl(
            2,
            3,
            vec![
                simple_seg(2, 3, "くる", 100),
                simple_seg(2, 3, "あさ", 999),
            ],
        );
        let result = segfilter_roku(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        match &result[0].1.segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "あさ"),
            _ => panic!("expected Kana variant"),
        }
    }
}
