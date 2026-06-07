//! Port of `ichiran/dict:map-word-info-kana` (`dict.lisp:1727`).
//!
//! Apply `fn_` to a word-info's kana. When the kana is a list, map `fn_`
//! over each element, simplify the readings, and join with `separator`;
//! when it is a single string, return `fn_` of it. A nil kana takes the
//! list branch and yields the empty string.

use super::simplify_reading_list::simplify_reading_list;
use super::word_info_class::{WordInfo, WordInfoKana};
use crate::characters::text::join;

pub fn map_word_info_kana<F>(fn_: F, word_info: &WordInfo, separator: &str) -> String
where
    F: Fn(&Option<WordInfoKana>) -> String,
{
    let wkana = &word_info.kana;
    match wkana {
        // (listp wkana) is nil for a string -> (funcall fn wkana).
        Some(WordInfoKana::Single(_)) => fn_(wkana),
        // (listp wkana) is t for a list -> (mapcar fn wkana).
        Some(WordInfoKana::Multi(elements)) => {
            let mapped: Vec<String> = elements.iter().map(|element| fn_(element)).collect();
            join(separator, &simplify_reading_list(&mapped))
        }
        // (listp nil) is t -> (mapcar fn nil) = nil -> join over empty = "".
        None => join(separator, &simplify_reading_list(&[])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wi(kana: Option<WordInfoKana>) -> WordInfo {
        WordInfo { kana, ..Default::default() }
    }

    fn single(text: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(text.to_string()))
    }

    /// `fn` analogous to the REPL `string-upcase` probe.
    fn upcase(element: &Option<WordInfoKana>) -> String {
        match element {
            Some(WordInfoKana::Single(text)) => text.to_uppercase(),
            other => format!("{other:?}"),
        }
    }

    /// `fn` analogous to the REPL `identity` probe (kana element as text).
    fn ident(element: &Option<WordInfoKana>) -> String {
        match element {
            Some(WordInfoKana::Single(text)) => text.clone(),
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn map_word_info_kana_fixtures() {
        // REPL fixtures (.103, ichiran/dict::map-word-info-kana), 2026-05-23.

        // String branch: (funcall fn wkana).
        assert_eq!(map_word_info_kana(upcase, &wi(single("neko")), "/"), "NEKO");

        // List branch, default separator "/".
        let inu = Some(WordInfoKana::Multi(vec![single("neko"), single("inu")]));
        assert_eq!(map_word_info_kana(upcase, &wi(inu.clone()), "/"), "NEKO/INU");

        // List branch, identity fn -> simplify-reading-list merges the
        // shared de-spaced reading with a MIDDLE_DOT boundary.
        let merge = Some(WordInfoKana::Multi(vec![single("a b"), single("ab")]));
        assert_eq!(map_word_info_kana(ident, &wi(merge), "/"), "a\u{00B7}b");

        // nil kana -> list branch -> "".
        assert_eq!(map_word_info_kana(upcase, &wi(None), "/"), "");

        // Separator override.
        assert_eq!(map_word_info_kana(ident, &wi(inu), "+"), "neko+inu");

        // Single-element list.
        let one = Some(WordInfoKana::Multi(vec![single("neko")]));
        assert_eq!(map_word_info_kana(upcase, &wi(one), "/"), "NEKO");
    }
}
