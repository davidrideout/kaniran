//! Port of `ichiran/dict:synergy-na-adjectives` (`dict-grammar.lisp:870`).
//!
//! ```lisp
//! (def-generic-synergy synergy-na-adjectives (l r)
//!   (filter-is-pos ("adj-na") (segment k p c l) (or k l (and p c)))
//!   (filter-in-seq-set 2029110 2028990) ;; な ; に
//!   :description "na-adjective"
//!   :score 15
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - The `filter-is-pos` filter (`dict-grammar.lisp:871`,
//!   `(or k l (and p c))`) is built via [`filter_is_pos`].
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use std::sync::Arc;

use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_pos_macro::filter_is_pos;
use super::kani_lite_segment::{KaniLiteSegment, POS_ADJ_NA};
use super::kani_lite_segment_list::{make_kani_lite_segment_list_from, KaniLiteSegmentList};
use super::synergy_struct::Synergy;

pub fn synergy_na_adjectives(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    let start = l.end;
    let end = r.start;
    // dict-grammar.lisp:731-746 (def-generic-synergy expansion)
    if start != end {
        return vec![];
    }
    // dict-grammar.lisp:871 (filter-is-pos ("adj-na") (or k l (and p c)))
    let test_left = filter_is_pos(POS_ADJ_NA, |k, p, c, l| k || l || (p && c));
    let test_right = filter_in_seq_set(vec![2029110, 2028990]);
    let left: Vec<Arc<KaniLiteSegment>> =
        l.segments.iter().filter(|s| test_left(s)).cloned().collect();
    let right: Vec<Arc<KaniLiteSegment>> =
        r.segments.iter().filter(|s| test_right(s)).cloned().collect();
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    let syn = Synergy {
        description: Some("na-adjective".to_string()),
        connector: Some(" ".to_string()),
        score: 15,
        start,
        end,
    };
    vec![(
        Arc::new(make_kani_lite_segment_list_from(r, right)),
        syn,
        Arc::new(make_kani_lite_segment_list_from(l, left)),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
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

    fn seg(
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score: None,
            info: Some(KaniSegmentInfo {
                posi: posi.into_iter().map(String::from).collect(),
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
            }),
            top: None,
            text: None,
        }
    }

    fn lite_sl_owned(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    // REPL probes (/tmp/probe_synergies.lisp on .103, 2026-05-18).

    #[test]
    fn positive_na() {
        // na-adj/positive-na: l adj-na with k=T, r seq 2029110.
        // RIGHT-SL start=2 end=3 segs=1, SYNERGY desc="na-adjective"
        // conn=" " score=15 start=2 end=2, LEFT-SL start=0 end=2 segs=1.
        let l = lite_sl_owned(0, 2, vec![seg((true, false, false, false), vec!["adj-na"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![2029110])]);
        let got = synergy_na_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 2);
        assert_eq!(right_sl.end, 3);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("na-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 2);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_ni() {
        // na-adj/positive-ni: l adj-na with l=T, r seq 2028990 (に).
        let l = lite_sl_owned(0, 2, vec![seg((false, false, false, true), vec!["adj-na"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![2028990])]);
        let got = synergy_na_adjectives(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 15);
        assert_eq!(got[0].1.start, 2);
        assert_eq!(got[0].1.end, 2);
    }

    #[test]
    fn wrong_posi_empty() {
        // na-adj/neg-wrong-posi: l posi=("v5k"), not adj-na -> NIL.
        let l = lite_sl_owned(0, 2, vec![seg((true, false, false, false), vec!["v5k"], vec![])]);
        let r = lite_sl_owned(2, 3, vec![seg((false, false, false, false), vec![], vec![2029110])]);
        assert!(synergy_na_adjectives(&l, &r).is_empty());
    }
}
