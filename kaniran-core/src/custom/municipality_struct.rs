//! Port of `ichiran/custom:municipality` (`dict-custom.lisp:140`).
//!
//! In-memory record for one row of the municipalities CSV — a
//! prefecture, city, town, or village name with its kana reading and
//! romanized definition.

#[derive(Debug, Clone)]
pub struct Municipality {
    pub text: String,
    pub reading: String,
    pub definition: String,
    pub r#type: char,
    pub prefecture: Option<String>,
}
