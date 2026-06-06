//! Port of `ichiran/numbers:not-a-number` (`numbers.lisp:67`).
//!
//! Error raised by [`super::parse_number::parse_number`] when its input
//! string contains a character that isn't a recognized numeric glyph.
//! Carries the offending input and a free-form reason string.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{text:?} is not a number: {reason}")]
pub struct NotANumber {
    pub text: String,
    pub reason: String,
}
