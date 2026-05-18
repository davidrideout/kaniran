//! Port of `ichiran/dict:filter-is-compound-end` (`dict-grammar.lisp:786`).
//!
//! Returns a predicate that tests whether a [`Segment`]'s word is a
//! compound whose last child's seq matches any of the supplied seqs.
//! Upstream:
//!
//! ```lisp
//! (declaim (inline filter-is-compound-end))
//! (defun filter-is-compound-end (&rest seqs)
//!   (lambda (segment)
//!     (let* ((word (segment-word segment))
//!            (seq (seq word)))
//!       (and seq (listp seq)
//!            (member (car (last seq)) seqs)))))
//! ```
//!
//! Used by the `def-segfilter-must-follow` callsites at
//! `dict-grammar.lisp:1118` (`segfilter-sae`) and `:1123`
//! (`segfilter-janai`) via `(complement (filter-is-compound-end …))`.
//!
//! Divergences from Lisp:
//! - Lisp `&rest seqs` (variadic) is taken as `Vec<i32>`.
//! - The closure's `(member …)` value is consumed only as a predicate;
//!   the Rust port returns `bool` per CONVENTIONS §4.1.
//! - `(and seq (listp seq))` is true only for compound-text upstream:
//!   `seq` returns a list only for [`KaniWordDispatchEnum::Compound`]
//!   (`dict.lisp:617`), and `nil` slips through neither leg of the
//!   `and`. The Rust port matches `Compound` directly.
//! - `(car (last seq))` is the last child's `seq`. Lisp `member` with
//!   default `eql` matches only integer seqs — nested lists (from
//!   nested compound children) and `nil` (from sourceless counter
//!   children) never match.

use super::kani_word::KaniWordDispatchEnum;
use super::seq::seq;
use super::segment_struct::Segment;
use super::word_info_class::WordInfoSeq;

pub fn filter_is_compound_end(seqs: Vec<i32>) -> impl Fn(&Segment) -> bool {
    move |segment: &Segment| -> bool {
        // dict-grammar.lisp:788 (let* ((word ...) (seq (seq word))))
        // — (and seq (listp seq)) gates on compound-text only.
        let c = match &segment.word {
            KaniWordDispatchEnum::Compound(c) => c,
            _ => return false,
        };
        let last_word = match c.words.last() {
            Some(w) => w,
            None => return false,
        };
        // dict-grammar.lisp:791 (member (car (last seq)) seqs)
        // — `(seq last_word)` is the last child's seq; only Single
        // integer seqs can match the integer `seqs` list.
        match seq(last_word) {
            Some(WordInfoSeq::Single(s)) => seqs.contains(&s),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::kana_text_dao::KanaText;
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

    fn compound(child_seqs: &[i32]) -> KaniWordDispatchEnum {
        let words: Vec<KaniWordDispatchEnum> = child_seqs
            .iter()
            .enumerate()
            .map(|(i, s)| KaniWordDispatchEnum::Kana(kana(&format!("w{i}"), *s)))
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

    fn simple(seq: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(kana("x", seq))
    }

    fn seg_with_word(word: KaniWordDispatchEnum) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word,
            score: None,
            info: None,
            top: None,
            text: None,
        }
    }

    // REPL probes from `/tmp/probe_410_414.lisp` (this session).

    #[test]
    fn compound_last_seq_matches() {
        // REPL: filter (100) on compound (99 100) => T
        let s = seg_with_word(compound(&[99, 100]));
        let f = filter_is_compound_end(vec![100]);
        assert!(f(&s));
    }

    #[test]
    fn compound_last_seq_no_match() {
        // REPL: filter (200) on compound (99 100) => NIL
        let s = seg_with_word(compound(&[99, 100]));
        let f = filter_is_compound_end(vec![200]);
        assert!(!f(&s));
    }

    #[test]
    fn compound_last_seq_in_list() {
        // REPL: filter (99 100) on compound (50 99) => T
        let s = seg_with_word(compound(&[50, 99]));
        let f = filter_is_compound_end(vec![99, 100]);
        assert!(f(&s));
    }

    #[test]
    fn simple_word_never_matches() {
        // REPL: filter (100) on simple => NIL
        let s = seg_with_word(simple(100));
        let f = filter_is_compound_end(vec![100]);
        assert!(!f(&s));
    }

    #[test]
    fn empty_seqs_never_matches() {
        // REPL: filter () empty on compound (99 100) => NIL
        let s = seg_with_word(compound(&[99, 100]));
        let f = filter_is_compound_end(vec![]);
        assert!(!f(&s));
    }
}
