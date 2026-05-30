//! Port of `ichiran/dict:get-synergies` (`dict-grammar.lisp:957`).
//!
//! Walks [`SYNERGY_LIST`] in order, gathering every synergy that fires
//! between `segment_list_left` and `segment_list_right`. Each entry is
//! a 3-element path `(seg_right, synergy, seg_left)` — distinct from
//! [`get_penalties`] which returns at most one such path.
//!
//! ```lisp
//! (defun get-synergies (segment-list-left segment-list-right)
//!   (loop for fn in *synergy-list*
//!      nconc (funcall fn segment-list-left segment-list-right)))
//! ```
//!
//! Divergences from Lisp:
//! - Each individual synergy fn returns `Vec<(Arc<KaniLiteSegmentList>,
//!   Synergy, Arc<KaniLiteSegmentList>)>`; here each tuple is wrapped
//!   into a `Vec<KaniLitePathElement>` matching the path-payload
//!   variant set (CONVENTIONS §4.3).
//!
//! [`SYNERGY_LIST`]: super::_star_synergy_list_star_::SYNERGY_LIST
//! [`get_penalties`]: super::get_penalties::get_penalties

use super::_star_synergy_list_star_::SYNERGY_LIST;
use super::kani::KaniLiteSegmentList;
use super::kani::KaniLitePathElement;

pub fn get_synergies(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
) -> Vec<Vec<KaniLitePathElement>> {
    let mut out = vec![];
    for synergy_fn in SYNERGY_LIST {
        // dict-grammar.lisp:958-959 (`nconc (funcall fn l r)`)
        for (right_sl, syn, left_sl) in synergy_fn(segment_list_left, segment_list_right) {
            out.push(vec![
                KaniLitePathElement::SegmentList(right_sl),
                KaniLitePathElement::Synergy(syn),
                KaniLitePathElement::SegmentList(left_sl),
            ]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
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

    fn seg(kpcl: (bool, bool, bool, bool), posi: Vec<&str>, seq_set: Vec<i32>) -> Segment {
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

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> KaniLiteSegmentList {
        KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        })
    }

    fn unwrap_synergy(path: &[KaniLitePathElement]) -> &crate::dict::synergy_struct::Synergy {
        match &path[1] {
            KaniLitePathElement::Synergy(s) => s,
            other => panic!("expected Synergy at [1], got {:?}", other),
        }
    }

    fn unwrap_sl(elem: &KaniLitePathElement) -> &KaniLiteSegmentList {
        match elem {
            KaniLitePathElement::SegmentList(sl) => sl,
            other => panic!("expected SegmentList, got {:?}", other),
        }
    }

    // REPL probes (/tmp/probe_449_451.lisp on .103, 2026-05-18).

    #[test]
    fn a_none_fire() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["zzz"], vec![9999])]);
        let r = lite_sl(1, 2, vec![seg((false, false, false, false), vec!["zzz"], vec![8888])]);
        assert!(get_synergies(&l, &r).is_empty());
    }

    #[test]
    fn b_only_no_adjectives() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl(1, 2, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.connector.as_deref(), Some(" "));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
        assert_eq!(unwrap_sl(&got[0][0]).start, 1);
        assert_eq!(unwrap_sl(&got[0][0]).end, 2);
        assert_eq!(unwrap_sl(&got[0][2]).start, 0);
        assert_eq!(unwrap_sl(&got[0][2]).end, 1);
    }

    #[test]
    fn c_only_to_adverbs() {
        let l = lite_sl(0, 2, vec![seg((true, false, false, false), vec!["adv-to"], vec![])]);
        let r = lite_sl(2, 3, vec![seg((false, false, false, false), vec![], vec![1008490])]);
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("to-adverb"));
        assert_eq!(syn.score, 30);
        assert_eq!(syn.start, 2);
        assert_eq!(syn.end, 2);
    }

    #[test]
    fn d_noun_da_only() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl(1, 2, vec![seg((false, false, false, false), vec![], vec![2089020])]);
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("noun+da"));
        assert_eq!(syn.score, 10);
    }

    #[test]
    fn e_noun_particle_only() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["n"], vec![])]);
        let r = lite_sl(1, 2, vec![seg((false, false, false, false), vec![], vec![2028920])]);
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 1);
        let syn = unwrap_synergy(&got[0]);
        assert_eq!(syn.description.as_deref(), Some("noun+prt"));
        assert_eq!(syn.score, 14);
    }

    #[test]
    fn f_two_synergies_order_mirrors_synergy_list() {
        let l = lite_sl(
            0,
            1,
            vec![seg((true, false, false, false), vec!["adj-no"], vec![1469800])],
        );
        let r = lite_sl(
            1,
            2,
            vec![
                seg((false, false, false, false), vec![], vec![1469800]),
                seg((false, false, false, false), vec![], vec![2089020]),
            ],
        );
        let got = get_synergies(&l, &r);
        assert_eq!(got.len(), 2);
        let syn0 = unwrap_synergy(&got[0]);
        assert_eq!(syn0.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn0.score, 15);
        let syn1 = unwrap_synergy(&got[1]);
        assert_eq!(syn1.description.as_deref(), Some("no da/desu"));
        assert_eq!(syn1.score, 15);
    }

    #[test]
    fn g_non_adjacent() {
        let l = lite_sl(0, 1, vec![seg((true, false, false, false), vec!["adj-no"], vec![])]);
        let r = lite_sl(5, 6, vec![seg((false, false, false, false), vec![], vec![1469800])]);
        assert!(get_synergies(&l, &r).is_empty());
    }
}
