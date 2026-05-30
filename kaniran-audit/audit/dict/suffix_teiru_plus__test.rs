//! Manual fixture-replay runner for `ICHIRAN/DICT:SUFFIX-TEIRU+`.
//! Source under test: `src/dict/suffix_teiru_plus_.rs`.
//!
//! Run with:
//!   cargo run --release --bin suffix_teiru_plus__test -- \
//!       --path corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/suffix_teiru_plus_.parquet
//!
//! `ICHIRAN/DICT:SUFFIX-TEIRU+` is a `def-simple-suffix`
//! (`dict-grammar.lisp:398`). The macro signature is `(root sv kf)`,
//! so the captured args are `[<root>, <sv>, <kf KANA-TEXT envelope>]`
//! and the result is `[<list> | null]` — `null` ↔ Lisp nil, else a
//! list of COMPOUND-TEXT envelopes.
//!
//! Comparison: every projected slot of every compound is compared
//! recursively via `format!("{:?}", c)` Debug fingerprints, then
//! `id: NNN,` substrings are stripped before equality (see id rationale
//! below). Debug derives on CompoundText / KanaText / KanjiText /
//! ProxyText / SimpleText / WordConjugations / ScoreMod are complete,
//! so the fingerprint covers every output field at every depth
//! (recursive primary / words / score_base, and each leaf simple-text's
//! seq / text / ord / common / common_tags / conjugate_p / nokanji /
//! best_* / state.conjugations / state.hintedp). `pair-words-by-conj`
//! walks `hash-table-values` (undefined order), so compound
//! fingerprints are sorted before comparing — a length divergence still
//! fails (sorted vecs of differing length are unequal).
//!
//! ## id slot is stripped before comparison
//!
//! This suffix reaches `find-word` during extraction. When the
//! segmenter has `*substring-hash*` bound, the synthesis path at
//! `dict.lisp:493` reconstructs DAOs via `(apply 'make-instance init)`
//! without `:id`. Captures emit JSON null → audit-side `id = 0`. Rust
//! replay hits the production DB and returns real ids. Strict-equality
//! on id would fail every synthesized row. Same treatment as
//! `suffix_rashii_test`.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::compound_text_class::CompoundText;
use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::suffix_teiru_plus_::suffix_teiru_plus_;

use common::{parse_captured_simple_text, parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:SUFFIX-TEIRU+";

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

    let actual = suffix_teiru_plus_(ctx, root, sv, &kf)
        .await
        .map_err(|e| format!("suffix_teiru_plus_: {}", e))?;

    let expected = parse_expected(unwrap_result(&row.result)?)?;

    let actual_fp = canonical_fingerprints(&actual);
    let expected_fp = canonical_fingerprints_word(&expected);

    if actual_fp != expected_fp {
        return Err(format!(
            "compounds mismatch on root={:?} sv={:?}:\n  rust ={:?}\n  lisp ={:?}",
            root, sv, actual_fp, expected_fp
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
