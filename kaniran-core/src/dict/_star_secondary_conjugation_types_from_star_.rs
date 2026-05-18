//! Port of `ichiran/dict:*secondary-conjugation-types-from*` (`dict-load.lisp:312`).
//!
//! Upstream form is `` `(5 6 7 8 ,+conj-causative-su+) ``; the final
//! `53` is `+conj-causative-su+` (`dict-errata.lisp:1239`), inlined
//! following the [`super::_star_weak_conj_forms_star_`] precedent
//! (the `+conj-*+` `defconstant` forms aren't picked up by the
//! introspector).

pub static SECONDARY_CONJUGATION_TYPES_FROM: &[i32] = &[5, 6, 7, 8, 53];
