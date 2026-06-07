//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-NON-ARCH-POSI`.
//! Source under test: `src/dict/get_non_arch_posi.rs`.
//!
//! Run with:
//!   cargo run --bin get_non_arch_posi_test -- \
//!       --path corpus/<corpus_tag>/dict/get_non_arch_posi.parquet
//!
//! Replays a captured list of seqs through `get_non_arch_posi` and
//! compares the returned non-archaic part-of-speech strings against
//! the Lisp result (sorted on each side, since the query has no
//! ORDER BY).

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::scoring::score::get_non_arch_posi;

use common::CapturedRow;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-NON-ARCH-POSI";

fn parse_seq_set(v: &Value) -> Result<Vec<i32>, String> {
    let arr = v
        .as_array()
        .ok_or_else(|| format!("seq-set: expected array, got {}", v))?;
    let mut out = Vec::with_capacity(arr.len());
    for elt in arr {
        let i = elt
            .as_i64()
            .ok_or_else(|| format!("seq-set element not i64: {}", elt))?;
        out.push(i as i32);
    }
    Ok(out)
}

fn parse_pos_strings(v: &Value) -> Result<Vec<String>, String> {
    match v {
        // Lisp NIL — empty result list.
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for elt in arr {
                let s = elt
                    .as_str()
                    .ok_or_else(|| format!("pos string not string: {}", elt))?;
                out.push(s.to_string());
            }
            Ok(out)
        }
        other => Err(format!("pos list: expected null or array, got {}", other)),
    }
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    if row.result.len() != 2 {
        return Err(format!(
            "expected 2 result values (list, count), got {}",
            row.result.len()
        ));
    }
    let seq_set = parse_seq_set(&row.args[0])?;
    let lisp_list = parse_pos_strings(&row.result[0])?;
    let lisp_count = row.result[1]
        .as_i64()
        .ok_or_else(|| format!("count not i64: {}", row.result[1]))?;

    // Field 2: count tautology check (postmodern contract). Detects
    // projector drift on the secondary-value side.
    if lisp_count != lisp_list.len() as i64 {
        return Err(format!(
            "\n  seq-set: {:?}\n  capture invariant violated: \
             lisp count={} ≠ lisp list len={}",
            seq_set,
            lisp_count,
            lisp_list.len()
        ));
    }

    // Field 1: list comparison. Sort both sides — upstream DISTINCT
    // imposes no ORDER BY.
    let rust_list = get_non_arch_posi(ctx, &seq_set)
        .await
        .map_err(|e| format!("rust query error: {}", e))?;

    let rust_sorted = sorted(rust_list);
    let lisp_sorted = sorted(lisp_list);
    if rust_sorted != lisp_sorted {
        return Err(format!(
            "\n  seq-set: {:?}\n  rust: {:?}\n  lisp: {:?}",
            seq_set, rust_sorted, lisp_sorted
        ));
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await
}
