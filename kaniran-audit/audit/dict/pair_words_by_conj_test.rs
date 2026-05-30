//! Manual fixture-replay runner for `ICHIRAN/DICT:PAIR-WORDS-BY-CONJ`.
//! Source under test: `src/dict/pair_words_by_conj.rs`.
//!
//! Run with:
//!   cargo run --release --bin pair_words_by_conj_test -- \
//!       --path corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/pair_words_by_conj.parquet
//!
//! Args shape (always 2 word-groups for the captured chunk-C corpus —
//! the sole upstream callsite at `dict-grammar.lisp:512` passes two
//! `find-word-with-conj-type` results):
//!   `[ [<word>, …] | null, [<word>, …] | null ]`
//! Each word is a KANA-TEXT, KANJI-TEXT, or COMPOUND-TEXT envelope (no
//! PROXY-TEXT or counter-text observed in the chunk-C corpus). `null`
//! word-group → empty list (CL `nil` projects to JSON `null`).
//!
//! Result shape: `[ [<bucket>, …] | null ]` (`(values list)`). Each
//! bucket is a 2-cell list mirroring the upstream `loop … collect nil`
//! initialiser; bucket cell `[idx]` is the word from word-group `idx`
//! sharing the conjugation key, or `null` when no word from that group
//! matched. The outer list is in SBCL hash-table iteration order —
//! non-deterministic; this runner sorts buckets canonically before
//! comparing.
//!
//! Cell equality is a deep all-fields comparison: both sides parse via
//! [`parse_captured_word`], producing the same Rust structs (full
//! fidelity of every projected slot — `id`, `seq`, `text`, `ord`,
//! `common`, `common_tags`, `conjugate_p`, `nokanji`, `best_*`,
//! `conjugations`, `hintedp` for simple-text; `text`, `kana`,
//! `primary`, `words`, `score_base`, `score_mod` for compound-text).
//! The fingerprint is `format!("{:?}", word)` — `Debug` on the word
//! enum visits every field recursively, so any divergence on any slot
//! surfaces as a fingerprint mismatch.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::kani::KaniWordDispatchEnum;
use kaniran_core::dict::grammar::find_word::pair_words_by_conj;

use common::{parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:PAIR-WORDS-BY-CONJ";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    let mut word_groups: Vec<Vec<KaniWordDispatchEnum>> = Vec::with_capacity(row.args.len());
    for (g_idx, group_val) in row.args.iter().enumerate() {
        let group = parse_group(group_val)
            .map_err(|e| format!("arg {} (word-group): {}", g_idx, e))?;
        word_groups.push(group);
    }

    let actual = pair_words_by_conj(ctx, &word_groups)
        .await
        .map_err(|e| format!("pair_words_by_conj: {}", e))?;

    let expected_value = unwrap_result(&row.result)?;
    let expected = parse_captured_buckets(expected_value, word_groups.len())?;

    let actual_fp = canonical_fingerprints(&actual);
    let expected_fp = canonical_fingerprints(&expected);

    if actual_fp != expected_fp {
        return Err(format!(
            "buckets mismatch:\n  rust ={:?}\n  lisp ={:?}",
            actual_fp, expected_fp
        ));
    }
    Ok(())
}

fn parse_group(value: &Value) -> Result<Vec<KaniWordDispatchEnum>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for (idx, w) in arr.iter().enumerate() {
                out.push(
                    parse_captured_word(w).map_err(|e| format!("entry {}: {}", idx, e))?,
                );
            }
            Ok(out)
        }
        other => Err(format!("expected array or null, got {}", other)),
    }
}

fn unwrap_result(result: &[Value]) -> Result<&Value, String> {
    if result.is_empty() {
        return Err("expected at least 1 result value, got 0".into());
    }
    Ok(&result[0])
}

fn parse_captured_buckets(
    value: &Value,
    expected_arity: usize,
) -> Result<Vec<Vec<Option<KaniWordDispatchEnum>>>, String> {
    let arr: Vec<&Value> = match value {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr.iter().collect(),
        other => return Err(format!("result outer: expected array/null, got {}", other)),
    };
    let mut rows: Vec<Vec<Option<KaniWordDispatchEnum>>> = Vec::with_capacity(arr.len());
    for (b_idx, bucket) in arr.iter().enumerate() {
        let bucket_arr = bucket
            .as_array()
            .ok_or_else(|| format!("bucket {} not array: {}", b_idx, bucket))?;
        if bucket_arr.len() != expected_arity {
            return Err(format!(
                "bucket {} arity {} ≠ word-groups {}",
                b_idx,
                bucket_arr.len(),
                expected_arity
            ));
        }
        let mut row = Vec::with_capacity(bucket_arr.len());
        for (c_idx, cell) in bucket_arr.iter().enumerate() {
            row.push(if cell.is_null() {
                None
            } else {
                Some(
                    parse_captured_word(cell)
                        .map_err(|e| format!("bucket {} cell {}: {}", b_idx, c_idx, e))?,
                )
            });
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Bucket fingerprint covering every projected field: both sides build
/// `KaniWordDispatchEnum` values via [`parse_captured_word`], so a
/// derived `Debug` impl walks each slot identically and `format!` gives
/// a deterministic comparable string. Sorted across buckets to absorb
/// SBCL's non-deterministic hash-table iteration order.
fn canonical_fingerprints(
    buckets: &[Vec<Option<KaniWordDispatchEnum>>],
) -> Vec<Vec<Option<String>>> {
    let mut fps: Vec<Vec<Option<String>>> = buckets
        .iter()
        .map(|bucket| {
            bucket
                .iter()
                .map(|cell| cell.as_ref().map(|w| format!("{:?}", w)))
                .collect()
        })
        .collect();
    fps.sort();
    fps
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
