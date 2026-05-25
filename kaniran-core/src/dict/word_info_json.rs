//! Port of `ichiran/dict:word-info-json` (`dict.lisp:1262`).
//!
//! ```lisp
//! (defun word-info-json (word-info)
//!   (with-slots (type text true-text kana seq conjugations score components
//!                alternative primary start end counter skipped)
//!       word-info
//!     (jsown:new-js
//!       ("type" (symbol-name type))
//!       ("text" text)
//!       ("truetext" true-text)
//!       ("kana" kana)
//!       ("seq" seq)
//!       ("conjugations" (if (eql conjugations :root) "ROOT" conjugations))
//!       ("score" score)
//!       ("components" (mapcar #'word-info-json components))
//!       ("alternative" alternative)
//!       ("primary" primary)
//!       ("start" start)
//!       ("end" end)
//!       ("counter" counter)
//!       ("skipped" skipped))))
//! ```
//!
//! Returns a [`serde_json::Value`] object with key order preserved (crate
//! `preserve_order`). jsown renders CL `nil` as `[]`, so every absent/false
//! slot serializes as an empty array; `t`→`true`; `:root`→`"ROOT"`.

use serde_json::{Map, Number, Value};

use super::simple_text_class::WordConjugations;
use super::word_info_class::{WordInfo, WordInfoKana, WordInfoSeq, WordInfoType};

/// jsown renders CL `nil` as `[]`; shared empty-array sentinel.
fn nil() -> Value {
    Value::Array(Vec::new())
}

/// `(symbol-name type)` — the keyword's print name.
fn type_name(t: WordInfoType) -> &'static str {
    match t {
        WordInfoType::Kanji => "KANJI",
        WordInfoType::Kana => "KANA",
        WordInfoType::Gap => "GAP",
    }
}

fn kana_json(kana: &WordInfoKana) -> Value {
    match kana {
        WordInfoKana::Single(s) => Value::String(s.clone()),
        WordInfoKana::Multi(items) => Value::Array(
            items
                .iter()
                .map(|item| item.as_ref().map_or_else(nil, kana_json))
                .collect(),
        ),
    }
}

fn seq_json(seq: &WordInfoSeq) -> Value {
    match seq {
        WordInfoSeq::Single(n) => Value::Number(Number::from(*n)),
        WordInfoSeq::Multi(items) => Value::Array(
            items
                .iter()
                .map(|item| item.as_ref().map_or_else(nil, seq_json))
                .collect(),
        ),
    }
}

pub fn word_info_json(word_info: &WordInfo) -> Value {
    let mut js = Map::new();
    js.insert(
        "type".to_owned(),
        Value::String(type_name(word_info.kind).to_owned()),
    );
    js.insert("text".to_owned(), Value::String(word_info.text.clone()));
    js.insert(
        "truetext".to_owned(),
        word_info
            .true_text
            .as_ref()
            .map_or_else(nil, |t| Value::String(t.clone())),
    );
    js.insert(
        "kana".to_owned(),
        word_info.kana.as_ref().map_or_else(nil, kana_json),
    );
    js.insert(
        "seq".to_owned(),
        word_info.seq.as_ref().map_or_else(nil, seq_json),
    );
    js.insert(
        "conjugations".to_owned(),
        match &word_info.conjugations {
            None => nil(),
            Some(WordConjugations::Root) => Value::String("ROOT".to_owned()),
            Some(WordConjugations::Ids(ids)) => {
                Value::Array(ids.iter().map(|id| Value::Number(Number::from(*id))).collect())
            }
        },
    );
    js.insert(
        "score".to_owned(),
        word_info
            .score
            .map_or_else(nil, |n| Value::Number(Number::from(n))),
    );
    js.insert(
        "components".to_owned(),
        Value::Array(word_info.components.iter().map(word_info_json).collect()),
    );
    js.insert(
        "alternative".to_owned(),
        if word_info.alternative { Value::Bool(true) } else { nil() },
    );
    js.insert(
        "primary".to_owned(),
        if word_info.primary { Value::Bool(true) } else { nil() },
    );
    js.insert(
        "start".to_owned(),
        word_info
            .start
            .map_or_else(nil, |n| Value::Number(Number::from(n))),
    );
    js.insert(
        "end".to_owned(),
        word_info
            .end
            .map_or_else(nil, |n| Value::Number(Number::from(n))),
    );
    js.insert(
        "counter".to_owned(),
        match &word_info.counter {
            None => nil(),
            Some((value_string, ordinalp)) => Value::Array(vec![
                Value::String(value_string.clone()),
                if *ordinalp { Value::Bool(true) } else { nil() },
            ]),
        },
    );
    js.insert("skipped".to_owned(), Value::Number(Number::from(word_info.skipped)));
    Value::Object(js)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_kana(s: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(s.to_owned()))
    }

    /// REPL fixtures (.103, `jsown:to-json` of `word-info-json` on
    /// `simple-segment` output), 2026-05-25. serde_json emits raw UTF-8 where
    /// jsown emitted `\uXXXX`; the values/order are identical JSON.
    #[test]
    fn word_info_json_fixtures() {
        // 食べた — plain kanji word: single kana/seq, nil conjugations/counter,
        // primary t, alternative/truetext present.
        let tabeta = WordInfo {
            kind: WordInfoType::Kanji,
            text: "食べた".to_owned(),
            true_text: Some("食べた".to_owned()),
            kana: single_kana("たべた"),
            seq: Some(WordInfoSeq::Single(10092229)),
            score: Some(336),
            start: Some(0),
            end: Some(3),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&tabeta)).unwrap(),
            r#"{"type":"KANJI","text":"食べた","truetext":"食べた","kana":"たべた","seq":10092229,"conjugations":[],"score":336,"components":[],"alternative":[],"primary":true,"start":0,"end":3,"counter":[],"skipped":0}"#
        );

        // 5番目 — ordinal counter: counter array [value-string, t], nil truetext.
        let go_banme = WordInfo {
            kind: WordInfoType::Kanji,
            text: "5番目".to_owned(),
            true_text: None,
            kana: single_kana("ごばんめ"),
            seq: Some(WordInfoSeq::Single(1482410)),
            score: Some(667),
            start: Some(0),
            end: Some(3),
            counter: Some(("Value: 5th".to_owned(), true)),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&go_banme)).unwrap(),
            r#"{"type":"KANJI","text":"5番目","truetext":[],"kana":"ごばんめ","seq":1482410,"conjugations":[],"score":667,"components":[],"alternative":[],"primary":true,"start":0,"end":3,"counter":["Value: 5th",true],"skipped":0}"#
        );

        // 走っている — compound: Multi seq, recursive components, conjugations as
        // an id list (走って) and the :root sentinel (いる, primary nil→[]),
        // component start/end nil→[].
        let hashitteiru = WordInfo {
            kind: WordInfoType::Kanji,
            text: "走っている".to_owned(),
            true_text: None,
            kana: single_kana("はしっている"),
            seq: Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10063379)),
                Some(WordInfoSeq::Single(1577980)),
            ])),
            score: Some(406),
            components: vec![
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "走って".to_owned(),
                    true_text: Some("走って".to_owned()),
                    kana: single_kana("はしって"),
                    seq: Some(WordInfoSeq::Single(10063379)),
                    conjugations: Some(WordConjugations::Ids(vec![63591])),
                    score: Some(0),
                    start: None,
                    end: None,
                    ..WordInfo::default()
                },
                WordInfo {
                    kind: WordInfoType::Kana,
                    text: "いる".to_owned(),
                    true_text: Some("いる".to_owned()),
                    kana: single_kana("いる"),
                    seq: Some(WordInfoSeq::Single(1577980)),
                    conjugations: Some(WordConjugations::Root),
                    score: Some(0),
                    primary: false,
                    start: None,
                    end: None,
                    ..WordInfo::default()
                },
            ],
            start: Some(0),
            end: Some(5),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&hashitteiru)).unwrap(),
            r#"{"type":"KANJI","text":"走っている","truetext":[],"kana":"はしっている","seq":[10063379,1577980],"conjugations":[],"score":406,"components":[{"type":"KANJI","text":"走って","truetext":"走って","kana":"はしって","seq":10063379,"conjugations":[63591],"score":0,"components":[],"alternative":[],"primary":true,"start":[],"end":[],"counter":[],"skipped":0},{"type":"KANA","text":"いる","truetext":"いる","kana":"いる","seq":1577980,"conjugations":"ROOT","score":0,"components":[],"alternative":[],"primary":[],"start":[],"end":[],"counter":[],"skipped":0}],"alternative":[],"primary":true,"start":0,"end":5,"counter":[],"skipped":0}"#
        );

        // 何 — alternative branch: Multi kana (string list), Multi seq (int list),
        // alternative t, two components.
        let nani = WordInfo {
            kind: WordInfoType::Kanji,
            text: "何".to_owned(),
            true_text: None,
            kana: Some(WordInfoKana::Multi(vec![
                Some(WordInfoKana::Single("なに".to_owned())),
                Some(WordInfoKana::Single("なん".to_owned())),
            ])),
            seq: Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1577100)),
                Some(WordInfoSeq::Single(2846738)),
            ])),
            score: Some(24),
            components: vec![
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "何".to_owned(),
                    true_text: Some("何".to_owned()),
                    kana: single_kana("なに"),
                    seq: Some(WordInfoSeq::Single(1577100)),
                    score: Some(24),
                    start: Some(0),
                    end: Some(1),
                    ..WordInfo::default()
                },
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "何".to_owned(),
                    true_text: Some("何".to_owned()),
                    kana: single_kana("なん"),
                    seq: Some(WordInfoSeq::Single(2846738)),
                    score: Some(16),
                    start: Some(0),
                    end: Some(1),
                    ..WordInfo::default()
                },
            ],
            alternative: true,
            start: Some(0),
            end: Some(1),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&nani)).unwrap(),
            r#"{"type":"KANJI","text":"何","truetext":[],"kana":["なに","なん"],"seq":[1577100,2846738],"conjugations":[],"score":24,"components":[{"type":"KANJI","text":"何","truetext":"何","kana":"なに","seq":1577100,"conjugations":[],"score":24,"components":[],"alternative":[],"primary":true,"start":0,"end":1,"counter":[],"skipped":0},{"type":"KANJI","text":"何","truetext":"何","kana":"なん","seq":2846738,"conjugations":[],"score":16,"components":[],"alternative":[],"primary":true,"start":0,"end":1,"counter":[],"skipped":0}],"alternative":true,"primary":true,"start":0,"end":1,"counter":[],"skipped":0}"#
        );
    }
}
