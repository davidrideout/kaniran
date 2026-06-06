//! Manual fixture-replay runner for `ICHIRAN/CHARACTERS:NORMALIZE`.
//! Source under test: `src/characters/normalize.rs`.
//!
//! Run with:
//!   cargo run --bin normalize_test -- \
//!       --path corpus/<corpus_tag>/normalize.parquet

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::characters::normalize::normalize;
use kaniran_core::characters::to_normal_char::NormalizationContext;

use common::{single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/CHARACTERS:NORMALIZE";


fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let input = match row.args.first() {
        Some(Value::String(s)) => s.as_str(),
        Some(other) => return Err(format!("arg 0: expected string, got {}", other)),
        None => return Err("expected at least 1 arg".to_string()),
    };

    let context = parse_context_keyword(&row.args[1..])?;

    let actual = normalize(input, context);

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

/// Walk `:CONTEXT :KANA` / `:CONTEXT :DEFAULT` from the keyword tail.
/// Empty tail → `Default` (the upstream default).
fn parse_context_keyword(tail: &[Value]) -> Result<NormalizationContext, String> {
    if tail.is_empty() {
        return Ok(NormalizationContext::Default);
    }
    if tail.len() != 2 {
        return Err(format!("keyword tail wants 2 elements, got {}", tail.len()));
    }
    let key = tail[0]
        .as_str()
        .ok_or_else(|| format!("keyword name not string: {}", tail[0]))?;
    if key != ":CONTEXT" {
        return Err(format!("expected :CONTEXT, got {}", key));
    }
    let value = tail[1]
        .as_str()
        .ok_or_else(|| format!("keyword value not string: {}", tail[1]))?;
    match value {
        ":DEFAULT" => Ok(NormalizationContext::Default),
        ":KANA" => Ok(NormalizationContext::Kana),
        other => Err(format!("unknown NormalizationContext: {}", other)),
    }
}


fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
