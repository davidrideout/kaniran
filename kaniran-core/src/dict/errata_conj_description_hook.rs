//! Port of `ichiran/dict:errata-conj-description-hook` (`dict-errata.lisp:1242`).
//!
//! Adds the five ichiran-internal conjugation types
//! (`+conj-adverbial+`=50 … `+conj-adjective-literary+`=54) to the
//! conj-id → description map after it is loaded from conj.csv.

use std::collections::HashMap;

pub fn errata_conj_description_hook(hash: &mut HashMap<i32, String>) {
    hash.insert(50, "Adverbial".to_string()); // +conj-adverbial+
    hash.insert(51, "Adjective Stem".to_string()); // +conj-adjective-stem+
    hash.insert(52, "Negative Stem".to_string()); // +conj-negative-stem+
    hash.insert(53, "Causative (~su)".to_string()); // +conj-causative-su+
    hash.insert(54, "Old/literary form".to_string()); // +conj-adjective-literary+
}
