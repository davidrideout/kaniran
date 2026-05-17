//! Port of `ichiran/dict:filter-is-compound-end-text` (`dict-grammar.lisp:794`).
//!
//! ```lisp
//! (defun filter-is-compound-end-text (&rest texts)
//!   (lambda (segment)
//!     (let* ((word (segment-word segment))
//!            (seq (seq word)))
//!       (and seq (listp (seq word))
//!            (find (get-text (car (last (words word)))) texts :test 'equal)))))
//! ```
//!
//! Closure returns `bool` per §4.1 predicate-closure pattern.
//! `(and seq (listp (seq word)))` collapses to a `Compound` match —
//! only `KaniWordDispatchEnum::Compound` produces `WordInfoSeq::Multi`.

use super::get_text::get_text;
use super::kani_word::KaniWordDispatchEnum;
use super::segment_struct::Segment;

pub fn filter_is_compound_end_text(texts: Vec<String>) -> impl Fn(&Segment) -> bool {
    move |segment: &Segment| -> bool {
        let c = match &segment.word {
            KaniWordDispatchEnum::Compound(c) => c,
            _ => return false,
        };
        let last = match c.words.last() {
            Some(w) => w,
            None => return false,
        };
        let last_text = get_text(last);
        texts.iter().any(|t| t == last_text.as_ref())
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
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn compound(child_texts: &[&str]) -> CompoundText {
        let words: Vec<KaniWordDispatchEnum> = child_texts
            .iter()
            .enumerate()
            .map(|(i, t)| KaniWordDispatchEnum::Kana(kana(t, 9900 + i as i32)))
            .collect();
        let primary = Box::new(words[0].clone());
        CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        }
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

    // REPL probes from `/tmp/probe_compound_end.lisp` (filter texts:
    // "ちゃい" "いか" "とか" "とき" "い"):
    //   compound-end ((chai))      -> ちゃい
    //   compound-end ((abc)(ika))  -> いか
    //   compound-end ((toki))      -> とき
    //   compound-end ((xxx))       -> NIL
    //   compound-end SIMPLE 'toka' -> NIL
    //   no-texts ((ika))           -> NIL

    fn badend_texts() -> Vec<String> {
        vec![
            "ちゃい".to_string(),
            "いか".to_string(),
            "とか".to_string(),
            "とき".to_string(),
            "い".to_string(),
        ]
    }

    #[test]
    fn compound_with_single_child_chai_matches() {
        let seg = seg_with_word(KaniWordDispatchEnum::Compound(compound(&["ちゃい"])));
        let f = filter_is_compound_end_text(badend_texts());
        assert!(f(&seg));
    }

    #[test]
    fn compound_last_child_ika_matches() {
        let seg =
            seg_with_word(KaniWordDispatchEnum::Compound(compound(&["abc", "いか"])));
        let f = filter_is_compound_end_text(badend_texts());
        assert!(f(&seg));
    }

    #[test]
    fn compound_last_child_toki_matches() {
        let seg = seg_with_word(KaniWordDispatchEnum::Compound(compound(&["とき"])));
        let f = filter_is_compound_end_text(badend_texts());
        assert!(f(&seg));
    }

    #[test]
    fn compound_last_child_no_match() {
        let seg = seg_with_word(KaniWordDispatchEnum::Compound(compound(&["xxx"])));
        let f = filter_is_compound_end_text(badend_texts());
        assert!(!f(&seg));
    }

    #[test]
    fn simple_text_never_matches() {
        let seg = seg_with_word(KaniWordDispatchEnum::Kana(kana("とか", 9996)));
        let f = filter_is_compound_end_text(badend_texts());
        assert!(!f(&seg));
    }

    #[test]
    fn empty_texts_never_matches() {
        // REPL: (filter-is-compound-end-text) on compound(("いか"))) -> NIL
        let seg = seg_with_word(KaniWordDispatchEnum::Compound(compound(&["いか"])));
        let f = filter_is_compound_end_text(vec![]);
        assert!(!f(&seg));
    }
}
