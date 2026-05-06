//! Port of `ichiran/dict:test-conj-prop` (`dict-errata.lisp:1336`).
//!
//! Predicate: does [`ConjProp`] match any element of `forms`? The
//! Lisp builds `(pos conj-type conj-neg conj-fml)` and runs `some`
//! over `forms`, dispatching on form length:
//!
//! * length 3 → match `(conj-type neg fml)` against the prop's last
//!   three slots.
//! * length 4 → match `(pos conj-type neg fml)` against all four.
//!
//! A cell `:any` always matches; otherwise compare with `EQL`.
//! The Rust port models the closed cell vocabulary as
//! [`super::kani_conj_form::FormToken`] and dispatches on the
//! [`super::kani_conj_form::ConjForm`] variant.

use super::conj_prop_dao::ConjProp;
use super::kani_conj_form::{ConjForm, FormToken};

pub fn test_conj_prop(prop: &ConjProp, forms: &[ConjForm]) -> bool {
    forms.iter().any(|form| match form {
        ConjForm::Triple(ct, neg, fml) => {
            match_conj_type(*ct, prop.conj_type)
                && match_bool(*neg, prop.neg)
                && match_bool(*fml, prop.fml)
        }
        ConjForm::Quadruple(pos, ct, neg, fml) => {
            match_pos(*pos, &prop.pos)
                && match_conj_type(*ct, prop.conj_type)
                && match_bool(*neg, prop.neg)
                && match_bool(*fml, prop.fml)
        }
    })
}

fn match_conj_type(token: FormToken, value: i32) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Int(n) => n == value,
        _ => false,
    }
}

fn match_pos(token: FormToken, value: &str) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Str(s) => s == value,
        _ => false,
    }
}

fn match_bool(token: FormToken, value: Option<bool>) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Bool(b) => value == Some(b),
        FormToken::DbNull => value.is_none(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(pos: &str, conj_type: i32, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
        ConjProp { id: 0, conj_id: 0, conj_type, pos: pos.to_string(), neg, fml }
    }

    #[test]
    fn triple_matches_on_conj_type_and_any_wildcards() {
        let p = prop("v1", 51, Some(true), None);
        let forms = [ConjForm::Triple(FormToken::Int(51), FormToken::Any, FormToken::Any)];
        assert!(test_conj_prop(&p, &forms));
    }

    #[test]
    fn triple_rejects_when_conj_type_differs() {
        let p = prop("v1", 51, None, None);
        let forms = [ConjForm::Triple(FormToken::Int(50), FormToken::Any, FormToken::Any)];
        assert!(!test_conj_prop(&p, &forms));
    }

    #[test]
    fn bool_token_distinguishes_true_false_and_dbnull() {
        let true_prop = prop("v5t", 2, Some(true), None);
        let false_prop = prop("v5t", 2, Some(false), None);
        let null_prop = prop("v5t", 2, None, None);

        let want_true = [ConjForm::Triple(FormToken::Int(2), FormToken::Bool(true), FormToken::Any)];
        assert!(test_conj_prop(&true_prop, &want_true));
        assert!(!test_conj_prop(&false_prop, &want_true));
        assert!(!test_conj_prop(&null_prop, &want_true));

        let want_dbnull = [ConjForm::Triple(FormToken::Int(2), FormToken::DbNull, FormToken::Any)];
        assert!(!test_conj_prop(&true_prop, &want_dbnull));
        assert!(test_conj_prop(&null_prop, &want_dbnull));
    }

    #[test]
    fn quadruple_requires_pos_match_in_addition_to_triple() {
        let p = prop("vs-s", 5, None, None);
        let matching = [ConjForm::Quadruple(
            FormToken::Str("vs-s"),
            FormToken::Int(5),
            FormToken::Any,
            FormToken::Any,
        )];
        let wrong_pos = [ConjForm::Quadruple(
            FormToken::Str("v5t"),
            FormToken::Int(5),
            FormToken::Any,
            FormToken::Any,
        )];
        assert!(test_conj_prop(&p, &matching));
        assert!(!test_conj_prop(&p, &wrong_pos));
    }

    #[test]
    fn any_in_one_form_in_a_list_is_enough() {
        let p = prop("v1", 13, Some(true), Some(false));
        let forms = [
            ConjForm::Triple(FormToken::Int(99), FormToken::Any, FormToken::Any), // miss
            ConjForm::Triple(FormToken::Int(13), FormToken::Bool(true), FormToken::Bool(false)), // hit
        ];
        assert!(test_conj_prop(&p, &forms));
    }
}
