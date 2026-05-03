//! Port of `ichiran/numbers:*power-kanji*` (`numbers.lisp:7`).
//!
//! Power-of-ten kanji glyph table indexed by exponent. Slot `i` holds
//! the kanji for `10^i`, with ASCII spaces filling the four exponents
//! (`5..=7`, `9..=11`, `13..=15`) where Japanese has no dedicated
//! single-character form. [`super::number_to_kanji::number_to_kanji`]
//! reads this in order and skips space slots while picking the largest
//! power that fits the target integer.

pub const POWER_KANJI: &str = "一十百千万   億   兆   京";
