//! Port of `ichiran/dict:word-info-reading-str` (`dict.lisp:1733`).
//!
//! Renders a [`WordInfo`]'s text/kana into a reading string via
//! [`reading_str_star_`] — kanji (or a counter with a seq) gets both
//! sides; everything else passes only the text as the kana side.

use super::reading_str_star_::reading_str_star_;
use super::word_info_class::{WordInfo, WordInfoKana, WordInfoType};
use std::borrow::Cow;

pub fn word_info_reading_str(word_info: &WordInfo) -> Option<String> {
    if word_info.kind == WordInfoType::Kanji
        || (word_info.counter.is_some() && word_info.seq.is_some())
    {
        let kana = word_info.kana.as_ref().map(princ_kana);
        reading_str_star_(Some(&word_info.text), kana.as_deref())
    } else {
        reading_str_star_(None, Some(&word_info.text))
    }
}

// `~a`/princ rendering of a kana value: a string prints verbatim, a list
// prints `(elem ...)` with nil → NIL and nested lists recursing.
fn princ_kana(kana: &WordInfoKana) -> Cow<'_, str> {
    match kana {
        WordInfoKana::Single(reading) => Cow::Borrowed(reading),
        WordInfoKana::Multi(readings) => {
            let elems: Vec<String> = readings
                .iter()
                .map(|reading| match reading {
                    None => "NIL".to_string(),
                    Some(reading) => princ_kana(reading).into_owned(),
                })
                .collect();
            Cow::Owned(format!("({})", elems.join(" ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::word_info_class::WordInfoSeq;

    fn wi(
        kind: WordInfoType,
        text: &str,
        kana: Option<WordInfoKana>,
        counter: Option<(String, bool)>,
        seq: Option<WordInfoSeq>,
    ) -> WordInfo {
        WordInfo {
            kind,
            text: text.to_string(),
            kana,
            counter,
            seq,
            ..Default::default()
        }
    }

    /// REPL fixtures (.103, `ichiran/dict::word-info-reading-str`), 2026-05-24.
    /// Covers the kanji-type branch (single-string / list / nested-list-with-nil
    /// / nil kana → `~a` princ rendering), the counter+seq branch reaching the
    /// same body, and the else branch (counter without seq, kana type, gap type).
    #[test]
    fn word_info_reading_str_fixtures() {
        use WordInfoKana::{Multi, Single};
        let single = |reading: &str| Single(reading.to_string());
        let cases: Vec<(WordInfo, &str)> = vec![
            (
                wi(WordInfoType::Kanji, "日本", Some(single("にほん")), None, None),
                "日本 【にほん】",
            ),
            (
                wi(
                    WordInfoType::Kanji,
                    "日本",
                    Some(Multi(vec![Some(single("にほん")), Some(single("にっぽん"))])),
                    None,
                    None,
                ),
                "日本 【(にほん にっぽん)】",
            ),
            (
                wi(
                    WordInfoType::Kanji,
                    "X",
                    Some(Multi(vec![
                        Some(single("あ")),
                        None,
                        Some(Multi(vec![Some(single("い")), Some(single("う"))])),
                    ])),
                    None,
                    None,
                ),
                "X 【(あ NIL (い う))】",
            ),
            (
                wi(WordInfoType::Kanji, "日本", None, None, None),
                "日本 【NIL】",
            ),
            (
                wi(
                    WordInfoType::Kana,
                    "三冊",
                    Some(single("さんさつ")),
                    Some(("3".to_string(), false)),
                    Some(WordInfoSeq::Single(12345)),
                ),
                "三冊 【さんさつ】",
            ),
            (
                wi(
                    WordInfoType::Kana,
                    "三冊",
                    Some(single("さんさつ")),
                    Some(("3".to_string(), false)),
                    None,
                ),
                "三冊",
            ),
            (
                wi(WordInfoType::Kana, "ねこ", Some(single("ねこ")), None, None),
                "ねこ",
            ),
            (
                wi(WordInfoType::Gap, "?", Some(single("?")), None, None),
                "?",
            ),
        ];
        for (word_info, expected) in &cases {
            assert_eq!(
                word_info_reading_str(word_info).as_deref(),
                Some(*expected),
                "text={:?}",
                word_info.text
            );
        }
    }
}
