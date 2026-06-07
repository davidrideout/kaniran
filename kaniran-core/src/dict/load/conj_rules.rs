use crate::dict::errata::{errata_conj_description_hook, errata_conj_rules_hook};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `ichiran/dict:*conj-description*` (`dict-load.lisp:257`, `csv-hash *conj-description*`).
///
/// conj-id → English conjugation-type description.
pub fn conj_description() -> &'static HashMap<i32, String> {
    static CONJ_DESCRIPTION: OnceLock<HashMap<i32, String>> = OnceLock::new();
    CONJ_DESCRIPTION.get_or_init(load_conj_description)
}

/// Port of `ichiran/dict:*conj-rules*` (`dict-load.lisp:268`, `csv-hash *conj-rules*`).
///
/// pos-id → list of `ConjugationRule`, stored in cons-prepend order
/// (the reader reverses on read).
pub fn conj_rules() -> &'static HashMap<i32, Vec<ConjugationRule>> {
    static CONJ_RULES: OnceLock<HashMap<i32, Vec<ConjugationRule>>> = OnceLock::new();
    CONJ_RULES.get_or_init(load_conj_rules)
}

/// Port of `ichiran/dict:get-conj-description` (`dict-load.lisp:257`, `csv-hash *conj-description*` accessor).
///
/// Looks up a conj-id's description in [`conj_description`]; `None` when
/// the id is absent (upstream `gethash` → `nil`).
pub fn get_conj_description(key: i32) -> Option<&'static str> {
    conj_description().get(&key).map(String::as_str)
}

/// Port of `ichiran/dict:load-conj-description` (`dict-load.lisp:257`, `csv-hash *conj-description*` expansion).
///
/// Builds the conj-id → description map by parsing the tab-separated
/// conj.csv (header skipped) and applying
/// [`errata_conj_description_hook`].
const CONJ_CSV: &str = include_str!("../../../data/conj.csv");

pub fn load_conj_description() -> HashMap<i32, String> {
    let mut hash = HashMap::new();
    for row in CONJ_CSV.lines().skip(1) {
        let mut cols = row.split('\t');
        let conj_id = cols
            .next()
            .expect("conj.csv row missing id column")
            .parse::<i32>()
            .expect("conj.csv id column not an integer");
        let description = cols.next().expect("conj.csv row missing name column");
        hash.insert(conj_id, description.to_string());
    }
    errata_conj_description_hook(&mut hash);
    hash
}

/// Port of `ichiran/dict:conjugation-rule` (`dict-load.lisp:262`).
///
/// In-memory record carrying one row of `conjo.csv` — the conjugation
/// rules table mapping a (part-of-speech, conjugation-id, neg, fml,
/// onum) key to the stem index and three okurigana / euphonic
/// fragments (`okuri`, `euphr`, `euphk`) used to assemble the
/// conjugated form.
#[derive(Debug, Clone)]
pub struct ConjugationRule {
    pub pos: i32,
    pub conj: i32,
    pub neg: bool,
    pub fml: bool,
    pub onum: i32,
    pub stem: i32,
    pub okuri: String,
    pub euphr: String,
    pub euphk: String,
}

/// Port of `ichiran/dict:get-conj-rules` (`dict-load.lisp:268`, `csv-hash *conj-rules*` accessor).
///
/// Look up the conjugation rules for a numeric pos id and return them
/// in CSV order. Missing keys produce an empty list.
pub fn get_conj_rules(key: i32) -> Vec<ConjugationRule> {
    // dict-load.lisp:281 — (val (reverse val))
    conj_rules()
        .get(&key)
        .map(|val| val.iter().rev().cloned().collect())
        .unwrap_or_default()
}

/// Port of `ichiran/dict:load-conj-rules` (`dict-load.lisp:268`, `csv-hash *conj-rules*` expansion).
///
/// Builds the pos-id → list-of-`ConjugationRule` map by parsing the
/// tab-separated conjo.csv (header skipped) and applying
/// [`errata_conj_rules_hook`]. Per-row values are prepended into the
/// per-pos list, so [`crate::dict::load::conj_rules::get_conj_rules`] reverses
/// on read.
const CONJO_CSV: &str = include_str!("../../../data/conjo.csv");

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

/// Port of `ichiran/dict:*do-not-conjugate*` (`dict-load.lisp:303`).
pub static DO_NOT_CONJUGATE: &[&str] = &["n", "vs", "adj-na"];

/// Port of `ichiran/dict:*do-not-conjugate-seq*` (`dict-load.lisp:305`).
pub static DO_NOT_CONJUGATE_SEQ: &[i32] = &[2765070, 2835284];

/// Port of `ichiran/dict:*pos-with-conj-rules*` (`dict-load.lisp:307`).
pub static POS_WITH_CONJ_RULES: &[&str] = &[
    "adj-i", "adj-ix", "cop-da", "v1", "v1-s", "v5aru", "v5b", "v5g", "v5k", "v5k-s", "v5m", "v5n",
    "v5r", "v5r-i", "v5s", "v5t", "v5u", "v5u-s", "vk", "vs-s", "vs-i",
];

/// Port of `ichiran/dict:*secondary-conjugation-types-from*` (`dict-load.lisp:312`).
///
/// Source conjugation-type ids a secondary conjugation may derive from;
/// the trailing `53` is `+conj-causative-su+`.
pub static SECONDARY_CONJUGATION_TYPES_FROM: &[i32] = &[5, 6, 7, 8, 53];

/// Port of `ichiran/dict:*secondary-conjugation-types*` (`dict-load.lisp:314`).
pub static SECONDARY_CONJUGATION_TYPES: &[i32] = &[2, 3, 4, 9, 10, 11, 12, 13];

#[cfg(test)]
mod tests;
