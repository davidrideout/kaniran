//! Manual fixture-replay runner for `ICHIRAN:LEFTMOST-ATOM`.
//! Source under test: `src/core/leftmost_atom.rs`.
//!
//! Run with:
//!   cargo run --bin leftmost_atom_test -- \
//!       --path corpus/<corpus_tag>/leftmost_atom.parquet
//!
//! Args are `[<cc-tree>]` (an array of nodes, or `null` for the empty
//! tree); the one result value is the leftmost atom — a keyword, a char
//! meta, or `null` (nil).

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::core::leftmost_atom::leftmost_atom;

use common::{parse_cc_atom, parse_cc_tree, single_result, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN:LEFTMOST-ATOM";

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    let cc_tree = match row.args.first() {
        Some(value) => parse_cc_tree(value)?,
        None => return Err("expected 1 arg".to_string()),
    };

    let actual = leftmost_atom(&cc_tree);
    let expected = parse_cc_atom(single_result(&row.result)?)?;

    if actual == expected {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, expected))
    }
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
