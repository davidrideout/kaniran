//! Manual fixture-replay runner for `ICHIRAN/DICT:SEQ`.
//! Source under test: `src/dict/seq.rs`.
//!
//! Run with:
//!   cargo run --bin seq_test -- \
//!       --path corpus/<corpus_tag>/dict/seq.parquet
//!
//! Replays a captured word row through `seq` and compares the returned
//! sequence id against the Lisp result. Input classes whose `.seq` the
//! Rust port reads directly (conjugation/sense/entry/...) are skipped.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::counters::methods::seq;
use kaniran_core::dict::word_info_class::WordInfoSeq;

use common::{captured_class, parse_captured_word, single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:SEQ";


fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let class = captured_class(&row.args[0])?;
    if matches!(
        class,
        "CONJUGATION" | "SENSE" | "SENSE-PROP" | "RESTRICTED-READINGS" | "ENTRY" | "CONJ-SOURCE-READING"
    ) {
        return Ok(());
    }

    let word = parse_captured_word(&row.args[0])?;
    let actual = seq(&word);

    let actual_repr = repr_actual(&actual);
    let expected_repr = repr_expected(single_result(&row.result)?)?;
    if actual_repr == expected_repr {
        Ok(())
    } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual_repr, expected_repr))
    }
}

fn repr_actual(v: &Option<WordInfoSeq>) -> String {
    match v {
        None => "NIL".into(),
        Some(WordInfoSeq::Single(n)) => format!("{}", n),
        Some(WordInfoSeq::Multi(items)) => {
            let s: Vec<String> = items.iter().map(repr_actual).collect();
            format!("({})", s.join(" "))
        }
    }
}

fn repr_expected(v: &Value) -> Result<String, String> {
    if v.is_null() {
        return Ok("NIL".into());
    }
    if let Some(n) = v.as_i64() {
        return Ok(format!("{}", n));
    }
    if let Some(arr) = v.as_array() {
        let mut parts = Vec::with_capacity(arr.len());
        for item in arr {
            parts.push(repr_expected(item)?);
        }
        return Ok(format!("({})", parts.join(" ")));
    }
    Err(format!("seq result shape: {}", v))
}


fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
