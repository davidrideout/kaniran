//! Manual fixture-replay runner for `ICHIRAN/DICT:COMMON`.
//! Source under test: `src/dict/common.rs`.
//!
//! Run with:
//!   cargo run --bin common_test -- \
//!       --path corpus/<corpus_tag>/dict/common.parquet
//!
//! Out-of-Rust-polymorphism input classes (entry, sense, conjugation
//! et al.) are skipped — the Rust callsites for those hold a typed
//! DAO and read `.common` directly without the dispatcher.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::counters::dispatchers::common as common_fn;
use kaniran_core::dict::counters::classes::Common;

use common::{captured_class, parse_captured_word, single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:COMMON";


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
    let actual = common_fn(&word);

    let actual_repr = repr_actual(&actual);
    let expected_repr = repr_expected(single_result(&row.result)?)?;
    if actual_repr == expected_repr {
        Ok(())
    } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual_repr, expected_repr))
    }
}

fn repr_actual(c: &Common) -> String {
    match c {
        Common::Score(n) => format!("{}", n),
        Common::Null => "NULL".into(),
        Common::Inherit => "INHERIT".into(),
    }
}

fn repr_expected(v: &Value) -> Result<String, String> {
    if v.is_null() {
        return Ok("NULL".into());
    }
    if let Some(s) = v.as_str() {
        if s == ":NULL" {
            return Ok("NULL".into());
        }
        return Err(format!("common result string: {}", s));
    }
    if let Some(n) = v.as_i64() {
        return Ok(format!("{}", n));
    }
    Err(format!("common result shape: {}", v))
}


fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
