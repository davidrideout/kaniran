//! Manual fixture-replay runner for `ICHIRAN/DICT:KANJI-BREAK-PENALTY`.
//! Source under test: `src/dict/kanji_break_penalty.rs`.
//!
//! Run with:
//!   cargo run --release --bin kanji_break_penalty_test -- \
//!       --path corpus/extracted_chunk_b_segmentation_2026_05_14/dict/kanji_break_penalty.parquet
//!
//! Replays a captured kanji-break list (plus score, info, text,
//! use-length, score-mod) through `kanji_break_penalty` and compares
//! the returned penalized score against the Lisp result.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::text_classes::ScoreMod;
use kaniran_core::dict::conj::ConjData;
use kaniran_core::dict::scoring::score::kanji_break_penalty;
use kaniran_core::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo};

use common::{
    parse_conj_list, parse_int_list, parse_kpcl, parse_opt_i32, parse_score_info,
    parse_string_list, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:KANJI-BREAK-PENALTY";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() < 2 {
        return Err(format!("kbp args: expected ≥2, got {}", row.args.len()));
    }
    let kanji_break = parse_kanji_break(&row.args[0])?;
    let score = row.args[1]
        .as_i64()
        .ok_or_else(|| format!("score not int: {}", row.args[1]))? as i32;
    let (info, text, use_length, score_mod) = walk_keywords(&row.args[2..])?;

    let actual = kanji_break_penalty(
        ctx,
        &kanji_break,
        score,
        info.as_ref(),
        &text,
        use_length,
        score_mod.as_ref(),
    )
    .await
    .map_err(|e| format!("kanji_break_penalty: {}", e))?;

    if row.result.len() != 1 {
        return Err(format!("expected 1 result, got {}", row.result.len()));
    }
    let expected = row.result[0]
        .as_i64()
        .ok_or_else(|| format!("result[0] not int: {}", row.result[0]))?
        as i32;
    if actual != expected {
        return Err(format!("score: rust={} lisp={}", actual, expected));
    }
    Ok(())
}

fn parse_kanji_break(v: &Value) -> Result<Vec<usize>, String> {
    match v {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr
            .iter()
            .map(|n| {
                n.as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| format!("kanji-break entry not uint: {}", n))
            })
            .collect::<Result<_, _>>(),
        other => Err(format!("kanji-break: expected array/null, got {}", other)),
    }
}

fn walk_keywords(
    tail: &[Value],
) -> Result<(Option<KaniSegmentInfo>, String, Option<i32>, Option<ScoreMod>), String> {
    let mut info: Option<KaniSegmentInfo> = None;
    let mut text: String = String::new();
    let mut use_length: Option<i32> = None;
    let mut score_mod: Option<ScoreMod> = None;
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
            ":INFO" => {
                info = match v {
                    Value::Null => None,
                    other => Some(parse_info_plist(other)?),
                };
            }
            ":TEXT" => {
                text = match v {
                    Value::Null => String::new(),
                    Value::String(s) => s.clone(),
                    other => return Err(format!(":TEXT: expected string/null, got {}", other)),
                };
            }
            ":USE-LENGTH" => {
                use_length = match v {
                    Value::Null => None,
                    Value::Number(n) => Some(
                        n.as_i64()
                            .ok_or_else(|| format!(":USE-LENGTH not i64: {}", n))?
                            as i32,
                    ),
                    other => {
                        return Err(format!(":USE-LENGTH: expected int/null, got {}", other))
                    }
                };
            }
            ":SCORE-MOD" => {
                score_mod = parse_score_mod(v)?;
            }
            other => return Err(format!("kbp: unexpected keyword {}", other)),
        }
        i += 2;
    }
    Ok((info, text, use_length, score_mod))
}

fn parse_score_mod(v: &Value) -> Result<Option<ScoreMod>, String> {
    match v {
        Value::Null => Ok(None),
        Value::Number(n) => Ok(Some(ScoreMod::Single(
            n.as_i64()
                .ok_or_else(|| format!("score-mod int not i64: {}", n))?,
        ))),
        Value::Array(arr) => {
            let mut items = Vec::with_capacity(arr.len());
            for elt in arr {
                let i = elt.as_i64().ok_or_else(|| {
                    format!(
                        "score-mod list entry not i64 (likely a (constantly N) closure — \
                         projector clause for function-typed slots missing): {}",
                        elt
                    )
                })?;
                items.push(ScoreMod::Single(i));
            }
            Ok(Some(ScoreMod::Stack(items)))
        }
        other => Err(format!(
            "score-mod: expected int/list/null, got {}",
            other
        )),
    }
}

/// Parse a captured `:INFO` plist back into a [`KaniSegmentInfo`]. The
/// captured shape mirrors the one calc-score writes:
///   `(:POSI (..) :SEQ-SET (..) :CONJ (..) :COMMON .. :SCORE-INFO (..) :KPCL (..))`
fn parse_info_plist(v: &Value) -> Result<KaniSegmentInfo, String> {
    let arr = v
        .as_array()
        .ok_or_else(|| format!("info: expected plist array, got {}", v))?;
    if arr.len() % 2 != 0 {
        return Err(format!("info: odd plist length {}", arr.len()));
    }
    let mut posi: Vec<String> = Vec::new();
    let mut seq_set: Vec<i32> = Vec::new();
    let mut conj: Vec<ConjData> = Vec::new();
    let mut common: Option<i32> = None;
    let mut score_info: Option<KaniScoreInfo> = None;
    let mut kpcl: Option<(bool, bool, bool, bool)> = None;
    let mut i = 0;
    while i < arr.len() {
        let key = arr[i]
            .as_str()
            .ok_or_else(|| format!("info key at {} not string: {}", i, arr[i]))?;
        let val = &arr[i + 1];
        match key {
            ":POSI" => posi = parse_string_list(val, "posi")?,
            ":SEQ-SET" => seq_set = parse_int_list(val, "seq-set")?,
            ":CONJ" => conj = parse_conj_list(val)?,
            ":COMMON" => common = parse_opt_i32(val, "common")?,
            ":SCORE-INFO" => score_info = Some(parse_score_info(val)?),
            ":KPCL" => kpcl = Some(parse_kpcl(val)?),
            other => return Err(format!("info: unknown key {}", other)),
        }
        i += 2;
    }
    let score_info = score_info.ok_or_else(|| "info missing :SCORE-INFO".to_string())?;
    let kpcl = kpcl.ok_or_else(|| "info missing :KPCL".to_string())?;
    Ok(KaniSegmentInfo {
        posi,
        seq_set,
        conj,
        common,
        score_info,
        kpcl,
    })
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
