//! Port of `ichiran/dict:filter-short-kana` (`dict-grammar.lisp:986-994`).
//!
//! ```lisp
//! (declaim (inline filter-short-kana))
//! (defun filter-short-kana (len &key except)
//!   (lambda (segment-list)
//!     (let ((seg (car (segment-list-segments segment-list))))
//!       (and seg
//!            (<= (- (segment-list-end segment-list)
//!                   (segment-list-start segment-list)) len)
//!            (not (car (getf (segment-info seg) :kpcl)))
//!            (not (and except (member (get-text seg) except :test 'equal)))))))
//! ```
//!
//! Divergences from Lisp:
//! - `&key except` → positional `Vec<String>` (CONVENTIONS §4.6).
//! - Closure return collapses to `bool` (CONVENTIONS §4.1).
//! - Closure takes the lite list; first-segment kpcl-bit-0 and text
//!   are precomputed on [`KaniLiteSegment`] so no `setf`-on-`seg.text`
//!   is required (upstream's caching is moot when the data is already
//!   on the lite layer).

use super::kani::KPCL_K;
use super::kani::KaniLiteSegmentList;

pub fn filter_short_kana(
    len: usize,
    except: Vec<String>,
) -> impl Fn(&KaniLiteSegmentList) -> bool {
    move |segment_list| -> bool {
        let seg = match segment_list.segments.first() {
            Some(s) => s,
            None => return false,
        };
        if segment_list.end - segment_list.start > len {
            return false;
        }
        if seg.kpcl & KPCL_K != 0 {
            return false;
        }
        if !except.is_empty() && except.iter().any(|e| e.as_str() == seg.text.as_ref()) {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
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

    fn info_with(kpcl: (bool, bool, bool, bool), seq_set: Vec<i32>) -> KaniSegmentInfo {
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
            kpcl,
        }
    }

    fn seg(start: usize, end: usize, info: Option<KaniSegmentInfo>, text: Option<&str>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info,
            top: None,
            text: text.map(str::to_string),
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_b.lisp on .103, 2026-05-18).

    #[test]
    fn c1_empty_segments_is_false() {
        let f = filter_short_kana(1, vec![]);
        assert!(!f(&lite_sl(0, 1, vec![])));
    }

    #[test]
    fn c2_span_exceeds_len_is_false() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(0, 2, Some(info_with((false, false, false, false), vec![999])), Some("あい"));
        assert!(!f(&lite_sl(0, 2, vec![s])));
    }

    #[test]
    fn c3_kpcl_first_set_is_false() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(0, 1, Some(info_with((true, false, false, false), vec![999])), Some("あ"));
        assert!(!f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c4_all_pass_no_except_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(0, 1, Some(info_with((false, false, false, false), vec![999])), Some("あ"));
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c5_except_matches_text_is_false() {
        let f = filter_short_kana(1, vec!["と".to_string()]);
        let s = seg(0, 1, Some(info_with((false, false, false, false), vec![999])), Some("と"));
        assert!(!f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c6_except_differs_from_text_is_true() {
        let f = filter_short_kana(1, vec!["と".to_string()]);
        let s = seg(0, 1, Some(info_with((false, false, false, false), vec![999])), Some("あ"));
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c8_kpcl_second_set_first_nil_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(0, 1, Some(info_with((false, true, false, false), vec![999])), Some("あ"));
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c9_no_info_plist_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(0, 1, None, Some("あ"));
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c10_span_equals_len_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(5, 6, Some(info_with((false, false, false, false), vec![999])), Some("あ"));
        assert!(f(&lite_sl(5, 6, vec![s])));
    }

    #[test]
    fn c11_only_first_seg_examined() {
        let f = filter_short_kana(1, vec![]);
        let s_good = seg(0, 1, Some(info_with((false, false, false, false), vec![999])), Some("あ"));
        let s_kpcl = seg(0, 1, Some(info_with((true, false, false, false), vec![888])), Some("あ"));
        assert!(f(&lite_sl(0, 1, vec![s_good, s_kpcl])));
    }

    #[test]
    fn c12_no_except_kw_text_to_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(0, 1, Some(info_with((false, false, false, false), vec![999])), Some("と"));
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c13_except_empty_text_to_is_true() {
        let f = filter_short_kana(1, vec![]);
        let s = seg(0, 1, Some(info_with((false, false, false, false), vec![999])), Some("と"));
        assert!(f(&lite_sl(0, 1, vec![s])));
    }

    #[test]
    fn c14_len_two_span_two_is_true() {
        let f = filter_short_kana(2, vec![]);
        let s = seg(0, 2, Some(info_with((false, false, false, false), vec![999])), Some("あい"));
        assert!(f(&lite_sl(0, 2, vec![s])));
    }
}
