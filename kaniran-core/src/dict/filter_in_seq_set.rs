//! Port of `ichiran/dict:filter-in-seq-set` (`dict-grammar.lisp:767`).
//!
//! Returns a predicate that tests whether a [`Segment`]'s `info`
//! plist `:seq-set` shares any seq with the supplied list. Upstream:
//!
//! ```lisp
//! (declaim (inline filter-in-seq-set))
//! (defun filter-in-seq-set (&rest seqs)
//!   (lambda (segment)
//!     (intersection seqs (getf (segment-info segment) :seq-set))))
//! ```
//!
//! Used by the `def-generic-synergy` machinery
//! (`dict-grammar.lisp:731-746`, e.g. `synergy-noun-particle` at
//! `dict-grammar.lisp:827`) and the segfilter family
//! (`dict-grammar.lisp:1077-` etc.) as a predicate over a
//! [`SegmentList`]'s segments.
//!
//! Divergences from Lisp:
//! - Lisp `&rest seqs` (variadic) is taken as `Vec<i32>` so callers
//!   already holding a list (e.g. `(apply #'filter-in-seq-set
//!   *noun-particles*)`) pass it directly; literal callsites
//!   (`(filter-in-seq-set 2089020)`) build the vec at the call site.
//! - The closure's Lisp `intersection` return value is consumed only
//!   for truthiness by `remove-if-not` / `classify` / `member` /
//!   `complement` / `funcall`+`some`; the Rust port returns `bool`
//!   directly. Per CONVENTIONS §4.1 this is a closure whose contract
//!   is "predicate over Segment"; no upstream caller inspects the
//!   intersection list's contents.
//! - When the segment has no `info` plist (`info = None`), Lisp's
//!   `(getf nil :seq-set)` returns `nil` and the intersection is
//!   empty (false). The Rust port matches by returning `false` for
//!   `None` info.

use super::segment_struct::Segment;

pub fn filter_in_seq_set(seqs: Vec<i32>) -> impl Fn(&Segment) -> bool {
    move |segment: &Segment| -> bool {
        let info = match &segment.info {
            Some(info) => info,
            None => return false,
        };
        seqs.iter().any(|s| info.seq_set.contains(s))
    }
}

#[cfg(test)]
mod tests {
    use super::super::conj_data_struct::ConjData;
    use super::super::kana_text_dao::KanaText;
    use super::super::kani_word::KaniWordDispatchEnum;
    use super::super::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};
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

    fn segment_with_info(info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info,
            top: None,
            text: None,
        }
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

    #[test]
    fn match_when_intersection_nonempty() {
        // REPL: filter (200 400) on seg-a (:seq-set (100 200)) -> truthy=T
        let seg = segment_with_info(Some(info_with_seq_set(vec![100, 200])));
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(f(&seg));
    }

    #[test]
    fn no_match_when_disjoint() {
        // REPL: filter (200 400) on seg-b (:seq-set (300)) -> truthy=NIL
        let seg = segment_with_info(Some(info_with_seq_set(vec![300])));
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(!f(&seg));
    }

    #[test]
    fn no_match_when_info_absent() {
        // REPL: filter (200 400) on seg-no-info -> truthy=NIL
        let seg = segment_with_info(None);
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(!f(&seg));
    }

    #[test]
    fn empty_seqs_never_matches() {
        // REPL: (filter-in-seq-set) on seg-a -> truthy=NIL
        let seg = segment_with_info(Some(info_with_seq_set(vec![100, 200])));
        let f = filter_in_seq_set(vec![]);
        assert!(!f(&seg));
    }
}
