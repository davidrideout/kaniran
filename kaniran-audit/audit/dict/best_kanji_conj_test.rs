//! Manual fixture-replay runner for `ICHIRAN/DICT:BEST-KANJI-CONJ`.
//! Source under test: `src/dict/best_kanji_conj.rs`.
//!
//! Run with:
//!   cargo run --bin best_kanji_conj_test -- \
//!       --path corpus/<corpus_tag>/dict/best_kanji_conj.parquet
//!
//! Args: `(<KANA-TEXT row>)`.
//! Result: `(<kanji string or :NULL>)` — single value, `:NULL` ↔ `None`.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::best_kanji_conj::best_kanji_conj;

use common::{captured_class, single_result, CapturedKanaText, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:BEST-KANJI-CONJ";


async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let class = captured_class(&row.args[0])?;
    if class != "KANA-TEXT" {
        return Err(format!("expected KANA-TEXT, got :{}", class));
    }
    let captured: CapturedKanaText = serde_json::from_value(row.args[0].clone())
        .map_err(|err| format!("kana-text parse: {}", err))?;
    let kana = captured.into_dao();

    let actual = best_kanji_conj(ctx, &kana)
        .await
        .map_err(|err| format!("best_kanji_conj: {}", err))?;

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
