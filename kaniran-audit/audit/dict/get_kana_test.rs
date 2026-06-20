//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-KANA`.
//! Source under test: `src/dict/get_kana.rs`.
//!
//! Run with:
//!   cargo run --bin get_kana_test -- \
//!       --path corpus/<corpus_tag>/dict/get_kana.parquet
//!
//! Replays captured word rows through `get_kana` and compares the
//! returned kana string against the Lisp result.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::dict::accessors::get_kana;

use common::{
    describe_word, extract_disable_hints_meta, parse_captured_word, single_result, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-KANA";


fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!(
            "expected 2 args (word + hint-state meta), got {}",
            row.args.len()
        ));
    }
    let disable_hints = extract_disable_hints_meta(&row.args[1])
        .ok_or_else(|| format!("missing _meta.context.disable_hints on args[1]: {}", row.args[1]))?;
    let word = parse_captured_word(&row.args[0])?;

    let ctx2 = ctx.with_disable_hints(disable_hints);
    let actual = get_kana(&ctx2, &word)
        
        .map_err(|err| format!("get_kana: {} ({})", err, describe_word(&word)))?;

    let result = single_result(&row.result)?;
    let expected = result
        .as_str()
        .ok_or_else(|| format!("result[0] not string: {}", result))?;

    // Upstream never returns nil for get-kana — it crashes via
    // `(text nil)` instead. The Rust port surfaces that case as
    // None. If the upstream returned a string (always, in captured
    // rows) and we got None, that's a divergence.
    let actual_str = actual.as_deref().unwrap_or("<None — upstream returned a string>");
    if actual.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(format!(
            "input={} disable_hints={}\n  rust: {:?}\n  lisp: {:?}",
            describe_word(&word),
            disable_hints,
            actual_str,
            expected,
        ))
    }
}


fn main() {
    common::run_async(EXPECTED_FQN, audit_one);
}
