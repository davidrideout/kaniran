//! Port of `ichiran/dict:*pos-index*` (`dict-load.lisp:249`, `csv-hash *pos-index*`).
//!
//! part-of-speech tag → (numeric id, English description). Lazily built
//! once by [`load_pos_index`] (vendored kwpos.csv) and cached, mirroring
//! the upstream `defparameter *pos-index* nil` that `load-pos-index`
//! fills on first `get-pos-index`.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::load_pos_index::load_pos_index;

static POS_INDEX: OnceLock<HashMap<String, (i32, String)>> = OnceLock::new();

pub fn pos_index() -> &'static HashMap<String, (i32, String)> {
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
