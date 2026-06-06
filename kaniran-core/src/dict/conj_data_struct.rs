//! Port of `ichiran/dict:conj-data` (`dict.lisp:327`).
//!
//! In-memory record carrying one (entry, conjugation-from, optional
//! via-entry, conj-prop, source-text-pairs) tuple describing a single
//! conjugation candidate.

use super::conj_prop_dao::ConjProp;

#[derive(Debug, Clone)]
pub struct ConjData {
    pub seq: Option<i32>,
    pub from: Option<i32>,
    pub via: Option<i32>,
    pub prop: Option<ConjProp>,
    pub src_map: Vec<(String, String)>,
}
