//! Rust-only sidecar: closed enum tagging the three kinds of character
//! used by the Japanese number system. Mirrors the `:jd` / `:p` / `:ad`
//! keywords the Lisp uses inline in `*char-number-class*`
//! (`numbers.lisp:9`) without a named type:
//!
//! - [`NumClass::Jd`] — japanese digit (e.g. `〇一二…`, `壱弐参…`),
//!   value is `0..=9`.
//! - [`NumClass::P`] — power-of-ten kanji (e.g. `十百千万億兆京`),
//!   value is the exponent (`1, 2, 3, 4, 8, 12, 16`).
//! - [`NumClass::Ad`] — ASCII / full-width digit (`0..9`, `０..９`),
//!   value is `0..=9`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumClass {
    /// Japanese digit kanji (`〇一二三四五六七八九` and the legal forms).
    Jd,
    /// Power-of-ten kanji (`十百千万億兆京`).
    P,
    /// ASCII or full-width digit (`0-9`, `０-９`).
    Ad,
}
