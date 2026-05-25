//! Port of `ichiran/dict:match-kana-kanji` (`dict.lisp:1507`).
//!
//! ```lisp
//! (defun match-kana-kanji (kana-reading kanji-reading restricted)
//!   (cond ((nokanji kana-reading) nil)
//!         (t (let* ((kana-text (text kana-reading))
//!                   (restr (loop for (rt kt) in restricted when (equal kana-text rt) collect kt)))
//!              (if restr
//!                  (find (text kanji-reading) restr :test 'equal)
//!                  t)))))
//! ```
//!
//! `restricted` is a list of `(reading text)` rows from
//! `restricted-readings`. The upstream return is the CL generalized
//! boolean `(or null (eql t) string)`, modeled as
//! `Option<MatchKanaKanjiResult>`: `None` = `nil`, `Some(Yes)` = `t`,
//! `Some(Found(s))` = the matched kanji surface from `(find …)`. The
//! sole caller [`super::match_sense_restrictions`] uses it as a
//! predicate via `some`.

use super::kani_word::KaniWordDispatchEnum;
use super::nokanji::nokanji;
use super::text::text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchKanaKanjiResult {
    /// Lisp `t` — the kana reading carries no restricted-reading
    /// constraint, so any kanji pairs.
    Yes,
    /// Lisp `(find (text kanji-reading) restr :test 'equal)` — the
    /// matched kanji surface.
    Found(String),
}

pub fn match_kana_kanji(
    kana_reading: &KaniWordDispatchEnum,
    kanji_reading: &KaniWordDispatchEnum,
    restricted: &[(String, String)],
) -> Option<MatchKanaKanjiResult> {
    // dict.lisp:1508 ((nokanji kana-reading) nil) — `nokanji` has no
    // method for compound-text upstream (no-applicable-method); the
    // dispatcher returns None there and `.expect` surfaces that error.
    if nokanji(kana_reading).expect("nokanji: no method for compound-text") {
        return None;
    }
    // dict.lisp:1509 (kana-text (text kana-reading))
    let kana_text = text(kana_reading);
    let kana_text = kana_text.as_ref();
    // dict.lisp:1510 (restr (loop for (rt kt) in restricted when (equal kana-text rt) collect kt))
    let restr: Vec<&str> = restricted
        .iter()
        .filter(|(rt, _kt)| rt.as_str() == kana_text)
        .map(|(_rt, kt)| kt.as_str())
        .collect();
    // dict.lisp:1511-1513 (if restr (find (text kanji-reading) restr :test 'equal) t)
    if !restr.is_empty() {
        let kanji_text = text(kanji_reading);
        if restr.iter().any(|kt| *kt == kanji_text.as_ref()) {
            Some(MatchKanaKanjiResult::Found(kanji_text.into_owned()))
        } else {
            None
        }
    } else {
        Some(MatchKanaKanjiResult::Yes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::simple_text_class::SimpleText;

    fn kana(text: &str, nokanji: bool) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0, seq: 0, text: text.into(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji, best_kanji: None, state: SimpleText::default(),
        })
    }

    fn kanji(text: &str) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kanji(KanjiText {
            id: 0, seq: 0, text: text.into(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kana: None, state: SimpleText::default(),
        })
    }

    fn restr(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(r, t)| (r.to_string(), t.to_string())).collect()
    }

    /// REPL fixtures (.103, `ichiran/dict::match-kana-kanji`), 2026-05-24.
    /// Readings are seq-1339160 forms: だし (nokanji nil), ダシ
    /// (nokanji t), 出し / 出汁 kanji.
    #[test]
    fn match_kana_kanji_fixtures() {
        let dashi = kana("だし", false);
        let dashi_kata = kana("ダシ", true);
        let dashi_k = kanji("出し");
        let dashi_kanji = kanji("出汁");

        let cases: &[(&KaniWordDispatchEnum, &KaniWordDispatchEnum, Vec<(String, String)>, Option<MatchKanaKanjiResult>)] = &[
            // restricted empty → t
            (&dashi, &dashi_k, restr(&[]), Some(MatchKanaKanjiResult::Yes)),
            // restr has だし→出し, kanji is 出し → found
            (&dashi, &dashi_k, restr(&[("だし", "出し")]), Some(MatchKanaKanjiResult::Found("出し".into()))),
            // restr has だし→出汁, kanji is 出し → not found
            (&dashi, &dashi_k, restr(&[("だし", "出汁")]), None),
            // restr keyed on ダシ only → filters to empty → t
            (&dashi, &dashi_k, restr(&[("ダシ", "出し")]), Some(MatchKanaKanjiResult::Yes)),
            // two rows for だし; kanji 出汁 matches the 2nd → found
            (&dashi, &dashi_kanji, restr(&[("だし", "出し"), ("だし", "出汁")]), Some(MatchKanaKanjiResult::Found("出汁".into()))),
            // two rows but kanji 出汁 absent among the だし-keyed ones → not found
            (&dashi, &dashi_kanji, restr(&[("だし", "出し"), ("ダシ", "出汁")]), None),
            // nokanji kana reading → nil regardless of restricted
            (&dashi_kata, &dashi_k, restr(&[("ダシ", "出し")]), None),
            (&dashi_kata, &dashi_k, restr(&[]), None),
        ];
        for (kana_reading, kanji_reading, restricted, expected) in cases {
            assert_eq!(
                &match_kana_kanji(kana_reading, kanji_reading, restricted),
                expected,
                "kana={:?} kanji={:?} restricted={:?}",
                text(kana_reading), text(kanji_reading), restricted,
            );
        }
    }
}
