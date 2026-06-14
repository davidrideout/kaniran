//! Manual fixture-replay runner for `ICHIRAN/DICT:WORD-INFO-FROM-SEGMENT-LIST`.
//! Source under test: `src/dict/word_info_from_segment_list.rs`.
//!
//! Run with:
//!   cargo run --bin word_info_from_segment_list_test -- \
//!       --path corpus/<corpus_tag>/dict/word_info_from_segment_list.parquet
//!
//! Args: `(<segment-list>)`.
//! Result: `(<word-info>)`.

#[path = "../common/mod.rs"]
mod common;

use common::{
    compare_captured_word_info, parse_captured_segment_list, single_result, CapturedRow,
};
use kaniran_core::dict::word_info::word_info_from_segment_list;

const EXPECTED_FQN: &str = "ICHIRAN/DICT:WORD-INFO-FROM-SEGMENT-LIST";

fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!(
            "expected 1 arg (segment-list), got {}",
            row.args.len()
        ));
    }
    let mut segment_list = parse_captured_segment_list(&row.args[0])?;

    let actual = word_info_from_segment_list(ctx, &mut segment_list)
        
        .map_err(|err| format!("word_info_from_segment_list: {}", err))?;

    let expected = single_result(&row.result)?;
    compare_captured_word_info(&actual, expected)
}

fn main() {
    common::run_async(EXPECTED_FQN, audit_one);
}
