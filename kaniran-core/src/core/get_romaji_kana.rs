//! Port of `ichiran:get-romaji-kana` (`deromanize.lisp:7`, `csv-hash *romaji-kana*` expansion).
//!
//! Looks up the romaji prefix `key` in the romaji-map, returning its
//! [`RmapItem`] rule or `None` when absent.

use super::_star_romaji_kana_star_::romaji_kana;
use super::rmap_item_struct::RmapItem;

pub fn get_romaji_kana(key: &str) -> Option<&'static RmapItem> {
    romaji_kana().get(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_romaji_kana_fixtures() {
        // REPL fixtures (.103, ichiran::get-romaji-kana), 2026-05-26.
        // (key, Some(text, kana, next) | None). Covers a plain rule, a
        // doubled-consonant rule (`next` present), a missing key, and
        // the empty key.
        let cases: &[(&str, Option<(&str, &str, Option<&str>)>)] = &[
            ("a", Some(("a", "あ", None))),
            ("ka", Some(("ka", "か", None))),
            ("shi", Some(("shi", "し", None))),
            ("n", Some(("n", "ん", None))),
            ("bb", Some(("bb", "っ", Some("b")))),
            ("kk", Some(("kk", "っ", Some("k")))),
            ("pp", Some(("pp", "っ", Some("p")))),
            ("xyz", None),
            ("", None),
        ];
        for (key, expected) in cases {
            let got = get_romaji_kana(key)
                .map(|rmi| (rmi.text.as_str(), rmi.kana.as_str(), rmi.next.as_deref()));
            assert_eq!(got, *expected, "key={key:?}");
        }
    }
}
