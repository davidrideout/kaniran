//! Port of `ichiran/dict:make-segment-list-from` (`dict-grammar.lisp:718`).
//!
//! Clones a [`SegmentList`] and swaps in a different `segments`
//! vector — used by `def-generic-synergy` and
//! `def-segfilter-must-follow` macro expansions
//! (`dict-grammar.lisp:741`, `:746`, `:1054`, `:1062`, `:1065-1066`)
//! to produce a derived [`SegmentList`] over the same `(start, end)`
//! slice but with a filtered segment subset.
//!
//! Upstream:
//!
//! ```lisp
//! (defun make-segment-list-from (old-segment-list segments)
//!   (let ((new-segment-list (copy-segment-list old-segment-list)))
//!     (setf (segment-list-segments new-segment-list) segments)
//!     new-segment-list))
//! ```
//!
//! Lisp's `copy-segment-list` is the `defstruct`-auto-generated
//! shallow copier; the Rust analog is [`Clone`] on
//! [`SegmentList`]. The `top` and `matches` slots are carried over
//! verbatim, matching the upstream behavior of a shallow copy that
//! then overwrites only `segments`.

use super::segment_list_struct::SegmentList;
use super::segment_struct::Segment;

pub fn make_segment_list_from(old_segment_list: &SegmentList, segments: Vec<Segment>) -> SegmentList {
    let mut new_segment_list = old_segment_list.clone();
    new_segment_list.segments = segments;
    new_segment_list
}

#[cfg(test)]
mod tests {
    use super::super::kana_text_dao::KanaText;
    use super::super::kani_word::KaniWordDispatchEnum;
    use super::super::simple_text_class::SimpleText;
    use super::*;

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

    fn seg_with_score(score: i32) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    #[test]
    fn swaps_segments_preserves_other_slots() {
        // REPL:
        //   src segments len=2, dst segments len=1
        //   dst start=0 end=2 matches=3
        //   src not mutated: src segments len=2
        //   first dst seg score=20
        let seg1 = seg_with_score(10);
        let seg2 = seg_with_score(20);
        let sl = SegmentList {
            segments: vec![seg1.clone(), seg2.clone()],
            start: 0,
            end: 2,
            top: None,
            matches: 3,
        };
        let new_sl = make_segment_list_from(&sl, vec![seg2.clone()]);
        assert_eq!(sl.segments.len(), 2);
        assert_eq!(new_sl.segments.len(), 1);
        assert_eq!(new_sl.start, 0);
        assert_eq!(new_sl.end, 2);
        assert_eq!(new_sl.matches, 3);
        assert_eq!(new_sl.segments[0].score, Some(20));
    }
}
