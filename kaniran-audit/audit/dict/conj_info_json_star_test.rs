//! Fixture-replay runner for `ICHIRAN/DICT:CONJ-INFO-JSON*` (`dict.lisp:1664`).
//!
//! Builds the `conj` JSON array for a sequence (the unfiltered form, with
//! no `readok` filter). Replays the captured `(seq &key conjugations text
//! has-gloss)` call.
//!
//! Run with:
//!   cargo run --release --bin conj_info_json_star_test -- \
//!     --path corpus/<tag>/dict/conj_info_json_star_.parquet

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::conj::conj_info_json_star_;
use kaniran_core::dict::conj::FilterPropsText;
use kaniran_core::dict::dao::WordConjugations;

use common::{single_result, CapturedRow};
use serde_json::{Map, Value};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:CONJ-INFO-JSON*";

fn kw_value<'a>(args: &'a [Value], kw: &str) -> Option<&'a Value> {
    args.iter().position(|a| a.as_str() == Some(kw)).and_then(|i| args.get(i + 1))
}

fn parse_conjugations(v: Option<&Value>) -> Result<Option<WordConjugations>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s == ":ROOT" => Ok(Some(WordConjugations::Root)),
        Some(Value::Array(arr)) => {
            let mut ids = Vec::with_capacity(arr.len());
            for x in arr {
                ids.push(x.as_i64().ok_or_else(|| format!("conj-id not int: {x}"))? as i32);
            }
            Ok(Some(WordConjugations::Ids(ids)))
        }
        Some(other) => Err(format!("conjugations: unexpected {other}")),
    }
}

enum TextArg {
    None,
    One(String),
    Many(Vec<String>),
}

fn parse_text(v: Option<&Value>) -> Result<TextArg, String> {
    match v {
        None | Some(Value::Null) => Ok(TextArg::None),
        Some(Value::String(s)) => Ok(TextArg::One(s.clone())),
        Some(Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for x in arr {
                out.push(x.as_str().ok_or_else(|| format!("text entry not string: {x}"))?.to_owned());
            }
            Ok(TextArg::Many(out))
        }
        Some(other) => Err(format!("text: unexpected {other}")),
    }
}

fn normalize(v: &Value) -> Value {
    match v {
        Value::Array(xs) => {
            let mut items: Vec<Value> = xs.iter().map(normalize).collect();
            items.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            Value::Array(items)
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = Map::new();
            for k in keys {
                out.insert(k.clone(), normalize(&map[k]));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn brief(s: &str) -> String {
    match s.char_indices().nth(200) {
        Some((cut, _)) => format!("{}…", &s[..cut]),
        None => s.to_string(),
    }
}

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
        _ => Some(format!("{path}: rust={} lisp={}", brief(&rust.to_string()), brief(&lisp.to_string()))),
    }
}

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    let seq = row
        .args
        .first()
        .and_then(Value::as_i64)
        .ok_or("arg 0 (seq) not int")? as i32;
    let conjugations = parse_conjugations(kw_value(&row.args, ":CONJUGATIONS"))?;
    let text_owned = parse_text(kw_value(&row.args, ":TEXT"))?;
    let has_gloss = matches!(kw_value(&row.args, ":HAS-GLOSS"), Some(Value::Bool(true)));

    let many_refs: Vec<&str> = match &text_owned {
        TextArg::Many(v) => v.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    };
    let text = match &text_owned {
        TextArg::None => FilterPropsText::None,
        TextArg::One(s) => FilterPropsText::One(s.as_str()),
        TextArg::Many(_) => FilterPropsText::Many(&many_refs),
    };

    let actual = conj_info_json_star_(ctx, seq, conjugations.as_ref(), text, has_gloss)
        .await
        .map_err(|e| e.to_string())?;
    let actual = normalize(&Value::Array(actual));

    let expected_str = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?;
    let expected: Value =
        serde_json::from_str(expected_str).map_err(|e| format!("expected JSON parse: {e}"))?;
    let expected = normalize(&expected);

    if actual == expected {
        Ok(())
    } else {
        let diff = first_diff(&actual, &expected, "$").unwrap_or_else(|| "(equal?)".into());
        Err(format!("seq={seq} diff: {diff}"))
    }
}

#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
