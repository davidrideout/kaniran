//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-TEXT`.
//! Source under test: `src/dict/get_text.rs` + `src/dict/segment_struct.rs::Segment::get_text`.
//!
//! Run with:
//!   cargo run --bin get_text_test -- \
//!       --path corpus/<corpus_tag>/dict/get_text.parquet
//!
//! Two surfaces tested by the same fixture:
//! - SEGMENT input → `Segment::get_text()` (lazy memoization).
//! - word-shaped input (KANA-TEXT / KANJI-TEXT / COMPOUND-TEXT /
//!   counter family / proxy) → free-fn `get_text(&word)`.
//!
//! ENTRY input is skipped — the entry method needs DB context (not
//! plumbed through this sync runner).

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::get_text::get_text;
use kaniran_core::dict::segment_struct::Segment;

use common::{captured_class, get_usize, parse_captured_word, single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-TEXT";


fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let class = captured_class(&row.args[0])?;
    if class == "ENTRY" {
        // entry method needs DB; out of scope.
        return Ok(());
    }

    let actual: String = if class == "SEGMENT" {
        let word_value = row.args[0]
            .get("word")
            .ok_or_else(|| "segment missing word".to_string())?;
        let word = parse_captured_word(word_value)?;
        let start = get_usize(&row.args[0], "start");
        let end = get_usize(&row.args[0], "end");
        // text: load-bearing for get-text's lazy-memoization short-circuit
        // (segment_struct.rs:73). Plumb through so cached rows replay the
        // cache hit instead of the fresh-compute branch.
        let text = match row.args[0].get("text") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(other) => return Err(format!("segment.text not string/null: {}", other)),
        };
        // score / info / top: not read by Segment::get_text (impl only
        // touches self.text and self.word). Leaving as None.
        let mut seg = Segment {
            start,
            end,
            word,
            score: None,
            info: None,
            top: None,
            text,
        };
        seg.get_text().to_string()
    } else {
        let word = parse_captured_word(&row.args[0])?;
        get_text(&word).into_owned()
    };

    let result = single_result(&row.result)?;
    let expected = result
        .as_str()
        .ok_or_else(|| format!("result[0] not string: {}", result))?;

    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, expected))
    }
}


fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
