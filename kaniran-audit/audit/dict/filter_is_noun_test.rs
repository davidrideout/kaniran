//! Manual fixture-replay runner for `ICHIRAN/DICT:FILTER-IS-NOUN`.
//! Source under test: `src/dict/filter_is_noun.rs`.
//!
//! Run with:
//!   cargo run --release --bin filter_is_noun_test -- \
//!       --path corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/filter_is_noun.parquet
//!
//! Captured shapes (`extracted_chunk_c_suffix_abbr_2026_05_16` corpus):
//! - `args = [<segment>]` — single Segment input; the captured Segment
//!   carries a full info plist (`:posi`, `:seq-set`, `:conj`, `:common`,
//!   `:score-info`, `:kpcl`) because gen-score runs upstream before any
//!   synergy filter sees the segment.
//! - `result = [<intersection-or-null>]` — single-value return. Upstream
//!   returns either the truthy intersection / seq-set list (e.g.
//!   `[["n"]]` or `[[1471510]]`) or `nil` (captured as `[null]`).
//!
//! ## Comparison policy
//!
//! The Rust port returns `bool` per CONVENTIONS §4.1's collapse
//! exception (predicate-only callers across the synergy machinery —
//! `filter_is_noun` is consumed via `#'filter-is-noun` reference by
//! `synergy_noun_da` / `synergy_noun_particle` only, both of which use
//! it as a filter predicate). Compare `actual_bool == !primary.is_null()`.

#[path = "../common/mod.rs"]
mod common;

use std::sync::Arc;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::conj_data_struct::ConjData;
use kaniran_core::dict::grammar::filter::filter_is_noun;
use kaniran_core::dict::kani::KaniLiteSegment;
use kaniran_core::dict::segment_struct::{
    KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment,
};

use common::{
    captured_class, parse_captured_word, parse_conj_list, parse_int_list, parse_kpcl,
    parse_opt_i32, parse_score_info, parse_string_list, single_result, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FILTER-IS-NOUN";

async fn audit_one(_ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!(
            "expected 1 arg (segment), got {}",
            row.args.len()
        ));
    }
    let segment = parse_segment_full(&row.args[0])
        .map_err(|err| format!("arg 0 (segment): {}", err))?;

    let lite = Arc::new(KaniLiteSegment::from_segment(Arc::new(segment)));
    let actual = filter_is_noun(&lite);

    let expected_value = single_result(&row.result)?;
    let expected_bool = !expected_value.is_null();
    if actual != expected_bool {
        return Err(format!(
            "result: rust={} lisp_truthy={} (lisp={})",
            actual, expected_bool, expected_value
        ));
    }
    Ok(())
}

// ----- captured-Segment parser (full: includes info) ------------------------
//
// Mirror of `audit/dict/get_seg_initial_test.rs:parse_segment_full`
// (kept inline per the audit-binary independence convention).

fn parse_segment_full(value: &Value) -> Result<Segment, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT" {
        return Err(format!("expected SEGMENT class, got :{}", class));
    }
    let start = require_field(value, "start")?
        .as_i64()
        .ok_or_else(|| format!("start not int: {}", value))? as usize;
    let end = require_field(value, "end")?
        .as_i64()
        .ok_or_else(|| format!("end not int: {}", value))? as usize;
    let word = parse_captured_word(require_field(value, "word")?)?;
    let score = match require_field(value, "score")? {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("score not i64: {}", n))? as i32,
        ),
        other => return Err(format!("score: expected number / null, got {}", other)),
    };
    let text = match require_field(value, "text")? {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => return Err(format!("text: expected string / null, got {}", other)),
    };
    let info = match require_field(value, "info")? {
        Value::Null => None,
        other => Some(parse_info_plist(other)?),
    };
    Ok(Segment {
        start,
        end,
        word,
        score,
        info,
        top: None,
        text,
    })
}

// ----- info plist parsing -------------------------------------------------

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
    let mut score_info: KaniScoreInfo = KaniScoreInfo {
        prop_score: 0,
        kanji_break: Vec::new(),
        use_length_bonus: 0,
        split_info: KaniSplitInfo::None,
    };
    let mut kpcl: (bool, bool, bool, bool) = (false, false, false, false);
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
            ":SCORE-INFO" => score_info = parse_score_info(val)?,
            ":KPCL" => kpcl = parse_kpcl(val)?,
            other => return Err(format!("info: unknown key {}", other)),
        }
        i += 2;
    }
    Ok(KaniSegmentInfo {
        posi,
        seq_set,
        conj,
        common,
        score_info,
        kpcl,
    })
}

fn require_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field {} on: {}", key, value))
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
