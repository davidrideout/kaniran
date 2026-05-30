//! Fixture-replay runner for `ICHIRAN/DICT:SELECT-CONJS-AND-PROPS`
//! (`dict.lisp:1638`).
//!
//! Replays `(seq &optional conj-ids text)` and compares the returned
//! `(conjugation, filtered-props, sort-key)` tuples to the captured DAO
//! list. Each tuple is fingerprinted by conjugation content
//! (`seq`/`from`/`via`) — not the surrogate `id`, which differs between
//! duplicate dictionary rows the database returns in unstable order
//! (same rationale as the suffix runners) — plus a sorted prop content
//! list and the sort key. Tuples are compared as a sorted MULTISET, so a
//! genuine extra/missing conjugation survives while pure row-order
//! reordering and id-only differences collapse to equal. Under
//! [`common::run_async`] (group-by-args, pass-if-any).
//!
//! Run with:
//!   cargo run --release --bin select_conjs_and_props_test -- \
//!     --path corpus/<tag>/dict/select_conjs_and_props.parquet

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::dao::ConjProp;
use kaniran_core::dict::dao::Conjugation;
use kaniran_core::dict::conj_data::FilterPropsText;
use kaniran_core::dict::conj_data::select_conjs_and_props;
use kaniran_core::dict::text_classes::WordConjugations;

use common::{single_result, CapturedRow};
use serde_json::Value;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:SELECT-CONJS-AND-PROPS";

fn parse_conjugations(v: &Value) -> Result<Option<WordConjugations>, String> {
    match v {
        Value::Null => Ok(None),
        Value::String(s) if s == ":ROOT" => Ok(Some(WordConjugations::Root)),
        Value::Array(arr) => {
            let mut ids = Vec::with_capacity(arr.len());
            for x in arr {
                ids.push(x.as_i64().ok_or_else(|| format!("conj-id not int: {x}"))? as i32);
            }
            Ok(Some(WordConjugations::Ids(ids)))
        }
        other => Err(format!("conj-ids: unexpected {other}")),
    }
}

enum TextArg {
    None,
    One(String),
    Many(Vec<String>),
}

// `text` is &optional: nil, a single string, or a list of strings.
fn parse_text(v: &Value) -> Result<TextArg, String> {
    match v {
        Value::Null => Ok(TextArg::None),
        Value::String(s) => Ok(TextArg::One(s.clone())),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for x in arr {
                out.push(x.as_str().ok_or_else(|| format!("text entry not string: {x}"))?.to_owned());
            }
            Ok(TextArg::Many(out))
        }
        other => Err(format!("text: unexpected {other}")),
    }
}

// t→Some(true); nil renders as JSON null→Some(false); :NULL→None.
fn tristate(v: Option<&Value>) -> Option<bool> {
    match v {
        Some(Value::Bool(b)) => Some(*b),
        Some(Value::String(s)) if s == ":NULL" => None,
        _ => Some(false),
    }
}

// ":NULL"→None; int→Some(n).
fn opt_i32(v: Option<&Value>) -> Option<i32> {
    v.and_then(Value::as_i64).map(|n| n as i32)
}

/// Canonical, order-independent fingerprint of one conjugation tuple.
fn tuple_fp(
    seq: i32,
    from: i32,
    via: Option<i32>,
    key: [i32; 2],
    mut props: Vec<String>,
) -> String {
    props.sort();
    format!("seq={seq} from={from} via={via:?} key={key:?} props=[{}]", props.join(";"))
}

fn prop_fp(conj_type: i32, pos: &str, neg: Option<bool>, fml: Option<bool>) -> String {
    format!("{conj_type}|{pos}|{neg:?}|{fml:?}")
}

fn rust_fps(tuples: &[(Conjugation, Vec<ConjProp>, [i32; 2])]) -> Vec<String> {
    let mut fps: Vec<String> = tuples
        .iter()
        .map(|(c, props, key)| {
            let props = props.iter().map(|p| prop_fp(p.conj_type, &p.pos, p.neg, p.fml)).collect();
            tuple_fp(c.seq, c.seq_from, c.seq_via, *key, props)
        })
        .collect();
    fps.sort();
    fps
}

fn captured_fps(v: &Value) -> Result<Vec<String>, String> {
    let tuples = match v {
        Value::Null => return Ok(Vec::new()),
        Value::Array(arr) => arr,
        other => return Err(format!("result: expected array or null, got {other}")),
    };
    let mut fps = Vec::with_capacity(tuples.len());
    for t in tuples {
        let tup = t.as_array().ok_or_else(|| format!("tuple not array: {t}"))?;
        if tup.len() != 3 {
            return Err(format!("tuple len {} (want 3): {t}", tup.len()));
        }
        let conj = &tup[0];
        let seq = conj.get("seq").and_then(Value::as_i64).ok_or("conj.seq missing")? as i32;
        let from = conj.get("from").and_then(Value::as_i64).ok_or("conj.from missing")? as i32;
        let via = opt_i32(conj.get("via"));
        // fprops nil (no surviving props) flattens to JSON null, not [].
        let empty: Vec<Value> = Vec::new();
        let props_arr = match &tup[1] {
            Value::Null => &empty,
            Value::Array(a) => a,
            other => return Err(format!("props not array: {other}")),
        };
        let mut props = Vec::with_capacity(props_arr.len());
        for p in props_arr {
            let conj_type = p.get("conj_type").and_then(Value::as_i64).ok_or("prop.conj_type missing")? as i32;
            let pos = p.get("pos").and_then(Value::as_str).ok_or("prop.pos missing")?;
            props.push(prop_fp(conj_type, pos, tristate(p.get("neg")), tristate(p.get("fml"))));
        }
        let key_arr = tup[2].as_array().ok_or_else(|| format!("key not array: {}", tup[2]))?;
        if key_arr.len() != 2 {
            return Err(format!("key len {} (want 2)", key_arr.len()));
        }
        let key = [
            key_arr[0].as_i64().ok_or("key[0] not int")? as i32,
            key_arr[1].as_i64().ok_or("key[1] not int")? as i32,
        ];
        fps.push(tuple_fp(seq, from, via, key, props));
    }
    fps.sort();
    Ok(fps)
}

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 3 {
        return Err(format!("expected 3 args, got {}", row.args.len()));
    }
    let seq = row.args[0].as_i64().ok_or("arg 0 (seq) not int")? as i32;
    let conj_ids = parse_conjugations(&row.args[1])?;
    let text_owned = parse_text(&row.args[2])?;
    let many_refs: Vec<&str> = match &text_owned {
        TextArg::Many(v) => v.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    };
    let text = match &text_owned {
        TextArg::None => FilterPropsText::None,
        TextArg::One(s) => FilterPropsText::One(s.as_str()),
        TextArg::Many(_) => FilterPropsText::Many(&many_refs),
    };

    let actual = select_conjs_and_props(ctx, seq, conj_ids.as_ref(), text)
        .await
        .map_err(|e| e.to_string())?;
    let actual_fps = rust_fps(&actual);

    let expected_fps = captured_fps(single_result(&row.result)?)?;

    if actual_fps == expected_fps {
        Ok(())
    } else {
        Err(format!("seq={seq} tuples mismatch:\n  rust ={actual_fps:?}\n  lisp ={expected_fps:?}"))
    }
}

#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
