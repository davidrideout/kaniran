//! Port of `ichiran/dict:errata-conj-rules-hook` (`dict-errata.lisp:1250`).
//!
//! Post-load fixups on the conjugation-rules hash (pos-id → list of
//! `conjugation-rule`): adds adverbial / stem / literary rules for
//! `adj-i` and `adj-ix`, a `v5aru` irregular, patches negative-formal
//! okurigana for `v1`/`v1-s` and the negative-conditional for `v5u`,
//! drops `vs-s` potential forms, and (over every entry) rewrites godan
//! causative-su and adds a negative-stem rule for `v5*`.

use std::collections::HashMap;

use super::conjugation_rule_struct::ConjugationRule;
use super::get_pos::get_pos;
use super::get_pos_index::get_pos_index;

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

#[cfg(test)]
mod tests {
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
