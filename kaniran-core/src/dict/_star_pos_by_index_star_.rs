//! Port of `ichiran/dict:*pos-by-index*` (`dict-load.lisp:253`, `csv-hash *pos-by-index*`).
//!
//! numeric id → part-of-speech tag, from the vendored kwpos.csv.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::load_pos_by_index::load_pos_by_index;

static POS_BY_INDEX: OnceLock<HashMap<i32, String>> = OnceLock::new();

pub fn pos_by_index() -> &'static HashMap<i32, String> {
    POS_BY_INDEX.get_or_init(load_pos_by_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `OnceLock` builder wiring runs `load_pos_by_index` (92
    /// entries per the .103 REPL).
    #[test]
    fn builds_once() {
        assert_eq!(pos_by_index().len(), 92);
    }
}
