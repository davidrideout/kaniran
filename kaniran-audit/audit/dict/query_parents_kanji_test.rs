//! Manual fixture-replay runner for `ICHIRAN/DICT:QUERY-PARENTS-KANJI`.
//! Source under test: `src/dict/query_parents_kanji.rs`.
//!
//! Run with:
//!   cargo run --bin query_parents_kanji_test -- \
//!       --path corpus/<corpus_tag>/dict/query_parents_kanji.parquet
//!
//! Replays a captured `(seq, text)` through `query_parents_kanji` and
//! compares the returned `(kt-id, conj-id)` rows against the Lisp
//! result. The join has no ORDER BY, so both sides are sorted first.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::readings::query_parents_kanji;

use common::CapturedRow;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:QUERY-PARENTS-KANJI";


async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!("expected 2 args, got {}", row.args.len()));
    }
    let seq = row.args[0]
        .as_i64()
        .ok_or_else(|| format!("arg 0 not int: {}", row.args[0]))? as i32;
    let text = row.args[1]
        .as_str()
        .ok_or_else(|| format!("arg 1 not string: {}", row.args[1]))?;

    let mut actual = query_parents_kanji(ctx, seq, text)
        .await
        .map_err(|err| format!("query_parents_kanji: {}", err))?;

    if row.result.len() < 1 {
        return Err(format!("expected ≥1 result value, got {}", row.result.len()));
    }
    let expected_rows = match &row.result[0] {
        Value::Null => Vec::new(),
        Value::Array(arr) => parse_pairs(arr)?,
        other => return Err(format!("result[0] not list/null: {}", other)),
    };

    let mut expected = expected_rows;
    actual.sort_unstable();
    expected.sort_unstable();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, expected))
    }
}

fn parse_pairs(arr: &[Value]) -> Result<Vec<(i32, i32)>, String> {
    arr.iter()
        .map(|v| {
            let pair = v
                .as_array()
                .ok_or_else(|| format!("row not array: {}", v))?;
            if pair.len() != 2 {
                return Err(format!("row not 2-element: {}", v));
            }
            let pid = pair[0]
                .as_i64()
                .ok_or_else(|| format!("pid not int: {}", pair[0]))? as i32;
            let cid = pair[1]
                .as_i64()
                .ok_or_else(|| format!("cid not int: {}", pair[1]))? as i32;
            Ok((pid, cid))
        })
        .collect()
}


#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
