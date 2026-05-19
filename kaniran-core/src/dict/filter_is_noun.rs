//! Port of `ichiran/dict:filter-is-noun` (`dict-grammar.lisp:748`).
//!
//! ```lisp
//! (defun filter-is-noun (segment)
//!   (or (destructuring-bind (k p c l) (getf (segment-info segment) :kpcl)
//!         (and (or l k (and p c))
//!              (intersection '("n" "n-adv" "n-t" "adj-na" "n-suf" "pn")
//!                            (getf (segment-info segment) :posi)
//!                            :test 'equal)))
//!       (and (typep (segment-word segment) 'counter-text)
//!            (getf (segment-info segment) :seq-set))))
//! ```
//!
//! Divergences from Lisp:
//! - Used as `#'filter-is-noun` (a function reference) by
//!   [`super::synergy_noun_da::synergy_noun_da`] and
//!   [`super::synergy_noun_particle::synergy_noun_particle`]; the Rust
//!   port is the equivalent `pub fn`.
//! - The Lisp `intersection` return value is consumed only for
//!   truthiness here (`or` arm) per CONVENTIONS §4.1; the Rust port
//!   returns `bool`.
//! - When `segment.info` is `None`, upstream's `destructuring-bind`
//!   over `(getf nil :kpcl) = nil` raises `arg-count-error` at
//!   runtime — verified via REPL. Real callers never feed an
//!   info-less segment (synergy filters run after `gen-score`
//!   populates `info`); the port returns `false` to match the
//!   "doesn't pass the filter" outcome without crashing, matching
//!   the [`super::filter_in_seq_set::filter_in_seq_set`] convention
//!   for the same edge.
//! - `typep word 'counter-text` matches every counter subclass
//!   because CLOS `typep` covers subclasses; the Rust analog is the
//!   [`KaniWordDispatchEnum::Counter`] variant which wraps the whole
//!   [`Counter`] family per CONVENTIONS §4.7.

use super::kani_word::KaniWordDispatchEnum;
use super::segment_struct::Segment;

const NOUN_POS_TAGS: &[&str] = &["n", "n-adv", "n-t", "adj-na", "n-suf", "pn"];

pub fn filter_is_noun(segment: &Segment) -> bool {
    let info = match &segment.info {
        Some(info) => info,
        None => return false,
    };
    let (k, p, c, l) = info.kpcl;
    let first_branch = (l || k || (p && c))
        && info
            .posi
            .iter()
            .any(|x| NOUN_POS_TAGS.iter().any(|tag| tag == x));
    if first_branch {
        return true;
    }
    matches!(segment.word, KaniWordDispatchEnum::Counter(_)) && !info.seq_set.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::counter_text_class::Counter;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_counter_args::{CounterArgs, CounterClass};
    use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};
    use crate::dict::simple_text_class::SimpleText;

    fn dummy_kana_word() -> KaniWordDispatchEnum {
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

    fn counter_word() -> KaniWordDispatchEnum {
        // Construct via Counter::new — mirrors the REPL probe's
        // (make-instance 'counter-text :text "個" :kana "か"
        //                 :number-text "5") receiver.
        let args = CounterArgs::new(CounterClass::Text, "個", "か");
        let counter = Counter::new(&args, "5").unwrap();
        KaniWordDispatchEnum::Counter(counter)
    }

    fn seg(
        word: KaniWordDispatchEnum,
        kpcl: (bool, bool, bool, bool),
        posi: Vec<&str>,
        seq_set: Vec<i32>,
    ) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word,
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

    fn seg_no_info(word: KaniWordDispatchEnum) -> Segment {
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

    // REPL probes (/tmp/probe_437_441.lisp on .103, 2026-05-18).

    #[test]
    fn pos_kpcl_k_posi_n() {
        // filter-is-noun/pos-kpcl-k-posi-n => ("n")
        assert!(filter_is_noun(&seg(
            dummy_kana_word(),
            (true, false, false, false),
            vec!["n"],
            vec![]
        )));
    }

    #[test]
    fn pos_kpcl_l_posi_n_adv() {
        // filter-is-noun/pos-kpcl-l-posi-n-adv => ("n-adv")
        assert!(filter_is_noun(&seg(
            dummy_kana_word(),
            (false, false, false, true),
            vec!["n-adv"],
            vec![]
        )));
    }

    #[test]
    fn pos_kpcl_pc_posi_n_t() {
        // filter-is-noun/pos-kpcl-pc-posi-n-t => ("n-t")
        assert!(filter_is_noun(&seg(
            dummy_kana_word(),
            (false, true, true, false),
            vec!["n-t"],
            vec![]
        )));
    }

    #[test]
    fn pos_kpcl_k_posi_adj_na() {
        // filter-is-noun/pos-kpcl-k-posi-adj-na => ("adj-na")
        assert!(filter_is_noun(&seg(
            dummy_kana_word(),
            (true, false, false, false),
            vec!["adj-na"],
            vec![]
        )));
    }

    #[test]
    fn pos_kpcl_k_posi_n_suf() {
        // filter-is-noun/pos-kpcl-k-posi-n-suf => ("n-suf")
        assert!(filter_is_noun(&seg(
            dummy_kana_word(),
            (true, false, false, false),
            vec!["n-suf"],
            vec![]
        )));
    }

    #[test]
    fn pos_kpcl_k_posi_pn() {
        // filter-is-noun/pos-kpcl-k-posi-pn => ("pn")
        assert!(filter_is_noun(&seg(
            dummy_kana_word(),
            (true, false, false, false),
            vec!["pn"],
            vec![]
        )));
    }

    #[test]
    fn neg_kpcl_k_posi_v5k() {
        // filter-is-noun/neg-kpcl-k-posi-v5k => NIL
        assert!(!filter_is_noun(&seg(
            dummy_kana_word(),
            (true, false, false, false),
            vec!["v5k"],
            vec![]
        )));
    }

    #[test]
    fn neg_kpcl_all_nil_posi_n() {
        // filter-is-noun/neg-kpcl-all-nil-posi-n => NIL
        assert!(!filter_is_noun(&seg(
            dummy_kana_word(),
            (false, false, false, false),
            vec!["n"],
            vec![]
        )));
    }

    #[test]
    fn neg_kpcl_p_only() {
        // filter-is-noun/neg-kpcl-p-only => NIL
        // (and p c) requires BOTH p and c.
        assert!(!filter_is_noun(&seg(
            dummy_kana_word(),
            (false, true, false, false),
            vec!["n"],
            vec![]
        )));
    }

    #[test]
    fn neg_kpcl_c_only() {
        // filter-is-noun/neg-kpcl-c-only => NIL
        assert!(!filter_is_noun(&seg(
            dummy_kana_word(),
            (false, false, true, false),
            vec!["n"],
            vec![]
        )));
    }

    #[test]
    fn pos_counter_with_seqset() {
        // filter-is-noun/pos-counter-with-seqset => (1471510)
        // First branch nil (kpcl all nil), second branch T via
        // (typep word counter-text) AND non-empty seq-set.
        assert!(filter_is_noun(&seg(
            counter_word(),
            (false, false, false, false),
            vec!["ctr"],
            vec![1471510]
        )));
    }

    #[test]
    fn neg_counter_empty_seqset() {
        // filter-is-noun/neg-counter-empty-seqset => NIL
        // word IS counter but seq-set empty → second branch nil.
        assert!(!filter_is_noun(&seg(
            counter_word(),
            (false, false, false, false),
            vec!["ctr"],
            vec![]
        )));
    }

    #[test]
    fn pos_both_branches() {
        // filter-is-noun/pos-both-branches => ("n")
        // Counter word + kpcl k=T + posi=("n") → first branch wins.
        assert!(filter_is_noun(&seg(
            counter_word(),
            (true, false, false, false),
            vec!["n"],
            vec![1471510]
        )));
    }

    #[test]
    fn pos_counter_nokpcl_pass_via_second_branch() {
        // filter-is-noun/pos-counter-nokpcl-pass-via-second-branch => (12345)
        // First branch nil (kpcl all nil + posi="foo" not in tags),
        // second branch T (counter + seq-set).
        assert!(filter_is_noun(&seg(
            counter_word(),
            (false, false, false, false),
            vec!["foo"],
            vec![12345]
        )));
    }

    #[test]
    fn neg_non_counter_no_kpcl() {
        // First branch nil (kpcl all nil), second branch nil
        // (word is Kana not Counter). seq-set non-empty does not
        // save it because the typep guard fires first.
        assert!(!filter_is_noun(&seg(
            dummy_kana_word(),
            (false, false, false, false),
            vec!["n"],
            vec![12345]
        )));
    }

    #[test]
    fn neg_info_none_returns_false() {
        // Upstream crashes (destructuring-bind on nil); Rust port
        // mirrors filter_in_seq_set by returning false. Realistic
        // callsites never reach this case — gen-score populates
        // segment.info before any synergy filter runs.
        assert!(!filter_is_noun(&seg_no_info(dummy_kana_word())));
        assert!(!filter_is_noun(&seg_no_info(counter_word())));
    }
}
