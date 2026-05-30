//! Manual fixture-replay runner for `ICHIRAN:ROMANIZE-CORE`.
//! Source under test: `src/core/romanize_core.rs`.
//!
//! Run with:
//!   cargo run --bin romanize_core_test -- \
//!       --path corpus/<corpus_tag>/romanize_core.parquet
//!
//! Args are `[<method>, <cc-tree>]`; the one result value is the
//! concatenated romanization of the tree.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::rules::romanize_core;

use common::{parse_cc_tree, single_result, with_captured_method, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:ROMANIZE-CORE";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let method_value = row.args.first().ok_or("expected 2 args")?;
    let cc_tree = parse_cc_tree(row.args.get(1).ok_or("expected 2 args")?)?;
    let expected = single_result(&row.result)?
        .as_str()
        .ok_or("result not a string")?
        .to_string();

    with_captured_method(method_value, |method| {
        let actual = romanize_core(method, &cc_tree);
        if actual == expected {
            Ok(())
        } else {
            Err(format!("\n  rust: {actual:?}\n  lisp: {expected:?}"))
        }
    })
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
