//! Manual fixture-replay runner for `ICHIRAN:GET-CHARACTER-CLASSES`.
//! Source under test: `src/core/get_character_classes.rs`.
//!
//! Run with:
//!   cargo run --bin get_character_classes_test -- \
//!       --path corpus/<corpus_tag>/get_character_classes.parquet
//!
//! Args are `["word"]`; the one result value is the character-class list
//! (keyword atoms for kana, char metas for everything else).

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::core::rules::get_character_classes;

use common::{parse_cc_list, single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:GET-CHARACTER-CLASSES";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let word = match row.args.first() {
        Some(Value::String(s)) => s.as_str(),
        Some(other) => return Err(format!("arg 0: expected string, got {}", other)),
        None => return Err("expected 1 arg".to_string()),
    };

    let actual = get_character_classes(word);
    let expected = parse_cc_list(single_result(&row.result)?)?;

    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, expected))
    }
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
