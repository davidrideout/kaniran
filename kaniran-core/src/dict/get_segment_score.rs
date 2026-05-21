//! Port of `ichiran/dict:get-segment-score` (`dict.lisp:1040`).
//!
//! Generic-function dispatcher returning the segment-style score of a
//! [`Segment`], [`SegmentList`], or [`Synergy`]. Upstream three-arm
//! defgeneric:
//!
//! ```lisp
//! (defgeneric get-segment-score (seg)
//!   (:method ((seg segment))           (segment-score seg))
//!   (:method ((seg-list segment-list))
//!     (let ((seg (car (segment-list-segments seg-list))))
//!       (if seg (segment-score seg) 0))))
//! ;; dict-grammar.lisp:715
//! (defmethod get-segment-score ((syn synergy)) (synergy-score syn))
//! ```
//!
//! The three receiver types share no Lisp base class; the Rust
//! dispatcher wraps them in a sidecar enum
//! [`KaniSegmentScoreArg`] per CONVENTIONS §4.7 (per-type methods,
//! sibling enum dispatcher). The enum is defined here rather than as a
//! separate `kani_<name>.rs` sidecar because no other ported gf
//! dispatches on the same union today.
//!
//! Return type is [`Option<i32>`] — the Lisp accessor
//! `(segment-score seg)` may return [`nil`](Option::None) when the
//! score slot is unset (segments before `gen-score` runs), an integer
//! after scoring, or `0` for the empty-segment-list arm. The
//! `synergy-score` accessor on [`Synergy`] is always an integer in the
//! Rust port (slot is non-optional), so the synergy arm always yields
//! [`Some`].

use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::segment_list_struct::SegmentList;
use super::segment_struct::Segment;
use super::synergy_struct::Synergy;

#[derive(Debug, Clone)]
pub enum KaniSegmentScoreArg<'a> {
    Segment(&'a Segment),
    SegmentList(&'a SegmentList),
    KaniLiteSegmentList(&'a KaniLiteSegmentList),
    Synergy(&'a Synergy),
}

pub fn get_segment_score(seg: &KaniSegmentScoreArg) -> Option<i32> {
    match seg {
        // dict.lisp:1042-1043 (:method ((seg segment)))
        KaniSegmentScoreArg::Segment(s) => s.score,
        // dict.lisp:1044-1046 (:method ((seg-list segment-list)))
        KaniSegmentScoreArg::SegmentList(sl) => match sl.segments.first() {
            Some(first) => first.score,
            None => Some(0),
        },
        // Same shape as SegmentList arm, but reads the precomputed
        // `score` off the lite segment instead of dereffing into
        // `info` / `Segment.score`.
        KaniSegmentScoreArg::KaniLiteSegmentList(sl) => match sl.segments.first() {
            Some(first) => first.score,
            None => Some(0),
        },
        // dict-grammar.lisp:715-716 (defmethod get-segment-score ((syn synergy)))
        KaniSegmentScoreArg::Synergy(syn) => Some(syn.score),
    }
}

#[cfg(test)]
mod tests {
    //! All assertions back-checked via REPL on the .103 SBCL — see
    //! `/tmp/probe_gss.lisp` 2026-05-17 run.
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
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

    fn seg(score: Option<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score,
            info: None,
            top: None,
            text: None,
        }
    }

    #[test]
    fn synergy_returns_score() {
        // REPL: get-segment-score on synergy with score=7 -> 7
        let s = Synergy {
            description: Some("x".into()),
            connector: Some(String::new()),
            score: 7,
            start: 0,
            end: 1,
        };
        assert_eq!(get_segment_score(&KaniSegmentScoreArg::Synergy(&s)), Some(7));
    }

    #[test]
    fn segment_returns_score_when_present() {
        // REPL: segment with score=13 -> 13
        let s = seg(Some(13));
        assert_eq!(get_segment_score(&KaniSegmentScoreArg::Segment(&s)), Some(13));
    }

    #[test]
    fn segment_returns_none_when_score_unset() {
        // REPL: segment with no score -> NIL
        let s = seg(None);
        assert_eq!(get_segment_score(&KaniSegmentScoreArg::Segment(&s)), None);
    }

    #[test]
    fn empty_segment_list_returns_zero() {
        // REPL: segment-list with no segments -> 0
        let sl = SegmentList {
            segments: Vec::new(),
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(0)
        );
    }

    #[test]
    fn segment_list_returns_first_segment_score() {
        // REPL: segment-list with two segs (99, 50) -> 99
        let sl = SegmentList {
            segments: vec![seg(Some(99)), seg(Some(50))],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(99)
        );
    }

    #[test]
    fn segment_list_returns_none_when_first_segment_score_unset() {
        // REPL: segment-list with one nil-score seg -> NIL
        let sl = SegmentList {
            segments: vec![seg(None)],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            None
        );
    }

    #[test]
    fn single_segment_list_returns_that_score() {
        // REPL: segment-list with one seg (42) -> 42
        let sl = SegmentList {
            segments: vec![seg(Some(42))],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(42)
        );
    }
}
