//! Port of `ichiran:branches-next` (`deromanize.lisp:69`).
//!
//! ```lisp
//! (defun branches-next (branches)
//!   (flet ((key (b) (length (kr-rest b))))
//!     (let* ((kr (car branches))
//!            (new-branches
//!             (sort
//!              (nconc (loop for k in (romaji-next (kr-rest kr)) collect (kr-concat kr k)) (cdr branches))
//!              '>
//!              :key #'key))
//!            (new-len (length new-branches)))
//!       (if (= new-len 1)
//!           (setf (kr-branch (car new-branches)) (length (kr-pattern (car new-branches)))))
//!       (if (and (> new-len 1) (= (key (car new-branches)) (key (car (last new-branches)))))
//!           (list (join-branches new-branches))
//!           new-branches))))
//! ```
//!
//! Advances the deromanizer search one step: extends the first branch
//! with every `romaji-next` continuation of its remaining romaji,
//! appends the other branches, and sorts the result by remaining-romaji
//! length descending. A lone surviving branch has its `branch` index
//! reset to its full pattern length; when the longest and shortest
//! remaining romaji are equal, the branches merge via `join-branches`.

use super::join_branches::join_branches;
use super::kana_representation_struct::KanaRepresentation;
use super::kr_concat::kr_concat;
use super::romaji_next::romaji_next;

pub fn branches_next(branches: &[KanaRepresentation]) -> Vec<KanaRepresentation> {
    let kr = &branches[0];
    // (nconc (loop for k in (romaji-next (kr-rest kr)) collect (kr-concat kr k)) (cdr branches))
    let mut new_branches: Vec<KanaRepresentation> =
        romaji_next(&kr.rest).iter().map(|k| kr_concat(kr, k)).collect();
    new_branches.extend(branches[1..].iter().cloned());
    // (sort … '> :key #'key) — key = (length (kr-rest b)); stable like SBCL's list merge sort
    new_branches.sort_by(|left, right| right.rest.chars().count().cmp(&left.rest.chars().count()));
    let new_len = new_branches.len();
    if new_len == 1 {
        // (setf (kr-branch (car new-branches)) (length (kr-pattern (car new-branches))))
        new_branches[0].branch = new_branches[0].pattern.chars().count() as i32;
    }
    if new_len > 1
        && new_branches[0].rest.chars().count() == new_branches[new_len - 1].rest.chars().count()
    {
        vec![join_branches(&new_branches)]
    } else {
        new_branches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kr(canonical: &str, pattern: &str, rest: &str, branch: i32) -> KanaRepresentation {
        KanaRepresentation {
            canonical: canonical.to_string(),
            pattern: pattern.to_string(),
            rest: rest.to_string(),
            branch,
        }
    }

    fn show(krs: &[KanaRepresentation]) -> Vec<(String, String, String, i32)> {
        krs.iter()
            .map(|item| (item.canonical.clone(), item.pattern.clone(), item.rest.clone(), item.branch))
            .collect()
    }

    #[test]
    fn branches_next_fixtures() {
        // REPL fixtures (.103, ichiran::branches-next, captured per step of
        // romaji-kana on "nippon"/"konnichiwa"), 2026-05-26. (in, out).
        let cases: &[(Vec<KanaRepresentation>, Vec<KanaRepresentation>)] = &[
            // single branch → two candidates (ambiguous n), no merge (rest lengths differ)
            (
                vec![kr("", "", "nippon", 0)],
                vec![kr("ん", "ん", "ippon", 0), kr("に", "に", "ppon", 0)],
            ),
            // two branches, equal remaining length → join (head empty, shorter canonical kept)
            (
                vec![kr("ん", "ん", "ippon", 0), kr("に", "に", "ppon", 0)],
                vec![kr("に", "(んい|に)", "ppon", 6)],
            ),
            // lone branch → branch index reset to pattern length (6→7)
            (
                vec![kr("に", "(んい|に)", "ppon", 6)],
                vec![kr("にっ", "(んい|に)っ", "pon", 7)],
            ),
            // lone branch, long-vowel continuation, branch reset (→10)
            (
                vec![kr("にっ", "(んい|に)っ", "pon", 7)],
                vec![kr("にっぽ", "(んい|に)っぽう?", "n", 10)],
            ),
            // single-char tail consumed, branch reset (→11)
            (
                vec![kr("にっぽ", "(んい|に)っぽう?", "n", 10)],
                vec![kr("にっぽん", "(んい|に)っぽう?ん", "", 11)],
            ),
            // single branch → branch reset on a fresh long-vowel head (0→3)
            (
                vec![kr("", "", "konnichiwa", 0)],
                vec![kr("こ", "こう?", "nnichiwa", 3)],
            ),
            // single branch → two candidates, rest lengths differ (6 vs 5) → no merge
            (
                vec![kr("こん", "こう?ん", "nichiwa", 4)],
                vec![
                    kr("こんん", "こう?んん", "ichiwa", 4),
                    kr("こんに", "こう?んに", "chiwa", 4),
                ],
            ),
            // two branches, equal remaining length, non-empty head (branch 4) → join
            (
                vec![
                    kr("こんん", "こう?んん", "ichiwa", 4),
                    kr("こんに", "こう?んに", "chiwa", 4),
                ],
                vec![kr("こんに", "こう?ん(んい|に)", "chiwa", 10)],
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(show(&branches_next(input)), show(expected), "in={input:?}");
        }
    }
}
