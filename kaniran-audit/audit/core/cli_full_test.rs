//! Fixture-replay runner for the e2e complete-result JSON — ichiran's
//! `ichiran-cli --full` output, captured via the synthetic entry point
//! `CL-USER::CLI-FULL` = `(jsown:to-json (romanize* text :limit 5))`.
//!
//! Run with:
//!   cargo run --release --bin cli_full_test -- \
//!     --path corpus/extracted_romanize_json_2026_05_25/cl-user/cli_full.parquet

#[path = "../common/mod.rs"]
mod common;

// mimalloc over macOS system malloc: measured 1.57x end-to-end on the
// allocation-bound segmentation pipeline (perf pass 2026-06-10).
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::core::methods::hepburn_traditional;
use kaniran_core::core::methods::RomanizationMethod;
use kaniran_core::core::kani_romanize_method::KaniRomanizeMethod;
use kaniran_core::core::romanize::{romanize_star_, RomanizeStarSegment};
use kaniran_core::dict::word_info_str::word_info_gloss_json;

use common::{single_result, CapturedRow};
use serde_json::{json, Map, Value};

const EXPECTED_FQN: &str = "CL-USER::CLI-FULL";

/// Reproduce `(jsown:to-json (romanize* text :limit 5))`. `:method` is the
/// default (traditional-hepburn) the capture ran under; `wordprop-fn` is
/// `(constantly nil)` upstream, and jsown renders the resulting nil `prop`
/// as `[]`.
fn full_json(ctx: &KaniranContext, text: &str) -> Result<Value, kaniran_core::conn::KaniDbError> {
    let method = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
        hepburn_traditional(),
    ));
    let segments = romanize_star_(ctx, text, method, Some(5), |_, _| ())?;

    let mut top = Vec::with_capacity(segments.len());
    for segment in &segments {
        match segment {
            RomanizeStarSegment::Misc(s) => top.push(Value::String(s.clone())),
            RomanizeStarSegment::Word(alternatives) => {
                let mut alts = Vec::with_capacity(alternatives.len());
                for (words, score) in alternatives {
                    let mut word_jsons = Vec::with_capacity(words.len());
                    for (romaji, word_info, _prop) in words {
                        let gloss = word_info_gloss_json(ctx, word_info, false)?;
                        word_jsons.push(json!([romaji, gloss, []]));
                    }
                    alts.push(json!([word_jsons, score]));
                }
                top.push(Value::Array(alts));
            }
        }
    }
    Ok(Value::Array(top))
}

fn brief(s: &str) -> String {
    match s.char_indices().nth(220) {
        Some((cut, _)) => format!("{}…", &s[..cut]),
        None => s.to_string(),
    }
}

/// First structural divergence between actual (rust) and expected (lisp),
/// as a JSON-path string. Object comparison is key-wise (order-independent).
fn first_diff(rust: &Value, lisp: &Value, path: &str) -> Option<String> {
    match (rust, lisp) {
        (Value::Array(xs), Value::Array(ys)) => {
            if xs.len() != ys.len() {
                return Some(format!("{path}: array len rust={} lisp={}", xs.len(), ys.len()));
            }
            xs.iter()
                .zip(ys)
                .enumerate()
                .find_map(|(i, (x, y))| first_diff(x, y, &format!("{path}[{i}]")))
        }
        (Value::Object(xs), Value::Object(ys)) => {
            for (k, xv) in xs {
                match ys.get(k) {
                    None => return Some(format!("{path}.{k}: present rust, absent lisp")),
                    Some(yv) => {
                        if let Some(d) = first_diff(xv, yv, &format!("{path}.{k}")) {
                            return Some(d);
                        }
                    }
                }
            }
            ys.keys()
                .find(|k| !xs.contains_key(*k))
                .map(|k| format!("{path}.{k}: absent rust, present lisp"))
        }
        _ if rust == lisp => None,
        _ => Some(format!(
            "{path}: rust={} lisp={}",
            brief(&rust.to_string()),
            brief(&lisp.to_string())
        )),
    }
}

/// Canonicalize order-unstable parts before comparison. Two axes come
/// back in the database's unordered SQL row order (no `ORDER BY`
/// upstream), so the order carries no meaning on either side:
///   - a word's alternative readings, joined into the `/`-separated
///     romaji string (e.g. `naka/chū` vs `chū/naka`);
///   - the `alternative` array inside a gloss object.
/// Both are compared as sets: sort the `/`-tokens, sort the
/// `alternative` array, and sort object keys so that array sort is
/// itself order-independent. Genuine divergences (a different reading
/// or seq) survive because only pure reorderings collapse to equal.
fn normalize(v: &Value) -> Value {
    match v {
        Value::Array(xs) => Value::Array(xs.iter().map(normalize).collect()),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = Map::new();
            for k in keys {
                let mut nv = normalize(&map[k]);
                if k == "alternative" {
                    if let Value::Array(a) = &mut nv {
                        a.sort_by(|x, y| x.to_string().cmp(&y.to_string()));
                    }
                }
                out.insert(k.clone(), nv);
            }
            Value::Object(out)
        }
        Value::String(s) if s.contains('/') => {
            let mut parts: Vec<&str> = s.split('/').collect();
            parts.sort_unstable();
            Value::String(parts.join("/"))
        }
        other => other.clone(),
    }
}

fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    let text = row
        .args
        .first()
        .and_then(|v| v.as_str())
        .ok_or("missing sentence arg")?;
    let expected_str = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?;
    let expected: Value =
        serde_json::from_str(expected_str).map_err(|e| format!("expected JSON parse failed: {e}"))?;

    let actual = full_json(ctx, text).map_err(|e| e.to_string())?;

    let actual = normalize(&actual);
    let expected = normalize(&expected);
    if actual == expected {
        Ok(())
    } else {
        let diff = first_diff(&actual, &expected, "$").unwrap_or_else(|| "(equal?)".into());
        Err(format!("\n  sentence: {text:?}\n  diff: {diff}"))
    }
}

fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one);
}
