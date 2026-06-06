//! Port of `ichiran/characters:simplify-ngrams` (`characters.lisp:210-217`).
//!
//! Replace every occurrence of a `from` key in `s` with its `to` value,
//! choosing the leftmost-first match when several keys could match at
//! the same position. Used to fold combining-mark sequences into single
//! precomposed glyphs (`"か゛" → "が"`) and to ASCII-ize Japanese
//! punctuation. Keys are matched literally regardless of any regex
//! metacharacters in the data.

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
    use super::super::_star_dakuten_join_star_::dakuten_join;

    /// `"か゛"` (ka + combining dakuten) folds to `"が"`.
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
