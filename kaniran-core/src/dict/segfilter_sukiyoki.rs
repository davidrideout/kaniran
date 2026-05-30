//! Port of `ichiran/dict:segfilter-sukiyoki` (`dict-grammar.lisp:1101`).
//!
//! ```lisp
//! ;; some of adj-ix words end with 好い which produces a confusing 好き conjugation
//! ;; this should disable it
//! (def-segfilter-must-follow segfilter-sukiyoki (l r)
//!   (constantly nil)
//!   (lambda (segment)
//!     (and (funcall (filter-is-conjugation +conj-adjective-literary+) segment)
//!          (alexandria:ends-with-subseq "好き" (get-text segment)))))
//! ```
//!
//! Divergences from Lisp:
//! - The lambda's `(get-text segment)` upstream goes through the
//!   `((segment))` method (`dict.lisp:677-679`) which lazily caches
//!   the result back into `segment-text`. The Rust port reads through
//!   the lite-precomputed [`super::kani::KaniLiteSegment::text`]
//!   directly. Functionally identical.
//! - `+conj-adjective-literary+` (`dict-errata.lisp:1240`) is a plain
//!   `defconstant` with value `54`; no standalone Rust port file
//!   exists for it (`_star_weak_conj_forms_star_.rs` references the
//!   bare literal). Inlined as `54` with the same comment annotation.

use std::sync::Arc;

use super::def_segfilter_must_follow_macro::def_segfilter_must_follow_body;
use super::filter_is_conjugation::filter_is_conjugation;
use super::kani::KaniLiteSegment;
use super::kani::KaniLiteSegmentList;

const SUKI_SUFFIX: &str = "好き";

pub fn segfilter_sukiyoki(
    seg_left: Option<&Arc<KaniLiteSegmentList>>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)> {
    let conj_filter = filter_is_conjugation(54); // +conj-adjective-literary+
    def_segfilter_must_follow_body(
        seg_left,
        seg_right,
        |_: &Arc<KaniLiteSegment>| false,
        // dict-grammar.lisp:1103 (lambda) — and conj-type=54 ends-with "好き".
        |s| conj_filter(s) && s.text.ends_with(SUKI_SUFFIX),
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::conj_prop_dao::ConjProp;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
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
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn cdata_54() -> ConjData {
        ConjData {
            seq: Some(1),
            from: Some(2),
            via: None,
            prop: Some(ConjProp {
                id: 0,
                conj_id: 0,
                conj_type: 54,
                pos: String::new(),
                neg: None,
                fml: None,
            }),
            src_map: vec![],
        }
    }

    fn info(seq_set: Vec<i32>, conj: Vec<ConjData>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set,
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

    fn seg(start: usize, end: usize, t: &str, seq: i32, info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start,
            end,
            word: KaniWordDispatchEnum::Kana(kana(t, seq)),
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

    // REPL probes from `/tmp/probe_415_423.lisp` (this session).

    #[test]
    fn sk_a_l_nil_r_suki_conj54_empty() {
        // SK-A l=NIL r=suki-conj54 cnt=0
        // no allow-first; sat-r full, l=NIL → clause-2 (not l)=T, con-r empty → ()
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, "好き", 100, Some(info(vec![100], vec![cdata_54()])))],
        );
        let result = segfilter_sukiyoki(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn sk_b_l_nil_r_mixed() {
        // SK-B l=NIL r=mixed cnt=1
        // clause-2 (not l)=T, con-r non-empty → (NIL, mslf r con-r)
        let r = lite_sl(
            0,
            1,
            vec![
                seg(0, 1, "好き", 100, Some(info(vec![100], vec![cdata_54()]))),
                seg(0, 1, "abc", 999, Some(info(vec![999], vec![]))),
            ],
        );
        let result = segfilter_sukiyoki(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
        match &result[0].1.segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, 999);
                assert_eq!(k.text, "abc");
            }
            _ => panic!("expected Kana variant"),
        }
    }

    #[test]
    fn sk_c_l_nil_r_no_match() {
        // SK-C l=NIL r=no-match cnt=1 — clause-1
        let r = lite_sl(0, 1, vec![seg(0, 1, "abc", 999, Some(info(vec![999], vec![])))]);
        let result = segfilter_sukiyoki(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn sk_d_l_simple_r_suki_empty() {
        // SK-D l-simple r-suki cnt=0
        // sat-l empty (constantly nil), con-l full; sat-r full, con-r empty
        // → no base, no sat-l push (sat-l empty) → empty
        let l = lite_sl(0, 1, vec![seg(0, 1, "abc", 999, Some(info(vec![999], vec![])))]);
        let r = lite_sl(
            1,
            2,
            vec![seg(1, 2, "好き", 100, Some(info(vec![100], vec![cdata_54()])))],
        );
        let result = segfilter_sukiyoki(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn sk_e_l_simple_r_mixed_base_only() {
        // SK-E l-simple r-mixed cnt=1
        // sat-l empty, con-l full; sat-r=suki, con-r=other → base pair only
        let l = lite_sl(0, 1, vec![seg(0, 1, "abc", 999, Some(info(vec![999], vec![])))]);
        let r = lite_sl(
            1,
            2,
            vec![
                seg(1, 2, "好き", 100, Some(info(vec![100], vec![cdata_54()]))),
                seg(1, 2, "abc", 999, Some(info(vec![999], vec![]))),
            ],
        );
        let result = segfilter_sukiyoki(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
        match &result[0].1.segments[0].source.word {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.seq, 999),
            _ => panic!("expected Kana variant"),
        }
    }

    #[test]
    fn sk_f_gap_r_suki_empty() {
        // SK-F gap r-suki cnt=0 — clause-2 (l.end != r.start) with con-r empty → ()
        let l = lite_sl(0, 1, vec![seg(0, 1, "abc", 999, Some(info(vec![999], vec![])))]);
        let r = lite_sl(
            2,
            3,
            vec![seg(2, 3, "好き", 100, Some(info(vec![100], vec![cdata_54()])))],
        );
        let result = segfilter_sukiyoki(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn sk_g_l_nil_r_suki_no_conj_pass_through() {
        // SK-G l=NIL r=suki-no-conj cnt=1 — filter-right requires conj-54; without it sat-r empty
        let r = lite_sl(0, 1, vec![seg(0, 1, "好き", 100, Some(info(vec![100], vec![])))]);
        let result = segfilter_sukiyoki(None, &r);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn sk_h_l_nil_r_conj54_not_suki_pass_through() {
        // SK-H l=NIL r=conj54-not-suki cnt=1 — text doesn't end with 好き → sat-r empty
        let r = lite_sl(
            0,
            1,
            vec![seg(0, 1, "abc", 100, Some(info(vec![100], vec![cdata_54()])))],
        );
        let result = segfilter_sukiyoki(None, &r);
        assert_eq!(result.len(), 1);
    }
}
