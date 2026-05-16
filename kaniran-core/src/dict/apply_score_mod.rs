//! Port of `ichiran/dict:apply-score-mod` (`dict.lisp:735-742`).
//!
//! Upstream is a `defgeneric` with three method specializations:
//! `((score-mod integer) score len)` — `(* score score-mod len)`;
//! `((score-mod function) score len)` — `(funcall score-mod score)`;
//! and `((score-mod list) score len)` — recursive reduce.
//!
//! Per CONVENTIONS §4.7, generic-function dispatch is replicated as a
//! `match` against the closed
//! [`crate::dict::compound_text_class::ScoreMod`] enum. All three
//! methods are reachable on real input:
//!
//! - [`ScoreMod::Single`] ← `((score-mod integer) …)`. Most
//!   `def-simple-suffix` callsites pass an integer literal
//!   (e.g. `dict-grammar.lisp:370` `:score 5`).
//! - [`ScoreMod::Constant`] ← `((score-mod function) …)`. Four upstream
//!   callsites construct a `(constantly N)` closure as `:score`:
//!   `suffix-kudasai` (`dict-grammar.lisp:404`, `360`), `suffix-sou`
//!   (`dict-grammar.lisp:448`, `40/0/100/70` depending on `root`),
//!   `suffix-desu` (`dict-grammar.lisp:516`, `200`), `suffix-desho`
//!   (`dict-grammar.lisp:532`, `300`). The closure ignores its
//!   argument and returns `N`; the Rust variant carries `N` directly.
//! - [`ScoreMod::Stack`] ← `((score-mod list) …)`. Built by
//!   `adjoin-word`'s cons/list growth at `dict.lisp:651` when the
//!   compound is extended a second time. Elements are themselves
//!   `Single` or `Constant`; the recursive `apply-score-mod` call
//!   dispatches each element through the same match.
//!
//! ## `Constant` is narrow on purpose
//!
//! [`ScoreMod::Constant(i32)`] models **only** the closures produced
//! by `(constantly N)` — closures whose body ignores their argument
//! and returns the captured `N`. The upstream `((score-mod function)
//! …)` method at `dict.lisp:739-740` dispatches an arbitrary
//! `funcall score-mod score`, so a non-`constantly` closure (one
//! whose body actually consults `score`) would silently produce the
//! wrong value if forced through this variant.
//!
//! All four current upstream callsites pass `(constantly N)` for some
//! integer `N` evaluated at adjoin-word time (`suffix-sou`'s `(cond
//! …)` runs once before `constantly` wraps it). If a future upstream
//! addition introduces a non-`constantly` closure as `:score-mod`,
//! **add a new variant** (e.g. `Callable(fn(i64) -> i64)` or an enum
//! of named closures) — do not reuse `Constant`. The same parity rule
//! applies the other direction: when `suffix-sou` is ported, the Rust
//! caller MUST evaluate the four-case `cond` on `root` at dispatch
//! time and pass `ScoreMod::Constant(40 | 0 | 100 | 70)` based on the
//! result; no compile-time check enforces table parity with
//! `dict-grammar.lisp:448-452`, so the porting site needs a
//! `// PARITY:` comment back-referencing that range.
//!
//! ## Integer width
//!
//! Every numeric value (stored `score-mod`, `score`, `len`, and the
//! return) is `i64`, matching SBCL's 63-bit fixnum. The product
//! `score * sm * len` reaches `~3.6e12` for plausible upstream inputs
//! (`prop-score` ≤ `1_000_000` × `score-mod` up to `360` × `len` ≤
//! `10_000`), well past `i32::MAX` but inside `i64` with margin.
//! Going all-`i64` removes the cast boilerplate at the function
//! boundary and keeps the type story aligned with CL's fixnum
//! semantics.

use crate::dict::compound_text_class::ScoreMod;

pub fn apply_score_mod(score_mod: &ScoreMod, score: i64, len: i64) -> i64 {
    match score_mod {
        // dict.lisp:737-738 — (:method ((score-mod integer) score len)
        //                       (* score score-mod len))
        ScoreMod::Single(n) => score * n * len,
        // dict.lisp:739-740 — (:method ((score-mod function) score len)
        //                       (funcall score-mod score)).
        // Every reachable upstream value is (constantly N), which
        // ignores its argument and returns N.
        ScoreMod::Constant(n) => *n,
        // dict.lisp:741-742 — (:method ((score-mod list) score len)
        //                       (reduce '+ score-mod
        //                         :key (lambda (sm) (apply-score-mod sm score len))))
        ScoreMod::Stack(stack) => stack
            .iter()
            .map(|sm| apply_score_mod(sm, score, len))
            .sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // All assertions REPL-pinned against upstream ichiran.
    #[test]
    fn integer_method_positive() {
        // (apply-score-mod 3 10 4) = 120
        assert_eq!(apply_score_mod(&ScoreMod::Single(3), 10, 4), 120);
    }

    #[test]
    fn integer_method_zero_score_mod() {
        // (apply-score-mod 0 10 4) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Single(0), 10, 4), 0);
    }

    #[test]
    fn integer_method_negative_score_mod() {
        // (apply-score-mod -2 5 3) = -30
        assert_eq!(apply_score_mod(&ScoreMod::Single(-2), 5, 3), -30);
    }

    #[test]
    fn integer_method_zero_score() {
        // (apply-score-mod 7 0 5) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Single(7), 0, 5), 0);
    }

    #[test]
    fn integer_method_zero_len() {
        // (apply-score-mod 1 100 0) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Single(1), 100, 0), 0);
    }

    // ----- function method, reached via (constantly N) -----

    #[test]
    fn constant_method_positive() {
        // (apply-score-mod (constantly 360) 10 4) = 360
        assert_eq!(apply_score_mod(&ScoreMod::Constant(360), 10, 4), 360);
    }

    #[test]
    fn constant_method_zero_args() {
        // (apply-score-mod (constantly 200) 0 0) = 200
        assert_eq!(apply_score_mod(&ScoreMod::Constant(200), 0, 0), 200);
    }

    #[test]
    fn constant_method_ignores_score_and_len() {
        // (apply-score-mod (constantly 300) 50 7) = 300
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 50, 7), 300);
    }

    #[test]
    fn constant_method_negative() {
        // (apply-score-mod (constantly -5) 10 4) = -5
        assert_eq!(apply_score_mod(&ScoreMod::Constant(-5), 10, 4), -5);
    }

    #[test]
    fn constant_method_zero_value() {
        // (apply-score-mod (constantly 0) 10 4) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Constant(0), 10, 4), 0);
    }

    // ----- list method -----

    #[test]
    fn list_method_two_integer_elements() {
        // (apply-score-mod '(3 4) 10 2) = 140
        // = (3*10*2) + (4*10*2) = 60 + 80 = 140
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![ScoreMod::Single(3), ScoreMod::Single(4)]),
                10,
                2
            ),
            140
        );
    }

    #[test]
    fn list_method_empty() {
        // (apply-score-mod '() 10 2) = 0
        assert_eq!(apply_score_mod(&ScoreMod::Stack(vec![]), 10, 2), 0);
    }

    #[test]
    fn list_method_mixed_signs() {
        // (apply-score-mod '(5 -2 1) 10 3) = 120
        // = (5*10*3) + (-2*10*3) + (1*10*3) = 150 - 60 + 30 = 120
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![
                    ScoreMod::Single(5),
                    ScoreMod::Single(-2),
                    ScoreMod::Single(1),
                ]),
                10,
                3
            ),
            120
        );
    }

    #[test]
    fn list_method_single_integer() {
        // (apply-score-mod '(1) 10 5) = 50
        assert_eq!(
            apply_score_mod(&ScoreMod::Stack(vec![ScoreMod::Single(1)]), 10, 5),
            50
        );
    }

    #[test]
    fn list_method_all_zeros() {
        // (apply-score-mod '(0 0 0) 10 5) = 0
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![
                    ScoreMod::Single(0),
                    ScoreMod::Single(0),
                    ScoreMod::Single(0),
                ]),
                10,
                5
            ),
            0
        );
    }

    // ----- list method, mixed function + integer elements -----

    #[test]
    fn list_constant_then_integer() {
        // (apply-score-mod (list (constantly 360) 5) 10 2) = 460
        // = 360 + (5*10*2) = 360 + 100
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![ScoreMod::Constant(360), ScoreMod::Single(5)]),
                10,
                2
            ),
            460
        );
    }

    #[test]
    fn list_integer_then_constant() {
        // (apply-score-mod (list 3 (constantly 100)) 10 2) = 160
        // = (3*10*2) + 100 = 60 + 100
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![ScoreMod::Single(3), ScoreMod::Constant(100)]),
                10,
                2
            ),
            160
        );
    }

    #[test]
    fn list_two_constants() {
        // (apply-score-mod (list (constantly 200) (constantly 300)) 10 2) = 500
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![
                    ScoreMod::Constant(200),
                    ScoreMod::Constant(300),
                ]),
                10,
                2
            ),
            500
        );
    }

    #[test]
    fn list_constant_then_two_integers() {
        // (apply-score-mod (cons (constantly 360) (list 5 7)) 10 2) = 600
        // = 360 + (5*10*2) + (7*10*2) = 360 + 100 + 140
        assert_eq!(
            apply_score_mod(
                &ScoreMod::Stack(vec![
                    ScoreMod::Constant(360),
                    ScoreMod::Single(5),
                    ScoreMod::Single(7),
                ]),
                10,
                2
            ),
            600
        );
    }

    // ----- Constant method exercised by real ichiran segmentation -----
    //
    // The four upstream `(constantly N)` callsites
    // (`suffix-kudasai`/`sou`/`desu`/`desho` at
    // `dict-grammar.lisp:404, 448, 516, 532`) only fire on specific
    // sentence shapes. The (score, len) arguments below were captured
    // by hooking `apply-score-mod` via `sb-int:encapsulate` and
    // running `(simple-segment <sentence>)` through the production
    // segmenter; every recorded function-typed call is preserved
    // verbatim so a regression on the Constant method's arithmetic is
    // detectable end-to-end, not just on synthetic inputs.

    #[test]
    fn kudasai_real_sentence_taberu_te_kudasai() {
        // "食べてください" → suffix-kudasai (dict-grammar.lisp:404)
        //   (apply-score-mod (constantly 360) 14 4) = 360
        assert_eq!(apply_score_mod(&ScoreMod::Constant(360), 14, 4), 360);
    }

    #[test]
    fn sou_real_sentence_omoshiroi_sou() {
        // "面白そう" → suffix-sou default branch (root ∉ {から, い, 出来},
        // dict-grammar.lisp:448-452 falls through to N=70)
        //   (apply-score-mod (constantly 70) 14 2) = 70
        assert_eq!(apply_score_mod(&ScoreMod::Constant(70), 14, 2), 70);
    }

    #[test]
    fn sou_real_sentence_dekiru_sou() {
        // "出来そう" → suffix-sou produces two segmenter parses:
        //   parse 1 (root not "出来"): (apply-score-mod (constantly 70) 14 2) = 70
        //   parse 2 (root = "出来"):   (apply-score-mod (constantly 100) 9  2) = 100
        assert_eq!(apply_score_mod(&ScoreMod::Constant(70), 14, 2), 70);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(100), 9, 2), 100);
    }

    #[test]
    fn desu_real_sentence_tabenai_desu() {
        // "食べないです" → suffix-desu (dict-grammar.lisp:516)
        //   (apply-score-mod (constantly 200) 21 2) = 200
        assert_eq!(apply_score_mod(&ScoreMod::Constant(200), 21, 2), 200);
    }

    #[test]
    fn desho_real_sentence_tabenai_deshou() {
        // "食べないでしょう" → suffix-desho (dict-grammar.lisp:532).
        // Segmenter explores 6 parses, all hitting the same compound
        // but with different (score, len) pairs. All six return 300
        // because the FUNCTION method ignores score and len.
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 4, 3), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 10, 3), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 4, 2), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 10, 2), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 21, 3), 300);
        assert_eq!(apply_score_mod(&ScoreMod::Constant(300), 21, 2), 300);
    }
}
