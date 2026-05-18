//! Port of `ichiran/dict:filter-in-seq-set-simple` (`dict-grammar.lisp:772`).
//!
//! Returns a predicate that tests whether a [`Segment`]'s
//! `(segment-word segment)` has a non-list `seq` and whose `info`
//! plist `:seq-set` shares any seq with the supplied list. Upstream:
//!
//! ```lisp
//! (declaim (inline filter-in-seq-set-simple))
//! (defun filter-in-seq-set-simple (&rest seqs)
//!   "Same as above but checks that the word is not compound"
//!   (lambda (segment)
//!     (let ((seq (seq (segment-word segment))))
//!       (and (not (listp seq))
//!            (intersection seqs (getf (segment-info segment) :seq-set))))))
//! ```
//!
//! Used by `segfilter-n` (`dict-grammar.lisp:1086-1088`) via
//! `(complement (apply 'filter-in-seq-set-simple *noun-particles*))`.
//!
//! Divergences from Lisp:
//! - Lisp `&rest seqs` (variadic) is taken as `Vec<i32>` to match
//!   sibling [`super::filter_in_seq_set::filter_in_seq_set`].
//! - The closure's Lisp `intersection` return value is consumed only
//!   for truthiness; the Rust port returns `bool` per CONVENTIONS §4.1.
//! - Lisp `(listp seq)` is true for both lists and `nil`; the
//!   `(not (listp seq))` test therefore admits only integer seqs.
//!   In Rust [`WordInfoSeq`] that is exactly `Some(Single(_))` — both
//!   [`WordInfoSeq::Multi`] and [`None`] (sourceless counter) fail.

use super::seq::seq;
use super::segment_struct::Segment;
use super::word_info_class::WordInfoSeq;

pub fn filter_in_seq_set_simple(seqs: Vec<i32>) -> impl Fn(&Segment) -> bool {
    move |segment: &Segment| -> bool {
        // dict-grammar.lisp:775 (let seq = (seq (segment-word ...)))
        // — only Single passes the (not (listp seq)) gate.
        match seq(&segment.word) {
            Some(WordInfoSeq::Single(_)) => {}
            _ => return false,
        }
        let info = match &segment.info {
            Some(info) => info,
            None => return false,
        };
        seqs.iter().any(|s| info.seq_set.contains(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};
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

    fn compound_word(child_seqs: &[i32]) -> KaniWordDispatchEnum {
        let words: Vec<KaniWordDispatchEnum> = child_seqs
            .iter()
            .map(|s| KaniWordDispatchEnum::Kana(kana("c", *s)))
            .collect();
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

    fn seg(word: KaniWordDispatchEnum, info: Option<KaniSegmentInfo>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word,
            score: None,
            info,
            top: None,
            text: None,
        }
    }

    // REPL probes from `/tmp/probe_410_414.lisp` (this session).

    #[test]
    fn simple_word_seq_set_intersects() {
        // REPL: filter-simple (100) on simple seq-set=(100) => T
        let s = seg(simple_word(100), Some(info_with_seq_set(vec![100])));
        let f = filter_in_seq_set_simple(vec![100]);
        assert!(f(&s));
    }

    #[test]
    fn simple_word_seq_set_disjoint() {
        // REPL: filter-simple (200) on simple seq-set=(100) => NIL
        let s = seg(simple_word(100), Some(info_with_seq_set(vec![100])));
        let f = filter_in_seq_set_simple(vec![200]);
        assert!(!f(&s));
    }

    #[test]
    fn compound_word_always_false() {
        // REPL: filter-simple (100) on compound seq-set=(100) => NIL
        // compound's `seq` is a list — (not (listp seq)) is NIL.
        let s = seg(compound_word(&[99, 100]), Some(info_with_seq_set(vec![100])));
        let f = filter_in_seq_set_simple(vec![100]);
        assert!(!f(&s));
    }

    #[test]
    fn empty_seqs_never_matches() {
        // REPL: filter-simple () empty on simple => NIL
        let s = seg(simple_word(100), Some(info_with_seq_set(vec![100])));
        let f = filter_in_seq_set_simple(vec![]);
        assert!(!f(&s));
    }

    #[test]
    fn no_info_returns_false() {
        // REPL: filter-simple (100) on simple-no-info => NIL
        let s = seg(simple_word(100), None);
        let f = filter_in_seq_set_simple(vec![100]);
        assert!(!f(&s));
    }
}
