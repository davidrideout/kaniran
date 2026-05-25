//! Port of `ichiran/dict:filter-is-pos` (`dict-grammar.lisp:757`).
//!
//! ```lisp
//! (defmacro filter-is-pos (pos-list (segment &rest kpcl-vars) &body kpcl-test)
//!   `(lambda (,segment)
//!      (destructuring-bind ,kpcl-vars (getf (segment-info ,segment) :kpcl)
//!        (declare (ignorable ,@kpcl-vars))
//!        (and (progn ,@kpcl-test)
//!             (intersection ',pos-list
//!                           (getf (segment-info ,segment) :posi)
//!                           :test 'equal)))))
//! ```
//!
//! Builds the segment predicate the macro's `(lambda (segment) ...)`
//! expands to: the `kpcl_test` body over the `(kanji-or-katakana,
//! primary, common, long)` quad, AND-ed with overlap between
//! `pos_mask` and the segment's parts-of-speech. Used as the
//! `filter-left` / `filter-right` argument at the six
//! `def-generic-synergy` callsites (`dict-grammar.lisp:864`, `871`,
//! `878`, `915`, `922`, `952`).
//!
//! Divergences from Lisp:
//! - The macro is factored into one closure-returning helper (its
//!   expansion is identical across callsites up to `pos-list` and
//!   `kpcl-test`); each callsite collapses to
//!   `filter_is_pos(pos_mask, |k, p, c, l| ...)`.
//! - `pos-list` (POS strings intersected against `:posi`) becomes
//!   `pos_mask: u16`: the lite [`KaniLiteSegment`] stores
//!   parts-of-speech only as the precomputed bitmask, so the
//!   intersection is a bit-and.
//! - `kpcl-test` (a `&body`) becomes the `kpcl_test` closure over the
//!   four kpcl bools; the closure returns `bool` (the `(and ...)`
//!   truthiness, consumed only by `remove-if-not`).
//! - Operates on `&Arc<KaniLiteSegment>` like the rest of the filter
//!   family.

use std::sync::Arc;

use super::kani_lite_segment::{KaniLiteSegment, KPCL_C, KPCL_K, KPCL_L, KPCL_P};

pub fn filter_is_pos(
    pos_mask: u16,
    kpcl_test: impl Fn(bool, bool, bool, bool) -> bool,
) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        let k = segment.kpcl & KPCL_K != 0;
        let p = segment.kpcl & KPCL_P != 0;
        let c = segment.kpcl & KPCL_C != 0;
        let l = segment.kpcl & KPCL_L != 0;
        kpcl_test(k, p, c, l) && (segment.pos & pos_mask) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data_struct::ConjData;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_lite_segment::{
        POS_ADJ_NA, POS_ADJ_NO, POS_ADV_TO, POS_CTR, POS_N,
    };
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

    fn lite(kpcl: (bool, bool, bool, bool), posi: &[&str]) -> Arc<KaniLiteSegment> {
        let info = KaniSegmentInfo {
            posi: posi.iter().map(|s| s.to_string()).collect(),
            seq_set: vec![],
            conj: vec![] as Vec<ConjData>,
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl,
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

    // kpcl-test bodies used at the upstream `filter-is-pos` callsites
    // plus a few that isolate the kpcl gate from the pos gate.
    fn adj(k: bool, p: bool, c: bool, l: bool) -> bool {
        k || l || (p && c)
    } // dict-grammar.lisp:864/871 (or k l (and p c))
    fn advto(k: bool, p: bool, _c: bool, l: bool) -> bool {
        k || l || p
    } // dict-grammar.lisp:878 (or k l p)
    fn orkl(k: bool, _p: bool, _c: bool, l: bool) -> bool {
        k || l
    } // dict-grammar.lisp:915 (or k l)
    fn konly(k: bool, _p: bool, _c: bool, _l: bool) -> bool {
        k
    } // dict-grammar.lisp:922 (k)
    fn always(_k: bool, _p: bool, _c: bool, _l: bool) -> bool {
        true
    } // dict-grammar.lisp:952 (t)
    fn ponly(_k: bool, p: bool, _c: bool, _l: bool) -> bool {
        p
    } // isolation case
    fn pandc(_k: bool, p: bool, c: bool, _l: bool) -> bool {
        p && c
    } // isolation case

    #[test]
    fn filter_is_pos_fixtures() {
        // REPL fixtures (.103, `ichiran/dict::filter-is-pos` applied to
        // a `gen-score`d segment), 2026-05-24. Columns:
        // (label, kpcl (k p c l), posi, pos_mask, kpcl_test, expected).
        type Test = fn(bool, bool, bool, bool) -> bool;
        let cases: &[(&str, (bool, bool, bool, bool), &[&str], u16, Test, bool)] = &[
            // 普通 — kpcl=(T T T NIL) posi=(adj-na adj-no adv n)
            ("futsuu adj-no/adj", (true, true, true, false), &["adj-na", "adj-no", "adv", "n"], POS_ADJ_NO, adj, true),
            ("futsuu adj-na/adj", (true, true, true, false), &["adj-na", "adj-no", "adv", "n"], POS_ADJ_NA, adj, true),
            ("futsuu adv-to/advto", (true, true, true, false), &["adj-na", "adj-no", "adv", "n"], POS_ADV_TO, advto, false),
            ("futsuu n/orkl", (true, true, true, false), &["adj-na", "adj-no", "adv", "n"], POS_N, orkl, true),
            ("futsuu ctr/t", (true, true, true, false), &["adj-na", "adj-no", "adv", "n"], POS_CTR, always, false),
            // 政府 — kpcl=(T T T NIL) posi=(n)
            ("seifu adj-no/adj", (true, true, true, false), &["n"], POS_ADJ_NO, adj, false),
            ("seifu n/orkl", (true, true, true, false), &["n"], POS_N, orkl, true),
            // 静か — kpcl=(T T T NIL) posi=(adj-na)
            ("shizuka adj-na/adj", (true, true, true, false), &["adj-na"], POS_ADJ_NA, adj, true),
            ("shizuka n/orkl", (true, true, true, false), &["adj-na"], POS_N, orkl, false),
            // 個 — kpcl=(T T T NIL) posi=(ctr n)
            ("ko ctr/t", (true, true, true, false), &["ctr", "n"], POS_CTR, always, true),
            // 三 — kpcl=(T T T NIL) posi=(num): num maps to no bit → empty intersection
            ("san adj-no/adj (num→0)", (true, true, true, false), &["num"], POS_ADJ_NO, adj, false),
            ("san n/orkl (num→0)", (true, true, true, false), &["num"], POS_N, orkl, false),
            // ゆっくり — kpcl=(NIL T T NIL) posi=(adv adv-to vs)
            ("yukkuri adv-to/advto (k=F)", (false, true, true, false), &["adv", "adv-to", "vs"], POS_ADV_TO, advto, true),
            ("yukkuri adv-to/konly (pos-match,test-F)", (false, true, true, false), &["adv", "adv-to", "vs"], POS_ADV_TO, konly, false),
            ("yukkuri adj-no/adj", (false, true, true, false), &["adv", "adv-to", "vs"], POS_ADJ_NO, adj, false),
            // 本 — kpcl=(T NIL T NIL) posi=(ctr n)
            ("hon ctr/t", (true, false, true, false), &["ctr", "n"], POS_CTR, always, true),
            ("hon n/konly (k=T)", (true, false, true, false), &["ctr", "n"], POS_N, konly, true),
            ("hon n/ponly (pos-match,test-F)", (true, false, true, false), &["ctr", "n"], POS_N, ponly, false),
            ("hon n/pandc (pos-match,test-F)", (true, false, true, false), &["ctr", "n"], POS_N, pandc, false),
            ("hon adj-no/adj (p=F)", (true, false, true, false), &["ctr", "n"], POS_ADJ_NO, adj, false),
        ];
        for (label, kpcl, posi, pos_mask, kpcl_test, expected) in cases {
            let seg = lite(*kpcl, posi);
            let predicate = filter_is_pos(*pos_mask, kpcl_test);
            assert_eq!(predicate(&seg), *expected, "case={label}");
        }
    }
}
