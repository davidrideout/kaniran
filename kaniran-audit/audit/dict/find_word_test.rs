//! Manual fixture-replay runner for `ICHIRAN/DICT:FIND-WORD`.
//! Source under test: `src/dict/find_word.rs`.
//!
//! Run with:
//!   cargo run --bin find_word_test -- \
//!       --path corpus/<corpus_tag>/dict/find_word.parquet
//!
//! Args: `("<word>" :ROOT-ONLY t-or-nil)` (`:ROOT-ONLY` may be absent).
//! Result: list of kanji-text or kana-text DAOs, OR a 2-element envelope
//! `(<list> _ignored)` for the two-value-list form upstream sometimes
//! emits — the audit accepts either shape.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::readings::{find_word, FindWordRows};
use kaniran_core::dict::dao::KanaText;
use kaniran_core::dict::dao::KanjiText;

use common::{captured_class, CapturedKanaText, CapturedKanjiText, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FIND-WORD";


fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("find-word args empty".into());
    }
    let word = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 not string: {}", row.args[0]))?;
    let root_only = walk_keywords_for_root_only(&row.args[1..])?;

    let actual = find_word(ctx, word, root_only)
        
        .map_err(|err| format!("find_word query: {}", err))?
        .into_owned();

    let expected_list = unwrap_result_list(&row.result)?;
    compare(actual, expected_list)
}

fn walk_keywords_for_root_only(tail: &[Value]) -> Result<bool, String> {
    let mut i = 0;
    let mut root_only = false;
    while i < tail.len() {
        let key = tail[i]
            .as_str()
            .ok_or_else(|| format!("keyword at {} not string: {}", i, tail[i]))?;
        if i + 1 >= tail.len() {
            return Err(format!("keyword {} missing value", key));
        }
        let v = &tail[i + 1];
        match key {
            ":ROOT-ONLY" => root_only = v.as_bool().unwrap_or(false),
            other => return Err(format!("find-word: unknown keyword {}", other)),
        }
        i += 2;
    }
    Ok(root_only)
}

/// Lisp `(values list ...)` shapes up as either `[list]` or `[list, ...]`
/// depending on how the projector flattens. Take the first element.
fn unwrap_result_list(result: &[Value]) -> Result<&Value, String> {
    if result.is_empty() {
        return Err("expected at least 1 result value, got 0".into());
    }
    Ok(&result[0])
}

fn compare(actual: FindWordRows, expected: &Value) -> Result<(), String> {
    let expected_arr: Vec<&Value> = if expected.is_null() {
        Vec::new()
    } else {
        expected
            .as_array()
            .ok_or_else(|| format!("expected list, got {}", expected))?
            .iter()
            .collect()
    };

    match actual {
        FindWordRows::Kana(mut actual_rows) => {
            let mut captured: Vec<CapturedKanaText> = expected_arr
                .iter()
                .map(|item| {
                    let class = captured_class(item)?;
                    if class != "KANA-TEXT" {
                        return Err(format!(
                            "rust returned KANA-TEXT rows but lisp row is :{}",
                            class
                        ));
                    }
                    serde_json::from_value((*item).clone())
                        .map_err(|err| format!("kana-text parse: {}", err))
                })
                .collect::<Result<_, _>>()?;
            if actual_rows.len() != captured.len() {
                return Err(format!(
                    "row count: rust={} lisp={}",
                    actual_rows.len(),
                    captured.len()
                ));
            }
            actual_rows.sort_by(kana_sort_key);
            captured.sort_by(captured_kana_sort_key);
            for (idx, (a, c)) in actual_rows.iter().zip(&captured).enumerate() {
                if !c.matches(a) {
                    return Err(format!(
                        "row {}: rust={:?} lisp={:?}",
                        idx, a, c
                    ));
                }
            }
        }
        FindWordRows::Kanji(mut actual_rows) => {
            let mut captured: Vec<CapturedKanjiText> = expected_arr
                .iter()
                .map(|item| {
                    let class = captured_class(item)?;
                    if class != "KANJI-TEXT" {
                        return Err(format!(
                            "rust returned KANJI-TEXT rows but lisp row is :{}",
                            class
                        ));
                    }
                    serde_json::from_value((*item).clone())
                        .map_err(|err| format!("kanji-text parse: {}", err))
                })
                .collect::<Result<_, _>>()?;
            if actual_rows.len() != captured.len() {
                return Err(format!(
                    "row count: rust={} lisp={}",
                    actual_rows.len(),
                    captured.len()
                ));
            }
            actual_rows.sort_by(kanji_sort_key);
            captured.sort_by(captured_kanji_sort_key);
            for (idx, (a, c)) in actual_rows.iter().zip(&captured).enumerate() {
                if !c.matches(a) {
                    return Err(format!(
                        "row {}: rust={:?} lisp={:?}",
                        idx, a, c
                    ));
                }
            }
        }
    }
    Ok(())
}

fn kana_sort_key(a: &KanaText, b: &KanaText) -> std::cmp::Ordering {
    (a.seq, &a.text, a.ord, a.id).cmp(&(b.seq, &b.text, b.ord, b.id))
}

fn kanji_sort_key(a: &KanjiText, b: &KanjiText) -> std::cmp::Ordering {
    (a.seq, &a.text, a.ord, a.id).cmp(&(b.seq, &b.text, b.ord, b.id))
}

fn captured_kana_sort_key(a: &CapturedKanaText, b: &CapturedKanaText) -> std::cmp::Ordering {
    (a.seq, &a.text, a.ord, a.id).cmp(&(b.seq, &b.text, b.ord, b.id))
}

fn captured_kanji_sort_key(a: &CapturedKanjiText, b: &CapturedKanjiText) -> std::cmp::Ordering {
    (a.seq, &a.text, a.ord, a.id).cmp(&(b.seq, &b.text, b.ord, b.id))
}


fn main() {
    common::run_async(EXPECTED_FQN, audit_one);
}
