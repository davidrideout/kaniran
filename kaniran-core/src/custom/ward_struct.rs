//! Port of `ichiran/custom:ward` (`dict-custom.lisp:269`).
//!
//! In-memory record for one row of the wards CSV — a 区 subdivision of
//! a designated city, with its kana reading and romanized definition.

#[derive(Debug, Clone)]
pub struct Ward {
    pub text: String,
    pub reading: String,
    pub definition: String,
    pub city: String,
}
