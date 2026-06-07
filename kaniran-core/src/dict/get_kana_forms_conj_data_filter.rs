//! Port of `ichiran/dict:get-kana-forms-conj-data-filter` (`dict-grammar.lisp:10`).
//!
//! Short-circuits to empty if [`skip_by_conj_data`] applies; otherwise
//! collects `conj_id`s of props that are NOT in [`WEAK_CONJ_FORMS`].

use super::errata::WEAK_CONJ_FORMS;
use super::conj_data_struct::ConjData;
use super::errata::skip_by_conj_data;
use super::errata::test_conj_prop;

pub fn get_kana_forms_conj_data_filter(conj_data: &[ConjData]) -> Vec<i32> {
    if skip_by_conj_data(conj_data) {
        return Vec::new();
    }
    conj_data
        .iter()
        .filter_map(|cd| {
            let prop = cd.prop.as_ref()?;
            if test_conj_prop(prop, WEAK_CONJ_FORMS) {
                None
            } else {
                Some(prop.conj_id)
            }
        })
        .collect()
}
