//! Sidecar — Rust-only types for the conjugation-form match-list
//! data registered in `*skip-conj-forms*` and `*weak-conj-forms*` and
//! consumed by `test-conj-prop`, pinning the heterogeneous Lisp match
//! cells (`pos` string, `conj-type` int, `:any` / `T` / `NIL` /
//! `:null`) to a closed token vocabulary of Rust enums.

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
