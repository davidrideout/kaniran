//! Port of `ichiran/dict:conj-data` (`dict.lisp:327`).
//!
//! In-memory record carrying one (entry, conjugation-from, optional
//! via-entry, conj-prop, source-text-pairs) tuple — the
//! `defstruct conj-data seq from via prop src-map` defined in
//! `dict.lisp`. Built by [`super::get_conj_data`] (one record per
//! `conj-prop` row attached to a `conjugation` row) and consumed by
//! the segmenter / scoring layer to filter and label conjugation
//! candidates.
//!
//! All five Lisp slots default to `NIL` per the bare `defstruct` form;
//! the Rust port preserves that permissiveness with [`Option`] for
//! the four scalar slots. `src_map` is the list-of-pairs slot — empty
//! list when no source-text mapping is recorded — and stays a plain
//! [`Vec`].

use super::conj_prop_dao::ConjProp;

#[derive(Debug, Clone)]
pub struct ConjData {
    pub seq: Option<i32>,
    pub from: Option<i32>,
    pub via: Option<i32>,
    pub prop: Option<ConjProp>,
    pub src_map: Vec<(String, String)>,
}
