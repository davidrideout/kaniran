use super::*;

// --- root_diff ---
#[test]
fn root_diff_fixtures() {
    let cases: &[(&str, &str, (usize, usize))] = &[
        ("食べる", "食べる", (0, 0)),
        ("尋ねる", "たずねる", (1, 2)),
        ("ね", "たずね", (0, 2)),
        ("ABCDE", "XYZ", (5, 3)),
        ("abc", "", (3, 0)),
        ("", "abc", (0, 3)),
        ("食べた", "たべた", (1, 1)),
        ("思う", "おもう", (1, 2)),
        ("学校", "がっこう", (2, 4)),
        ("本", "ほん", (1, 2)),
        ("言葉", "ことば", (2, 3)),
    ];
    for (base, reading, expected) in cases {
        assert_eq!(
            root_diff(base, reading),
            *expected,
            "base={base:?} reading={reading:?}",
        );
    }
}

// --- root_diff_fn ---
#[test]
fn root_diff_fn_fixtures() {
    let f1 = root_diff_fn("尋ねる", "たずねる");
    assert_eq!(f1("訪ねる"), "たずねる");
    assert_eq!(f1("尋ねた"), "たずねた");
    let f2 = root_diff_fn("食べる", "たべる");
    assert_eq!(f2("食べた"), "たべた");
    let f3 = root_diff_fn("猫", "猫");
    assert_eq!(f3("犬"), "犬");
    let f4 = root_diff_fn("ね", "たずね");
    assert_eq!(f4("ね"), "たずね");
    assert_eq!(f4("ねた"), "たずねた");
}

// --- add_deha_ja_readings ---
/// Rewrites a leading では to じゃ, including the exactly-two-char では.
#[cfg(feature = "loaders")]
#[test]
fn rewrite_deha_to_ja_cases() {
    let cases: &[(&str, &str)] = &[
        ("ではない", "じゃない"),
        ("ではなかった", "じゃなかった"),
        ("ではありませんでした", "じゃありませんでした"),
        ("ではないで", "じゃないで"),
        ("ではなくて", "じゃなくて"),
        ("ではなかったら", "じゃなかったら"),
        ("ではありませんでしたら", "じゃありませんでしたら"),
        ("ではありません", "じゃありません"),
        ("ではないです", "じゃないです"),
        ("では", "じゃ"),
    ];
    for (input, expected) in cases {
        assert_eq!(&rewrite_deha_to_ja(input), expected, "input={input}");
    }
}

// --- errata_conj_rules_hook ---
type RuleTuple = (i32, i32, bool, bool, i32, i32, String, String, String);

fn mk(
    pos: i32,
    conj: i32,
    neg: bool,
    fml: bool,
    onum: i32,
    stem: i32,
    okuri: &str,
) -> ConjugationRule {
    ConjugationRule {
        pos,
        conj,
        neg,
        fml,
        onum,
        stem,
        okuri: okuri.to_string(),
        euphr: String::new(),
        euphk: String::new(),
    }
}

fn tup(r: &ConjugationRule) -> RuleTuple {
    (
        r.pos,
        r.conj,
        r.neg,
        r.fml,
        r.onum,
        r.stem,
        r.okuri.clone(),
        r.euphr.clone(),
        r.euphk.clone(),
    )
}

fn rule(
    pos: i32,
    conj: i32,
    neg: bool,
    fml: bool,
    onum: i32,
    stem: i32,
    okuri: &str,
    euphr: &str,
    euphk: &str,
) -> RuleTuple {
    (
        pos,
        conj,
        neg,
        fml,
        onum,
        stem,
        okuri.to_string(),
        euphr.to_string(),
        euphk.to_string(),
    )
}

/// The errata hook applies a distinct fixup per part-of-speech:
/// adj-i (1) / adj-ix (7) prepend rules into a fresh entry; v5aru
/// (30) gets the irregular then a v5 negative-stem; v1 (28) patches
/// the formal-negative okurigana and leaves the non-formal row intact;
/// v5u (41) patches the negative-conditional, converts causative-su,
/// and adds the negative-stem; vs-s (47) drops the conj-5 row; v5r
/// (37) gets the negative-stem and leaves a conj-7/onum-1 row
/// untouched; n (17) shows causative-su conversion applies to every
/// part-of-speech, not just verbs.
#[test]
fn errata_fixups() {
    let mut hash: HashMap<i32, Vec<ConjugationRule>> = HashMap::new();
    hash.insert(30, vec![mk(30, 1, true, false, 1, 2, "らない")]);
    hash.insert(
        28,
        vec![
            mk(28, 1, true, true, 1, 2, "OLD"),
            mk(28, 1, true, false, 1, 2, "KEEP"),
        ],
    );
    hash.insert(
        41,
        vec![
            mk(41, 11, true, false, 1, 2, "OLD11"),
            mk(41, 1, true, false, 1, 2, "わない"),
            mk(41, 7, false, false, 2, 2, "CAUS"),
        ],
    );
    hash.insert(
        47,
        vec![
            mk(47, 5, false, false, 1, 2, "REMOVE"),
            mk(47, 2, false, false, 1, 2, "KEEP"),
        ],
    );
    hash.insert(
        37,
        vec![
            mk(37, 1, true, false, 1, 2, "らない"),
            mk(37, 7, false, false, 1, 2, "NOCHANGE7"),
        ],
    );
    hash.insert(17, vec![mk(17, 7, false, false, 2, 2, "NCAUS")]);

    errata_conj_rules_hook(&mut hash);

    let cases: &[(i32, Vec<RuleTuple>)] = &[
        (
            1,
            vec![
                rule(1, 54, false, false, 1, 1, "き", "", ""),
                rule(1, 51, false, false, 1, 1, "", "", ""),
                rule(1, 50, false, false, 1, 1, "く", "", ""),
            ],
        ),
        (
            7,
            vec![
                rule(7, 54, false, false, 1, 1, "き", "よ", ""),
                rule(7, 51, false, false, 1, 1, "", "よ", ""),
                rule(7, 50, false, false, 1, 1, "く", "よ", ""),
            ],
        ),
        (
            30,
            vec![
                rule(30, 52, true, false, 1, 2, "ら", "", ""),
                rule(30, 3, false, false, 2, 1, "り", "", ""),
                rule(30, 1, true, false, 1, 2, "らない", "", ""),
            ],
        ),
        (
            28,
            vec![
                rule(28, 1, true, true, 1, 2, "ません", "", ""),
                rule(28, 1, true, false, 1, 2, "KEEP", "", ""),
            ],
        ),
        (
            41,
            vec![
                rule(41, 52, true, false, 1, 2, "わ", "", ""),
                rule(41, 11, true, false, 1, 2, "わなかったら", "", ""),
                rule(41, 1, true, false, 1, 2, "わない", "", ""),
                rule(41, 53, false, false, 1, 2, "CAUS", "", ""),
            ],
        ),
        (47, vec![rule(47, 2, false, false, 1, 2, "KEEP", "", "")]),
        (
            37,
            vec![
                rule(37, 52, true, false, 1, 2, "ら", "", ""),
                rule(37, 1, true, false, 1, 2, "らない", "", ""),
                rule(37, 7, false, false, 1, 2, "NOCHANGE7", "", ""),
            ],
        ),
        (17, vec![rule(17, 53, false, false, 1, 2, "NCAUS", "", "")]),
    ];
    for (key, expected) in cases {
        let actual: Vec<RuleTuple> = hash
            .get(key)
            .expect("key present")
            .iter()
            .map(tup)
            .collect();
        assert_eq!(&actual, expected, "key={key}");
    }
}

// --- test_conj_prop ---
fn prop(pos: &str, conj_type: i32, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
    ConjProp {
        id: 0,
        conj_id: 0,
        conj_type,
        pos: pos.to_string(),
        neg,
        fml,
    }
}

#[test]
fn triple_matches_on_conj_type_and_any_wildcards() {
    let p = prop("v1", 51, Some(true), None);
    let forms = [ConjForm::Triple(
        FormToken::Int(51),
        FormToken::Any,
        FormToken::Any,
    )];
    assert!(test_conj_prop(&p, &forms));
}

#[test]
fn triple_rejects_when_conj_type_differs() {
    let p = prop("v1", 51, None, None);
    let forms = [ConjForm::Triple(
        FormToken::Int(50),
        FormToken::Any,
        FormToken::Any,
    )];
    assert!(!test_conj_prop(&p, &forms));
}

#[test]
fn bool_token_distinguishes_true_false_and_dbnull() {
    let true_prop = prop("v5t", 2, Some(true), None);
    let false_prop = prop("v5t", 2, Some(false), None);
    let null_prop = prop("v5t", 2, None, None);

    let want_true = [ConjForm::Triple(
        FormToken::Int(2),
        FormToken::Bool(true),
        FormToken::Any,
    )];
    assert!(test_conj_prop(&true_prop, &want_true));
    assert!(!test_conj_prop(&false_prop, &want_true));
    assert!(!test_conj_prop(&null_prop, &want_true));

    let want_dbnull = [ConjForm::Triple(
        FormToken::Int(2),
        FormToken::DbNull,
        FormToken::Any,
    )];
    assert!(!test_conj_prop(&true_prop, &want_dbnull));
    assert!(test_conj_prop(&null_prop, &want_dbnull));
}

#[test]
fn quadruple_requires_pos_match_in_addition_to_triple() {
    let p = prop("vs-s", 5, None, None);
    let matching = [ConjForm::Quadruple(
        FormToken::Str("vs-s"),
        FormToken::Int(5),
        FormToken::Any,
        FormToken::Any,
    )];
    let wrong_pos = [ConjForm::Quadruple(
        FormToken::Str("v5t"),
        FormToken::Int(5),
        FormToken::Any,
        FormToken::Any,
    )];
    assert!(test_conj_prop(&p, &matching));
    assert!(!test_conj_prop(&p, &wrong_pos));
}

#[test]
fn any_in_one_form_in_a_list_is_enough() {
    let p = prop("v1", 13, Some(true), Some(false));
    let forms = [
        ConjForm::Triple(FormToken::Int(99), FormToken::Any, FormToken::Any), // miss
        ConjForm::Triple(
            FormToken::Int(13),
            FormToken::Bool(true),
            FormToken::Bool(false),
        ), // hit
    ];
    assert!(test_conj_prop(&p, &forms));
}
