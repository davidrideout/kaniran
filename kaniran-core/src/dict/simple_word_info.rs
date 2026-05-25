//! Port of `ichiran/dict:simple-word-info` (`dict.lisp:1282`).
//!
//! ```lisp
//! (defun simple-word-info (seq text reading type &key (as :object))
//!   (let ((obj (make-instance 'word-info
//!                :type type :text text :true-text text :seq seq :kana reading)))
//!     (cond ((eql as :object) obj)
//!           ((eql as :json) (word-info-json obj)))))
//! ```
//!
//! `:as` is a binary keyword (§4.4) selecting the return shape; the
//! keyword-discriminated return is a tagged enum (§4.1/§4.3). `:true-text` is
//! set to `text`; every other slot takes its `word-info` initform.

use serde_json::Value;

use super::word_info_class::{WordInfo, WordInfoKana, WordInfoSeq, WordInfoType};
use super::word_info_json::word_info_json;

/// The `:as` keyword — selects [`simple_word_info`]'s return shape.
#[derive(Debug, Clone, Copy)]
pub enum SimpleWordInfoAs {
    Object,
    Json,
}

/// Tagged return for [`simple_word_info`]: `:object` yields the [`WordInfo`],
/// `:json` its [`word_info_json`] serialization.
#[derive(Debug)]
pub enum KaniSimpleWordInfo {
    Object(WordInfo),
    Json(Value),
}

pub fn simple_word_info(
    seq: Option<WordInfoSeq>,
    text: &str,
    reading: Option<WordInfoKana>,
    kind: WordInfoType,
    as_: SimpleWordInfoAs,
) -> KaniSimpleWordInfo {
    let obj = WordInfo {
        kind,
        text: text.to_owned(),
        true_text: Some(text.to_owned()),
        seq,
        kana: reading,
        ..WordInfo::default()
    };
    match as_ {
        SimpleWordInfoAs::Object => KaniSimpleWordInfo::Object(obj),
        SimpleWordInfoAs::Json => KaniSimpleWordInfo::Json(word_info_json(&obj)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL fixture (.103, `simple-word-info ... :as :json`), 2026-05-25.
    #[test]
    fn simple_word_info_json() {
        let out = simple_word_info(
            Some(WordInfoSeq::Single(1234567)),
            "テスト",
            Some(WordInfoKana::Single("てすと".to_owned())),
            WordInfoType::Kana,
            SimpleWordInfoAs::Json,
        );
        let KaniSimpleWordInfo::Json(v) = out else {
            panic!("expected Json variant");
        };
        assert_eq!(
            serde_json::to_string(&v).unwrap(),
            r#"{"type":"KANA","text":"テスト","truetext":"テスト","kana":"てすと","seq":1234567,"conjugations":[],"score":0,"components":[],"alternative":[],"primary":true,"start":[],"end":[],"counter":[],"skipped":0}"#
        );
    }

    /// `:as :object` (the default) returns the constructed word-info;
    /// `:true-text` mirrors `text` and the unset slots take their initforms.
    #[test]
    fn simple_word_info_object() {
        let out = simple_word_info(
            Some(WordInfoSeq::Single(10092229)),
            "食べた",
            Some(WordInfoKana::Single("たべた".to_owned())),
            WordInfoType::Kanji,
            SimpleWordInfoAs::Object,
        );
        let KaniSimpleWordInfo::Object(wi) = out else {
            panic!("expected Object variant");
        };
        assert_eq!(wi.text, "食べた");
        assert_eq!(wi.true_text.as_deref(), Some("食べた"));
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.score, Some(0));
        assert!(wi.primary);
    }
}
