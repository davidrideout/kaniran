//! Port of `ichiran/dict:*weak-conj-forms*` (`dict-errata.lisp:1316`).
//!
//! Conjugation forms whose hits the segmenter scores down rather than
//! drops outright (the "weak" tier).

use super::kani_conj_form::{ConjForm, FormToken};

pub static WEAK_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(51), FormToken::Any,         FormToken::Any), // +conj-adjective-stem+
    ConjForm::Triple(FormToken::Int(52), FormToken::Any,         FormToken::Any), // +conj-negative-stem+
    ConjForm::Triple(FormToken::Int(53), FormToken::Any,         FormToken::Any), // +conj-causative-su+
    ConjForm::Triple(FormToken::Int(54), FormToken::Any,         FormToken::Any), // +conj-adjective-literary+
    ConjForm::Triple(FormToken::Int(9),  FormToken::Bool(true),  FormToken::Any),
];
