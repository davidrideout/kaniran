//! Manual fixture-replay runner for `ICHIRAN/DICT:SUFFIX-RA`.
//! Source under test: `src/dict/suffix_ra.rs`.
//!
//! Run with:
//!   cargo run --release --bin suffix_ra_test -- \
//!       --path corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/suffix_ra.parquet
//!
//! Args shape (per `def-simple-suffix` expansion at
//! `dict-grammar.lisp:340-368`):
//!   `[<root>, <sv>, <kf KANA-TEXT envelope>]`
//!
//! Result shape: `[<list> | null]`. `null` ↔ Lisp nil (empty
//! result — either the UNLESS branch fired for a root ending in "ら"
//! or both `or-as-hiragana` / `find-word-seq` missed); otherwise a
//! list of COMPOUND-TEXT envelopes — one per primary word returned
//! by the lookup. Mapcar preserves the upstream lookup order, which
//! is unspecified by the SQL (no ORDER BY), so the runner sorts
//! compound fingerprints before comparing.
//!
//! Comparison: every projected slot of every compound is compared
//! via [`parse_captured_word`]-derived `Debug` fingerprints (text,
//! kana, primary, words, score_base, score_mod — and recursively
//! every simple-text slot id/seq/text/ord/common/common_tags/
//! conjugate_p/nokanji/best_*/conjugations/hintedp; proxy-text rows
//! include their wrapped source).

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::text_classes::CompoundText;
use kaniran_core::dict::dao::KanaText;
use kaniran_core::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::grammar::suffix_rules::suffix_ra;

use common::{parse_captured_simple_text, parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:SUFFIX-RA";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 3 {
        return Err(format!("expected 3 args, got {}", row.args.len()));
    }
    let root = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 (root) not string: {}", row.args[0]))?;
    let sv = row.args[1]
        .as_str()
        .ok_or_else(|| format!("arg 1 (sv) not string: {}", row.args[1]))?;
    let suf = parse_kana_kf(&row.args[2])?;

    let actual = suffix_ra(ctx, root, sv, &suf)
        .await
        .map_err(|e| format!("suffix_ra: {}", e))?;

    let expected = parse_expected(unwrap_result(&row.result)?)?;

    let actual_fp = canonical_fingerprints(&actual);
    let expected_fp = canonical_fingerprints_word(&expected);

    if actual_fp != expected_fp {
        return Err(format!(
            "compounds mismatch:\n  rust ={:?}\n  lisp ={:?}",
            actual_fp, expected_fp
        ));
    }
    Ok(())
}

fn parse_kana_kf(value: &Value) -> Result<KanaText, String> {
    match parse_captured_simple_text(value)? {
        KaniSimpleTextDispatchEnum::Kana(k) => Ok(k),
        other => Err(format!("kf: expected KANA-TEXT, got {:?}", other)),
    }
}

fn unwrap_result(result: &[Value]) -> Result<&Value, String> {
    if result.is_empty() {
        return Err("expected at least 1 result value, got 0".into());
    }
    Ok(&result[0])
}

fn parse_expected(value: &Value) -> Result<Vec<KaniWordDispatchEnum>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for (idx, item) in arr.iter().enumerate() {
                out.push(
                    parse_captured_word(item)
                        .map_err(|e| format!("compound {}: {}", idx, e))?,
                );
            }
            Ok(out)
        }
        other => Err(format!("result: expected array or null, got {}", other)),
    }
}

fn canonical_fingerprints(compounds: &[CompoundText]) -> Vec<String> {
    let mut fps: Vec<String> = compounds.iter().map(|c| format!("{:?}", c)).collect();
    fps.sort();
    fps
}

fn canonical_fingerprints_word(words: &[KaniWordDispatchEnum]) -> Vec<String> {
    let mut fps: Vec<String> = words
        .iter()
        .map(|w| match w {
            KaniWordDispatchEnum::Compound(c) => format!("{:?}", c),
            other => format!("{:?}", other),
        })
        .collect();
    fps.sort();
    fps
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
