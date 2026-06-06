//! Port of `ichiran/dict:load-conj-rules` (`dict-load.lisp:268`, `csv-hash *conj-rules*` expansion).
//!
//! Builds the pos-id → list-of-`ConjugationRule` map by parsing the
//! tab-separated conjo.csv (header skipped) and applying
//! [`errata_conj_rules_hook`]. Per-row values are prepended into the
//! per-pos list, so [`super::get_conj_rules::get_conj_rules`] reverses
//! on read.

use std::collections::HashMap;

use super::conjugation_rule_struct::ConjugationRule;
use super::errata_conj_rules_hook::errata_conj_rules_hook;

const CONJO_CSV: &str = include_str!("../../data/conjo.csv");

pub fn load_conj_rules() -> HashMap<i32, Vec<ConjugationRule>> {
    let mut hash: HashMap<i32, Vec<ConjugationRule>> = HashMap::new();
    for row in CONJO_CSV.lines().skip(1) {
        let mut cols = row.split('\t');
        let pos_id_str = cols.next().expect("conjo.csv row missing pos column");
        let conj_id_str = cols.next().expect("conjo.csv row missing conj column");
        let neg_str = cols.next().expect("conjo.csv row missing neg column");
        let fml_str = cols.next().expect("conjo.csv row missing fml column");
        let onum_str = cols.next().expect("conjo.csv row missing onum column");
        let stem_str = cols.next().expect("conjo.csv row missing stem column");
        let okuri = cols.next().expect("conjo.csv row missing okuri column");
        let euphr = cols.next().expect("conjo.csv row missing euphr column");
        let euphk = cols.next().expect("conjo.csv row missing euphk column");
        // pos2 column dropped — matches `(pos-id conj-id neg fml onum stem okuri euphr euphk pos2)`
        // row-def binding at dict-load.lisp:270 where `pos2` is unused in the value-form.
        let pos = pos_id_str
            .parse::<i32>()
            .expect("conjo.csv pos column not an integer");
        // dict-load.lisp:275-276 (case (char neg 0) (#\t t) (#\f nil))
        let neg = match neg_str.chars().next() {
            Some('t') => true,
            Some('f') => false,
            _ => panic!("conjo.csv neg column not 't' or 'f': {neg_str:?}"),
        };
        let fml = match fml_str.chars().next() {
            Some('t') => true,
            Some('f') => false,
            _ => panic!("conjo.csv fml column not 't' or 'f': {fml_str:?}"),
        };
        let rule = ConjugationRule {
            pos,
            conj: conj_id_str
                .parse::<i32>()
                .expect("conjo.csv conj column not an integer"),
            neg,
            fml,
            onum: onum_str
                .parse::<i32>()
                .expect("conjo.csv onum column not an integer"),
            stem: stem_str
                .parse::<i32>()
                .expect("conjo.csv stem column not an integer"),
            okuri: unquote_csv(okuri),
            euphr: unquote_csv(euphr),
            euphk: unquote_csv(euphk),
        };
        // dict-load.lisp:280 — value-form (cons new-rule (gethash pos *conj-rules* nil))
        hash.entry(pos).or_default().insert(0, rule);
    }
    errata_conj_rules_hook(&mut hash);
    hash
}

/// Minimal CSV unquoter for the fields conjo.csv quotes — `""` is a
/// quoted empty string (6 rows in conjo.csv use this for `okuri`).
/// Strip the outer quotes and collapse the doubled-quote escape, the
/// same way `cl-csv:read-csv` does at dict-load.lisp:222.
fn unquote_csv(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].replace("\"\"", "\"")
    } else {
        s.to_string()
    }
}
