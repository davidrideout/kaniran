//! Manual fixture-replay runner for `ICHIRAN/DICT:TRUE-KANA`.
//! Source under test: `src/dict/true_kana.rs`.
//!
//! Run with:
//!   cargo run --bin true_kana_test -- \
//!       --path corpus/<corpus_tag>/dict/true_kana.parquet
//!
//! Args (post-projector): `(<word-row> {"_meta":{"context":
//! {"disable_hints":<bool>}}})`. true-kana delegates to get-kana
//! (on the leaf for proxy-text, on `obj` otherwise) so the inner
//! get-kana's `:around` method consults the same `disable_hints`
//! state — the trailing meta element captures the value at entry
//! and the runner rebinds the audit ctx via
//! [`KaniranContext::with_disable_hints`] before the impl call.
//!
//! Result: `(<kana string>)` — single value, always a string.

#[path = "../common/mod.rs"]
mod common;

use kaniran_core::dict::best_text::true_kana;

use common::{
    describe_word, extract_disable_hints_meta, parse_captured_word, single_result, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:TRUE-KANA";


async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!(
            "expected 2 args (obj + hint-state meta), got {}",
            row.args.len()
        ));
    }
    let disable_hints = extract_disable_hints_meta(&row.args[1])
        .ok_or_else(|| format!("missing _meta.context.disable_hints on args[1]: {}", row.args[1]))?;
    let obj = parse_captured_word(&row.args[0])?;

    let ctx2 = ctx.with_disable_hints(disable_hints);
    let actual = true_kana(&ctx2, &obj)
        .await
        .map_err(|err| format!("true_kana: {} ({})", err, describe_word(&obj)))?;

    let result = single_result(&row.result)?;
    let expected = result
        .as_str()
        .ok_or_else(|| format!("result[0] not string: {}", result))?;

    // Upstream `true-kana` returns the inner get-kana string (never
    // nil — would crash via `(text nil)` first). Rust port surfaces
    // that crash case as None; mismatch against a captured String
    // is a divergence.
    let actual_str = actual.as_deref().unwrap_or("<None — upstream returned a string>");
    if actual.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(format!(
            "input={} disable_hints={}\n  rust: {:?}\n  lisp: {:?}",
            describe_word(&obj),
            disable_hints,
            actual_str,
            expected,
        ))
    }
}


#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
