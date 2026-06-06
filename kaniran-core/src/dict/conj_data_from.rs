//! Port of `ichiran/dict:conj-data-from` (`dict.lisp:325`).
//!
//! Returns a [`ConjData`]'s `from` field — the seq id of the source
//! entry the conjugation derives from.

use super::conj_data_struct::ConjData;

pub fn conj_data_from(cd: &ConjData) -> Option<i32> {
    cd.from
}
