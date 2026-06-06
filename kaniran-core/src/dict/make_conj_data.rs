//! Port of `ichiran/dict:make-conj-data` (`dict.lisp:325`).
//!
//! Constructor for the `conj-data` struct (slots seq, from, via, prop,
//! src-map).

use super::conj_data_struct::ConjData;
use super::conj_prop_dao::ConjProp;

pub fn make_conj_data(
    seq: Option<i32>,
    from: Option<i32>,
    via: Option<i32>,
    prop: Option<ConjProp>,
    src_map: Vec<(String, String)>,
) -> ConjData {
    ConjData { seq, from, via, prop, src_map }
}
