//! Port of `ichiran/dict:filter-in-seq-set` (`dict-grammar.lisp:767`).
//!
//! Returns a predicate that tests whether a segment's `:seq-set`
//! shares any seq with the supplied list.

use std::sync::Arc;

use super::kani_lite_segment::KaniLiteSegment;

pub fn filter_in_seq_set(seqs: Vec<i32>) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool { seqs.iter().any(|s| segment.seq_set.contains(s)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
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

    fn lite_with_seq_set(seq_set: Vec<i32>) -> Arc<KaniLiteSegment> {
        let info = KaniSegmentInfo {
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
        };
        Arc::new(KaniLiteSegment::from_segment(Arc::new(Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: None,
        })))
    }

    fn lite_no_info() -> Arc<KaniLiteSegment> {
        Arc::new(KaniLiteSegment::from_segment(Arc::new(Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: None,
            top: None,
            text: None,
        })))
    }

    #[test]
    fn match_when_intersection_nonempty() {
        // REPL: filter (200 400) on seg-a (:seq-set (100 200)) -> truthy=T
        let seg = lite_with_seq_set(vec![100, 200]);
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(f(&seg));
    }

    #[test]
    fn no_match_when_disjoint() {
        // REPL: filter (200 400) on seg-b (:seq-set (300)) -> truthy=NIL
        let seg = lite_with_seq_set(vec![300]);
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(!f(&seg));
    }

    #[test]
    fn no_match_when_info_absent() {
        // REPL: filter (200 400) on seg-no-info -> truthy=NIL
        let seg = lite_no_info();
        let f = filter_in_seq_set(vec![200, 400]);
        assert!(!f(&seg));
    }

    #[test]
    fn empty_seqs_never_matches() {
        // REPL: (filter-in-seq-set) on seg-a -> truthy=NIL
        let seg = lite_with_seq_set(vec![100, 200]);
        let f = filter_in_seq_set(vec![]);
        assert!(!f(&seg));
    }
}
