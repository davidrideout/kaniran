//! Port of `ichiran/dict:apply-score-mod` (`dict.lisp:735-742`).
//!
//! Computes a compound word's score modifier: an integer modifier
//! gives `score * score-mod * len`, a constant modifier returns its
//! captured value, and a list modifier sums the result over each
//! element.

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
