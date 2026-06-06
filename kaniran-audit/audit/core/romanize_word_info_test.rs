//! Manual fixture-replay runner for `ICHIRAN:ROMANIZE-WORD-INFO`.
//! Source under test: `src/core/romanize_word_info.rs`.
//!
//! Run with:
//!   cargo run --bin romanize_word_info_test -- \
//!       --path corpus/<corpus_tag>/romanize_word_info.parquet
//!
//! Captured args are `[<word-info>, ":METHOD", <method>]`; the one result
//! value is the romanized (or hint-stripped kana) string.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::kani_romanize_method::KaniRomanizeMethod;
use kaniran_core::core::romanize_word_info::romanize_word_info;
use kaniran_core::dict::word_info_class::WordInfo;

use common::{
    get_string, parse_captured_kana, run_sync_streaming, single_result, with_captured_method,
    CapturedRow,
};
use serde_json::Value;

const EXPECTED_FQN: &str = "ICHIRAN:ROMANIZE-WORD-INFO";

fn kw_value<'a>(args: &'a [Value], keyword: &str) -> Option<&'a Value> {
    args.iter()
        .position(|v| v.as_str() == Some(keyword))
        .and_then(|idx| args.get(idx + 1))
}

fn check(actual: String, expected: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {actual:?}\n  lisp: {expected:?}"))
    }
}

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let wi_value = row.args.first().ok_or("missing word-info arg")?;
    let method_value = kw_value(&row.args, ":METHOD").ok_or("missing :METHOD")?;

    let text = get_string(wi_value, "text");
    let kana = parse_captured_kana(wi_value.get("kana").ok_or("missing kana slot")?)?;
    let word_info = WordInfo { text, kana, ..Default::default() };

    let expected = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?
        .to_string();

    if method_value.as_str() == Some(":KANA") {
        return check(
            romanize_word_info(&word_info, KaniRomanizeMethod::Kana),
            &expected,
        );
    }
    with_captured_method(method_value, |method| {
        check(
            romanize_word_info(&word_info, KaniRomanizeMethod::Method(method)),
            &expected,
        )
    })
}

fn main() {
    run_sync_streaming(EXPECTED_FQN, audit_one);
}
