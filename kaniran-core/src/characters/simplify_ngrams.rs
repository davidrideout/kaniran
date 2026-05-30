//! Port of `ichiran/characters:simplify-ngrams` (`characters.lisp:210-217`).
//!
//! Replace every occurrence of a `from` key in `s` with its `to` value,
//! choosing the leftmost-first match when several keys could match at
//! the same position. Used to fold combining-mark sequences into single
//! precomposed glyphs (`"か゛" → "が"`) and to ASCII-ize Japanese
//! punctuation.
//!
//! The Lisp builds the matcher from a flat plist of alternating
//! `from`/`to`; the Rust port takes a paired `&[(S, T)]` slice generic
//! over `AsRef<str>` so both static `&[(&str, &str)]` (e.g.
//! [`super::constants::PUNCTUATION_MARKS`]) and
//! the runtime `Vec<(String, String)>` from
//! [`super::voicing_tables::dakuten_join`] work without
//! conversion. Keys are passed through `fancy_regex::escape` before
//! alternation — cl-ppcre's parse-tree DSL already treats string
//! elements as literal character sequences (verified: `(:alternation
//! "a.b")` matches `"a.b"` but not `"axb"`), so escaping is how Rust
//! preserves that semantics across the engine boundary, not a safety
//! margin. Both implementations match keys literally regardless of
//! regex metacharacters in the data.
//!
//! A new regex is compiled per call — caller-driven maps have unbounded
//! cardinality, so caching by map identity wouldn't be sound.

use fancy_regex::{Captures, Regex};

pub fn simplify_ngrams<S, T>(s: &str, map: &[(S, T)]) -> String
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    if map.is_empty() {
        return s.to_string();
    }
    let pattern: String = map
        .iter()
        .map(|(k, _)| fancy_regex::escape(k.as_ref()).into_owned())
        .collect::<Vec<_>>()
        .join("|");
    let re = Regex::new(&pattern).expect("simplify-ngrams alternation compiles");
    re.replace_all(s, |caps: &Captures| -> String {
        let m = caps.get(0).expect("alternation always has group 0").as_str();
        map.iter()
            .find(|(k, _)| k.as_ref() == m)
            .map(|(_, v)| v.as_ref().to_string())
            .unwrap_or_default()
    })
    .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::voicing_tables::dakuten_join;

    /// The runtime-derived `dakuten_join()` map feeds into
    /// `simplify_ngrams` correctly — `"か゛"` (ka + combining dakuten)
    /// folds to `"が"`. Pins the *integration* between the two ports,
    /// not the contents of either.
    #[test]
    fn folds_combining_dakuten_via_runtime_map() {
        assert_eq!(simplify_ngrams("か゛", dakuten_join()), "が");
        assert_eq!(simplify_ngrams("ハ゜", dakuten_join()), "パ");
    }

    /// Empty map is a no-op.
    #[test]
    fn empty_map_returns_input_unchanged() {
        let map: &[(&str, &str)] = &[];
        assert_eq!(simplify_ngrams("hello", map), "hello");
    }
}
