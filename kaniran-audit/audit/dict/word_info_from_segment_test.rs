//! Manual fixture-replay runner for `ICHIRAN/DICT:WORD-INFO-FROM-SEGMENT`.
//! Source under test: `src/dict/word_info_from_segment.rs`.
//!
//! Run with:
//!   cargo run --bin word_info_from_segment_test -- \
//!       --path corpus/<corpus_tag>/dict/word_info_from_segment.parquet
//!
//! Args (post-projector): `(<segment>)` — a single SEGMENT capture.
//! Result: `(<word-info>)` — single WORD-INFO produced by the upstream.

#[path = "../common/mod.rs"]
mod common;

use common::{
    compare_captured_word_info, parse_captured_segment, single_result, CapturedRow,
};
use kaniran_core::dict::word_info::word_info_from_segment;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:WORD-INFO-FROM-SEGMENT";

fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!(
            "expected 1 arg (segment), got {}",
            row.args.len()
        ));
    }
    let segment = parse_captured_segment(&row.args[0])?;

    let actual = word_info_from_segment(ctx, &segment)
        
        .map_err(|err| format!("word_info_from_segment: {}", err))?;

    let expected = single_result(&row.result)?;
    compare_captured_word_info(&actual, expected)
}

fn main() {
    common::run_async(EXPECTED_FQN, audit_one);
}
