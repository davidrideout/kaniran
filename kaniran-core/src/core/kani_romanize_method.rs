//! Rust-only sidecar: the `method` value the romanize entry points
//! dispatch on — either a `generic-romanization` instance or the keyword
//! `:kana` (`romanize.lisp:252`, `(if (eql method :kana) …)`).

use super::generic_romanization_class::RomanizationMethod;

#[derive(Debug, Clone, Copy)]
pub enum KaniRomanizeMethod<'a> {
    Kana,
    Method(RomanizationMethod<'a>),
}
