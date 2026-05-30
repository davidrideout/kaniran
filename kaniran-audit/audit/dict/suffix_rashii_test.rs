//! Manual fixture-replay runner for `ICHIRAN/DICT:SUFFIX-RASHII`.
//! Source under test: `src/dict/suffix_rashii.rs`.
//!
//! Run with:
//!   cargo run --release --bin suffix_rashii_test -- \
//!       --path corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/suffix_rashii.parquet
//!
//! Args shape (per `def-simple-suffix` expansion, suffix-rashii defined
//! at `dict-grammar.lisp:511`):
//!   `[<root>, <sv>, <kf KANA-TEXT envelope>]`
//!
//! Result shape: `[<list> | null]`. `null` ↔ Lisp nil (empty — either
//! `pair-words-by-conj` returned no buckets or the UNLESS branch fired);
//! otherwise a list of COMPOUND-TEXT envelopes — one per primary word
//! returned by the pair-words bucket sweep. `pair-words-by-conj` walks
//! `alexandria:hash-table-values` whose order is undefined, so the runner
//! sorts compound fingerprints before comparing.
//!
//! Comparison: every projected slot of every compound is compared via
//! `format!("{:?}", c)` Debug fingerprints, then `id: NNN,` substrings
//! are stripped before equality. Covers all CompoundText fields
//! (text, kana, primary, words, score_base, score_mod) and recursively
//! every simple-text field (seq, text, ord, common, common_tags,
//! conjugate_p, nokanji, best_*, conjugations, hintedp). Proxy-text
//! rows include their wrapped source.
//!
//! ## id slot is stripped before comparison
//!
//! suffix-rashii is reached via `find-word-with-conj-type → find-word`
//! during extraction. When the segmenter has `*substring-hash*` bound
//! (the substring cache path at `dict.lisp:493`), `find-word`
//! reconstructs root-word DAOs via `(apply 'make-instance init)` where
//! `init` lacks `:id`. The resulting unbound id surfaces in captures as
//! JSON null → audit-side `id = 0`. Rust replay hits the production
//! codepath end-to-end with real DB ids. Strict-equality on id would
//! fail every row whose primary came through the synthesis path. Same
//! treatment as `find_word_suffix_test`.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::compound_text_class::CompoundText;
use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::suffix_rashii::suffix_rashii;

use common::{parse_captured_simple_text, parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:SUFFIX-RASHII";

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
    let kf = parse_kana_kf(&row.args[2])?;

    let actual = suffix_rashii(ctx, root, sv, &kf)
        .await
        .map_err(|e| format!("suffix_rashii: {}", e))?;

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

/// Strip `id: NNN, ` substrings from the Debug fingerprint — see module
/// docstring for the synthesized-DAO rationale. Byte-level scan is safe:
/// "id: " and digit bytes are ASCII, and any multi-byte UTF-8 (Japanese
/// text) leading bytes are > 0x7F so they never falsely match the ASCII
/// pattern.
fn strip_id(s: String) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"id: ") {
            let mut j = i + 4;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if bytes[j..].starts_with(b", ") {
                j += 2;
            }
            i = j;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("strip_id preserves UTF-8 boundaries")
}

fn canonical_fingerprints(compounds: &[CompoundText]) -> Vec<String> {
    let mut fps: Vec<String> = compounds
        .iter()
        .map(|c| strip_id(format!("{:?}", c)))
        .collect();
    fps.sort();
    fps
}

fn canonical_fingerprints_word(words: &[KaniWordDispatchEnum]) -> Vec<String> {
    let mut fps: Vec<String> = words
        .iter()
        .map(|w| match w {
            KaniWordDispatchEnum::Compound(c) => strip_id(format!("{:?}", c)),
            other => strip_id(format!("{:?}", other)),
        })
        .collect();
    fps.sort();
    fps
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
