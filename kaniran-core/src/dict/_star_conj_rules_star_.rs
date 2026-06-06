//! Port of `ichiran/dict:*conj-rules*` (`dict-load.lisp:268`, `csv-hash *conj-rules*`).
//!
//! pos-id → list of `ConjugationRule`, stored in cons-prepend order
//! (the reader reverses on read).

use std::collections::HashMap;
use std::sync::OnceLock;

use super::conjugation_rule_struct::ConjugationRule;
use super::load_conj_rules::load_conj_rules;

static CONJ_RULES: OnceLock<HashMap<i32, Vec<ConjugationRule>>> = OnceLock::new();

pub fn conj_rules() -> &'static HashMap<i32, Vec<ConjugationRule>> {
    CONJ_RULES.get_or_init(load_conj_rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL probe (after `(load-conj-rules)`): `(hash-table-count
    /// *conj-rules*)` = 24 (one entry per pos that appears in
    /// conjo.csv, plus errata insertions for adj-i / adj-ix / v5aru /
    /// vs-s already-present pos ids).
    #[test]
    fn builds_once() {
        assert_eq!(conj_rules().len(), 24);
    }
}
