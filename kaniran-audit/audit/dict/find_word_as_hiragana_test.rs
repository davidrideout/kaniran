//! Manual fixture-replay runner for `ICHIRAN/DICT:FIND-WORD-AS-HIRAGANA`.
//! Source under test: `src/dict/find_word_as_hiragana.rs`.
//!
//! Run with:
//!   cargo run --bin find_word_as_hiragana_test -- \
//!       --path corpus/<corpus_tag>/dict/find_word_as_hiragana.parquet
//!
//! Args: `("<text>" :EXCLUDE (seq...) :FINDER fn-or-nil)` (keywords absent
//! → defaults). `:FINDER` non-nil is currently skipped — the audit harness
//! has no replay strategy for closures.
//! Result: list of proxy-text plists. Each compared by `(text, kana, source-id)`.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::find_word::find_word_as_hiragana;
use kaniran_core::dict::kani::KaniSimpleTextDispatchEnum;
use kaniran_core::dict::text_classes::ProxyText;

use common::{captured_class, get_string, CapturedKanaText, CapturedKanjiText, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FIND-WORD-AS-HIRAGANA";

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
}


async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("find-word-as-hiragana args empty".into());
    }
    let str_ = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 not string: {}", row.args[0]))?;

    let mut exclude: Vec<i32> = Vec::new();
    let mut finder_present_non_nil = false;
    let mut i = 1;
    while i < row.args.len() {
        let key = row.args[i]
            .as_str()
            .ok_or_else(|| format!("keyword at {} not string: {}", i, row.args[i]))?;
        if i + 1 >= row.args.len() {
            return Err(format!("keyword {} missing value", key));
        }
        let v = &row.args[i + 1];
        match key {
            ":EXCLUDE" => {
                if !v.is_null() {
                    let arr = v
                        .as_array()
                        .ok_or_else(|| format!(":EXCLUDE not list: {}", v))?;
                    for item in arr {
                        let n = item
                            .as_i64()
                            .ok_or_else(|| format!(":EXCLUDE element not int: {}", item))?;
                        exclude.push(n as i32);
                    }
                }
            }
            ":FINDER" => finder_present_non_nil = !v.is_null(),
            other => return Err(format!("find-word-as-hiragana: unknown keyword {}", other)),
        }
        i += 2;
    }
    if finder_present_non_nil {
        return Err(":FINDER non-nil — audit harness has no replay strategy for closures".into());
    }

    let actual = find_word_as_hiragana(ctx, str_, &exclude, None)
        .await
        .map_err(|err| format!("find_word_as_hiragana query: {}", err))?;

    if row.result.is_empty() {
        return Err("expected at least 1 result value, got 0".into());
    }
    compare(actual, &row.result[0])
}

fn compare(mut actual: Vec<ProxyText>, expected: &Value) -> Result<(), String> {
    let mut captured = project_expected(expected)?;

    if actual.len() != captured.len() {
        return Err(format!(
            "row count: rust={} lisp={}",
            actual.len(),
            captured.len()
        ));
    }
    actual.sort_by(actual_proxy_sort_key);
    captured.sort_by(captured_proxy_sort_key);
    for (idx, (a, c)) in actual.iter().zip(&captured).enumerate() {
        if a.text != c.text || a.kana != c.kana {
            return Err(format!(
                "row {} proxy text/kana mismatch:\n  rust: ({:?}, {:?})\n  lisp: ({:?}, {:?})",
                idx, a.text, a.kana, c.text, c.kana
            ));
        }
        match (a.source.as_ref(), &c.source) {
            (KaniSimpleTextDispatchEnum::Kana(actual_k), CapturedSource::Kana(captured_k)) => {
                if !captured_k.matches(actual_k) {
                    return Err(format!(
                        "row {} source mismatch:\n  rust: {:?}\n  lisp: {:?}",
                        idx, actual_k, captured_k
                    ));
                }
            }
            (KaniSimpleTextDispatchEnum::Kanji(actual_k), CapturedSource::Kanji(captured_k)) => {
                if !captured_k.matches(actual_k) {
                    return Err(format!(
                        "row {} source mismatch:\n  rust: {:?}\n  lisp: {:?}",
                        idx, actual_k, captured_k
                    ));
                }
            }
            (KaniSimpleTextDispatchEnum::Proxy(_), _) => {
                return Err(format!(
                    "row {}: nested proxy under find-word-as-hiragana — unexpected",
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

fn project_expected(v: &Value) -> Result<Vec<CapturedProxy>, String> {
    if v.is_null() {
        return Ok(Vec::new());
    }
    let arr = v
        .as_array()
        .ok_or_else(|| format!("expected list, got {}", v))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let class = captured_class(item)?;
        if class != "PROXY-TEXT" {
            return Err(format!("expected proxy-text in result, got :{}", class));
        }
        let source_value = item
            .get("source")
            .ok_or_else(|| "proxy result missing source".to_string())?;
        let source_class = captured_class(source_value)?;
        let source = match source_class {
            "KANA-TEXT" => CapturedSource::Kana(
                serde_json::from_value(source_value.clone())
                    .map_err(|err| format!("kana-text source parse: {}", err))?,
            ),
            "KANJI-TEXT" => CapturedSource::Kanji(
                serde_json::from_value(source_value.clone())
                    .map_err(|err| format!("kanji-text source parse: {}", err))?,
            ),
            other => return Err(format!("unsupported proxy source class: :{}", other)),
        };
        out.push(CapturedProxy {
            text: get_string(item, "text"),
            kana: get_string(item, "kana"),
            source,
        });
    }
    Ok(out)
}


#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
