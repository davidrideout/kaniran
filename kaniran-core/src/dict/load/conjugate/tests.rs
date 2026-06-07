use super::*;

// --- construct_conjugation ---
fn rule(stem: i32, okuri: &str, euphr: &str, euphk: &str) -> ConjugationRule {
    ConjugationRule {
        pos: 0,
        conj: 0,
        neg: false,
        fml: false,
        onum: 1,
        stem,
        okuri: okuri.to_string(),
        euphr: euphr.to_string(),
        euphk: euphk.to_string(),
    }
}

/// Exercises every branch: kana vs kanji ending, euphonic-reading vs
/// euphonic-kanji selection, the +1 stem bump when the applicable euphonic
/// fragment is non-empty (and no bump when empty), the plain no-euphonic
/// case, and the lower-bound guard on a one-character reading.
#[test]
fn construct_conjugation_paths() {
    // (reading, rule, expected)
    let cases: &[(&str, ConjugationRule, &str)] = &[
        // vs-i conj=5: euphr non-empty, kana ending -> euphr + bump
        ("する", rule(1, "る", "でき", "出来"), "できる"),
        // vs-i conj=5: euphk non-empty, kanji ending -> euphk + bump
        ("為る", rule(1, "る", "でき", "出来"), "出来る"),
        // adj-ix conj=2: euphr non-empty, kana ending -> euphr + bump
        ("いい", rule(1, "かった", "よ", ""), "よかった"),
        // adj-ix conj=2: euphk empty, kanji ending -> no bump, euphk ""
        ("良い", rule(1, "かった", "よ", ""), "良かった"),
        // vk conj=1: euphr non-empty, kana ending -> euphr + bump
        ("くる", rule(1, "ます", "き", ""), "きます"),
        // vk conj=1: euphk empty, kanji ending -> no bump, euphk ""
        ("来る", rule(1, "ます", "き", ""), "来ます"),
        // v5u conj=3: no euphonic fragments, kana ending -> no bump
        ("かう", rule(1, "って", "", ""), "かって"),
        // v5u conj=3: no euphonic fragments, kanji ending -> no bump
        ("買う", rule(1, "って", "", ""), "買って"),
        // one-character reading: (max 0 (- 1 2)) guard, stem=1 -> prefix ""
        ("る", rule(1, "わ", "", ""), "わ"),
    ];
    for (reading, conj_rule, expected) in cases {
        assert_eq!(
            construct_conjugation(reading, conj_rule),
            *expected,
            "reading={reading}"
        );
    }
}

// --- conjugate_word ---
/// Each row pins the first few conjugated forms for a verb or adjective
/// across part-of-speech classes, including the irregular verbs 行く and
/// 来る and a fully-kana ending (いう) that takes the euphonic-reading branch.
#[test]
fn conjugate_word_first_forms() {
    let cases: &[(&str, &str, Vec<&str>)] = &[
        (
            "食べる",
            "v1",
            vec!["食べる", "食べます", "食べない", "食べません"],
        ),
        (
            "美しい",
            "adj-i",
            vec![
                "美しい",
                "美しいです",
                "美しくない",
                "美しくないです",
                "美しくありません",
            ],
        ),
        (
            "走る",
            "v5r",
            vec!["走る", "走ります", "走らない", "走りません"],
        ),
        (
            "飲む",
            "v5m",
            vec!["飲む", "飲みます", "飲まない", "飲みません"],
        ),
        (
            "起きる",
            "v1",
            vec!["起きる", "起きます", "起きない", "起きません"],
        ),
        (
            "急ぐ",
            "v5g",
            vec!["急ぐ", "急ぎます", "急がない", "急ぎません"],
        ),
        (
            "持つ",
            "v5t",
            vec!["持つ", "持ちます", "持たない", "持ちません"],
        ),
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
        // fully-kana reading — takes the kana-ending branch.
        (
            "いう",
            "v5u",
            vec!["いう", "いいます", "いわない", "いいません", "いった"],
        ),
    ];
    for (word, pos, expected) in cases {
        let got = conjugate_word(word, pos);
        let got_forms: Vec<&str> = got
            .iter()
            .take(expected.len())
            .map(|(_, t)| t.as_str())
            .collect();
        assert_eq!(&got_forms, expected, "word={word:?} pos={pos:?}");
    }
}

/// Conjugating 食べる (v1) returns 60 rule/text pairs; this pins the last three.
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

/// An unknown part-of-speech returns an empty list, short-circuiting
/// before any database access.
#[test]
fn conjugate_word_unknown_pos_is_empty() {
    assert!(conjugate_word("xxx", "nonexistent-pos").is_empty());
}

// --- lex_compare ---
#[test]
fn smaller_at_last_position_is_less() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    assert!(cmp(&[1, 2, 3], &[1, 2, 4]));
}

#[test]
fn larger_at_last_position_is_not_less() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    assert!(!cmp(&[1, 2, 4], &[1, 2, 3]));
}

/// Equal sequences compare as not-less.
#[test]
fn equal_sequences_return_false() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    assert!(!cmp(&[1, 2, 3], &[1, 2, 3]));
}

/// List input behaves the same as slice input.
#[test]
fn list_inputs_behave_the_same() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    let s1: Vec<i32> = vec![1, 2, 3];
    let s2: Vec<i32> = vec![1, 2, 4];
    assert!(cmp(&s1, &s2));
}

#[test]
fn empty_sequences_return_false() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    let empty: [i32; 0] = [];
    assert!(!cmp(&empty, &empty));
}

/// Unequal lengths walk only the shared prefix; an equal shared prefix
/// compares as not-less.
#[test]
fn shorter_seq_with_equal_prefix_returns_false() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    assert!(!cmp(&[1, 2], &[1, 2, 3]));
}

/// A longer sequence with an equal shorter prefix compares as not-less.
#[test]
fn longer_seq_with_equal_prefix_returns_false() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    assert!(!cmp(&[1, 2, 3], &[1, 2]));
}

/// A non-empty sequence compared against an empty one is not-less.
#[test]
fn empty_vs_non_empty_returns_false() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    let empty: [i32; 0] = [];
    assert!(!cmp(&[1], &empty));
}

/// The first differing position settles the order: `(2,9) < (3,1)`.
#[test]
fn first_position_dominates_when_unequal() {
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    let result = cmp(&[2, 9], &[3, 1]);
    assert!(result);
}
