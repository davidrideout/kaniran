//! Sidecar — Rust-only types for the conjugation-form match-list
//! data registered in `*skip-conj-forms*` and `*weak-conj-forms*` and
//! consumed by `test-conj-prop`. The Lisp uses untyped lists where
//! each cell is heterogeneously a `pos` string, a `conj-type` int, or
//! one of `:any` / `T` / `NIL` / `:null`; this file pins the closed
//! token vocabulary to Rust enums so the data tables can be
//! `pub static` and the matcher dispatches without runtime type
//! checks.
//!
//! Shape conventions:
//!
//! * [`FormToken::Any`] is the wildcard `:any` keyword.
//! * [`FormToken::Bool`] / [`FormToken::DbNull`] cover the
//!   `T` / `NIL` / `:null` literals that match against
//!   [`super::conj_prop_dao::ConjProp::neg`] / `fml` (which are
//!   `Option<bool>` — `Some(true)` / `Some(false)` / `None`).
//! * [`FormToken::Int`] is the `conj-type` integer literal.
//! * [`FormToken::Str`] is the `pos` string literal (`v5t`, `adj-i`,
//!   `vs-s`, etc.).
//!
//! [`ConjForm`] is `Triple` for the 3-cell match (`conj-type`, `neg`,
//! `fml`) or `Quadruple` for the 4-cell match (`pos`, `conj-type`,
//! `neg`, `fml`). `test-conj-prop` switches on length; the Rust port
//! switches on the variant.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormToken {
    Any,
    Int(i32),
    Str(&'static str),
    Bool(bool),
    DbNull,
}

#[derive(Debug, Clone, Copy)]
pub enum ConjForm {
    /// `(conj-type neg fml)` — matched against `prop.conj_type`,
    /// `prop.neg`, `prop.fml`.
    Triple(FormToken, FormToken, FormToken),
    /// `(pos conj-type neg fml)` — matched against `prop.pos`,
    /// `prop.conj_type`, `prop.neg`, `prop.fml`.
    Quadruple(FormToken, FormToken, FormToken, FormToken),
}
