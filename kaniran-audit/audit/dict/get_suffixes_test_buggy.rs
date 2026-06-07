//! Fixture-replay runner for `ICHIRAN/DICT:GET-SUFFIXES`. **Archived
//! 2026-05-15** — renamed `_buggy` and removed from `Cargo.toml`'s
//! `[[bin]]` list; not built or run by default. Retained as evidence
//! of upstream nondeterminism (see `docs/known_issues.md`).
//!
//! Source under test: `src/dict/get_suffixes.rs`.
//!
//! Replays captured words through `get_suffixes` and compares the
//! returned suffix triples against the Lisp result. Against the
//! 2026-05-15 parquet its 504 failures (~0.13%) are all explained by
//! two known issues (unordered query results; mid-session state in
//! ichiran), none Rust port bugs — diff against those signatures
//! before declaring a regression.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::grammar::suffix::resolve::get_suffixes;
use kaniran_core::dict::kana_text_dao::KanaText;

use common::{single_result, CapturedKanaText, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-SUFFIXES";

fn normalize_keyword(captured: &str) -> String {
    // Captures emit `:TEIRU`; the Rust cache stores `teiru`. Strip the
    // leading `:` and lowercase to compare.
    captured.trim_start_matches(':').to_ascii_lowercase()
}

fn parse_captured_kf(value: &Value) -> Result<Option<CapturedKanaText>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s == ":NULL" => Ok(None),
        Value::Object(_) => serde_json::from_value::<CapturedKanaText>(value.clone())
            .map(Some)
            .map_err(|err| format!("kana-text parse: {}", err)),
        other => Err(format!("expected kana-text object or null, got: {}", other)),
    }
}

fn compare_triple(
    idx: usize,
    captured: &Value,
    actual: (&str, &str, Option<&KanaText>),
) -> Result<(), String> {
    let triple = captured
        .as_array()
        .ok_or_else(|| format!("triple[{}] not array: {}", idx, captured))?;
    if triple.len() != 3 {
        return Err(format!(
            "triple[{}] expected 3 elements, got {}: {}",
            idx,
            triple.len(),
            captured
        ));
    }
    let cap_substr = triple[0]
        .as_str()
        .ok_or_else(|| format!("triple[{}].suffix not string: {}", idx, triple[0]))?;
    let cap_key_raw = triple[1]
        .as_str()
        .ok_or_else(|| format!("triple[{}].keyword not string: {}", idx, triple[1]))?;
    let cap_key = normalize_keyword(cap_key_raw);
    let cap_kf = parse_captured_kf(&triple[2])?;

    let (rust_substr, rust_key, rust_kf) = actual;
    if rust_substr != cap_substr {
        return Err(format!(
            "triple[{}].suffix mismatch: rust={:?} lisp={:?}",
            idx, rust_substr, cap_substr
        ));
    }
    if rust_key != cap_key {
        return Err(format!(
            "triple[{}].keyword mismatch: rust={:?} lisp={:?}",
            idx, rust_key, cap_key
        ));
    }
    match (cap_kf, rust_kf) {
        (None, None) => Ok(()),
        (None, Some(actual_kf)) => Err(format!(
            "triple[{}].kf: rust=Some(seq={}) lisp=null",
            idx, actual_kf.seq
        )),
        (Some(captured_kf), None) => Err(format!(
            "triple[{}].kf: rust=None lisp=Some(seq={})",
            idx, captured_kf.seq
        )),
        (Some(captured_kf), Some(actual_kf)) => {
            if captured_kf.matches(actual_kf) {
                Ok(())
            } else {
                Err(format!(
                    "triple[{}].kf field mismatch (rust seq={}, lisp seq={})",
                    idx, actual_kf.seq, captured_kf.seq
                ))
            }
        }
    }
}

async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let word = row.args[0]
        .as_str()
        .ok_or_else(|| format!("args[0] not string: {}", row.args[0]))?;

    let actual = get_suffixes(ctx, word);

    let result = single_result(&row.result)?;
    // Lisp NIL is both null and empty list; capture serialises it as
    // `null`. Treat it as an empty expected sequence.
    let empty: Vec<Value> = Vec::new();
    let expected: &[Value] = match result {
        Value::Null => &empty,
        Value::Array(a) => a.as_slice(),
        other => return Err(format!("result[0] not array or null: {}", other)),
    };

    if actual.len() != expected.len() {
        return Err(format!(
            "length mismatch: rust={} lisp={}",
            actual.len(),
            expected.len()
        ));
    }
    for (idx, (cap, act)) in expected.iter().zip(actual.iter()).enumerate() {
        compare_triple(idx, cap, (act.0, act.1, act.2))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
