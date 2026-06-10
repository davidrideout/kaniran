//! Rust-only sidecar within the `ichiran/characters` port module.
//! Has no Lisp counterpart.
//!
//! Compile-once scanner for `simplify-ngrams` translation tables.
//! Every runtime table is fixed, so callers hold one scanner and reuse
//! it instead of rebuilding the alternation regex on every call.

use fancy_regex::{Captures, Regex};

/// A `simplify-ngrams` translation table with its alternation regex
/// compiled once. Alternation order = table order, so at equal start
/// positions the earlier table entry wins (leftmost-first), matching
/// the original ppcre scanner (`characters.lisp:212-213`).
///
/// ```text
/// KaniNgramScanner {
///     table: vec![("か\u{3099}".to_string(), "が".to_string()),
///                 ("き\u{3099}".to_string(), "ぎ".to_string()), ...],
///     regex: Some(Regex("か\u{3099}|き\u{3099}|...")),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct KaniNgramScanner {
    table: Vec<(String, String)>,
    /// `None` when the table is empty; `simplify` then returns the
    /// input unchanged (the nil-map short-circuit of `simplify-ngrams`).
    regex: Option<Regex>,
}

impl KaniNgramScanner {
    pub fn new<S, T>(map: &[(S, T)]) -> Self
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        let table: Vec<(String, String)> = map
            .iter()
            .map(|(from, to)| (from.as_ref().to_string(), to.as_ref().to_string()))
            .collect();
        let regex = if table.is_empty() {
            None
        } else {
            let pattern: String = table
                .iter()
                .map(|(from, _)| fancy_regex::escape(from).into_owned())
                .collect::<Vec<_>>()
                .join("|");
            Some(Regex::new(&pattern).expect("simplify-ngrams alternation compiles"))
        };
        KaniNgramScanner { table, regex }
    }

    /// `simplify-ngrams` replacement body (`characters.lisp:214-217`):
    /// replace every `from` occurrence with its `to`, leftmost-first
    /// when several keys match at the same position.
    pub fn simplify(&self, s: &str) -> String {
        let Some(regex) = &self.regex else {
            return s.to_string();
        };
        regex
            .replace_all(s, |caps: &Captures| -> String {
                let matched = caps
                    .get(0)
                    .expect("alternation always has group 0")
                    .as_str();
                self.table
                    .iter()
                    .find(|(from, _)| from.as_str() == matched)
                    .map(|(_, to)| to.clone())
                    .unwrap_or_default()
            })
            .into_owned()
    }
}
