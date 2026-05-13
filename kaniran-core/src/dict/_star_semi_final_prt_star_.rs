//! Port of `ichiran/dict:*semi-final-prt*` (`dict-errata.lisp:1196`).
//!
//! "Particles that are final, but also have other uses" — built as
//! `(append *final-prt* '(2029120 2086640 2029110 2029080 2029100))`.
//! Read by `calc-score` (`dict.lisp:832`) via
//! `(member seq *semi-final-prt*)` and by `dict-grammar.lisp:1005`
//! through `(apply 'filter-in-seq-set *semi-final-prt*)`.
//!
//! Derived lazily from [`super::_star_final_prt_star_::FINAL_PRT`]
//! at first call (CONVENTIONS §5.2) so it tracks any future change
//! to the source list without a hand-copy.

use std::sync::OnceLock;

use super::_star_final_prt_star_::FINAL_PRT;

static CACHE: OnceLock<Vec<i32>> = OnceLock::new();

pub fn semi_final_prt() -> &'static [i32] {
    CACHE
        .get_or_init(|| {
            let mut out: Vec<i32> = FINAL_PRT.to_vec();
            out.extend_from_slice(&[
                2029120, // さ
                2086640, // し
                2029110, // な
                2029080, // ね
                2029100, // わ
            ]);
            out
        })
        .as_slice()
}
