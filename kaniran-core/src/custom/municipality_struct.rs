//! Port of `ichiran/custom:municipality` (`dict-custom.lisp:140`).
//!
//! In-memory record carrying one row of the municipalities CSV — a
//! prefecture, city, town, or village name with its kana reading and
//! romanized definition. `text` is the surface form;
//! `r#type` is the trailing kanji classifier (都/道/府/県/市/町/村)
//! pulled from the last char of `text` at load time;
//! `definition` is the romanized pretty form built by the loader;
//! `prefecture` is `Some(romanized)` for rows below the prefecture
//! level and `None` for prefecture rows themselves.

#[derive(Debug, Clone)]
pub struct Municipality {
    pub text: String,
    pub reading: String,
    pub definition: String,
    pub r#type: char,
    pub prefecture: Option<String>,
}
