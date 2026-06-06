//! Manual fixture-replay runner for `ICHIRAN:PROCESS-MODIFIERS`.
//! Source under test: `src/core/process_modifiers.rs`.
//!
//! Run with:
//!   cargo run --bin process_modifiers_test -- \
//!       --path corpus/<corpus_tag>/process_modifiers.parquet
//!
//! Args are `[<cc-list>]` (a flat list of keyword / char atoms); the one
//! result value is the modifier tree.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::process_modifiers::process_modifiers;

use common::{parse_cc_list, parse_cc_tree, single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:PROCESS-MODIFIERS";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let cc_list = match row.args.first() {
        Some(value) => parse_cc_list(value)?,
        None => return Err("expected 1 arg".to_string()),
    };

    let actual = process_modifiers(&cc_list);
    let expected = parse_cc_tree(single_result(&row.result)?)?;

    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, expected))
    }
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
