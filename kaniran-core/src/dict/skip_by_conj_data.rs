//! Port of `ichiran/dict:skip-by-conj-data` (`dict-errata.lisp:1336`).
//!
//! True iff `conj_data` is non-empty and every prop matches
//! [`SKIP_CONJ_FORMS`] (empty list → false).

use super::_star_skip_conj_forms_star_::SKIP_CONJ_FORMS;
use super::conj_data_struct::ConjData;
use super::test_conj_prop::test_conj_prop;

pub fn skip_by_conj_data(conj_data: &[ConjData]) -> bool {
    !conj_data.is_empty() && conj_data.iter().all(matches)
}

fn matches(cd: &ConjData) -> bool {
    cd.prop
        .as_ref()
        .map(|prop| test_conj_prop(prop, SKIP_CONJ_FORMS))
        .unwrap_or(false)
}
