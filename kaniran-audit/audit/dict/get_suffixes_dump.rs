//! One-off: dump every failing row's args + captured Lisp result + Rust output.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::grammar::suffix::resolve::get_suffixes;
use kaniran_core::dict::dao::KanaText;

use common::{load_parquet, parse_path_arg, setup_ctx, CapturedKanaText, CapturedRow};

fn normalize_keyword(captured: &str) -> String {
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

fn diff_fields(c: &CapturedKanaText, a: &KanaText) -> Vec<String> {
    let mut diffs = Vec::new();
    if let Some(cap_id) = c.id {
        if cap_id != a.id {
            diffs.push(format!("id r={} l={}", a.id, cap_id));
        }
    }
    if c.seq != a.seq {
        diffs.push(format!("seq r={} l={}", a.seq, c.seq));
    }
    if c.text != a.text {
        diffs.push(format!("text r={:?} l={:?}", a.text, c.text));
    }
    if c.ord != a.ord {
        diffs.push(format!("ord r={} l={}", a.ord, c.ord));
    }
    if c.common != a.common {
        diffs.push(format!("common r={:?} l={:?}", a.common, c.common));
    }
    if c.common_tags != a.common_tags {
        diffs.push(format!("common_tags r={:?} l={:?}", a.common_tags, c.common_tags));
    }
    if c.conjugate_p != a.conjugate_p {
        diffs.push(format!("conjugate_p r={} l={}", a.conjugate_p, c.conjugate_p));
    }
    if c.nokanji != a.nokanji {
        diffs.push(format!("nokanji r={} l={}", a.nokanji, c.nokanji));
    }
    if c.best_kanji.as_deref() != a.best_kanji.as_deref() {
        diffs.push(format!("best_kanji r={:?} l={:?}", a.best_kanji, c.best_kanji));
    }
    if c.conjugations != a.state.conjugations {
        diffs.push(format!(
            "conjugations r={:?} l={:?}",
            a.state.conjugations, c.conjugations
        ));
    }
    if c.hintedp != a.state.hintedp {
        diffs.push(format!("hintedp r={} l={}", a.state.hintedp, c.hintedp));
    }
    diffs
}

fn compare_triple(
    idx: usize,
    captured: &Value,
    actual: (&str, &str, Option<&KanaText>),
) -> Result<(), String> {
    let triple = captured
        .as_array()
        .ok_or_else(|| format!("triple[{}] not array", idx))?;
    if triple.len() != 3 {
        return Err(format!("triple[{}] len {}", idx, triple.len()));
    }
    let cap_substr = triple[0].as_str().ok_or("suffix")?;
    let cap_key = normalize_keyword(triple[1].as_str().ok_or("keyword")?);
    let cap_kf = parse_captured_kf(&triple[2])?;
    let (rust_substr, rust_key, rust_kf) = actual;
    if rust_substr != cap_substr {
        return Err(format!("[{}] suffix r={:?} l={:?}", idx, rust_substr, cap_substr));
    }
    if rust_key != cap_key {
        return Err(format!("[{}] keyword r={:?} l={:?}", idx, rust_key, cap_key));
    }
    match (cap_kf, rust_kf) {
        (None, None) => Ok(()),
        (None, Some(a)) => Err(format!("[{}] kf r=Some(seq={}) l=null", idx, a.seq)),
        (Some(c), None) => Err(format!("[{}] kf r=None l=Some(seq={})", idx, c.seq)),
        (Some(c), Some(a)) => {
            if c.matches(a) {
                Ok(())
            } else {
                let diffs = diff_fields(&c, a);
                Err(format!("[{}] kf differs: {}", idx, diffs.join(", ")))
            }
        }
    }
}

fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    let word = row.args[0].as_str().ok_or("args[0] not string")?;
    let actual = get_suffixes(ctx, word);
    let result = row.result.first().ok_or("empty result")?;
    let empty: Vec<Value> = Vec::new();
    let expected: &[Value] = match result {
        Value::Null => &empty,
        Value::Array(a) => a.as_slice(),
        other => return Err(format!("result[0] not array or null: {}", other)),
    };
    if actual.len() != expected.len() {
        return Err(format!("len r={} l={}", actual.len(), expected.len()));
    }
    for (idx, (cap, act)) in expected.iter().zip(actual.iter()).enumerate() {
        compare_triple(idx, cap, (act.0, act.1, act.2))?;
    }
    Ok(())
}

fn rust_to_json(actual: Vec<(&str, &str, Option<&KanaText>)>) -> Vec<Value> {
    actual
        .into_iter()
        .map(|(substr, key, kf)| {
            Value::Array(vec![
                Value::String(substr.to_string()),
                Value::String(key.to_string()),
                kf.map(|k| {
                    serde_json::json!({
                        "id": k.id,
                        "seq": k.seq,
                        "text": k.text,
                        "ord": k.ord,
                        "common": k.common,
                        "common_tags": k.common_tags,
                        "conjugate_p": k.conjugate_p,
                        "nokanji": k.nokanji,
                        "best_kanji": k.best_kanji,
                        "conjugations": format!("{:?}", k.state.conjugations),
                        "hintedp": k.state.hintedp,
                    })
                })
                .unwrap_or(Value::Null),
            ])
        })
        .collect()
}

fn main() {
    let path = parse_path_arg();
    let file = load_parquet(&path);
    let ctx = setup_ctx();

    let mut fail_count: usize = 0;
    for (idx, row) in file.rows.iter().enumerate() {
        if let Err(err) = audit_one(&ctx, row) {
            fail_count += 1;
            let word = row.args[0].as_str().unwrap_or("?");
            let lisp = serde_json::to_string(&row.result).unwrap();
            let actual = get_suffixes(&ctx, word);
            let rust_json = rust_to_json(actual);
            println!("--- failure {} (row {}) ---", fail_count, idx + 1);
            println!("err:  {}", err);
            println!("word: {:?}", word);
            println!("lisp: {}", lisp);
            println!("rust: {}", serde_json::to_string(&rust_json).unwrap());
        }
    }
    eprintln!("total failures: {} / {}", fail_count, file.rows.len());
}
