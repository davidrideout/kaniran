//! Port of `ichiran/dict:synergy-noun-particle` (`dict-grammar.lisp:827`).
//!
//! ```lisp
//! (def-generic-synergy synergy-noun-particle (l r)
//!   #'filter-is-noun
//!  (apply #'filter-in-seq-set *noun-particles*)
//!   :description "noun+prt"
//!   :score (+ 10 (* 4 (- (segment-list-end r) (segment-list-start r))))
//!   :connector " ")
//! ```
//!
//! Divergences from Lisp:
//! - `(apply #'filter-in-seq-set *noun-particles*)` becomes a direct
//!   call with the global's slice cloned into a `Vec` — Rust's
//!   `filter_in_seq_set` takes `Vec<i32>` (see its file doc).
//! - `pushnew ',name *synergy-list*` from the `defsynergy` expansion
//!   moves to the `*synergy-list*` port (separate wave).

use std::sync::Arc;

use super::_star_noun_particles_star_::NOUN_PARTICLES;
use super::def_generic_synergy_macro::{def_generic_synergy_body, DefGenericSynergyOpts};
use super::filter_in_seq_set::filter_in_seq_set;
use super::filter_is_noun::filter_is_noun;
use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub fn synergy_noun_particle(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    // dict-grammar.lisp:831 (:score (+ 10 (* 4 (- (segment-list-end r) (segment-list-start r)))))
    let span = r.end - r.start;
    let score = 10 + 4 * (span as i32);
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(NOUN_PARTICLES.to_vec()),
        &DefGenericSynergyOpts {
            description: Some("noun+prt"),
            connector: " ",
            score,
        },
    )
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

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn positive_len1() {
        // noun-particle/positive-len1: l noun, r seq 2028920 (は), r.end-r.start=1.
        // SYNERGY desc="noun+prt" conn=" " score=14 start=1 end=1.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![2028920])]);
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        let (right_sl, syn, left_sl) = &got[0];
        assert_eq!(right_sl.start, 1);
        assert_eq!(right_sl.end, 2);
        assert_eq!(right_sl.segments.len(), 1);
        assert_eq!(syn.description.as_deref(), Some("noun+prt"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 14);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(left_sl.start, 0);
        assert_eq!(left_sl.end, 1);
        assert_eq!(left_sl.segments.len(), 1);
    }

    #[test]
    fn positive_len2() {
        // noun-particle/positive-len2: r seq 2215430 (には), span=2 -> score=18.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 3, vec![seg((false, false, false, false), vec![], vec![2215430])]);
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 18);
        assert_eq!(got[0].1.start, 1);
        assert_eq!(got[0].1.end, 1);
    }

    #[test]
    fn positive_len4() {
        // noun-particle/positive-len4: r seq 1009600 (にとって), span=4 -> score=26.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 5, vec![seg((false, false, false, false), vec![], vec![1009600])]);
        let got = synergy_noun_particle(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1.score, 26);
    }

    #[test]
    fn not_adjacent_empty() {
        // noun-particle/not-adjacent: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(5, 6, vec![seg((false, false, false, false), vec![], vec![2028920])]);
        assert!(synergy_noun_particle(&l, &r).is_empty());
    }

    #[test]
    fn right_misses_empty() {
        // noun-particle/right-misses: NIL.
        let l = lite_sl_owned(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl_owned(1, 2, vec![seg((false, false, false, false), vec![], vec![9999999])]);
        assert!(synergy_noun_particle(&l, &r).is_empty());
    }
}
