//! Port of `ichiran/custom:ward` (`dict-custom.lisp:269`).
//!
//! In-memory record carrying one row of the wards CSV — a 区
//! subdivision of a designated city. `text` is the ward surface form
//! (city prefix stripped at load time), `reading` its kana reading,
//! `definition` the romanized "Ward, City" pretty form, and `city`
//! the romanized city it belongs to.

#[derive(Debug, Clone)]
pub struct Ward {
    pub text: String,
    pub reading: String,
    pub definition: String,
    pub city: String,
}
