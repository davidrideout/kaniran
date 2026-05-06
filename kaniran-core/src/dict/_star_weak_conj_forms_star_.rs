//! Port of `ichiran/dict:*weak-conj-forms*` (`dict-errata.lisp:1316`).
//!
//! Conjugation forms whose hits the segmenter scores down rather than
//! drops outright (the "weak" tier). All but the last reference one
//! of the conjugation-type defconstants in `dict-errata.lisp:1236-1240`:
//!
//! | Constant | Value | Form |
//! |---|---:|---|
//! | `+conj-adverbial+` | 50 | (not in this list) |
//! | `+conj-adjective-stem+` | 51 | first triple |
//! | `+conj-negative-stem+` | 52 | second |
//! | `+conj-causative-su+` | 53 | third |
//! | `+conj-adjective-literary+` | 54 | fourth |
//!
//! The integer literals are inlined here because `defconstant` forms
//! aren't picked up by the introspector (no `_global.md` entry, no
//! `symbols.csv` row), so there's no upstream symbol to depend on.
//! Values are pinned by the populated `conj_type` column on every
//! `conj_prop` row in the live database — they cannot drift without
//! a wholesale dictionary rebuild.

use super::kani_conj_form::{ConjForm, FormToken};

pub static WEAK_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(51), FormToken::Any,         FormToken::Any), // +conj-adjective-stem+
    ConjForm::Triple(FormToken::Int(52), FormToken::Any,         FormToken::Any), // +conj-negative-stem+
    ConjForm::Triple(FormToken::Int(53), FormToken::Any,         FormToken::Any), // +conj-causative-su+
    ConjForm::Triple(FormToken::Int(54), FormToken::Any,         FormToken::Any), // +conj-adjective-literary+
    ConjForm::Triple(FormToken::Int(9),  FormToken::Bool(true),  FormToken::Any),
];
