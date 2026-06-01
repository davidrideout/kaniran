//! Port of `ichiran/dict:conjugate-word` (`dict-load.lisp:296`).
//!
//! Returns the list of (rule, conjugated-form) pairs produced by
//! applying every conjugation rule registered for `pos` to `word`.

use super::conjugation_rule_struct::ConjugationRule;
use super::construct_conjugation::construct_conjugation;
use super::get_conj_rules::get_conj_rules;
use super::get_pos_index::get_pos_index;

pub fn conjugate_word(word: &str, pos: &str) -> Vec<(ConjugationRule, String)> {
    let pos_id = match get_pos_index(pos) {
        Some(id) => id,
        // dict-load.lisp:297 — get-pos-index returns nil for unknown pos;
        // (get-conj-rules nil) returns nil; the loop collects nothing.
        None => return Vec::new(),
    };
    let rules = get_conj_rules(pos_id);
    // dict-load.lisp:299-300 (loop for rule in rules collect (cons rule (construct-conjugation word rule)))
    rules
        .into_iter()
        .map(|rule| {
            let conjugated = construct_conjugation(word, &rule);
            (rule, conjugated)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL probe (`(ichiran/dict::conjugate-word word pos)`), 2026-05-31.
    /// Each row pins the first few conjugated forms returned in the
    /// accessor order for a real verb / adjective: 食べる (v1), 美しい
    /// (adj-i), 走る (v5r), 飲む (v5m), 起きる (v1, upper-grade), 急ぐ
    /// (v5g), 持つ (v5t), 行く (v5k-s, the irregular godan), 来る (vk,
    /// the irregular k-row), and いう (v5u with a fully kana ending —
    /// hits the `euphr` branch in `construct-conjugation`).
    #[test]
    fn conjugate_word_first_forms() {
        let cases: &[(&str, &str, Vec<&str>)] = &[
            ("食べる", "v1", vec!["食べる", "食べます", "食べない", "食べません"]),
            (
                "美しい",
                "adj-i",
                vec!["美しい", "美しいです", "美しくない", "美しくないです", "美しくありません"],
            ),
            ("走る", "v5r", vec!["走る", "走ります", "走らない", "走りません"]),
            ("飲む", "v5m", vec!["飲む", "飲みます", "飲まない", "飲みません"]),
            ("起きる", "v1", vec!["起きる", "起きます", "起きない", "起きません"]),
            ("急ぐ", "v5g", vec!["急ぐ", "急ぎます", "急がない", "急ぎません"]),
            ("持つ", "v5t", vec!["持つ", "持ちます", "持たない", "持ちません"]),
            (
                "行く",
                "v5k-s",
                vec!["行く", "行きます", "行かない", "行きません", "行った"],
            ),
            (
                "来る",
                "vk",
                vec![
                    "来る",
                    "来ます",
                    "来ない",
                    "来ません",
                    "来た",
                    "来ました",
                    "来なかった",
                    "来ませんでした",
                ],
            ),
            (
                "買う",
                "v5u",
                vec![
                    "買う",
                    "買います",
                    "買わない",
                    "買いません",
                    "買った",
                    "買いました",
                    "買わなかった",
                    "買いませんでした",
                ],
            ),
            // fully-kana reading — exercises the `iskana=true` branch
            // selection inside `construct_conjugation`.
            ("いう", "v5u", vec!["いう", "いいます", "いわない", "いいません", "いった"]),
        ];
        for (word, pos, expected) in cases {
            let got = conjugate_word(word, pos);
            let got_forms: Vec<&str> = got.iter().take(expected.len()).map(|(_, t)| t.as_str()).collect();
            assert_eq!(&got_forms, expected, "word={word:?} pos={pos:?}");
        }
    }

    /// REPL: `(ichiran/dict::conjugate-word "食べる" "v1")` returns 60
    /// `(rule . text)` pairs. The REPL probe confirms the last three
    /// rules: `conj=12 okuri="なかったり" -> 食べなかったり`,
    /// `conj=12 okuri="ませんでしたり" -> 食べませんでしたり`,
    /// `conj=13 okuri="" -> 食べ`.
    #[test]
    fn conjugate_word_tail_for_v1() {
        let res = conjugate_word("食べる", "v1");
        assert_eq!(res.len(), 60);
        let tail: Vec<(i32, &str, &str)> = res[res.len() - 3..]
            .iter()
            .map(|(rule, text)| (rule.conj, rule.okuri.as_str(), text.as_str()))
            .collect();
        assert_eq!(
            tail,
            vec![
                (12, "なかったり", "食べなかったり"),
                (12, "ませんでしたり", "食べませんでしたり"),
                (13, "", "食べ"),
            ]
        );
    }

    /// REPL: `(ichiran/dict::conjugate-word "xxx" "nonexistent-pos")` = `NIL`.
    /// Unknown pos returns an empty list; `get_pos_index` is `None`,
    /// short-circuits before any DB access.
    #[test]
    fn conjugate_word_unknown_pos_is_empty() {
        assert!(conjugate_word("xxx", "nonexistent-pos").is_empty());
    }
}
