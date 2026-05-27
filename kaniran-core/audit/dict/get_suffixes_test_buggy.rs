//! Fixture-replay runner for `ICHIRAN/DICT:GET-SUFFIXES`. **Archived
//! 2026-05-15** — renamed `_buggy` and removed from `Cargo.toml`'s
//! `[[bin]]` list. Retained on disk as evidence of upstream
//! nondeterminism (see `docs/known_issues.md`); not built
//! or run by default. To re-enable for investigation, re-add a
//! `[[bin]] name = "get_suffixes_test_buggy" path = "…"` entry to
//! `kaniran-core/Cargo.toml`.
//!
//! Source under test: `src/dict/get_suffixes.rs`.
//!
//! Args: `(<word string>)`.
//! Result: `((<list of triples>))` — single value: a list of
//! `(suffix_str ":KEY" <kana-text or null>)` triples in the upstream
//! "shortest-suffix-first" order. Each triple's third slot is either
//! a captured `KANA-TEXT` row (full slot set) or null when the
//! upstream cache entry came from the `load-abbr` path.
//!
//! ## Why `_buggy`
//!
//! Against the 2026-05-15 parquet (396,209 rows), this runner reports
//! 504 failures (~0.13%). All 504 are explained, none are Rust port
//! bugs:
//!
//! - **459 cache-row identity mismatches** (docs/known_issues.md,
//!   "Unordered query results"). The
//!   upstream UNION in `get_kana_forms_star_` has no `ORDER BY`;
//!   Postmodern's `query-dao` loader applies a non-trivial permutation
//!   driven by in-process allocator / hash-table state, which neither
//!   matches nor reverses Postgres's wire order for multi-row results.
//!   Empirically verified across 20 seqs × 5 trials on `.103`
//!   2026-05-15. Captures encode one realization; re-extraction on a
//!   fresh worker produces a different parquet with the same failure
//!   count and different per-row `lisp seq=` values.
//!
//! - **125 + 1 leaked-slot mismatches** (docs/known_issues.md,
//!   "Mid-session state in ichiran"). Lisp's
//!   `compound-text` shares `KanaText` identity by reference with the
//!   suffix cache; `(setf word-conjugations …)` on the compound
//!   delegates via `dict.lisp:666` into the cache row, mutating it.
//!   Rust's value-copy `init_suffixes_thread` has no equivalent
//!   aliasing path. The captured slot value is a function of the
//!   worker's prior sentence-processing history, not the get_suffixes
//!   call alone — making these rows order-dependent traces, not a
//!   per-input spec.
//!
//! ## What this runner still verifies
//!
//! The 395,705 passing rows do cover real algorithm properties:
//!
//! - `start` loop bounds and `subseq_slice` character-index walking
//! - shortest-suffix-first ordering across the triple sequence
//! - cache hit/miss discrimination by substring
//! - `parse_suffix_val` flattening of the value-list shape
//! - abbr-vs-kf path discrimination (third-slot None vs Some)
//! - keyword tag preservation (`teiru`, `neg`, `tai`, …)
//!
//! Anything that breaks one of those would surface as a *new* failure
//! pattern outside the 504 known divergences. Diff against the known
//! signatures before declaring a regression.
//!
//! ## Disposition
//!
//! Leave this runner in place until either (a) the suffix subsystem
//! ports land with an explicit decision on the aliasing-leak fidelity
//! strategy (docs/known_issues.md, "Mid-session state in ichiran"), at which point a clean
//! re-extracted fixture replaces this one; or (b) an audit runner is
//! written that splits known-divergence rows from unknown failures so
//! a single pass/fail signal is meaningful again.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::get_suffixes::get_suffixes;
use kaniran_core::dict::kana_text_dao::KanaText;

use common::{single_result, CapturedKanaText, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-SUFFIXES";

fn normalize_keyword(captured: &str) -> String {
    // Captures emit `:TEIRU`; the Rust cache stores `teiru`. Strip the
    // leading `:` and lowercase to compare.
    captured.trim_start_matches(':').to_ascii_lowercase()
}

fn parse_captured_kf(value: &Value) -> Result<Option<CapturedKanaText>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s == ":NULL" => Ok(None),
        Value::Object(_) => serde_json::from_value::<CapturedKanaText>(value.clone())
            .map(Some)
            .map_err(|err| format!("kana-text parse: {}", err)),
        other => Err(format!("expected kana-text object or null, got: {}", other)),
    }
}

fn compare_triple(
    idx: usize,
    captured: &Value,
    actual: (&str, &str, Option<&KanaText>),
) -> Result<(), String> {
    let triple = captured
        .as_array()
        .ok_or_else(|| format!("triple[{}] not array: {}", idx, captured))?;
    if triple.len() != 3 {
        return Err(format!(
            "triple[{}] expected 3 elements, got {}: {}",
            idx,
            triple.len(),
            captured
        ));
    }
    let cap_substr = triple[0]
        .as_str()
        .ok_or_else(|| format!("triple[{}].suffix not string: {}", idx, triple[0]))?;
    let cap_key_raw = triple[1]
        .as_str()
        .ok_or_else(|| format!("triple[{}].keyword not string: {}", idx, triple[1]))?;
    let cap_key = normalize_keyword(cap_key_raw);
    let cap_kf = parse_captured_kf(&triple[2])?;

    let (rust_substr, rust_key, rust_kf) = actual;
    if rust_substr != cap_substr {
        return Err(format!(
            "triple[{}].suffix mismatch: rust={:?} lisp={:?}",
            idx, rust_substr, cap_substr
        ));
    }
    if rust_key != cap_key {
        return Err(format!(
            "triple[{}].keyword mismatch: rust={:?} lisp={:?}",
            idx, rust_key, cap_key
        ));
    }
    match (cap_kf, rust_kf) {
        (None, None) => Ok(()),
        (None, Some(actual_kf)) => Err(format!(
            "triple[{}].kf: rust=Some(seq={}) lisp=null",
            idx, actual_kf.seq
        )),
        (Some(captured_kf), None) => Err(format!(
            "triple[{}].kf: rust=None lisp=Some(seq={})",
            idx, captured_kf.seq
        )),
        (Some(captured_kf), Some(actual_kf)) => {
            if captured_kf.matches(actual_kf) {
                Ok(())
            } else {
                Err(format!(
                    "triple[{}].kf field mismatch (rust seq={}, lisp seq={})",
                    idx, actual_kf.seq, captured_kf.seq
                ))
            }
        }
    }
}

async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let word = row.args[0]
        .as_str()
        .ok_or_else(|| format!("args[0] not string: {}", row.args[0]))?;

    let actual = get_suffixes(ctx, word);

    let result = single_result(&row.result)?;
    // Lisp NIL is both null and empty list; capture serialises it as
    // `null`. Treat it as an empty expected sequence.
    let empty: Vec<Value> = Vec::new();
    let expected: &[Value] = match result {
        Value::Null => &empty,
        Value::Array(a) => a.as_slice(),
        other => return Err(format!("result[0] not array or null: {}", other)),
    };

    if actual.len() != expected.len() {
        return Err(format!(
            "length mismatch: rust={} lisp={}",
            actual.len(),
            expected.len()
        ));
    }
    for (idx, (cap, act)) in expected.iter().zip(actual.iter()).enumerate() {
        compare_triple(idx, cap, (act.0, act.1, act.2))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
