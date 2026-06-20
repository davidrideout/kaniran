use super::*;
use std::sync::Arc;

// --- has_successors ---
fn set(items: &[&str]) -> HashSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn has_successors_fixtures() {
    // (input, expected proper-prefix set).
    let cases: &[(&[&str], &[&str])] = &[
        // "cha"->{c,ch}, "chi"->{c,ch}, "ba"->{b}
        (&["cha", "chi", "ba"], &["c", "ch", "b"]),
        // single-char and empty strings yield no proper prefixes
        (&["a", "x", ""], &[]),
        // "kya"->{k,ky}, "kyo"->{k,ky}, "sha"->{s,sh}
        (&["kya", "kyo", "sha"], &["k", "ky", "s", "sh"]),
    ];
    for (input, expected) in cases {
        assert_eq!(has_successors(input), set(expected), "input={input:?}");
    }
}

// --- kr_concat ---
fn kr_concat_kr(canonical: &str, pattern: &str, rest: &str, branch: i32) -> KanaRepresentation {
    KanaRepresentation {
        canonical: canonical.to_string(),
        pattern: pattern.to_string(),
        rest: rest.to_string(),
        branch,
    }
}

#[test]
fn kr_concat_fixtures() {
    // Concatenating two kana representations takes `branch` from the first
    // and `rest` from the second.
    let cases: &[(KanaRepresentation, KanaRepresentation, KanaRepresentation)] = &[
        // na + go (long-vowel kr2)
        (
            kr_concat_kr("な", "な", "goya", 0),
            kr_concat_kr("ご", "ごう?", "ya", 0),
            kr_concat_kr("なご", "なごう?", "ya", 0),
        ),
        // ko (long-vowel) + n
        (
            kr_concat_kr("こ", "こう?", "nnichiwa", 0),
            kr_concat_kr("ん", "ん", "nichiwa", 0),
            kr_concat_kr("こん", "こう?ん", "nichiwa", 0),
        ),
        // branch from kr1 (2, not 9); rest from kr2 ("new")
        (
            kr_concat_kr("さ", "さあ?", "old", 2),
            kr_concat_kr("ん", "ん", "new", 9),
            kr_concat_kr("さん", "さあ?ん", "new", 2),
        ),
        // branch from kr1 (0); rest from kr2
        (
            kr_concat_kr("あ", "あ", "iueo", 0),
            kr_concat_kr("い", "い", "ueo", 7),
            kr_concat_kr("あい", "あい", "ueo", 0),
        ),
    ];
    for (kr1, kr2, expected) in cases {
        let got = kr_concat(kr1, kr2);
        assert_eq!(
            got.canonical, expected.canonical,
            "canonical, kr1={kr1:?} kr2={kr2:?}"
        );
        assert_eq!(
            got.pattern, expected.pattern,
            "pattern, kr1={kr1:?} kr2={kr2:?}"
        );
        assert_eq!(got.rest, expected.rest, "rest, kr1={kr1:?} kr2={kr2:?}");
        assert_eq!(
            got.branch, expected.branch,
            "branch, kr1={kr1:?} kr2={kr2:?}"
        );
    }
}

// --- possible_long_vowel_p ---
#[test]
fn possible_long_vowel_p_fixtures() {
    let cases: &[(&str, Option<char>)] = &[
        ("", None),        // empty
        ("ko", Some('o')), // ends o
        ("ku", Some('u')), // ends u
        ("ka", None),      // ends a
        ("o", Some('o')),  // single o
        ("u", Some('u')),  // single u
        ("shinbun", None), // ends n
        ("kyou", Some('u')),
        ("toukyou", Some('u')),
        ("sapporo", Some('o')),
        ("gakkou", Some('u')),
        ("arigatou", Some('u')),
        ("tomodachi", None), // ends i
        ("fujisan", None),   // ends n
        ("おう", None),      // last char う (not ASCII o/u)
        ("あo", Some('o')),  // multibyte prefix, last char o
        ("katsu", Some('u')),
    ];
    for (text, expected) in cases {
        assert_eq!(possible_long_vowel_p(text), *expected, "text={text:?}");
    }
}

// --- romaji_next ---
fn romaji_next_show(krs: &[KanaRepresentation]) -> Vec<(String, String, String, i32)> {
    krs.iter()
        .map(|kr| {
            (
                kr.canonical.clone(),
                kr.pattern.clone(),
                kr.rest.clone(),
                kr.branch,
            )
        })
        .collect()
}

#[test]
fn romaji_next_fixtures() {
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
        (
            "nippon",
            vec![("ん", "ん", "ippon", 0), ("に", "に", "ppon", 0)],
        ),
        // first prefix is neither a rule nor a successor → empty
        ("xyz", vec![]),
        // empty input → no iterations
        ("", vec![]),
    ];
    for (s, expected) in cases {
        let exp: Vec<(String, String, String, i32)> = expected
            .iter()
            .map(|(canonical, pattern, rest, branch)| {
                (
                    canonical.to_string(),
                    pattern.to_string(),
                    rest.to_string(),
                    *branch,
                )
            })
            .collect();
        assert_eq!(romaji_next_show(&romaji_next(s)), exp, "s={s:?}");
    }
}

// --- join_branches ---
fn join_branches_kr(canonical: &str, pattern: &str, rest: &str, branch: i32) -> KanaRepresentation {
    KanaRepresentation {
        canonical: canonical.to_string(),
        pattern: pattern.to_string(),
        rest: rest.to_string(),
        branch,
    }
}

#[test]
fn join_branches_fixtures() {
    // Exercises the branch-merge rules: shorter canonical wins, an
    // equal-length tie keeps the first, plus a three-branch join.
    let cases: &[(Vec<KanaRepresentation>, KanaRepresentation)] = &[
        // head empty, first canonical longer -> second wins
        (
            vec![
                join_branches_kr("んあ", "んあ", "goya", 0),
                join_branches_kr("な", "な", "goya", 0),
            ],
            join_branches_kr("な", "(んあ|な)", "goya", 6),
        ),
        // head non-empty (branch 4), first canonical longer -> second wins
        (
            vec![
                join_branches_kr("こんんい", "こう?んんい", "chiwa", 4),
                join_branches_kr("こんに", "こう?んに", "chiwa", 4),
            ],
            join_branches_kr("こんに", "こう?ん(んい|に)", "chiwa", 10),
        ),
        // first canonical shorter -> first kept
        (
            vec![
                join_branches_kr("あ", "あ?", "x", 1),
                join_branches_kr("いう", "いう", "x", 1),
            ],
            join_branches_kr("あ", "あ(?|う)", "x", 6),
        ),
        // equal canonical length -> tie keeps first
        (
            vec![
                join_branches_kr("かき", "かき", "yo", 0),
                join_branches_kr("くけ", "くけ", "yo", 0),
            ],
            join_branches_kr("かき", "(かき|くけ)", "yo", 7),
        ),
        // three branches
        (
            vec![
                join_branches_kr("さん", "さあ?ん", "z", 2),
                join_branches_kr("さに", "さあ?に", "z", 2),
                join_branches_kr("さ", "さあ?ぬ", "z", 2),
            ],
            join_branches_kr("さ", "さあ(?ん|?に|?ぬ)", "z", 12),
        ),
    ];
    for (branches, expected) in cases {
        let got = join_branches(branches);
        assert_eq!(
            got.canonical, expected.canonical,
            "canonical, branches={branches:?}"
        );
        assert_eq!(
            got.pattern, expected.pattern,
            "pattern, branches={branches:?}"
        );
        assert_eq!(got.rest, expected.rest, "rest, branches={branches:?}");
        assert_eq!(got.branch, expected.branch, "branch, branches={branches:?}");
    }
}

// --- branches_next ---
fn branches_next_kr(canonical: &str, pattern: &str, rest: &str, branch: i32) -> KanaRepresentation {
    KanaRepresentation {
        canonical: canonical.to_string(),
        pattern: pattern.to_string(),
        rest: rest.to_string(),
        branch,
    }
}

fn branches_next_show(krs: &[KanaRepresentation]) -> Vec<(String, String, String, i32)> {
    krs.iter()
        .map(|item| {
            (
                item.canonical.clone(),
                item.pattern.clone(),
                item.rest.clone(),
                item.branch,
            )
        })
        .collect()
}

#[test]
fn branches_next_fixtures() {
    // (in, out).
    let cases: &[(Vec<KanaRepresentation>, Vec<KanaRepresentation>)] = &[
        // single branch → two candidates (ambiguous n), no merge (rest lengths differ)
        (
            vec![branches_next_kr("", "", "nippon", 0)],
            vec![
                branches_next_kr("ん", "ん", "ippon", 0),
                branches_next_kr("に", "に", "ppon", 0),
            ],
        ),
        // two branches, equal remaining length → join (head empty, shorter canonical kept)
        (
            vec![
                branches_next_kr("ん", "ん", "ippon", 0),
                branches_next_kr("に", "に", "ppon", 0),
            ],
            vec![branches_next_kr("に", "(んい|に)", "ppon", 6)],
        ),
        // lone branch → branch index reset to pattern length (6→7)
        (
            vec![branches_next_kr("に", "(んい|に)", "ppon", 6)],
            vec![branches_next_kr("にっ", "(んい|に)っ", "pon", 7)],
        ),
        // lone branch, long-vowel continuation, branch reset (→10)
        (
            vec![branches_next_kr("にっ", "(んい|に)っ", "pon", 7)],
            vec![branches_next_kr("にっぽ", "(んい|に)っぽう?", "n", 10)],
        ),
        // single-char tail consumed, branch reset (→11)
        (
            vec![branches_next_kr("にっぽ", "(んい|に)っぽう?", "n", 10)],
            vec![branches_next_kr("にっぽん", "(んい|に)っぽう?ん", "", 11)],
        ),
        // single branch → branch reset on a fresh long-vowel head (0→3)
        (
            vec![branches_next_kr("", "", "konnichiwa", 0)],
            vec![branches_next_kr("こ", "こう?", "nnichiwa", 3)],
        ),
        // single branch → two candidates, rest lengths differ (6 vs 5) → no merge
        (
            vec![branches_next_kr("こん", "こう?ん", "nichiwa", 4)],
            vec![
                branches_next_kr("こんん", "こう?んん", "ichiwa", 4),
                branches_next_kr("こんに", "こう?んに", "chiwa", 4),
            ],
        ),
        // two branches, equal remaining length, non-empty head (branch 4) → join
        (
            vec![
                branches_next_kr("こんん", "こう?んん", "ichiwa", 4),
                branches_next_kr("こんに", "こう?んに", "chiwa", 4),
            ],
            vec![branches_next_kr("こんに", "こう?ん(んい|に)", "chiwa", 10)],
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(
            branches_next_show(&branches_next(input)),
            branches_next_show(expected),
            "in={input:?}"
        );
    }
}

// --- romaji_kana ---
#[test]
fn romaji_kana_fixtures() {
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

// --- romaji_suggest ---
fn ctx() -> Arc<KaniranContext> {
    crate::test_support::shared_ctx()
}

/// Full-JSON suggestions for words whose matched-kanji order is
/// deterministic: `neko`/`tegami`/`toukyou` (single kanji) and `hon` (two
/// kanji). Checks the object shape, the canonical hiragana entry, and the
/// katakana mapping.
#[test]
fn romaji_suggest_fixtures() {
    let ctx = ctx();
    let cases: &[(&str, &str)] = &[
        (
            "neko",
            r#"{"hiragana":["ねこ"],"katakana":["ネコ"],"kanji":["猫"]}"#,
        ),
        (
            "tegami",
            r#"{"hiragana":["てがみ"],"katakana":["テガミ"],"kanji":["手紙"]}"#,
        ),
        (
            "toukyou",
            r#"{"hiragana":["とうきょう"],"katakana":["トウキョウ"],"kanji":["東京"]}"#,
        ),
        (
            "hon",
            r#"{"hiragana":["ほん"],"katakana":["ホン"],"kanji":["本","品"]}"#,
        ),
    ];
    for (s, expected) in cases {
        let js = romaji_suggest(&ctx, s).unwrap().expect("deromanizes");
        assert_eq!(
            serde_json::to_string(&js).unwrap().as_str(),
            *expected,
            "s={s}"
        );
    }
}

/// `inu` deromanizes to canonical いぬ, but its pattern also matches the
/// alternative reading いんう, so hiragana carries both (canonical first)
/// and katakana maps each. Only the hiragana/katakana fields are checked;
/// the matched-kanji order is not deterministic here.
#[test]
fn romaji_suggest_multi_hiragana() {
    let ctx = ctx();
    let js = romaji_suggest(&ctx, "inu")
        
        .unwrap()
        .expect("deromanizes");
    assert_eq!(js["hiragana"], serde_json::json!(["いぬ", "いんう"]));
    assert_eq!(js["katakana"], serde_json::json!(["イヌ", "インウ"]));
}

/// `xyz` does not deromanize, so there is no suggestion.
#[test]
fn romaji_suggest_no_parse() {
    let ctx = ctx();
    assert!(romaji_suggest(&ctx, "xyz").unwrap().is_none());
}
