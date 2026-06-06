//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-HINT`.
//! Source under test: `src/dict/get_hint.rs`.
//!
//! Run with:
//!   cargo run --bin get_hint_test -- \
//!       --path corpus/<corpus_tag>/dict/get_hint.parquet
//!
//! Replays captured reading rows through `get_hint` and compares the
//! returned hint kana string against the Lisp result.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::get_hint::get_hint;

use common::{
    describe_word, extract_disable_hints_meta, parse_captured_word, single_result, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-HINT";


async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!(
            "expected 2 args (reading + hint-state meta), got {}",
            row.args.len()
        ));
    }
    let disable_hints = extract_disable_hints_meta(&row.args[1])
        .ok_or_else(|| format!("missing _meta.context.disable_hints on args[1]: {}", row.args[1]))?;
    let reading = parse_captured_word(&row.args[0])?;

    let ctx2 = ctx.with_disable_hints(disable_hints);
    let actual = get_hint(&ctx2, &reading)
        .await
        .map_err(|err| format!("get_hint: {} ({})", err, describe_word(&reading)))?;

    let result = single_result(&row.result)?;
    let expected: Option<String> = match result {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => return Err(format!("result[0] not string/null: {}", other)),
    };

    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "input={} disable_hints={}\n  rust: {:?}\n  lisp: {:?}",
            describe_word(&reading),
            disable_hints,
            actual,
            expected,
        ))
    }
}


#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
