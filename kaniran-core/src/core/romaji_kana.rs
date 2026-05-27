//! Port of `ichiran:romaji-kana` (`deromanize.lisp:84`).
//!
//! ```lisp
//! (defun romaji-kana (s)
//!   (loop with branches = (list (make-kana-representation :rest (string-downcase s)))
//!      with finished = nil
//!      while branches
//!      do (setf branches (branches-next branches))
//!      when (and branches (alexandria:emptyp (kr-rest (car branches))))
//!      do (setf finished (car branches) branches nil)
//!      finally
//!        (when finished
//!          (return (values (kr-canonical finished) (format nil "^~a$" (kr-pattern finished)))))))
//! ```
//!
//! Deromanizes `s`: seeds one branch holding the lowercased input as
//! its remaining romaji, repeatedly steps the search via
//! `branches-next` until a branch consumes all input, and returns its
//! canonical kana paired with the anchored kana regex `^pattern$`.
//!
//! The upstream `(values canonical pattern)` collapses to
//! `Option<(String, String)>`: `Some` when a branch finishes, `None`
//! when the search exhausts without consuming the input (upstream
//! returns `nil`).

use super::branches_next::branches_next;
use super::kana_representation_struct::KanaRepresentation;

pub fn romaji_kana(s: &str) -> Option<(String, String)> {
    let mut branches = vec![KanaRepresentation {
        rest: s.to_lowercase(),
        ..KanaRepresentation::default()
    }];
    let mut finished: Option<KanaRepresentation> = None;
    while !branches.is_empty() {
        branches = branches_next(&branches);
        if !branches.is_empty() && branches[0].rest.is_empty() {
            finished = Some(branches[0].clone());
            branches.clear();
        }
    }
    finished.map(|finished| (finished.canonical, format!("^{}$", finished.pattern)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn romaji_kana_fixtures() {
        // REPL fixtures (.103, ichiran::romaji-kana), 2026-05-26.
        // (input, Some(canonical, pattern) | None).
        let cases: &[(&str, Option<(&str, &str)>)] = &[
            // doubled p + long-vowel head folded into one anchored pattern
            ("nippon", Some(("にっぽん", "^(んい|に)っぽう?ん$"))),
            // ambiguous n resolved by a join mid-word
            ("konnichiwa", Some(("こんにちわ", "^こう?ん(んい|に)ちわ$"))),
            // gemination then long vowels
            ("gakkou", Some(("がっこう", "^がっこう?うう?$"))),
            // long vowels in two syllables
            ("tokyo", Some(("ときょ", "^とう?きょう?$"))),
            ("chuui", Some(("ちゅうい", "^ちゅう?うう?い$"))),
            ("sakura", Some(("さくら", "^さくう?ら$"))),
            // single-char inputs
            ("n", Some(("ん", "^ん$"))),
            ("a", Some(("あ", "^あ$"))),
            // string-downcase: uppercase normalizes to the same parse
            ("TOKYO", Some(("ときょ", "^とう?きょう?$"))),
            ("Nippon", Some(("にっぽん", "^(んい|に)っぽう?ん$"))),
            // no complete parse → nil
            ("xyz", None),
            ("tt", None),
            ("qz", None),
            ("", None),
        ];
        for (input, expected) in cases {
            let got = romaji_kana(input);
            let exp = expected.map(|(canonical, pattern)| (canonical.to_string(), pattern.to_string()));
            assert_eq!(got, exp, "input={input:?}");
        }
    }
}
