//! Rust-only sidecar: the `method` keyword value the romanize entry
//! points branch on. Upstream the slot holds either a
//! `generic-romanization` instance or the keyword `:kana`
//! (`romanize.lisp:252`, `(if (eql method :kana) …)`).
//! [`RomanizationMethod`] models only the instance case; this enum adds
//! the `:kana` arm so [`super::romanize_word_info`] can dispatch on it.

use super::generic_romanization_class::RomanizationMethod;

#[derive(Debug, Clone, Copy)]
pub enum KaniRomanizeMethod<'a> {
    Kana,
    Method(RomanizationMethod<'a>),
}
