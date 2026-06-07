//! Manual fixture-replay runner for `ICHIRAN/DICT:FIND-SUBSTRING-WORDS`.
//! Source under test: `src/dict/find_substring_words.rs`.
//!
//! Run with:
//!   cargo run --release --bin find_substring_words_test -- \
//!       --path corpus/find_substring_words_2026_05_26/dict/find_substring_words.parquet
//!
//! Replays the substring-hash built by `find_substring_words` against
//! the captured Lisp bucket map. Bucket rows are compared in order.

#[path = "../common/mod.rs"]
mod common;

use std::collections::HashMap;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::readings::find_substring_words;
use kaniran_core::dict::readings::FindWordRows;
use kaniran_core::dict::dao::KanaText;
use kaniran_core::dict::dao::KanjiText;

use common::{single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FIND-SUBSTRING-WORDS";

/// Order-preserving fingerprint of one bucketed row. Every captured data
/// field except `id`; `table` distinguishes the kana / kanji variant.
#[derive(Debug, PartialEq, Eq)]
struct RowFp {
    table: &'static str,
    seq: i32,
    ord: i32,
    common: Option<i32>,
    common_tags: String,
    conjugate_p: bool,
    nokanji: bool,
    best: Option<String>,
    text: String,
}

fn fp_from_kana(k: &KanaText) -> RowFp {
    RowFp {
        table: "kana-text",
        seq: k.seq,
        ord: k.ord,
        common: k.common,
        common_tags: k.common_tags.clone(),
        conjugate_p: k.conjugate_p,
        nokanji: k.nokanji,
        best: k.best_kanji.clone(),
        text: k.text.clone(),
    }
}

fn fp_from_kanji(k: &KanjiText) -> RowFp {
    RowFp {
        table: "kanji-text",
        seq: k.seq,
        ord: k.ord,
        common: k.common,
        common_tags: k.common_tags.clone(),
        conjugate_p: k.conjugate_p,
        nokanji: k.nokanji,
        best: k.best_kana.clone(),
        text: k.text.clone(),
    }
}

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    // --- args: [str, ":STICKY", null | [positions]] ---
    if row.args.len() != 3 {
        return Err(format!(
            "expected 3 args [str, :STICKY, sticky], got {}",
            row.args.len()
        ));
    }
    let str = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 (str) not string: {}", row.args[0]))?;
    match row.args[1].as_str() {
        Some(":STICKY") => {}
        other => return Err(format!("arg 1 not :STICKY keyword: {:?}", other)),
    }
    let sticky = parse_sticky(&row.args[2])?;

    let h = find_substring_words(ctx, str, &sticky)
        .await
        .map_err(|err| format!("find_substring_words query: {}", err))?;

    // --- actual: hash -> map<key, ordered RowFp vec> ---
    let actual: HashMap<&str, Vec<RowFp>> = h
        .iter()
        .map(|(k, rows)| {
            let v = match rows {
                FindWordRows::Kana(rs) => rs.iter().map(fp_from_kana).collect(),
                FindWordRows::Kanji(rs) => rs.iter().map(fp_from_kanji).collect(),
            };
            (k.as_str(), v)
        })
        .collect();

    // --- expected: the captured hash-as-object ---
    let expected_value = single_result(&row.result)?;
    let expected_obj = expected_value
        .as_object()
        .ok_or_else(|| format!("result not a JSON object: {}", expected_value))?;
    let mut expected: HashMap<&str, Vec<RowFp>> = HashMap::with_capacity(expected_obj.len());
    for (key, bucket) in expected_obj {
        let rows = match bucket {
            Value::Null => Vec::new(),
            Value::Array(arr) => arr
                .iter()
                .enumerate()
                .map(|(i, r)| fp_from_json(r).map_err(|e| format!("key {:?} row {}: {}", key, i, e)))
                .collect::<Result<Vec<_>, _>>()?,
            other => return Err(format!("key {:?} bucket not null/array: {}", key, other)),
        };
        expected.insert(key.as_str(), rows);
    }

    // --- key sets (order-insensitive) ---
    if actual.len() != expected.len() {
        return Err(format!(
            "key count: rust={} lisp={} (rust-only: {:?}, lisp-only: {:?})",
            actual.len(),
            expected.len(),
            key_diff(&actual, &expected),
            key_diff(&expected, &actual),
        ));
    }
    for key in expected.keys() {
        if !actual.contains_key(key) {
            return Err(format!(
                "key {:?} present in lisp, absent in rust (rust-only: {:?})",
                key,
                key_diff(&actual, &expected),
            ));
        }
    }

    // --- per-bucket order-sensitive compare ---
    for (key, expected_rows) in &expected {
        let actual_rows = &actual[key];
        if actual_rows != expected_rows {
            return Err(format!(
                "bucket {:?} differs (str={:?}):\n  rust seqs = {:?}\n  lisp seqs = {:?}\n  rust = {:#?}\n  lisp = {:#?}",
                key,
                str,
                actual_rows.iter().map(|r| r.seq).collect::<Vec<_>>(),
                expected_rows.iter().map(|r| r.seq).collect::<Vec<_>>(),
                actual_rows,
                expected_rows,
            ));
        }
    }
    Ok(())
}

fn key_diff<'a>(a: &HashMap<&'a str, Vec<RowFp>>, b: &HashMap<&str, Vec<RowFp>>) -> Vec<&'a str> {
    let mut only: Vec<&str> = a.keys().filter(|k| !b.contains_key(*k)).copied().collect();
    only.sort();
    only
}

fn parse_sticky(v: &Value) -> Result<Vec<usize>, String> {
    match v {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr
            .iter()
            .map(|p| {
                p.as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| format!("sticky position not a non-negative int: {}", p))
            })
            .collect(),
        other => Err(format!("sticky not null/array: {}", other)),
    }
}

fn fp_from_json(v: &Value) -> Result<RowFp, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| format!("row not an object: {}", v))?;
    let table = match obj.get("table").and_then(Value::as_str) {
        Some("kana-text") => "kana-text",
        Some("kanji-text") => "kanji-text",
        other => return Err(format!("row table not kana-text/kanji-text: {:?}", other)),
    };
    let best_key = if table == "kana-text" {
        "best_kanji"
    } else {
        "best_kana"
    };
    // Guard against capture drift: reject any unexpected slot.
    for key in obj.keys() {
        match key.as_str() {
            "table" | "id" | "seq" | "text" | "ord" | "common" | "common_tags"
            | "conjugate_p" | "nokanji" => {}
            k if k == best_key => {}
            other => return Err(format!("unhandled row slot: {} (capture drift)", other)),
        }
    }
    Ok(RowFp {
        table,
        seq: get_i32(obj, "seq")?,
        ord: get_i32(obj, "ord")?,
        common: parse_nullable_i32(obj.get("common"), "common")?,
        common_tags: get_str(obj, "common_tags")?,
        conjugate_p: parse_proj_bool(obj.get("conjugate_p"), "conjugate_p")?,
        nokanji: parse_proj_bool(obj.get("nokanji"), "nokanji")?,
        best: parse_nullable_str(obj.get(best_key), best_key)?,
        text: get_str(obj, "text")?,
    })
}

fn get_i32(obj: &serde_json::Map<String, Value>, key: &str) -> Result<i32, String> {
    obj.get(key)
        .and_then(Value::as_i64)
        .map(|n| n as i32)
        .ok_or_else(|| format!("{}: not an integer ({:?})", key, obj.get(key)))
}

fn get_str(obj: &serde_json::Map<String, Value>, key: &str) -> Result<String, String> {
    obj.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("{}: not a string ({:?})", key, obj.get(key)))
}

/// `Option<i32>` column: SQL NULL renders as the string `":NULL"` (and
/// defensively as JSON `null`); an integer renders as itself.
fn parse_nullable_i32(v: Option<&Value>, key: &str) -> Result<Option<i32>, String> {
    match v {
        Some(Value::String(s)) if s == ":NULL" => Ok(None),
        Some(Value::Null) | None => Ok(None),
        Some(Value::Number(n)) => Ok(Some(
            n.as_i64().ok_or_else(|| format!("{}: not i64 ({})", key, n))? as i32,
        )),
        Some(other) => Err(format!("{}: expected int/:NULL, got {}", key, other)),
    }
}

/// `Option<String>` column (`best_kanji` / `best_kana`): `":NULL"` (or
/// JSON `null`) is None, any other string is Some.
fn parse_nullable_str(v: Option<&Value>, key: &str) -> Result<Option<String>, String> {
    match v {
        Some(Value::String(s)) if s == ":NULL" => Ok(None),
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!("{}: expected string/:NULL, got {}", key, other)),
    }
}

/// Projected boolean slot: `t` renders as JSON `true`, `nil` as JSON
/// `null` (the projector's nil-boolean encoding).
fn parse_proj_bool(v: Option<&Value>, key: &str) -> Result<bool, String> {
    match v {
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::Null) | None => Ok(false),
        Some(Value::String(s)) if s == ":NULL" => Ok(false),
        Some(other) => Err(format!("{}: expected bool/null, got {}", key, other)),
    }
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
