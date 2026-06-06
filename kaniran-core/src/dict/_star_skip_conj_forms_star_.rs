//! Port of `ichiran/dict:*skip-conj-forms*` (`dict-errata.lisp:1310`).
//!
//! Conjugation forms whose hits the segmenter drops.

use super::kani_conj_form::{ConjForm, FormToken};

pub static SKIP_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(10), FormToken::Bool(true), FormToken::Any),
    ConjForm::Triple(FormToken::Int(3),  FormToken::Bool(true), FormToken::Bool(true)),
    ConjForm::Quadruple(
        FormToken::Str("vs-s"),
        FormToken::Int(5),
        FormToken::Any,
        FormToken::Any,
    ),
];
