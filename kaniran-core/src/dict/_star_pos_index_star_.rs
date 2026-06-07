//! Port of `ichiran/dict:*pos-index*` (`dict-load.lisp:249`, `csv-hash *pos-index*`).
//!
//! part-of-speech tag → (numeric id, English description), from the
//! vendored kwpos.csv.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::load_pos_index::load_pos_index;

pub fn pos_index() -> &'static HashMap<String, (i32, String)> {
    static POS_INDEX: OnceLock<HashMap<String, (i32, String)>> = OnceLock::new();
    POS_INDEX.get_or_init(load_pos_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `OnceLock` builder wiring runs `load_pos_index` (92 entries
    /// per the .103 REPL).
    #[test]
    fn builds_once() {
        assert_eq!(pos_index().len(), 92);
    }
}
