//! Port of `ichiran/dict:*conj-description*` (`dict-load.lisp:257`, `csv-hash *conj-description*`).
//!
//! conj-id → English conjugation-type description.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::load_conj_description::load_conj_description;

pub fn conj_description() -> &'static HashMap<i32, String> {
    static CONJ_DESCRIPTION: OnceLock<HashMap<i32, String>> = OnceLock::new();
    CONJ_DESCRIPTION.get_or_init(load_conj_description)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `OnceLock` builder wiring runs `load_conj_description`
    /// (18 entries per the .103 REPL).
    #[test]
    fn builds_once() {
        assert_eq!(conj_description().len(), 18);
    }
}
