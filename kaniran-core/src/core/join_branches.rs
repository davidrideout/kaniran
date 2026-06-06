//! Port of `ichiran:join-branches` (`deromanize.lisp:55`).
//!
//! Collapses sibling deromanizer branches that share the same remaining
//! input into one [`KanaRepresentation`] whose pattern is the alternation
//! `head(tail1|tail2|...)`. The canonical kana is the shortest branch
//! canonical (ties keep the first).

use super::kana_representation_struct::KanaRepresentation;

pub fn join_branches(branches: &[KanaRepresentation]) -> KanaRepresentation {
    let b0 = &branches[0];
    let tails: Vec<String> = branches
        .iter()
        .map(|b| b.pattern.chars().skip(b.branch as usize).collect())
        .collect();
    let head: String = b0.pattern.chars().take(b0.branch as usize).collect();
    let joined_kana = format!("{}({})", head, tails.join("|"));
    let canonical = branches
        .iter()
        .map(|b| &b.canonical)
        .reduce(|x, y| if x.chars().count() <= y.chars().count() { x } else { y })
        .expect("branches is non-empty")
        .clone();
    let branch = joined_kana.chars().count() as i32;
    KanaRepresentation {
        canonical,
        pattern: joined_kana,
        rest: b0.rest.clone(),
        branch,
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

    #[test]
    fn join_branches_fixtures() {
        // REPL fixtures (.103, ichiran::join-branches), 2026-05-25.
        // First two are real captures from romaji-kana("nagoya")/("konnichiwa")
        // tracing; the rest are direct calls exercising the canonical
        // reduce (first-shorter / equal-tie) and a three-branch join.
        let cases: &[(Vec<KanaRepresentation>, KanaRepresentation)] = &[
            // head empty, first canonical longer -> second wins
            (
                vec![kr("んあ", "んあ", "goya", 0), kr("な", "な", "goya", 0)],
                kr("な", "(んあ|な)", "goya", 6),
            ),
            // head non-empty (branch 4), first canonical longer -> second wins
            (
                vec![
                    kr("こんんい", "こう?んんい", "chiwa", 4),
                    kr("こんに", "こう?んに", "chiwa", 4),
                ],
                kr("こんに", "こう?ん(んい|に)", "chiwa", 10),
            ),
            // first canonical shorter -> first kept
            (
                vec![kr("あ", "あ?", "x", 1), kr("いう", "いう", "x", 1)],
                kr("あ", "あ(?|う)", "x", 6),
            ),
            // equal canonical length -> tie keeps first
            (
                vec![kr("かき", "かき", "yo", 0), kr("くけ", "くけ", "yo", 0)],
                kr("かき", "(かき|くけ)", "yo", 7),
            ),
            // three branches
            (
                vec![
                    kr("さん", "さあ?ん", "z", 2),
                    kr("さに", "さあ?に", "z", 2),
                    kr("さ", "さあ?ぬ", "z", 2),
                ],
                kr("さ", "さあ(?ん|?に|?ぬ)", "z", 12),
            ),
        ];
        for (branches, expected) in cases {
            let got = join_branches(branches);
            assert_eq!(got.canonical, expected.canonical, "canonical, branches={branches:?}");
            assert_eq!(got.pattern, expected.pattern, "pattern, branches={branches:?}");
            assert_eq!(got.rest, expected.rest, "rest, branches={branches:?}");
            assert_eq!(got.branch, expected.branch, "branch, branches={branches:?}");
        }
    }
}
