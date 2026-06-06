//! Manual fixture-replay runner for `ICHIRAN/DICT:APPLY-SCORE-MOD`.
//! Source under test: `src/dict/apply_score_mod.rs`.
//!
//! Run with:
//!   cargo run --bin apply_score_mod_test -- \
//!       --path corpus/<corpus_tag>/dict/apply_score_mod.parquet
//!
//! Applies a score modifier: an integer mod multiplies `score * mod *
//! len`; a list of mods sums each applied in turn. Args are `[score-mod,
//! score, len]`; the one result value is the modified score.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::apply_score_mod::apply_score_mod;
use kaniran_core::dict::compound_text_class::ScoreMod;

use common::{single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:APPLY-SCORE-MOD";

fn parse_score_mod(v: &Value) -> Result<ScoreMod, String> {
    match v {
        Value::Number(n) => {
            let i = n
                .as_i64()
                .ok_or_else(|| format!("score-mod int not i64: {}", n))?;
            Ok(ScoreMod::Single(i))
        }
        Value::Array(arr) => {
            let mut items = Vec::with_capacity(arr.len());
            for elt in arr {
                let i = elt.as_i64().ok_or_else(|| {
                    format!(
                        "score-mod list element not i64 (likely a (constantly N) closure \
                         — projector clause for function-typed slots missing): {}",
                        elt
                    )
                })?;
                items.push(ScoreMod::Single(i));
            }
            Ok(ScoreMod::Stack(items))
        }
        other => Err(format!(
            "score-mod: expected int or list-of-int, got {} (function closures not \
             yet representable in capture JSON)",
            other
        )),
    }
}

fn parse_i64(v: &Value, field: &str) -> Result<i64, String> {
    v.as_i64().ok_or_else(|| format!("{} not i64: {}", field, v))
}

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 3 {
        return Err(format!("expected 3 args, got {}", row.args.len()));
    }
    let score_mod = parse_score_mod(&row.args[0])?;
    let score = parse_i64(&row.args[1], "score")?;
    let len = parse_i64(&row.args[2], "len")?;

    let actual = apply_score_mod(&score_mod, score, len);
    let expected = parse_i64(single_result(&row.result)?, "result")?;

    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "\n  args: score-mod={:?} score={} len={}\n  rust: {}\n  lisp: {}",
            row.args[0], score, len, actual, expected
        ))
    }
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
