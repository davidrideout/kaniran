//! Port of `ichiran/dict:conj-data-prop` (`dict.lisp:325`).
//!
//! Returns a [`super::conj_data_struct::ConjData`]'s `prop` field — its
//! `conj-prop` row.

use super::conj_data_struct::ConjData;
use super::conj_prop_dao::ConjProp;

pub fn conj_data_prop(cd: &ConjData) -> Option<ConjProp> {
    cd.prop.clone()
}
