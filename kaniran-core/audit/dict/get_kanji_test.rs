//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-KANJI`.
//! Source under test: `src/dict/get_kanji.rs` + `Entry::get_kanji`.
//!
//! Run with:
//!   cargo run --bin get_kanji_test -- \
//!       --path corpus/<corpus_tag>/dict/get_kanji.parquet
//!
//! Args: `(<word-or-entry-row>)`.
//! Result: `(<kanji string or nil>)` — single value; `nil` ↔ `None`.
//!
//! The runner is async because the KANA-TEXT branch reaches the
//! database through [`best_kanji_conj`]. ENTRY input is skipped — the
//! captured envelope for ENTRY doesn't yet have a typed deserializer
//! in [`common`]; the ENTRY method is exercised separately when an
//! Entry-targeted parquet is captured.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::get_kanji::get_kanji;

use common::{captured_class, parse_captured_word, single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-KANJI";


async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let class = captured_class(&row.args[0])?;
    if class == "ENTRY" {
        // entry method covered by a separate audit when ENTRY fixtures land.
        return Ok(());
    }

    let word = parse_captured_word(&row.args[0])?;
    let actual = get_kanji(ctx, &word)
        .await
        .map_err(|err| format!("get_kanji: {}", err))?;

    let result = single_result(&row.result)?;
    let expected: Option<String> = match result {
        Value::Null => None,
        Value::String(s) if s == ":NULL" => None,
        Value::String(s) => Some(s.clone()),
        other => return Err(format!("result[0] not string/null/:NULL: {}", other)),
    };

    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, expected))
    }
}


#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
