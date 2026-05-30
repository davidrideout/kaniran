//! Port of `ichiran/dict:*skip-conj-forms*` (`dict-errata.lisp:1310`).
//!
//! Conjugation forms whose hits the segmenter must drop. Used by
//! [`super::test_conj_prop::test_conj_prop`] (in concert with the
//! `*skip-conj-forms*`-driven [`super::skip_by_conj_data`] pass) to
//! filter out variants that ichiran's heuristics never want to score:
//!
//! | Form | Triple/Quadruple | Meaning |
//! |---|---|---|
//! | `(10 t :any)` | Triple | conj-type 10 (potential past), neg=`T`, any fml |
//! | `(3 t t)` | Triple | conj-type 3 (negative), neg=`T`, fml=`T` |
//! | `("vs-s" 5 :any :any)` | Quadruple | pos `vs-s`, conj-type 5, any neg, any fml |

use super::kani::{ConjForm, FormToken};

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
