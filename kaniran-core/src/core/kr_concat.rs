//! Port of `ichiran:kr-concat` (`deromanize.lisp:25`).
//!
//! Concatenates two deromanizer branches: appends `kr2`'s canonical
//! kana and pattern onto `kr1`'s, takes `kr2`'s remaining romaji as
//! the result `rest`, and keeps `kr1`'s `branch` tie-breaker.

use super::kana_representation_struct::KanaRepresentation;

pub fn kr_concat(kr1: &KanaRepresentation, kr2: &KanaRepresentation) -> KanaRepresentation {
    KanaRepresentation {
        canonical: format!("{}{}", kr1.canonical, kr2.canonical),
        pattern: format!("{}{}", kr1.pattern, kr2.pattern),
        rest: kr2.rest.clone(),
        branch: kr1.branch,
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
    fn kr_concat_fixtures() {
        // REPL fixtures (.103, ichiran::kr-concat), 2026-05-26. First two
        // are real chains from deromanizing nagoya/konnichiwa; the manual
        // pair pins that `branch` comes from kr1 and `rest` from kr2.
        let cases: &[(KanaRepresentation, KanaRepresentation, KanaRepresentation)] = &[
            // na + go (long-vowel kr2)
            (
                kr("な", "な", "goya", 0),
                kr("ご", "ごう?", "ya", 0),
                kr("なご", "なごう?", "ya", 0),
            ),
            // ko (long-vowel) + n
            (
                kr("こ", "こう?", "nnichiwa", 0),
                kr("ん", "ん", "nichiwa", 0),
                kr("こん", "こう?ん", "nichiwa", 0),
            ),
            // branch from kr1 (2, not 9); rest from kr2 ("new")
            (
                kr("さ", "さあ?", "old", 2),
                kr("ん", "ん", "new", 9),
                kr("さん", "さあ?ん", "new", 2),
            ),
            // branch from kr1 (0); rest from kr2
            (
                kr("あ", "あ", "iueo", 0),
                kr("い", "い", "ueo", 7),
                kr("あい", "あい", "ueo", 0),
            ),
        ];
        for (kr1, kr2, expected) in cases {
            let got = kr_concat(kr1, kr2);
            assert_eq!(got.canonical, expected.canonical, "canonical, kr1={kr1:?} kr2={kr2:?}");
            assert_eq!(got.pattern, expected.pattern, "pattern, kr1={kr1:?} kr2={kr2:?}");
            assert_eq!(got.rest, expected.rest, "rest, kr1={kr1:?} kr2={kr2:?}");
            assert_eq!(got.branch, expected.branch, "branch, kr1={kr1:?} kr2={kr2:?}");
        }
    }
}
