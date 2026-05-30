//! Manual fixture-replay runner for `ICHIRAN/DICT:FIND-WORD-WITH-POS`.
//! Source under test: `src/dict/find_word_with_pos.rs`.
//!
//! Run with:
//!   cargo run --bin find_word_with_pos_test -- \
//!       --path corpus/<corpus_tag>/dict/find_word_with_pos.parquet
//!
//! Args: `("<word>" "<pos1>" "<pos2>" ...)` — the upstream `&rest posi`
//! becomes a tail of positional strings.
//! Result: list of kanji-text or kana-text DAOs.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::grammar::find_word::{find_word_with_pos, WordWithPosRows};
use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kanji_text_dao::KanjiText;

use common::{captured_class, CapturedKanaText, CapturedKanjiText, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FIND-WORD-WITH-POS";


async fn audit_one(
    ctx: &kaniran_core::conn::kani_context::KaniranContext,
    row: &CapturedRow,
) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("find-word-with-pos args empty".into());
    }
    let word = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 not string: {}", row.args[0]))?;
    let mut posi: Vec<&str> = Vec::with_capacity(row.args.len() - 1);
    for (idx, value) in row.args.iter().enumerate().skip(1) {
        let pos = value
            .as_str()
            .ok_or_else(|| format!("arg {} not string: {}", idx, value))?;
        posi.push(pos);
    }

    let actual = find_word_with_pos(ctx, word, &posi)
        .await
        .map_err(|err| format!("find_word_with_pos query: {}", err))?;

    let expected_list = unwrap_result_list(&row.result)?;
    compare(actual, expected_list)
}

/// Lisp `(values list ...)` shapes up as either `[list]` or `[list, ...]`
/// depending on how the projector flattens. Take the first element.
fn unwrap_result_list(result: &[Value]) -> Result<&Value, String> {
    if result.is_empty() {
        return Err("expected at least 1 result value, got 0".into());
    }
    Ok(&result[0])
}

fn compare(actual: WordWithPosRows, expected: &Value) -> Result<(), String> {
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
        WordWithPosRows::Kana(mut actual_rows) => {
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
                    return Err(format!("row {}: rust={:?} lisp={:?}", idx, a, c));
                }
            }
        }
        WordWithPosRows::Kanji(mut actual_rows) => {
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
                    return Err(format!("row {}: rust={:?} lisp={:?}", idx, a, c));
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


#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
