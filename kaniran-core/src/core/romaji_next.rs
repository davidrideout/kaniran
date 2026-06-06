//! Port of `ichiran:romaji-next` (`deromanize.lisp:46`).
//!
//! For each prefix `ss` of `s` (growing one character at a time),
//! collects the applied romaji rule when one matches, stopping once `ss`
//! is no longer a proper prefix of any romaji key. The stop is checked
//! after the collect, so the prefix that ends the scan still contributes.

use super::_star_romaji_kana_next_star_::romaji_kana_next;
use super::apply_rmap_item::apply_rmap_item;
use super::get_romaji_kana::get_romaji_kana;
use super::kana_representation_struct::KanaRepresentation;

pub fn romaji_next(s: &str) -> Vec<KanaRepresentation> {
    let mut result = Vec::new();
    for end in 1..=s.chars().count() {
        let ss: String = s.chars().take(end).collect();
        if let Some(rmi) = get_romaji_kana(&ss) {
            result.push(apply_rmap_item(s, rmi));
        }
        if !romaji_kana_next().contains(ss.as_str()) {
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(krs: &[KanaRepresentation]) -> Vec<(String, String, String, i32)> {
        krs.iter()
            .map(|kr| (kr.canonical.clone(), kr.pattern.clone(), kr.rest.clone(), kr.branch))
            .collect()
    }

    #[test]
    fn romaji_next_fixtures() {
        // REPL fixtures (.103, ichiran::romaji-next), 2026-05-26.
        // (s, [(canonical, pattern, rest, branch)…]).
        let cases: &[(&str, Vec<(&str, &str, &str, i32)>)] = &[
            // long-vowel rule (pattern gains う?), single match then stop
            ("tokyo", vec![("と", "とう?", "kyo", 0)]),
            // multi-char prefix (ch→cho), long vowel
            ("chotto", vec![("ちょ", "ちょう?", "tto", 0)]),
            // doubled-consonant rule (kk→っ, re-emits the tail consonant)
            ("kkou", vec![("っ", "っ", "kou", 0)]),
            // plain rule, no long vowel
            ("shinbun", vec![("し", "し", "nbun", 0)]),
            // single-char rule that has no successor → stops after one
            ("arigatou", vec![("あ", "あ", "rigatou", 0)]),
            // ambiguous n: "n" is itself a rule AND a proper prefix, so it
            // both collects and continues to "ni" → two candidates
            ("nippon", vec![("ん", "ん", "ippon", 0), ("に", "に", "ppon", 0)]),
            // first prefix is neither a rule nor a successor → empty
            ("xyz", vec![]),
            // empty input → no iterations
            ("", vec![]),
        ];
        for (s, expected) in cases {
            let exp: Vec<(String, String, String, i32)> = expected
                .iter()
                .map(|(canonical, pattern, rest, branch)| {
                    (canonical.to_string(), pattern.to_string(), rest.to_string(), *branch)
                })
                .collect();
            assert_eq!(show(&romaji_next(s)), exp, "s={s:?}");
        }
    }
}
