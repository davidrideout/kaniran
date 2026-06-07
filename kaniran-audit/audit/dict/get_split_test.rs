//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-SPLIT`.
//! Source under test: `src/dict/get_split.rs`.
//!
//! Run with:
//!   cargo run --bin get_split_test -- \
//!       --path corpus/<corpus_tag>/get_split.parquet
//!
//! Replays a captured reading (plus conj-of seqs) through `get_split`
//! and compares the returned split parts against the Lisp result.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::split::split::get_split;
use kaniran_core::dict::split::kani_split_part::SplitPart;
use kaniran_core::dict::kani_word::KaniWordDispatchEnum;

use common::{parse_captured_simple_text, CapturedKanaText, CapturedKanjiText, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-SPLIT";


async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!("expected 2 args, got {}", row.args.len()));
    }

    let reading = parse_captured_simple_text(&row.args[0])?;

    let conj_of: Vec<i32> = match &row.args[1] {
        Value::Null => Vec::new(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_i64()
                    .map(|int_val| int_val as i32)
                    .ok_or_else(|| format!("conj-of element not int: {}", item))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("conj-of: expected null/array, got {}", other)),
    };

    let actual = get_split(ctx, &reading, &conj_of)
        .await
        .map_err(|err| format!("get_split query: {}", err))?;

    // Result envelope: [null] for Lisp NIL, [[parts...], score] for a hit.
    if row.result.len() == 1 && row.result[0].is_null() {
        return match actual {
            None => Ok(()),
            Some(_) => Err("rust=Some lisp=null result".to_string()),
        };
    }

    if row.result.len() != 2 {
        return Err(format!(
            "expected 1 or 2 result values, got {}",
            row.result.len()
        ));
    }

    let expected_parts_value = &row.result[0];
    let expected_score = row.result[1]
        .as_i64()
        .ok_or_else(|| format!("score not int: {}", row.result[1]))?
        as i32;

    let (actual_parts, actual_score) = actual.ok_or("rust=None lisp=Some result".to_string())?;

    if actual_score != expected_score {
        return Err(format!(
            "score: rust={} lisp={}",
            actual_score, expected_score
        ));
    }

    let expected_parts = expected_parts_value
        .as_array()
        .ok_or_else(|| format!("expected parts not array: {}", expected_parts_value))?;

    if actual_parts.len() != expected_parts.len() {
        return Err(format!(
            "parts count: rust={} lisp={}",
            actual_parts.len(),
            expected_parts.len()
        ));
    }

    for (idx, (actual_part, expected_part)) in
        actual_parts.iter().zip(expected_parts.iter()).enumerate()
    {
        compare_split_part(actual_part, expected_part)
            .map_err(|err| format!("part {}: {}", idx, err))?;
    }

    Ok(())
}

fn compare_split_part(actual: &SplitPart, expected: &Value) -> Result<(), String> {
    if let Value::String(string_value) = expected {
        return match (string_value.as_str(), actual) {
            (":SCORE", SplitPart::Score) => Ok(()),
            (":PSCORE", SplitPart::PScore) => Ok(()),
            (":SCORE", _) => Err(format!("expected :SCORE, got {:?}", actual)),
            (":PSCORE", _) => Err(format!("expected :PSCORE, got {:?}", actual)),
            (other, _) => Err(format!("unexpected string in part: {}", other)),
        };
    }

    let class = expected
        .pointer("/_meta/class")
        .and_then(|class_value| class_value.as_str())
        .ok_or_else(|| format!("part missing _meta.class: {}", expected))?;

    match (actual, class) {
        (SplitPart::Word(KaniWordDispatchEnum::Kana(actual_kana)), "KANA-TEXT") => {
            let captured: CapturedKanaText = serde_json::from_value(expected.clone())
                .map_err(|err| format!("kana-text parse: {}", err))?;
            if !captured.matches(actual_kana) {
                return Err(format!(
                    "kana-text mismatch:\n  rust: {:?}\n  lisp: {:?}",
                    actual_kana, captured
                ));
            }
            Ok(())
        }
        (SplitPart::Word(KaniWordDispatchEnum::Kanji(actual_kanji)), "KANJI-TEXT") => {
            let captured: CapturedKanjiText = serde_json::from_value(expected.clone())
                .map_err(|err| format!("kanji-text parse: {}", err))?;
            if !captured.matches(actual_kanji) {
                return Err(format!(
                    "kanji-text mismatch:\n  rust: {:?}\n  lisp: {:?}",
                    actual_kanji, captured
                ));
            }
            Ok(())
        }
        (SplitPart::Word(other_variant), other_class) => Err(format!(
            "class/variant mismatch: rust={:?} lisp=:{}",
            other_variant, other_class
        )),
        (SplitPart::Score, _) | (SplitPart::PScore, _) => Err(format!(
            "rust returned :SCORE/:PSCORE marker, lisp returned word object"
        )),
    }
}


#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
