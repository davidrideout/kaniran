//! Port of `ichiran/dict:get-conj-rules` (`dict-load.lisp:268`, `csv-hash *conj-rules*` accessor).
//!
//! Look up the conjugation rules for a numeric pos id and return them
//! in CSV order. Missing keys produce an empty list (upstream returns
//! `nil`; `(reverse nil)` = `nil`). The lazy `(unless *conj-rules*
//! (load-conj-rules))` is the `OnceLock` in
//! [`super::_star_conj_rules_star_`]. The stored list is in cons-prepend
//! order (newest-first); the accessor reverses to give callers the
//! original CSV order.

use super::_star_conj_rules_star_::conj_rules;
use super::conjugation_rule_struct::ConjugationRule;

pub fn get_conj_rules(key: i32) -> Vec<ConjugationRule> {
    // dict-load.lisp:281 — (val (reverse val))
    conj_rules()
        .get(&key)
        .map(|val| val.iter().rev().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL probe (`(ichiran/dict::get-conj-rules pos-id)`), 2026-05-31.
    /// The cases pin both the post-errata row count and the first/last
    /// rules in CSV / accessor order — accessor reverses the stored
    /// cons-prepend list so the last element is the errata-injected
    /// `(conj=52)` v5* negative-stem when the pos starts with `v5`.
    #[test]
    fn get_conj_rules_lookups() {
        // pos 1 (adj-i): 23 rules; first conj=1 okuri="い", last conj=54 okuri="き"
        let r1 = get_conj_rules(1);
        assert_eq!(r1.len(), 23);
        assert_eq!(r1.first().map(|r| (r.conj, r.okuri.as_str())), Some((1, "い")));
        assert_eq!(r1.last().map(|r| (r.conj, r.okuri.as_str())), Some((54, "き")));

        // pos 41 (v5u): 61 rules; first conj=1 okuri="う"; last 2 rules pinned
        // (conj=13 okuri="い" from CSV, then errata-appended conj=52 okuri="わ").
        let r41 = get_conj_rules(41);
        assert_eq!(r41.len(), 61);
        assert_eq!(r41.first().map(|r| (r.conj, r.okuri.as_str())), Some((1, "う")));
        let tail2: Vec<_> = r41[r41.len() - 2..]
            .iter()
            .map(|r| (r.conj, r.okuri.as_str(), r.neg, r.fml))
            .collect();
        assert_eq!(tail2, vec![(13, "い", false, false), (52, "わ", true, false)]);

        // pos 28 (v1): 60 rules per the REPL probe.
        assert_eq!(get_conj_rules(28).len(), 60);

        // pos 30 (v5aru): 62 rules per the REPL probe.
        assert_eq!(get_conj_rules(30).len(), 62);

        // pos 47 (vs-s): 53 rules after potential-form (conj=5) removal.
        let r47 = get_conj_rules(47);
        assert_eq!(r47.len(), 53);
        // dict-errata.lisp:1287 removes conj=5 from vs-s entirely.
        assert!(r47.iter().all(|r| r.conj != 5));

        // missing key → empty (upstream returns nil; (reverse nil) = nil)
        assert!(get_conj_rules(9999).is_empty());
    }
}
