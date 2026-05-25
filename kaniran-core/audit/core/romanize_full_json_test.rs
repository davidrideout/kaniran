//! Fixture-replay runner for the e2e complete-result JSON — ichiran's
//! `ichiran-cli --full` output, captured under the synthetic FQN
//! `CL-USER::ROMANIZE-FULL-JSON` = `(jsown:to-json (romanize* text :limit 5))`.
//!
//! There is no library fn for the top-level JSON assembly: `cli.lisp` builds
//! it inline and that path is marked skip (a future kaniran-cli crate). This
//! runner assembles it test-locally from the ported pieces — `romanize_star_`
//! (segmentation + per-word romanize) and `word_info_gloss_json` (the per-word
//! JSON object) — and the trivial array nesting `jsown:to-json` performs over
//! the `romanize*` result. All substance under audit is library code; only the
//! nesting + the `prop` placeholder are test-local.
//!
//! Comparison is structural ([`serde_json::Value`] equality), so it is robust
//! to jsown's `\uXXXX` escaping vs serde's literal UTF-8 and to object key
//! order. Byte-exact serialization is a kaniran-cli concern, out of scope here.
//!
//! Run with:
//!   cargo run --release --bin romanize_full_json_test -- \
//!     --path corpus/extracted_romanize_json_2026_05_25/cl-user/romanize_full_json.parquet

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::core::_star_hepburn_traditional_star_::hepburn_traditional;
use kaniran_core::core::generic_romanization_class::RomanizationMethod;
use kaniran_core::core::kani_romanize_method::KaniRomanizeMethod;
use kaniran_core::core::romanize_star_::{romanize_star_, RomanizeStarSegment};
use kaniran_core::dict::word_info_gloss_json::word_info_gloss_json;

use common::{single_result, CapturedRow};
use serde_json::{json, Value};

const EXPECTED_FQN: &str = "CL-USER::ROMANIZE-FULL-JSON";

/// Reproduce `(jsown:to-json (romanize* text :limit 5))`. `:method` is the
/// default (traditional-hepburn) the capture ran under; `wordprop-fn` is
/// `(constantly nil)` upstream, and jsown renders the resulting nil `prop`
/// as `[]`.
async fn full_json(ctx: &KaniranContext, text: &str) -> Result<Value, sqlx::Error> {
    let method = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
        hepburn_traditional(),
    ));
    let segments = romanize_star_(ctx, text, method, Some(5), |_, _| ()).await?;

    let mut top = Vec::with_capacity(segments.len());
    for segment in &segments {
        match segment {
            RomanizeStarSegment::Misc(s) => top.push(Value::String(s.clone())),
            RomanizeStarSegment::Word(alternatives) => {
                let mut alts = Vec::with_capacity(alternatives.len());
                for (words, score) in alternatives {
                    let mut word_jsons = Vec::with_capacity(words.len());
                    for (romaji, word_info, _prop) in words {
                        let gloss = word_info_gloss_json(ctx, word_info, false).await?;
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

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
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

    let actual = full_json(ctx, text).await.map_err(|e| e.to_string())?;

    if actual == expected {
        Ok(())
    } else {
        let diff = first_diff(&actual, &expected, "$").unwrap_or_else(|| "(equal?)".into());
        Err(format!("\n  sentence: {text:?}\n  diff: {diff}"))
    }
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
