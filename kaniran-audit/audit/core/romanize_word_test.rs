//! Manual fixture-replay runner for `ICHIRAN:ROMANIZE-WORD`.
//! Source under test: `src/core/romanize_word.rs`.
//!
//! Run with:
//!   cargo run --bin romanize_word_test -- \
//!       --path corpus/<corpus_tag>/romanize_word.parquet
//!
//! Captured args are `[<word>, ":METHOD", <method>, ":ORIGINAL-SPELLING",
//! <orig>, ":NORMALIZE", <normalize>]`; the one result value is the
//! romanized string.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::romanize::romanize_word;

use common::{run_sync_streaming, single_result, with_captured_method, CapturedRow};
use serde_json::Value;

const EXPECTED_FQN: &str = "ICHIRAN:ROMANIZE-WORD";

/// Locate the value following a `&key` keyword symbol in the captured args.
fn kw_value<'a>(args: &'a [Value], keyword: &str) -> Option<&'a Value> {
    args.iter()
        .position(|v| v.as_str() == Some(keyword))
        .and_then(|idx| args.get(idx + 1))
}

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let word = row
        .args
        .first()
        .and_then(|v| v.as_str())
        .ok_or("arg 0 (word) not a string")?;
    let method_value = kw_value(&row.args, ":METHOD").ok_or("missing :METHOD")?;

    let original_spelling = match kw_value(&row.args, ":ORIGINAL-SPELLING") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(other) => return Err(format!("original-spelling not string/null: {other}")),
    };
    let normalize = match kw_value(&row.args, ":NORMALIZE") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(other) => return Err(format!("normalize not bool/null: {other}")),
    };

    let expected = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?
        .to_string();

    with_captured_method(method_value, |method| {
        let actual = romanize_word(word, method, original_spelling, normalize);
        if actual == expected {
            Ok(())
        } else {
            Err(format!("\n  rust: {actual:?}\n  lisp: {expected:?}"))
        }
    })
}

fn main() {
    run_sync_streaming(EXPECTED_FQN, audit_one);
}
