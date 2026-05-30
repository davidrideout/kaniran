//! Manual fixture-replay runner for `ICHIRAN/DICT:ABBR-DEWANAI`.
//! Source under test: `src/dict/abbr_dewanai.rs`.
//!
//! Run with:
//!   cargo run --release --bin abbr_dewanai_test -- \
//!       --path corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/abbr_dewanai.parquet
//!
//! ```lisp
//! (def-abbr-suffix abbr-dewanai :dewanai 4 (root)
//!   (find-word-full (concatenate 'string root "ではない")))
//! ```
//!
//! Args shape (the `def-abbr-suffix` macro signature is `(root sv suf)`
//! at `dict-grammar.lisp:547-579`; the third `,suf` is `(declare
//! (ignore ,suf))`d but still appears in the captured args):
//!   `[<root>, <sv>, <suf KANA-TEXT envelope | null>]`
//!
//! Result shape: `[<list> | null]`. `null` ↔ Lisp nil (find-word-full
//! returned nothing for `root + "ではない"`). Otherwise a list of word
//! envelopes — PROXY-TEXT for simple-text primaries (wrapped
//! `hintedp=T`), or the COMPOUND-TEXT itself with reassigned text/kana
//! (`dict-grammar.lisp:568-578`). The destem length is 4 (the
//! `:dewanai` stem). `:dewanai` is pushed onto `*suffix-unique-only*`
//! (`dict-grammar.lisp:621`).
//!
//! Comparison covers every projected slot of every returned word
//! recursively via `format!("{:?}", c)` Debug fingerprints, then
//! `id: NNN,` substrings are stripped before equality. Debug derives
//! on CompoundText / KanaText / KanjiText / ProxyText / SimpleText /
//! WordConjugations / ScoreMod are complete, so the fingerprint covers
//! every output field at every depth (including ProxyText's source
//! recursion and each leaf simple-text's seq / text / ord / common /
//! common_tags / conjugate_p / nokanji / best_* / state.conjugations /
//! state.hintedp).
//!
//! ## id slot is stripped before comparison
//!
//! abbr-dewanai reaches `find-word` via `find-word-full` during
//! extraction. When the segmenter has `*substring-hash*` bound, the
//! synthesis path at `dict.lisp:493` reconstructs DAOs via
//! `(apply 'make-instance init)` without `:id`. Captures emit JSON
//! null → audit-side `id = 0`. Rust replay hits the production DB and
//! returns real ids. Strict-equality on id would fail every
//! synthesized row.
//!
//! ## suf is ignored upstream
//!
//! The `def-abbr-suffix` macro at `dict-grammar.lisp:553-554` emits
//! `(declare (ignore ,suf))` — the captured third arg's content has
//! no behavioral influence. We parse it (to verify the projector
//! shape) but pass `None` to the function under test, mirroring the
//! ignored-slot semantics.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::grammar::abbr::abbr_dewanai;
use kaniran_core::dict::kani::KaniWordDispatchEnum;

use common::{parse_captured_simple_text, parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:ABBR-DEWANAI";

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
    verify_suf_shape(&row.args[2])?;

    let actual = abbr_dewanai(ctx, root, sv, None)
        .await
        .map_err(|e| format!("abbr_dewanai: {}", e))?;

    let expected = parse_expected(unwrap_result(&row.result)?)?;

    let actual_fp = canonical_fingerprints(&actual);
    let expected_fp = canonical_fingerprints(&expected);

    if actual_fp != expected_fp {
        return Err(format!(
            "result mismatch on root={:?} sv={:?}:\n  rust ={:?}\n  lisp ={:?}",
            root, sv, actual_fp, expected_fp
        ));
    }
    Ok(())
}

fn verify_suf_shape(value: &Value) -> Result<(), String> {
    match value {
        Value::Null => Ok(()),
        Value::Object(_) => parse_captured_simple_text(value).map(|_| ()),
        other => Err(format!(
            "arg 2 (suf): expected KANA-TEXT envelope or null, got {}",
            other
        )),
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
                        .map_err(|e| format!("word {}: {}", idx, e))?,
                );
            }
            Ok(out)
        }
        other => Err(format!("result: expected array or null, got {}", other)),
    }
}

/// Strip `id: NNN, ` substrings from the Debug fingerprint — see
/// module docstring for the synthesized-DAO rationale. Byte-level scan
/// is safe: "id: " and digit bytes are ASCII, and any multi-byte
/// UTF-8 (Japanese text) leading bytes are > 0x7F so they never
/// falsely match the ASCII pattern.
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

fn canonical_fingerprints(words: &[KaniWordDispatchEnum]) -> Vec<String> {
    let mut fps: Vec<String> = words
        .iter()
        .map(|w| strip_id(format!("{:?}", w)))
        .collect();
    fps.sort();
    fps
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
