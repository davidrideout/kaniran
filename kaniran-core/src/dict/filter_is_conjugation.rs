//! Port of `ichiran/dict:filter-is-conjugation` (`dict-grammar.lisp:780`).
//!
//! Returns a predicate that tests whether a [`Segment`]'s `info`
//! plist `:conj` includes a record whose [`ConjProp::conj_type`]
//! matches the supplied conj-type id. Upstream:
//!
//! ```lisp
//! (declaim (inline filter-is-conjugation))
//! (defun filter-is-conjugation (conj-type)
//!   (lambda (segment)
//!     (member conj-type (getf (segment-info segment) :conj)
//!             :key (lambda (cdata) (conj-type (conj-data-prop cdata))))))
//! ```
//!
//! Used by `def-segfilter-must-follow` callsites (the
//! `segfilter-aux-verb` rule at `dict-grammar.lisp:1077-1079` and the
//! `segfilter-sukiyoki` predicate at `dict-grammar.lisp:1101-1104`)
//! to split a [`SegmentList`]'s segments by conjugation form before
//! deciding how to combine them.
//!
//! Divergences from Lisp:
//! - The closure's `(member ...)` return value is consumed only as a
//!   predicate (`funcall`+`and`/`or`); the Rust port returns `bool`
//!   per CONVENTIONS §4.1's predicate-closure pattern.
//! - When `info` is `None` the upstream `(getf nil :conj)` yields
//!   `nil` and `member` returns `nil`; the Rust port matches by
//!   returning `false`.
//! - Lisp's `:key` lambda calls `(conj-type (conj-data-prop cdata))`;
//!   if `conj-data-prop` returned nil the gf dispatch would error. In
//!   practice every `conj-data` is constructed with a real `:prop`
//!   (`dict.lisp:362-368`), so the Rust port `expect`s the
//!   [`ConjData::prop`] slot.

use super::segment_struct::Segment;

pub fn filter_is_conjugation(conj_type: i32) -> impl Fn(&Segment) -> bool {
    move |segment: &Segment| -> bool {
        let info = match &segment.info {
            Some(info) => info,
            None => return false,
        };
        info.conj.iter().any(|cdata| {
            cdata
                .prop
                .as_ref()
                .expect("conj-data prop is always set by upstream construction")
                .conj_type
                == conj_type
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::conj_data_struct::ConjData;
    use super::super::conj_prop_dao::ConjProp;
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

    fn info_with_conj(conj: Vec<ConjData>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set: vec![],
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

    fn cdata(conj_type: i32) -> ConjData {
        ConjData {
            seq: None,
            from: None,
            via: None,
            prop: Some(ConjProp {
                id: 0,
                conj_id: 0,
                conj_type,
                pos: String::new(),
                neg: None,
                fml: None,
            }),
            src_map: vec![],
        }
    }

    #[test]
    fn match_when_conj_type_in_list() {
        // REPL: seg-a (cdata conj-type 13, cdata conj-type 3); filter13 match=T
        let seg = segment_with_info(Some(info_with_conj(vec![cdata(13), cdata(3)])));
        let f = filter_is_conjugation(13);
        assert!(f(&seg));
    }

    #[test]
    fn no_match_when_conj_type_absent() {
        // REPL: filter99 on seg-a match=NIL
        let seg = segment_with_info(Some(info_with_conj(vec![cdata(13), cdata(3)])));
        let f = filter_is_conjugation(99);
        assert!(!f(&seg));
    }

    #[test]
    fn no_match_when_info_absent() {
        // REPL: no-info match=NIL
        let seg = segment_with_info(None);
        let f = filter_is_conjugation(13);
        assert!(!f(&seg));
    }

    #[test]
    fn no_match_when_conj_empty() {
        // REPL: empty-conj match=NIL
        let seg = segment_with_info(Some(info_with_conj(vec![])));
        let f = filter_is_conjugation(13);
        assert!(!f(&seg));
    }
}
