//! Manual fixture-replay runner for `ICHIRAN/DICT:OR-AS-HIRAGANA`.
//! Source under test: `src/dict/or_as_hiragana.rs`.
//!
//! Run with:
//!   cargo run --bin or_as_hiragana_test -- \
//!       --path corpus/<corpus_tag>/dict/or_as_hiragana.parquet
//!
//! Replays a captured finder symbol plus word/pos args through
//! `or_as_hiragana` and compares the returned word rows against the
//! Lisp result (direct rows by `(seq, id, text)`, proxy rows by
//! `(text, kana, source-id)`).

#[path = "../common/mod.rs"]
mod common;

use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use kaniran_core::dict::find_word::FindWordRows;
use kaniran_core::dict::find_word_with_pos::{find_word_with_pos, WordWithPosRows};
use kaniran_core::dict::kani_word::KaniSimpleTextDispatchEnum;
use kaniran_core::dict::or_as_hiragana::{
    or_as_hiragana, OrAsHiraganaFinder, OrAsHiraganaRows,
};
use kaniran_core::dict::proxy_text_class::ProxyText;
use kaniran_core::dict::simple_text_class::WordConjugations;

use common::{captured_class, get_string, CapturedKanaText, CapturedKanjiText, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:OR-AS-HIRAGANA";

#[derive(Debug)]
enum CapturedSource {
    Kana(CapturedKanaText),
    Kanji(CapturedKanjiText),
}

#[derive(Debug)]
struct CapturedProxy {
    text: String,
    kana: String,
    source: CapturedSource,
    conjugations: Option<WordConjugations>,
    hintedp: bool,
}

fn parse_hintedp(value: &Value) -> Result<bool, String> {
    match value {
        Value::Null => Ok(false),
        Value::Bool(b) => Ok(*b),
        other => Err(format!("hintedp: expected null/bool, got {}", other)),
    }
}

fn parse_conjugations(value: &Value) -> Result<Option<WordConjugations>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s == ":ROOT" => Ok(Some(WordConjugations::Root)),
        Value::Array(arr) => {
            let mut ids = Vec::with_capacity(arr.len());
            for v in arr {
                let id = v
                    .as_i64()
                    .ok_or_else(|| format!("conj-id not int: {}", v))?;
                ids.push(id as i32);
            }
            Ok(Some(WordConjugations::Ids(ids)))
        }
        other => Err(format!(
            "conjugations: expected null / \":ROOT\" / int-array, got {}",
            other
        )),
    }
}

#[derive(Debug)]
enum CapturedRows {
    Empty,
    Kana(Vec<CapturedKanaText>),
    Kanji(Vec<CapturedKanjiText>),
    Proxy(Vec<CapturedProxy>),
}

async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.len() < 2 {
        return Err(format!("expected ≥2 args, got {}", row.args.len()));
    }
    let fn_name = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 not string: {}", row.args[0]))?;
    if fn_name != ":FIND-WORD-WITH-POS" {
        return Err(format!("unsupported fn arg: {}", fn_name));
    }
    let word = row.args[1]
        .as_str()
        .ok_or_else(|| format!("arg 1 not string: {}", row.args[1]))?
        .to_string();
    let mut posi: Vec<String> = Vec::with_capacity(row.args.len() - 2);
    for (idx, arg) in row.args.iter().enumerate().skip(2) {
        let s = arg
            .as_str()
            .ok_or_else(|| format!("posi arg {} not string: {}", idx, arg))?;
        posi.push(s.to_string());
    }

    let posi_arc = Arc::new(posi);
    let posi_for_closure = Arc::clone(&posi_arc);
    let finder: OrAsHiraganaFinder<'_> = Arc::new(move |w: String| {
        let posi_inner = Arc::clone(&posi_for_closure);
        Box::pin(async move {
            let refs: Vec<&str> = posi_inner.iter().map(|s| s.as_str()).collect();
            let rows = find_word_with_pos(ctx, &w, &refs).await?;
            Ok(match rows {
                WordWithPosRows::Kana(v) => FindWordRows::Kana(v),
                WordWithPosRows::Kanji(v) => FindWordRows::Kanji(v),
            })
        }) as Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });

    let actual = or_as_hiragana(ctx, &word, finder)
        .await
        .map_err(|err| format!("or_as_hiragana query: {}", err))?;

    if row.result.is_empty() {
        return Err("expected at least 1 result value, got 0".into());
    }
    compare(actual, &row.result[0])
}

fn project_expected(v: &Value) -> Result<CapturedRows, String> {
    if v.is_null() {
        return Ok(CapturedRows::Empty);
    }
    let arr = v
        .as_array()
        .ok_or_else(|| format!("expected list, got {}", v))?;
    if arr.is_empty() {
        return Ok(CapturedRows::Empty);
    }
    let first_class = captured_class(&arr[0])?.to_string();
    match first_class.as_str() {
        "KANA-TEXT" => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                let class = captured_class(item)?;
                if class != "KANA-TEXT" {
                    return Err(format!("heterogeneous row list (kana + {})", class));
                }
                out.push(
                    serde_json::from_value::<CapturedKanaText>(item.clone())
                        .map_err(|err| format!("kana-text parse: {}", err))?,
                );
            }
            Ok(CapturedRows::Kana(out))
        }
        "KANJI-TEXT" => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                let class = captured_class(item)?;
                if class != "KANJI-TEXT" {
                    return Err(format!("heterogeneous row list (kanji + {})", class));
                }
                out.push(
                    serde_json::from_value::<CapturedKanjiText>(item.clone())
                        .map_err(|err| format!("kanji-text parse: {}", err))?,
                );
            }
            Ok(CapturedRows::Kanji(out))
        }
        "PROXY-TEXT" => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                let class = captured_class(item)?;
                if class != "PROXY-TEXT" {
                    return Err(format!("heterogeneous row list (proxy + {})", class));
                }
                let source_value = item
                    .get("source")
                    .ok_or_else(|| "proxy missing source".to_string())?;
                let source_class = captured_class(source_value)?;
                let source = match source_class {
                    "KANA-TEXT" => CapturedSource::Kana(
                        serde_json::from_value(source_value.clone())
                            .map_err(|err| format!("kana source parse: {}", err))?,
                    ),
                    "KANJI-TEXT" => CapturedSource::Kanji(
                        serde_json::from_value(source_value.clone())
                            .map_err(|err| format!("kanji source parse: {}", err))?,
                    ),
                    other => return Err(format!("unsupported proxy source class: :{}", other)),
                };
                let conjugations = parse_conjugations(item.get("conjugations").unwrap_or(&Value::Null))?;
                let hintedp = parse_hintedp(item.get("hintedp").unwrap_or(&Value::Null))?;
                out.push(CapturedProxy {
                    text: get_string(item, "text"),
                    kana: get_string(item, "kana"),
                    source,
                    conjugations,
                    hintedp,
                });
            }
            Ok(CapturedRows::Proxy(out))
        }
        other => Err(format!("unsupported result class: :{}", other)),
    }
}

fn compare(actual: Option<OrAsHiraganaRows>, expected: &Value) -> Result<(), String> {
    let captured = project_expected(expected)?;
    match (actual, captured) {
        (None, CapturedRows::Empty) => Ok(()),
        (None, captured) => Err(format!("rust=None lisp={:?}", captured)),
        (Some(rows), CapturedRows::Empty) => {
            Err(format!("rust=Some({:?}) lisp=NIL", rows))
        }
        (Some(OrAsHiraganaRows::Direct(FindWordRows::Kana(mut a))), CapturedRows::Kana(mut c)) => {
            if a.len() != c.len() {
                return Err(format!("kana row count: rust={} lisp={}", a.len(), c.len()));
            }
            a.sort_by(|x, y| (x.seq, x.id, x.text.clone()).cmp(&(y.seq, y.id, y.text.clone())));
            c.sort_by(|x, y| (x.seq, x.id, x.text.clone()).cmp(&(y.seq, y.id, y.text.clone())));
            for (idx, (actual_row, captured_row)) in a.iter().zip(&c).enumerate() {
                if !captured_row.matches(actual_row) {
                    return Err(format!(
                        "row {} kana-text mismatch:\n  rust: {:?}\n  lisp: {:?}",
                        idx, actual_row, captured_row
                    ));
                }
            }
            Ok(())
        }
        (
            Some(OrAsHiraganaRows::Direct(FindWordRows::Kanji(mut a))),
            CapturedRows::Kanji(mut c),
        ) => {
            if a.len() != c.len() {
                return Err(format!("kanji row count: rust={} lisp={}", a.len(), c.len()));
            }
            a.sort_by(|x, y| (x.seq, x.id, x.text.clone()).cmp(&(y.seq, y.id, y.text.clone())));
            c.sort_by(|x, y| (x.seq, x.id, x.text.clone()).cmp(&(y.seq, y.id, y.text.clone())));
            for (idx, (actual_row, captured_row)) in a.iter().zip(&c).enumerate() {
                if !captured_row.matches(actual_row) {
                    return Err(format!(
                        "row {} kanji-text mismatch:\n  rust: {:?}\n  lisp: {:?}",
                        idx, actual_row, captured_row
                    ));
                }
            }
            Ok(())
        }
        (Some(OrAsHiraganaRows::AsHiragana(mut a)), CapturedRows::Proxy(mut c)) => {
            if a.len() != c.len() {
                return Err(format!("proxy row count: rust={} lisp={}", a.len(), c.len()));
            }
            a.sort_by(actual_proxy_sort_key);
            c.sort_by(captured_proxy_sort_key);
            for (idx, (actual_row, captured_row)) in a.iter().zip(&c).enumerate() {
                if actual_row.text != captured_row.text || actual_row.kana != captured_row.kana {
                    return Err(format!(
                        "row {} proxy text/kana mismatch:\n  rust: ({:?}, {:?})\n  lisp: ({:?}, {:?})",
                        idx, actual_row.text, actual_row.kana,
                        captured_row.text, captured_row.kana
                    ));
                }
                if actual_row.state.conjugations != captured_row.conjugations {
                    return Err(format!(
                        "row {} proxy conjugations mismatch:\n  rust: {:?}\n  lisp: {:?}",
                        idx, actual_row.state.conjugations, captured_row.conjugations
                    ));
                }
                if actual_row.state.hintedp != captured_row.hintedp {
                    return Err(format!(
                        "row {} proxy hintedp mismatch:\n  rust: {} lisp: {}",
                        idx, actual_row.state.hintedp, captured_row.hintedp
                    ));
                }
                match (actual_row.source.as_ref(), &captured_row.source) {
                    (
                        KaniSimpleTextDispatchEnum::Kana(actual_k),
                        CapturedSource::Kana(captured_k),
                    ) => {
                        if !captured_k.matches(actual_k) {
                            return Err(format!(
                                "row {} kana source mismatch:\n  rust: {:?}\n  lisp: {:?}",
                                idx, actual_k, captured_k
                            ));
                        }
                    }
                    (
                        KaniSimpleTextDispatchEnum::Kanji(actual_k),
                        CapturedSource::Kanji(captured_k),
                    ) => {
                        if !captured_k.matches(actual_k) {
                            return Err(format!(
                                "row {} kanji source mismatch:\n  rust: {:?}\n  lisp: {:?}",
                                idx, actual_k, captured_k
                            ));
                        }
                    }
                    (KaniSimpleTextDispatchEnum::Proxy(_), _) => {
                        return Err(format!(
                            "row {}: nested proxy under or-as-hiragana — unexpected",
                            idx
                        ));
                    }
                    (actual_simple, captured_source) => {
                        return Err(format!(
                            "row {} source class mismatch: rust={:?} lisp={:?}",
                            idx, actual_simple, captured_source
                        ));
                    }
                }
            }
            Ok(())
        }
        (actual_other, captured_other) => Err(format!(
            "shape mismatch:\n  rust: {:?}\n  lisp: {:?}",
            actual_other, captured_other
        )),
    }
}

fn actual_proxy_sort_key(a: &ProxyText, b: &ProxyText) -> std::cmp::Ordering {
    let a_id = source_sort_id(&a.source);
    let b_id = source_sort_id(&b.source);
    (a_id, &a.text, &a.kana).cmp(&(b_id, &b.text, &b.kana))
}

fn captured_proxy_sort_key(a: &CapturedProxy, b: &CapturedProxy) -> std::cmp::Ordering {
    let a_id = captured_source_sort_id(&a.source);
    let b_id = captured_source_sort_id(&b.source);
    (a_id, &a.text, &a.kana).cmp(&(b_id, &b.text, &b.kana))
}

fn source_sort_id(s: &KaniSimpleTextDispatchEnum) -> (i32, String) {
    match s {
        KaniSimpleTextDispatchEnum::Kana(k) => (k.seq, k.text.clone()),
        KaniSimpleTextDispatchEnum::Kanji(k) => (k.seq, k.text.clone()),
        KaniSimpleTextDispatchEnum::Proxy(_) => (0, String::new()),
    }
}

fn captured_source_sort_id(s: &CapturedSource) -> (i32, String) {
    match s {
        CapturedSource::Kana(k) => (k.seq, k.text.clone()),
        CapturedSource::Kanji(k) => (k.seq, k.text.clone()),
    }
}

#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
