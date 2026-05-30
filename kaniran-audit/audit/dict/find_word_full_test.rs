//! Manual fixture-replay runner for `ICHIRAN/DICT:FIND-WORD-FULL`.
//! Source under test: `src/dict/find_word_full.rs`.
//!
//! Run with:
//!   cargo run --release --bin find_word_full_test -- \
//!       --path corpus/extracted_find_word_layer_2026_05_21/dict/find_word_full.parquet
//!
//! Args shape (per `(find-word-full word &key as-hiragana counter)`):
//!   `[<word>, ":AS-HIRAGANA", <bool-or-null>, ":COUNTER", <null|":AUTO"|int>]`
//!
//! Result shape: `[<list-or-null>]` — a heterogeneous list of KANA-TEXT
//! / KANJI-TEXT / PROXY-TEXT / COMPOUND-TEXT / COUNTER-* rows, or `null`
//! for the no-match case.
//!
//! ## id-stripping
//!
//! Top-level KANA-TEXT / KANJI-TEXT rows from `find-word` are produced
//! via the `*substring-hash*` `make-instance` branch (`dict.lisp:491-493`)
//! when find-word-full runs under `join-substring-words*` — those rows
//! arrive with `id` slot unbound and the projector emits `"id": null`.
//! The Rust port reads ctx.substring_hash and would follow the same
//! branch, but the audit-side ctx is constructed via `from_env` with no
//! substring_hash bound, so its find-word returns DB rows with real ids.
//! The audit zeroes `id` on both sides before comparison so the
//! id-source asymmetry doesn't fire false-positive divergences. The
//! `seq + text + ord` triple already uniquely identifies the row.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::compound_text_class::CompoundText;
use kaniran_core::dict::counters::classes::{Counter, CounterSource, CounterText};
use kaniran_core::dict::find_word_full::{find_word_full, CounterArg};
use kaniran_core::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::proxy_text_class::ProxyText;

use common::{parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FIND-WORD-FULL";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("find-word-full args empty".into());
    }
    let word = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 (word) not string: {}", row.args[0]))?;
    let (as_hiragana, counter) = parse_keywords(&row.args[1..])?;

    let mut actual = find_word_full(ctx, word, as_hiragana, counter)
        .await
        .map_err(|err| format!("find_word_full query: {}", err))?;

    // dict.lisp:491-493 — *substring-hash* `make-instance` branch leaves
    // `id` unbound. Zero on both sides so Rust DB-loaded ids don't
    // diverge from the captured `null`s. See module doc.
    for w in actual.iter_mut() {
        strip_ids_word(w);
    }

    let expected_value = row
        .result
        .first()
        .ok_or_else(|| "expected at least 1 result value, got 0".to_string())?;
    let mut expected = parse_expected(expected_value)?;
    for w in expected.iter_mut() {
        strip_ids_word(w);
    }

    if actual.len() != expected.len() {
        return Err(format!(
            "row count: rust={} lisp={} (rust: {:?}, lisp: {:?})",
            actual.len(),
            expected.len(),
            canonical_fingerprints(&actual),
            canonical_fingerprints(&expected),
        ));
    }
    let actual_fp = canonical_fingerprints(&actual);
    let expected_fp = canonical_fingerprints(&expected);
    if actual_fp != expected_fp {
        // First differing index for a focused diff line.
        let mut first_diff: Option<usize> = None;
        for (idx, (a, e)) in actual_fp.iter().zip(expected_fp.iter()).enumerate() {
            if a != e {
                first_diff = Some(idx);
                break;
            }
        }
        let diff_idx = first_diff.unwrap_or(0);
        return Err(format!(
            "result mismatch at sorted idx {}:\n  rust = {}\n  lisp = {}",
            diff_idx, actual_fp[diff_idx], expected_fp[diff_idx],
        ));
    }
    Ok(())
}

/// Parse `:AS-HIRAGANA <v> :COUNTER <v>` keyword pairs. Both keywords
/// may appear in any order; missing → default (false / None). Order
/// also tolerates absence per `(find-word-full word &key ...)` shape.
fn parse_keywords(tail: &[Value]) -> Result<(bool, Option<CounterArg>), String> {
    let mut as_hiragana = false;
    let mut counter: Option<CounterArg> = None;
    let mut i = 0;
    while i < tail.len() {
        let key = tail[i]
            .as_str()
            .ok_or_else(|| format!("keyword at {} not string: {}", i, tail[i]))?;
        if i + 1 >= tail.len() {
            return Err(format!("keyword {} missing value", key));
        }
        let v = &tail[i + 1];
        match key {
            ":AS-HIRAGANA" => {
                as_hiragana = match v {
                    Value::Null => false,
                    Value::Bool(b) => *b,
                    // CL truthiness: any non-nil → T.
                    _ => true,
                };
            }
            ":COUNTER" => {
                counter = match v {
                    Value::Null => None,
                    Value::String(s) if s == ":AUTO" => Some(CounterArg::Auto),
                    Value::Number(n) => {
                        let idx = n
                            .as_u64()
                            .ok_or_else(|| format!(":COUNTER index not u64: {}", n))?;
                        Some(CounterArg::At(idx as usize))
                    }
                    other => return Err(format!(":COUNTER: unexpected value {}", other)),
                };
            }
            other => return Err(format!("find-word-full: unknown keyword {}", other)),
        }
        i += 2;
    }
    Ok((as_hiragana, counter))
}

fn parse_expected(value: &Value) -> Result<Vec<KaniWordDispatchEnum>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for (idx, item) in arr.iter().enumerate() {
                out.push(
                    parse_captured_word(item)
                        .map_err(|err| format!("result item {}: {}", idx, err))?,
                );
            }
            Ok(out)
        }
        other => Err(format!("result: expected array or null, got {}", other)),
    }
}

fn canonical_fingerprints(words: &[KaniWordDispatchEnum]) -> Vec<String> {
    let mut fps: Vec<String> = words.iter().map(|w| format!("{:?}", w)).collect();
    fps.sort();
    fps
}

// --- id-stripping (see module doc) ---------------------------------------

fn strip_ids_word(word: &mut KaniWordDispatchEnum) {
    match word {
        KaniWordDispatchEnum::Kana(k) => k.id = 0,
        KaniWordDispatchEnum::Kanji(k) => k.id = 0,
        KaniWordDispatchEnum::Proxy(p) => strip_ids_proxy(p),
        KaniWordDispatchEnum::Compound(c) => strip_ids_compound(c),
        KaniWordDispatchEnum::Counter(c) => strip_ids_counter(c),
    }
}

fn strip_ids_simple(simple: &mut KaniSimpleTextDispatchEnum) {
    match simple {
        KaniSimpleTextDispatchEnum::Kana(k) => k.id = 0,
        KaniSimpleTextDispatchEnum::Kanji(k) => k.id = 0,
        KaniSimpleTextDispatchEnum::Proxy(p) => strip_ids_proxy(p),
    }
}

fn strip_ids_proxy(proxy: &mut ProxyText) {
    strip_ids_simple(&mut proxy.source);
}

fn strip_ids_compound(compound: &mut CompoundText) {
    strip_ids_word(&mut compound.primary);
    for w in compound.words.iter_mut() {
        strip_ids_word(w);
    }
    if let Some(sb) = compound.score_base.as_mut() {
        strip_ids_word(sb);
    }
}

fn strip_ids_counter(counter: &mut Counter) {
    let base: &mut CounterText = match counter {
        Counter::Base(b) => b,
        Counter::NumberText(c) => &mut c.0,
        Counter::Tsu(c) => &mut c.0,
        Counter::Age(c) => &mut c.0,
        Counter::DaysKun(c) => &mut c.0,
        Counter::DaysOn(c) => &mut c.0,
        Counter::Halfhour(c) => &mut c.0,
        Counter::Months(c) => &mut c.0,
        Counter::People(c) => &mut c.0,
        Counter::Wari(c) => &mut c.0,
        Counter::Hifumi(c) => &mut c.base,
    };
    if let Some(src) = base.source.as_mut() {
        match src {
            CounterSource::Kanji(k) => k.id = 0,
            CounterSource::Kana(k) => k.id = 0,
        }
    }
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
