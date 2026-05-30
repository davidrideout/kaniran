//! Port of `ichiran/dict-errata.lisp` — the segmenter's hand-maintained
//! errata tier: ten data globals (skip / particle / kanji-break /
//! conj-form lists) plus four functions (two post-load hooks that fix
//! up the loaded conj-description / conj-rules tables, plus the
//! `skip-by-conj-data` / `test-conj-prop` matcher pair the segmenter
//! drives off the `*skip-conj-forms*` table).
//!
//! Section order mirrors upstream's `dict-errata.lisp:1155-1336`.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::conj_data::ConjData;
use super::dao::ConjProp;
use super::load::ConjugationRule;
use super::load::get_pos;
use super::load::get_pos_index;
use super::kani::{ConjForm, FormToken};

// =========================================================================
// *skip-words* (dict-errata.lisp:1155)
// =========================================================================

/// "Seq of words that aren't really words, like suffixes etc." —
/// `calc-score` (`dict.lisp:855`) returns 0 for any reading whose
/// `seq-set` intersects this list, removing the candidate from
/// segmentation.
pub static SKIP_WORDS: &[i32] = &[
    2822120, // ても良い
    2013800, // ちゃう
    2108590, // とく
    2029040, // ば
    2428180, // い
    2654250, // た
    2561100, // うまいな
    2210270, // ませんか
    2210710, // ましょうか
    2257550, // ない
    2210320, // ません
    2017560, // たい
    2394890, // とる
    2194000, // であ
    2568000, // れる/られる
    2537250, // しようとする
    2760890, // 三箱
    2831062, // てる
    2831063, // てく
    2029030, // ものの
    2568020, // せる
    900000,  // たそう
    2827357, // まう
];

// =========================================================================
// *final-prt* (dict-errata.lisp:1182)
// =========================================================================

/// "Words that only have meaning when they're final" — `calc-score`
/// (`dict.lisp:856`) returns 0 for any reading whose seq is in this
/// list unless the reading is the final segment of the path. Also
/// consumed as the seed for [`semi_final_prt`].
pub static FINAL_PRT: &[i32] = &[
    2017770, // かい
    // 1008450 // では (commented out upstream)
    2425930, // なの
    // 2780660 // もの (commented out upstream)
    2130430, // け / っけ
    2029130, // ぞ
    2834812, // ぜ
    2718360, // がな
    2201380, // わい
    2722170, // のう
    2751630, // かいな
];

// =========================================================================
// *semi-final-prt* (dict-errata.lisp:1196)
// =========================================================================

/// "Particles that are final, but also have other uses" — built as
/// `(append *final-prt* '(2029120 2086640 2029110 2029080 2029100))`.
/// Read by `calc-score` (`dict.lisp:832`) via `(member seq
/// *semi-final-prt*)` and by `dict-grammar.lisp:1005` through
/// `(apply 'filter-in-seq-set *semi-final-prt*)`.
///
/// Derived lazily from [`FINAL_PRT`] at first call (CONVENTIONS §5.2)
/// so it tracks any future change to the source list without a
/// hand-copy.
pub fn semi_final_prt() -> &'static [i32] {
    static CACHE: OnceLock<Vec<i32>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut out: Vec<i32> = FINAL_PRT.to_vec();
            out.extend_from_slice(&[
                2029120, // さ
                2086640, // し
                2029110, // な
                2029080, // ね
                2029100, // わ
            ]);
            out
        })
        .as_slice()
}

// =========================================================================
// *copulae* (dict-errata.lisp:1205)
// =========================================================================

/// JMdict seqs treated as copulae by `calc-score` (`dict.lisp:835`):
/// the segmenter intersects this list against the candidate's
/// `seq-set` to detect a copula `だ` and apply the cop-da scoring
/// branch.
pub static COPULAE: &[i32] = &[
    2089020, // だ
    // 2755350 // じゃない (commented out upstream)
];

// =========================================================================
// *non-final-prt* (dict-errata.lisp:1209)
// =========================================================================

/// "Particles that don't get final bonus" — read by `calc-score`
/// (`dict.lisp:833`) as `(member seq *non-final-prt*)` so the
/// particle does not receive the final-position score bump. The only
/// entry is the sentence-final-but-not-bonus particle `ん` (2139720).
pub static NON_FINAL_PRT: &[i32] = &[
    2139720, // ん
];

// =========================================================================
// *no-kanji-break-penalty* (dict-errata.lisp:1214)
// =========================================================================

/// "Words that get no kanji break penalty" — consulted by
/// `kanji-break-penalty` at `dict.lisp:709` as
/// `(intersection (getf info :seq-set) *no-kanji-break-penalty*)`;
/// when the candidate's `seq-set` intersects this list, the function
/// returns `score` unchanged (skips the kanji-break penalty branch).
pub static NO_KANJI_BREAK_PENALTY: &[i32] = &[
    1169870, // 飲む
    1198360, // 会議
    1277450, // 好き
    2028980, // で
    1423000, // 着る
    1164690, // 一段
    1587040, // 言う
    2827864, // なので
];

// =========================================================================
// *force-kanji-break* (dict-errata.lisp:1226)
// =========================================================================

/// Consulted by `dict.lisp:1103` as
/// `(find part *force-kanji-break* :test 'equal)` — when a candidate
/// substring matches one of these literals, the segmenter forces a
/// kanji break at that position.
pub static FORCE_KANJI_BREAK: &[&str] = &["です"];

// =========================================================================
// *no-kanji-break* (dict-errata.lisp:1229)
// =========================================================================

/// "Words that do not cause kanji break" — consulted by
/// `dict.lisp:1105` as `(find part *no-kanji-break* :test 'equal)`;
/// when a candidate substring matches one of these literals, the
/// segmenter suppresses the kanji break that would otherwise apply.
pub static NO_KANJI_BREAK: &[&str] = &["日置"];

// =========================================================================
// *skip-conj-forms* (dict-errata.lisp:1310)
// =========================================================================

/// Conjugation forms whose hits the segmenter must drop. Used by
/// [`test_conj_prop`] (in concert with the `*skip-conj-forms*`-driven
/// [`skip_by_conj_data`] pass) to filter out variants ichiran's
/// heuristics never want to score:
///
/// | Form | Triple/Quadruple | Meaning |
/// |---|---|---|
/// | `(10 t :any)` | Triple | conj-type 10 (potential past), neg=`T`, any fml |
/// | `(3 t t)` | Triple | conj-type 3 (negative), neg=`T`, fml=`T` |
/// | `("vs-s" 5 :any :any)` | Quadruple | pos `vs-s`, conj-type 5, any neg, any fml |
pub static SKIP_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(10), FormToken::Bool(true), FormToken::Any),
    ConjForm::Triple(FormToken::Int(3),  FormToken::Bool(true), FormToken::Bool(true)),
    ConjForm::Quadruple(
        FormToken::Str("vs-s"),
        FormToken::Int(5),
        FormToken::Any,
        FormToken::Any,
    ),
];

// =========================================================================
// *weak-conj-forms* (dict-errata.lisp:1316)
// =========================================================================

/// Conjugation forms whose hits the segmenter scores down rather than
/// drops outright (the "weak" tier). All but the last reference one of
/// the conjugation-type defconstants in `dict-errata.lisp:1236-1240`:
///
/// | Constant | Value | Form |
/// |---|---:|---|
/// | `+conj-adverbial+` | 50 | (not in this list) |
/// | `+conj-adjective-stem+` | 51 | first triple |
/// | `+conj-negative-stem+` | 52 | second |
/// | `+conj-causative-su+` | 53 | third |
/// | `+conj-adjective-literary+` | 54 | fourth |
///
/// The integer literals are inlined here because `defconstant` forms
/// aren't picked up by the introspector (no `_global.md` entry, no
/// `symbols.csv` row), so there's no upstream symbol to depend on.
/// Values are pinned by the populated `conj_type` column on every
/// `conj_prop` row in the live database — they cannot drift without
/// a wholesale dictionary rebuild.
pub static WEAK_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(51), FormToken::Any,         FormToken::Any), // +conj-adjective-stem+
    ConjForm::Triple(FormToken::Int(52), FormToken::Any,         FormToken::Any), // +conj-negative-stem+
    ConjForm::Triple(FormToken::Int(53), FormToken::Any,         FormToken::Any), // +conj-causative-su+
    ConjForm::Triple(FormToken::Int(54), FormToken::Any,         FormToken::Any), // +conj-adjective-literary+
    ConjForm::Triple(FormToken::Int(9),  FormToken::Bool(true),  FormToken::Any),
];

// =========================================================================
// errata-conj-description-hook (dict-errata.lisp:1242)
// =========================================================================

/// Adds the five ichiran-internal conjugation types
/// (`+conj-adverbial+`=50 … `+conj-adjective-literary+`=54) to the
/// conj-id → description map after it is loaded from conj.csv. Those
/// defconstants aren't captured by the introspector; their integer
/// values are inlined, matching [`WEAK_CONJ_FORMS`].
///
/// Upstream returns the last `setf` value; its only caller
/// (`load-conj-description`) discards it, so the port is a mutating
/// hook returning `()`.
pub fn errata_conj_description_hook(hash: &mut HashMap<i32, String>) {
    hash.insert(50, "Adverbial".to_string()); // +conj-adverbial+
    hash.insert(51, "Adjective Stem".to_string()); // +conj-adjective-stem+
    hash.insert(52, "Negative Stem".to_string()); // +conj-negative-stem+
    hash.insert(53, "Causative (~su)".to_string()); // +conj-causative-su+
    hash.insert(54, "Old/literary form".to_string()); // +conj-adjective-literary+
}

// =========================================================================
// errata-conj-rules-hook (dict-errata.lisp:1250)
// =========================================================================

/// Post-load fixups on the conjugation-rules hash (pos-id → list of
/// `conjugation-rule`) built by the `*conj-rules*` loader: adds
/// adverbial / stem / literary rules for `adj-i` and `adj-ix`, a
/// `v5aru` irregular, patches negative-formal okurigana for `v1`/`v1-s`
/// and the negative-conditional for `v5u`, drops `vs-s` potential
/// forms, and (over every entry) rewrites godan causative-su and adds
/// a negative-stem rule for `v5*`. Mutates `hash` in place; returns nil.
///
/// The `+conj-*+` ids (50–54) are inlined as integer literals — the
/// upstream `defconstant`s aren't introspected (no symbol to depend
/// on), matching [`WEAK_CONJ_FORMS`].
pub fn errata_conj_rules_hook(hash: &mut HashMap<i32, Vec<ConjugationRule>>) {
    // dict-errata.lisp:1251 — adj-i: adverbial / adjective-stem / literary
    let pos = get_pos_index("adj-i").expect("adj-i in *pos-index*");
    let rules = [
        ConjugationRule { pos, conj: 50, neg: false, fml: false, onum: 1, stem: 1, okuri: "く".to_string(), euphr: String::new(), euphk: String::new() },
        ConjugationRule { pos, conj: 51, neg: false, fml: false, onum: 1, stem: 1, okuri: String::new(), euphr: String::new(), euphk: String::new() },
        ConjugationRule { pos, conj: 54, neg: false, fml: false, onum: 1, stem: 1, okuri: "き".to_string(), euphr: String::new(), euphk: String::new() },
    ];
    for rule in rules {
        hash.entry(pos).or_default().insert(0, rule);
    }

    // dict-errata.lisp:1261 — adj-ix: same as adj-i with euphr "よ"
    let pos = get_pos_index("adj-ix").expect("adj-ix in *pos-index*");
    let rules = [
        ConjugationRule { pos, conj: 50, neg: false, fml: false, onum: 1, stem: 1, okuri: "く".to_string(), euphr: "よ".to_string(), euphk: String::new() },
        ConjugationRule { pos, conj: 51, neg: false, fml: false, onum: 1, stem: 1, okuri: String::new(), euphr: "よ".to_string(), euphk: String::new() },
        ConjugationRule { pos, conj: 54, neg: false, fml: false, onum: 1, stem: 1, okuri: "き".to_string(), euphr: "よ".to_string(), euphk: String::new() },
    ];
    for rule in rules {
        hash.entry(pos).or_default().insert(0, rule);
    }

    // dict-errata.lisp:1271 — v5aru irregular
    let pos = get_pos_index("v5aru").expect("v5aru in *pos-index*");
    hash.entry(pos).or_default().insert(
        0,
        ConjugationRule { pos, conj: 3, neg: false, fml: false, onum: 2, stem: 1, okuri: "り".to_string(), euphr: String::new(), euphk: String::new() },
    );

    // dict-errata.lisp:1276 — fix non-past negative formal for v1 v1-s
    let posi = ["v1", "v1-s"].map(|tag| get_pos_index(tag).expect("v1/v1-s in *pos-index*"));
    for pos in posi {
        if let Some(rules) = hash.get_mut(&pos) {
            for rule in rules.iter_mut() {
                if rule.conj == 1 && rule.fml && rule.neg {
                    rule.okuri = "ません".to_string();
                }
            }
        }
    }

    // dict-errata.lisp:1282 — fix incorrect negative conditional of v5u
    let pos = get_pos_index("v5u").expect("v5u in *pos-index*");
    if let Some(rules) = hash.get_mut(&pos) {
        for rule in rules.iter_mut() {
            if rule.conj == 11 && !rule.fml && rule.neg {
                rule.okuri = "わなかったら".to_string();
            }
        }
    }

    // dict-errata.lisp:1287 — remove potential forms of vs-s
    let pos = get_pos_index("vs-s").expect("vs-s in *pos-index*");
    hash.entry(pos).or_default().retain(|r| r.conj != 5);

    // dict-errata.lisp:1290 (maphash) — add conj-negative-stem for godan verbs
    for (key, val) in hash.iter_mut() {
        let pos = get_pos(*key);
        // dict-errata.lisp:1294 — conj 7 / onum 2 → causative-su, onum 1
        for r in val.iter_mut() {
            if r.conj == 7 && r.onum == 2 {
                r.conj = 53;
                r.onum = 1;
            }
        }
        // dict-errata.lisp:1298 (alexandria:starts-with-subseq "v5" pos)
        if pos.is_some_and(|p| p.starts_with("v5")) {
            // dict-errata.lisp:1299 — first non-formal negative (conj 1) rule
            if let Some(mut new_rule) = val.iter().find(|r| r.conj == 1 && r.neg && !r.fml).cloned() {
                let len = new_rule.okuri.chars().count();
                if len > 2 {
                    new_rule.conj = 52;
                    new_rule.okuri = new_rule.okuri.chars().take(len - 2).collect();
                    val.insert(0, new_rule);
                }
            }
        }
    }
}

// =========================================================================
// skip-by-conj-data + test-conj-prop (dict-errata.lisp:1336)
// =========================================================================

/// True iff `conj_data` is non-empty AND every prop matches
/// [`SKIP_CONJ_FORMS`]. Empty list → false (mirrors Lisp `(and nil _)`).
///
/// `cd.prop = None` is treated as "doesn't match" (degrades to false
/// overall) — upstream's `make-conj-data` always supplies `:prop`, so
/// this branch is defensive only.
pub fn skip_by_conj_data(conj_data: &[ConjData]) -> bool {
    !conj_data.is_empty() && conj_data.iter().all(skip_matches)
}

fn skip_matches(cd: &ConjData) -> bool {
    cd.prop
        .as_ref()
        .map(|prop| test_conj_prop(prop, SKIP_CONJ_FORMS))
        .unwrap_or(false)
}

/// Predicate: does [`ConjProp`] match any element of `forms`? The
/// Lisp builds `(pos conj-type conj-neg conj-fml)` and runs `some`
/// over `forms`, dispatching on form length:
///
/// * length 3 → match `(conj-type neg fml)` against the prop's last
///   three slots.
/// * length 4 → match `(pos conj-type neg fml)` against all four.
///
/// A cell `:any` always matches; otherwise compare with `EQL`.
/// The Rust port models the closed cell vocabulary as
/// [`super::kani::FormToken`] and dispatches on the
/// [`super::kani::ConjForm`] variant.
pub fn test_conj_prop(prop: &ConjProp, forms: &[ConjForm]) -> bool {
    forms.iter().any(|form| match form {
        ConjForm::Triple(ct, neg, fml) => {
            match_conj_type(*ct, prop.conj_type)
                && match_bool(*neg, prop.neg)
                && match_bool(*fml, prop.fml)
        }
        ConjForm::Quadruple(pos, ct, neg, fml) => {
            match_pos(*pos, &prop.pos)
                && match_conj_type(*ct, prop.conj_type)
                && match_bool(*neg, prop.neg)
                && match_bool(*fml, prop.fml)
        }
    })
}

fn match_conj_type(token: FormToken, value: i32) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Int(n) => n == value,
        _ => false,
    }
}

fn match_pos(token: FormToken, value: &str) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Str(s) => s == value,
        _ => false,
    }
}

fn match_bool(token: FormToken, value: Option<bool>) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Bool(b) => value == Some(b),
        FormToken::DbNull => value.is_none(),
        _ => false,
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod errata_conj_rules_hook_tests {
        use super::*;

        type RuleTuple = (i32, i32, bool, bool, i32, i32, String, String, String);

        fn mk(pos: i32, conj: i32, neg: bool, fml: bool, onum: i32, stem: i32, okuri: &str) -> ConjugationRule {
            ConjugationRule { pos, conj, neg, fml, onum, stem, okuri: okuri.to_string(), euphr: String::new(), euphk: String::new() }
        }

        fn tup(r: &ConjugationRule) -> RuleTuple {
            (r.pos, r.conj, r.neg, r.fml, r.onum, r.stem, r.okuri.clone(), r.euphr.clone(), r.euphk.clone())
        }

        fn rule(pos: i32, conj: i32, neg: bool, fml: bool, onum: i32, stem: i32, okuri: &str, euphr: &str, euphk: &str) -> RuleTuple {
            (pos, conj, neg, fml, onum, stem, okuri.to_string(), euphr.to_string(), euphk.to_string())
        }

        /// Synthetic conj-rules hash (real pos-ids from `get-pos-index`)
        /// driven through `ichiran/dict::errata-conj-rules-hook` on .103,
        /// 2026-05-24. Each pos exercises a distinct section: adj-i (1) /
        /// adj-ix (7) prepend rules into a fresh entry; v5aru (30) gets the
        /// irregular then a v5 negative-stem; v1 (28) patches the
        /// formal-negative okuri and leaves the non-formal row intact; v5u
        /// (41) patches the negative-conditional, converts causative-su, and
        /// adds the negative-stem; vs-s (47) drops the conj-5 row; v5r (37)
        /// gets the negative-stem and leaves a conj-7/onum-1 row untouched;
        /// n (17) shows causative-su conversion applies to every entry, not
        /// just v5.
        #[test]
        fn errata_fixups() {
            let mut hash: HashMap<i32, Vec<ConjugationRule>> = HashMap::new();
            hash.insert(30, vec![mk(30, 1, true, false, 1, 2, "らない")]);
            hash.insert(28, vec![mk(28, 1, true, true, 1, 2, "OLD"), mk(28, 1, true, false, 1, 2, "KEEP")]);
            hash.insert(41, vec![mk(41, 11, true, false, 1, 2, "OLD11"), mk(41, 1, true, false, 1, 2, "わない"), mk(41, 7, false, false, 2, 2, "CAUS")]);
            hash.insert(47, vec![mk(47, 5, false, false, 1, 2, "REMOVE"), mk(47, 2, false, false, 1, 2, "KEEP")]);
            hash.insert(37, vec![mk(37, 1, true, false, 1, 2, "らない"), mk(37, 7, false, false, 1, 2, "NOCHANGE7")]);
            hash.insert(17, vec![mk(17, 7, false, false, 2, 2, "NCAUS")]);

            errata_conj_rules_hook(&mut hash);

            let cases: &[(i32, Vec<RuleTuple>)] = &[
                (1, vec![
                    rule(1, 54, false, false, 1, 1, "き", "", ""),
                    rule(1, 51, false, false, 1, 1, "", "", ""),
                    rule(1, 50, false, false, 1, 1, "く", "", ""),
                ]),
                (7, vec![
                    rule(7, 54, false, false, 1, 1, "き", "よ", ""),
                    rule(7, 51, false, false, 1, 1, "", "よ", ""),
                    rule(7, 50, false, false, 1, 1, "く", "よ", ""),
                ]),
                (30, vec![
                    rule(30, 52, true, false, 1, 2, "ら", "", ""),
                    rule(30, 3, false, false, 2, 1, "り", "", ""),
                    rule(30, 1, true, false, 1, 2, "らない", "", ""),
                ]),
                (28, vec![
                    rule(28, 1, true, true, 1, 2, "ません", "", ""),
                    rule(28, 1, true, false, 1, 2, "KEEP", "", ""),
                ]),
                (41, vec![
                    rule(41, 52, true, false, 1, 2, "わ", "", ""),
                    rule(41, 11, true, false, 1, 2, "わなかったら", "", ""),
                    rule(41, 1, true, false, 1, 2, "わない", "", ""),
                    rule(41, 53, false, false, 1, 2, "CAUS", "", ""),
                ]),
                (47, vec![rule(47, 2, false, false, 1, 2, "KEEP", "", "")]),
                (37, vec![
                    rule(37, 52, true, false, 1, 2, "ら", "", ""),
                    rule(37, 1, true, false, 1, 2, "らない", "", ""),
                    rule(37, 7, false, false, 1, 2, "NOCHANGE7", "", ""),
                ]),
                (17, vec![rule(17, 53, false, false, 1, 2, "NCAUS", "", "")]),
            ];
            for (key, expected) in cases {
                let actual: Vec<RuleTuple> = hash.get(key).expect("key present").iter().map(tup).collect();
                assert_eq!(&actual, expected, "key={key}");
            }
        }
    }

    mod test_conj_prop_tests {
        use super::*;

        fn prop(pos: &str, conj_type: i32, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
            ConjProp { id: 0, conj_id: 0, conj_type, pos: pos.to_string(), neg, fml }
        }

        #[test]
        fn triple_matches_on_conj_type_and_any_wildcards() {
            let p = prop("v1", 51, Some(true), None);
            let forms = [ConjForm::Triple(FormToken::Int(51), FormToken::Any, FormToken::Any)];
            assert!(test_conj_prop(&p, &forms));
        }

        #[test]
        fn triple_rejects_when_conj_type_differs() {
            let p = prop("v1", 51, None, None);
            let forms = [ConjForm::Triple(FormToken::Int(50), FormToken::Any, FormToken::Any)];
            assert!(!test_conj_prop(&p, &forms));
        }

        #[test]
        fn bool_token_distinguishes_true_false_and_dbnull() {
            let true_prop = prop("v5t", 2, Some(true), None);
            let false_prop = prop("v5t", 2, Some(false), None);
            let null_prop = prop("v5t", 2, None, None);

            let want_true = [ConjForm::Triple(FormToken::Int(2), FormToken::Bool(true), FormToken::Any)];
            assert!(test_conj_prop(&true_prop, &want_true));
            assert!(!test_conj_prop(&false_prop, &want_true));
            assert!(!test_conj_prop(&null_prop, &want_true));

            let want_dbnull = [ConjForm::Triple(FormToken::Int(2), FormToken::DbNull, FormToken::Any)];
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
                ConjForm::Triple(FormToken::Int(13), FormToken::Bool(true), FormToken::Bool(false)), // hit
            ];
            assert!(test_conj_prop(&p, &forms));
        }
    }
}
