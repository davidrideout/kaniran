//! Fixture-replay runner for `ICHIRAN/DICT:CONJ-PROP-JSON` (`dict.lisp:285`).
//!
//! Pure, DB-free: reconstruct the captured `conj-prop` DAO and compare
//! `conj_prop_json`'s JSON object to the captured `jsown:to-json` string.
//! `neg`/`fml` arrive as `true` (t), `null` (nil, the projector renders a
//! false boolean as JSON null) or `":NULL"` (db-null). Only the `t` state
//! extends the object, so the nil/db-null distinction is immaterial to the
//! output, but it is mapped faithfully onto `Some(false)` / `None`.
//!
//! Run with:
//!   cargo run --release --bin conj_prop_json_test -- \
//!     --path corpus/<tag>/dict/conj_prop_json.parquet

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::dict::dao::ConjProp;
use kaniran_core::dict::conj_data::conj_prop_json;

use common::{single_result, CapturedRow};
use serde_json::Value;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:CONJ-PROP-JSON";

fn require_i32(obj: &Value, key: &str) -> Result<i32, String> {
    obj.get(key)
        .and_then(Value::as_i64)
        .map(|v| v as i32)
        .ok_or_else(|| format!("{key}: missing or not int in {obj}"))
}

// t→Some(true); nil renders as JSON null→Some(false); :NULL→None (db-null).
fn tristate(obj: &Value, key: &str) -> Option<bool> {
    match obj.get(key) {
        Some(Value::Bool(b)) => Some(*b),
        Some(Value::String(s)) if s == ":NULL" => None,
        _ => Some(false),
    }
}

fn parse_conj_prop(v: &Value) -> Result<ConjProp, String> {
    Ok(ConjProp {
        id: require_i32(v, "id")?,
        conj_id: require_i32(v, "conj_id")?,
        conj_type: require_i32(v, "conj_type")?,
        pos: v
            .get("pos")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("pos: missing or not string in {v}"))?
            .to_owned(),
        neg: tristate(v, "neg"),
        fml: tristate(v, "fml"),
    })
}

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let prop = parse_conj_prop(&row.args[0])?;
    let expected_str = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?;
    let expected: Value =
        serde_json::from_str(expected_str).map_err(|e| format!("expected JSON parse: {e}"))?;

    let actual = conj_prop_json(&prop);
    if actual == expected {
        Ok(())
    } else {
        Err(format!("rust={actual} lisp={expected}"))
    }
}

fn main() {
    common::run_sync_streaming(EXPECTED_FQN, audit_one);
}
