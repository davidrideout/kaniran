//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-SEGSPLIT`.
//! Source under test: `src/dict/get_segsplit.rs`.
//!
//! Run with:
//!   cargo run --release --bin get_segsplit_test -- \
//!       --path corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/get_segsplit.parquet
//!
//! Replays a captured segment through `get_segsplit` and compares the
//! returned compound-text-wrapping segment (or nil) against the Lisp
//! result.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::text_classes::{CompoundText, ScoreMod};
use kaniran_core::dict::conj::ConjData;
use kaniran_core::dict::dao::ConjProp;
use kaniran_core::dict::split::segsplit::get_segsplit;
use kaniran_core::dict::dao::KanaText;
use kaniran_core::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::dao::KanjiText;
use kaniran_core::dict::text_classes::ProxyText;
use kaniran_core::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};

use common::{
    captured_class, parse_captured_segment, parse_captured_word, parse_opt_bool, parse_opt_i32,
    short_plist, CapturedKanaText, CapturedKanjiText, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-SEGSPLIT";

fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let mut input = parse_captured_segment(&row.args[0])
        .map_err(|e| format!("input segment: {}", e))?;
    // parse_captured_segment leaves info=None (other audit consumers
    // don't read it); get-segsplit reads info.seq_set, so attach the
    // parsed info plist here.
    input.info = parse_info_from_segment_json(&row.args[0])
        .map_err(|e| format!("input segment.info: {}", e))?;

    let actual = get_segsplit(ctx, &input)
        
        .map_err(|e| format!("get_segsplit: {}", e))?;

    if row.result.len() != 1 {
        return Err(format!("expected 1 result value, got {}", row.result.len()));
    }

    match (&actual, &row.result[0]) {
        (None, Value::Null) => Ok(()),
        (Some(_), Value::Null) => Err("rust=Some lisp=null result".into()),
        (None, lisp) => Err(format!(
            "rust=None lisp=Some({})",
            short_plist(lisp)
        )),
        (Some(actual_seg), expected_seg) => compare_segments(actual_seg, expected_seg),
    }
}

/// Pull the `:info` plist out of a captured SEGMENT JSON envelope.
/// Mirrors the plist parsing done inline by [`compare_info`], but
/// constructs a [`KaniSegmentInfo`] for the input-segment branch where
/// get-segsplit reads `info.seq_set`.
fn parse_info_from_segment_json(value: &Value) -> Result<Option<KaniSegmentInfo>, String> {
    let info_val = require_field(value, "info")?;
    if info_val.is_null() {
        return Ok(None);
    }
    let arr = info_val
        .as_array()
        .ok_or_else(|| format!("info: expected plist array, got {}", info_val))?;
    if arr.len() % 2 != 0 {
        return Err(format!("info: odd plist length {}", arr.len()));
    }
    let mut info = KaniSegmentInfo {
        posi: Vec::new(),
        seq_set: Vec::new(),
        conj: Vec::new(),
        common: None,
        score_info: KaniScoreInfo {
            prop_score: 0,
            kanji_break: Vec::new(),
            use_length_bonus: 0,
            split_info: KaniSplitInfo::None,
        },
        kpcl: (false, false, false, false),
    };
    let mut i = 0;
    while i < arr.len() {
        let key = arr[i]
            .as_str()
            .ok_or_else(|| format!("info plist key at {} not string: {}", i, arr[i]))?;
        let val = &arr[i + 1];
        match key {
            ":POSI" => info.posi = parse_string_list(val, "posi")?,
            ":SEQ-SET" => info.seq_set = parse_int_list(val, "seq-set")?,
            ":CONJ" => info.conj = parse_conj_list(val)?,
            ":COMMON" => info.common = parse_opt_int(val, "common")?,
            ":SCORE-INFO" => info.score_info = parse_score_info(val)?,
            ":KPCL" => info.kpcl = parse_kpcl(val)?,
            other => return Err(format!("unknown info key {}", other)),
        }
        i += 2;
    }
    Ok(Some(info))
}

fn parse_opt_int(val: &Value, field: &str) -> Result<Option<i32>, String> {
    match val {
        Value::Null => Ok(None),
        Value::String(s) if s == ":NULL" => Ok(None),
        Value::Number(n) => Ok(Some(
            n.as_i64()
                .ok_or_else(|| format!("{} not i64: {}", field, n))? as i32,
        )),
        other => Err(format!("{}: expected null/int, got {}", field, other)),
    }
}

fn parse_score_info(val: &Value) -> Result<KaniScoreInfo, String> {
    let arr = val
        .as_array()
        .ok_or_else(|| format!("score-info: expected array, got {}", val))?;
    if arr.len() != 4 {
        return Err(format!("score-info: expected 4-tuple, got {}", arr.len()));
    }
    let prop_score = arr[0]
        .as_i64()
        .ok_or_else(|| format!("score-info[0] not int: {}", arr[0]))? as i32;
    let kanji_break: Vec<usize> = match &arr[1] {
        Value::Null => Vec::new(),
        Value::Array(items) => items
            .iter()
            .map(|v| {
                v.as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| format!("score-info kanji-break entry not uint: {}", v))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("score-info[1]: expected array/null, got {}", other)),
    };
    let use_length_bonus = arr[2]
        .as_i64()
        .ok_or_else(|| format!("score-info[2] not int: {}", arr[2]))? as i32;
    let split_info = parse_split_info(&arr[3])?;
    Ok(KaniScoreInfo {
        prop_score,
        kanji_break,
        use_length_bonus,
        split_info,
    })
}

fn parse_split_info(val: &Value) -> Result<KaniSplitInfo, String> {
    match val {
        Value::Null => Ok(KaniSplitInfo::None),
        Value::Number(n) => Ok(KaniSplitInfo::Score(
            n.as_i64()
                .ok_or_else(|| format!("split-info int not i64: {}", n))? as i32,
        )),
        Value::Array(arr) => {
            if arr.is_empty() {
                return Err("split-info: array empty".into());
            }
            let score_mod = arr[0]
                .as_i64()
                .ok_or_else(|| format!("split-info[0] not int: {}", arr[0]))?
                as i32;
            let part_scores: Vec<i32> = arr[1..]
                .iter()
                .map(|v| {
                    v.as_i64()
                        .map(|n| n as i32)
                        .ok_or_else(|| format!("split-info part not int: {}", v))
                })
                .collect::<Result<_, _>>()?;
            Ok(KaniSplitInfo::Parts { score_mod, part_scores })
        }
        other => Err(format!("split-info: unexpected shape {}", other)),
    }
}

fn parse_kpcl(val: &Value) -> Result<(bool, bool, bool, bool), String> {
    let arr = val
        .as_array()
        .ok_or_else(|| format!("kpcl: expected array, got {}", val))?;
    if arr.len() != 4 {
        return Err(format!("kpcl: expected 4-tuple, got {}", arr.len()));
    }
    // Per calc_score_test compare_kpcl: each slot is the raw CL `or`/`and`
    // result — first-truthy not necessarily T. Collapse any non-nil to true.
    let bools: Vec<bool> = arr
        .iter()
        .map(|v| match v {
            Value::Null => false,
            Value::Bool(b) => *b,
            _ => true,
        })
        .collect();
    Ok((bools[0], bools[1], bools[2], bools[3]))
}

fn parse_conj_list(val: &Value) -> Result<Vec<ConjData>, String> {
    let arr = match val {
        Value::Null => return Ok(Vec::new()),
        Value::Array(arr) => arr,
        other => return Err(format!("conj: expected array/null, got {}", other)),
    };
    let mut out = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
        out.push(parse_conj_data(item).map_err(|e| format!("conj[{}]: {}", idx, e))?);
    }
    Ok(out)
}

fn parse_conj_data(val: &Value) -> Result<ConjData, String> {
    let class = captured_class(val)?;
    if class != "CONJ-DATA" {
        return Err(format!("expected CONJ-DATA, got :{}", class));
    }
    let seq = parse_opt_i32(val.get("seq").unwrap_or(&Value::Null), "seq")?;
    let from = parse_opt_i32(val.get("from").unwrap_or(&Value::Null), "from")?;
    let via = parse_opt_i32(val.get("via").unwrap_or(&Value::Null), "via")?;
    let prop_val = val.get("prop").unwrap_or(&Value::Null);
    let prop = if prop_val.is_null() {
        None
    } else {
        Some(parse_conj_prop(prop_val)?)
    };
    let src_map_val = val.get("src_map").unwrap_or(&Value::Null);
    let src_map: Vec<(String, String)> = match src_map_val {
        Value::Null => Vec::new(),
        Value::Array(pairs) => pairs
            .iter()
            .map(|pair| {
                let pair_arr = pair
                    .as_array()
                    .ok_or_else(|| format!("src_map entry not array: {}", pair))?;
                if pair_arr.len() != 2 {
                    return Err(format!("src_map pair not 2-elem: {}", pair));
                }
                let a = pair_arr[0]
                    .as_str()
                    .ok_or_else(|| format!("src_map[0] not string: {}", pair_arr[0]))?
                    .to_string();
                let b = pair_arr[1]
                    .as_str()
                    .ok_or_else(|| format!("src_map[1] not string: {}", pair_arr[1]))?
                    .to_string();
                Ok((a, b))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("src_map: expected array/null, got {}", other)),
    };
    Ok(ConjData { seq, from, via, prop, src_map })
}

fn parse_conj_prop(val: &Value) -> Result<ConjProp, String> {
    let class = captured_class(val)?;
    if class != "CONJ-PROP" {
        return Err(format!("expected CONJ-PROP, got :{}", class));
    }
    Ok(ConjProp {
        id: val.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        conj_id: val.get("conj_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        conj_type: val.get("conj_type").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        pos: val.get("pos").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        neg: parse_opt_bool(val.get("neg").unwrap_or(&Value::Null), "neg")?,
        fml: parse_opt_bool(val.get("fml").unwrap_or(&Value::Null), "fml")?,
    })
}

fn parse_string_list(val: &Value, field: &str) -> Result<Vec<String>, String> {
    match val {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr
            .iter()
            .map(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| format!("{}: entry not string: {}", field, v))
            })
            .collect(),
        other => Err(format!("{}: expected array/null, got {}", field, other)),
    }
}

fn parse_int_list(val: &Value, field: &str) -> Result<Vec<i32>, String> {
    match val {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr
            .iter()
            .map(|v| {
                v.as_i64()
                    .map(|n| n as i32)
                    .ok_or_else(|| format!("{}: entry not int: {}", field, v))
            })
            .collect(),
        other => Err(format!("{}: expected array/null, got {}", field, other)),
    }
}

// --- result-segment comparison --------------------------------------------

fn compare_segments(actual: &Segment, expected: &Value) -> Result<(), String> {
    let class = captured_class(expected)?;
    if class != "SEGMENT" {
        return Err(format!("result class: expected SEGMENT, got :{}", class));
    }
    // start / end — dict-split.lisp:798 copy-segment preserves them from
    // the input segment; the parallel setf at :802-807 never writes them.
    let exp_start = require_i32(expected, "start")? as usize;
    let exp_end = require_i32(expected, "end")? as usize;
    if actual.start != exp_start {
        return Err(format!("start: rust={} lisp={}", actual.start, exp_start));
    }
    if actual.end != exp_end {
        return Err(format!("end: rust={} lisp={}", actual.end, exp_end));
    }
    // score
    let exp_score = match require_field(expected, "score")? {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("score not i64: {}", n))? as i32,
        ),
        other => return Err(format!("score: expected number/null, got {}", other)),
    };
    if actual.score != exp_score {
        return Err(format!("score: rust={:?} lisp={:?}", actual.score, exp_score));
    }
    // text
    let exp_text = match require_field(expected, "text")? {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => return Err(format!("text: expected string/null, got {}", other)),
    };
    if actual.text != exp_text {
        return Err(format!("text: rust={:?} lisp={:?}", actual.text, exp_text));
    }
    // word — must be a compound-text in this branch.
    let exp_word = require_field(expected, "word")?;
    compare_compound_word(&actual.word, exp_word).map_err(|e| format!("word: {}", e))?;
    // info plist
    let exp_info = require_field(expected, "info")?;
    compare_info_opt(actual.info.as_ref(), exp_info)?;
    Ok(())
}

fn require_i32(value: &Value, field: &str) -> Result<i32, String> {
    let v = value
        .get(field)
        .ok_or_else(|| format!("missing field `{}` on {}", field, value))?;
    v.as_i64()
        .map(|n| n as i32)
        .ok_or_else(|| format!("`{}` not int: {}", field, v))
}

fn require_field<'a>(value: &'a Value, field: &str) -> Result<&'a Value, String> {
    value
        .get(field)
        .ok_or_else(|| format!("missing field `{}` on {}", field, value))
}

fn compare_compound_word(actual: &KaniWordDispatchEnum, expected: &Value) -> Result<(), String> {
    let class = captured_class(expected)?;
    if class != "COMPOUND-TEXT" {
        return Err(format!(
            "expected COMPOUND-TEXT (get-segsplit always wraps in compound), got :{}",
            class
        ));
    }
    let actual_compound = match actual {
        KaniWordDispatchEnum::Compound(c) => c,
        other => {
            return Err(format!(
                "rust word not Compound (got {:?})",
                discriminant_name(other)
            ))
        }
    };
    let exp_compound: CompoundText = parse_captured_compound(expected)?;

    if actual_compound.text != exp_compound.text {
        return Err(format!(
            "compound.text: rust={:?} lisp={:?}",
            actual_compound.text, exp_compound.text
        ));
    }
    if actual_compound.kana != exp_compound.kana {
        return Err(format!(
            "compound.kana: rust={:?} lisp={:?}",
            actual_compound.kana, exp_compound.kana
        ));
    }
    compare_simple_word(&actual_compound.primary, &exp_compound.primary)
        .map_err(|e| format!("primary: {}", e))?;
    if actual_compound.words.len() != exp_compound.words.len() {
        return Err(format!(
            "words.len: rust={} lisp={}",
            actual_compound.words.len(),
            exp_compound.words.len()
        ));
    }
    for (idx, (a, e)) in actual_compound
        .words
        .iter()
        .zip(exp_compound.words.iter())
        .enumerate()
    {
        compare_simple_word(a, e).map_err(|err| format!("words[{}]: {}", idx, err))?;
    }
    match (&actual_compound.score_base, &exp_compound.score_base) {
        (None, None) => {}
        (Some(a), Some(e)) => compare_simple_word(a, e)
            .map_err(|err| format!("score_base: {}", err))?,
        (a, e) => {
            return Err(format!(
                "score_base: rust={} lisp={}",
                if a.is_some() { "Some" } else { "None" },
                if e.is_some() { "Some" } else { "None" }
            ))
        }
    }
    compare_score_mod(&actual_compound.score_mod, &exp_compound.score_mod)?;
    Ok(())
}

fn parse_captured_compound(value: &Value) -> Result<CompoundText, String> {
    // Reuse the full word dispatcher and extract the compound variant.
    let parsed = parse_captured_word(value)?;
    match parsed {
        KaniWordDispatchEnum::Compound(c) => Ok(c),
        _ => Err("captured class COMPOUND-TEXT but parsed not Compound".into()),
    }
}

fn discriminant_name(w: &KaniWordDispatchEnum) -> &'static str {
    match w {
        KaniWordDispatchEnum::Kanji(_) => "Kanji",
        KaniWordDispatchEnum::Kana(_) => "Kana",
        KaniWordDispatchEnum::Proxy(_) => "Proxy",
        KaniWordDispatchEnum::Compound(_) => "Compound",
        KaniWordDispatchEnum::Counter(_) => "Counter",
    }
}

fn compare_simple_word(
    actual: &KaniWordDispatchEnum,
    expected: &KaniWordDispatchEnum,
) -> Result<(), String> {
    match (actual, expected) {
        (KaniWordDispatchEnum::Kanji(a), KaniWordDispatchEnum::Kanji(e)) => {
            compare_kanji(a, e)
        }
        (KaniWordDispatchEnum::Kana(a), KaniWordDispatchEnum::Kana(e)) => {
            compare_kana(a, e)
        }
        (KaniWordDispatchEnum::Proxy(a), KaniWordDispatchEnum::Proxy(e)) => {
            compare_proxy(a, e)
        }
        (a, e) => Err(format!(
            "variant: rust={} lisp={}",
            discriminant_name(a),
            discriminant_name(e)
        )),
    }
}

fn compare_kanji(actual: &KanjiText, expected: &KanjiText) -> Result<(), String> {
    // Mirror CapturedKanjiText::matches — when the captured id is the
    // sentinel 0 (was JSON `null`), tolerate any rust id (synthesized
    // rows have none); otherwise compare strictly.
    let id_ok = expected.id == 0 || actual.id == expected.id;
    if !(id_ok
        && actual.seq == expected.seq
        && actual.text == expected.text
        && actual.ord == expected.ord
        && actual.common == expected.common
        && actual.common_tags == expected.common_tags
        && actual.conjugate_p == expected.conjugate_p
        && actual.nokanji == expected.nokanji
        && actual.best_kana == expected.best_kana
        && actual.state.conjugations == expected.state.conjugations
        && actual.state.hintedp == expected.state.hintedp)
    {
        return Err(format!(
            "kanji-text mismatch:\n  rust: {:?}\n  lisp: {:?}",
            actual, expected
        ));
    }
    Ok(())
}

fn compare_proxy(actual: &ProxyText, expected: &ProxyText) -> Result<(), String> {
    // dict.lisp:548-552 proxy-text slots: text, kana, source.
    // dict.lisp:550 (defclass proxy-text (simple-text)) inherits state
    // (conjugations, hintedp).
    if actual.text != expected.text {
        return Err(format!(
            "proxy.text: rust={:?} lisp={:?}",
            actual.text, expected.text
        ));
    }
    if actual.kana != expected.kana {
        return Err(format!(
            "proxy.kana: rust={:?} lisp={:?}",
            actual.kana, expected.kana
        ));
    }
    if actual.state.conjugations != expected.state.conjugations {
        return Err(format!(
            "proxy.state.conjugations: rust={:?} lisp={:?}",
            actual.state.conjugations, expected.state.conjugations
        ));
    }
    if actual.state.hintedp != expected.state.hintedp {
        return Err(format!(
            "proxy.state.hintedp: rust={:?} lisp={:?}",
            actual.state.hintedp, expected.state.hintedp
        ));
    }
    compare_simple_source(&actual.source, &expected.source).map_err(|e| format!("proxy.source: {}", e))
}

fn compare_simple_source(
    actual: &KaniSimpleTextDispatchEnum,
    expected: &KaniSimpleTextDispatchEnum,
) -> Result<(), String> {
    match (actual, expected) {
        (KaniSimpleTextDispatchEnum::Kanji(a), KaniSimpleTextDispatchEnum::Kanji(e)) => {
            compare_kanji(a, e)
        }
        (KaniSimpleTextDispatchEnum::Kana(a), KaniSimpleTextDispatchEnum::Kana(e)) => {
            compare_kana(a, e)
        }
        (KaniSimpleTextDispatchEnum::Proxy(a), KaniSimpleTextDispatchEnum::Proxy(e)) => {
            compare_proxy(a, e)
        }
        (a, e) => Err(format!(
            "proxy.source variant mismatch: rust={:?} lisp={:?}",
            std::mem::discriminant(a),
            std::mem::discriminant(e)
        )),
    }
}

fn compare_kana(actual: &KanaText, expected: &KanaText) -> Result<(), String> {
    let id_ok = expected.id == 0 || actual.id == expected.id;
    if !(id_ok
        && actual.seq == expected.seq
        && actual.text == expected.text
        && actual.ord == expected.ord
        && actual.common == expected.common
        && actual.common_tags == expected.common_tags
        && actual.conjugate_p == expected.conjugate_p
        && actual.nokanji == expected.nokanji
        && actual.best_kanji == expected.best_kanji
        && actual.state.conjugations == expected.state.conjugations
        && actual.state.hintedp == expected.state.hintedp)
    {
        return Err(format!(
            "kana-text mismatch:\n  rust: {:?}\n  lisp: {:?}",
            actual, expected
        ));
    }
    Ok(())
}

fn compare_score_mod(actual: &ScoreMod, expected: &ScoreMod) -> Result<(), String> {
    if discriminant_score_mod(actual) != discriminant_score_mod(expected) || !actual_eq(actual, expected) {
        return Err(format!(
            "score_mod: rust={:?} lisp={:?}",
            actual, expected
        ));
    }
    Ok(())
}

fn discriminant_score_mod(s: &ScoreMod) -> &'static str {
    match s {
        ScoreMod::Single(_) => "Single",
        ScoreMod::Constant(_) => "Constant",
        ScoreMod::Stack(_) => "Stack",
    }
}

fn actual_eq(a: &ScoreMod, b: &ScoreMod) -> bool {
    match (a, b) {
        (ScoreMod::Single(x), ScoreMod::Single(y)) => x == y,
        (ScoreMod::Constant(x), ScoreMod::Constant(y)) => x == y,
        (ScoreMod::Stack(x), ScoreMod::Stack(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| actual_eq(p, q))
        }
        _ => false,
    }
}

// --- result-info comparison (lifted from calc_score_test) -----------------

fn compare_info_opt(
    actual: Option<&KaniSegmentInfo>,
    expected: &Value,
) -> Result<(), String> {
    if expected.is_null() {
        return match actual {
            None => Ok(()),
            Some(a) => Err(format!("info: rust present (posi={:?}) lisp=null", a.posi)),
        };
    }
    let a = actual.ok_or_else(|| format!("info: rust=None lisp present ({})", short_plist(expected)))?;
    compare_info(a, expected)
}

fn compare_info(actual: &KaniSegmentInfo, expected: &Value) -> Result<(), String> {
    let arr = expected
        .as_array()
        .ok_or_else(|| format!("info: expected plist array, got {}", expected))?;
    if arr.len() % 2 != 0 {
        return Err(format!("info: odd plist length {}", arr.len()));
    }
    let mut i = 0;
    while i < arr.len() {
        let key = arr[i]
            .as_str()
            .ok_or_else(|| format!("info plist key at {} not string: {}", i, arr[i]))?;
        let val = &arr[i + 1];
        match key {
            ":POSI" => compare_string_list_field(&actual.posi, val, "posi")?,
            ":SEQ-SET" => compare_int_list_field(&actual.seq_set, val, "seq-set")?,
            ":CONJ" => compare_conj_count(&actual.conj, val)?,
            ":COMMON" => compare_common(actual.common, val)?,
            ":SCORE-INFO" => compare_score_info(&actual.score_info, val)?,
            ":KPCL" => compare_kpcl(actual.kpcl, val)?,
            other => return Err(format!("unknown info key {}", other)),
        }
        i += 2;
    }
    Ok(())
}

fn compare_string_list_field(actual: &[String], val: &Value, field: &str) -> Result<(), String> {
    let expected = parse_string_list(val, field)?;
    let mut a: Vec<&String> = actual.iter().collect();
    let mut e: Vec<&String> = expected.iter().collect();
    a.sort();
    e.sort();
    if a != e {
        return Err(format!("{}: rust={:?} lisp={:?}", field, actual, expected));
    }
    Ok(())
}

fn compare_int_list_field(actual: &[i32], val: &Value, field: &str) -> Result<(), String> {
    let expected = parse_int_list(val, field)?;
    if actual != expected.as_slice() {
        return Err(format!("{}: rust={:?} lisp={:?}", field, actual, expected));
    }
    Ok(())
}

fn compare_common(actual: Option<i32>, val: &Value) -> Result<(), String> {
    let expected = match val {
        Value::Null => None,
        Value::String(s) if s == ":NULL" => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("common not i64: {}", n))? as i32,
        ),
        other => return Err(format!("common: expected null/int, got {}", other)),
    };
    if actual != expected {
        return Err(format!("common: rust={:?} lisp={:?}", actual, expected));
    }
    Ok(())
}

fn compare_score_info(actual: &KaniScoreInfo, val: &Value) -> Result<(), String> {
    let arr = val
        .as_array()
        .ok_or_else(|| format!("score-info: expected array, got {}", val))?;
    if arr.len() != 4 {
        return Err(format!("score-info: expected 4-tuple, got {}", arr.len()));
    }
    let exp_prop_score = arr[0]
        .as_i64()
        .ok_or_else(|| format!("score-info[0] not int: {}", arr[0]))? as i32;
    if actual.prop_score != exp_prop_score {
        return Err(format!(
            "score-info.prop_score: rust={} lisp={}",
            actual.prop_score, exp_prop_score
        ));
    }
    let exp_kanji_break: Vec<usize> = match &arr[1] {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr
            .iter()
            .map(|v| {
                v.as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| format!("score-info kanji-break entry not uint: {}", v))
            })
            .collect::<Result<_, _>>()?,
        other => {
            return Err(format!(
                "score-info[1]: expected array/null, got {}",
                other
            ))
        }
    };
    if actual.kanji_break != exp_kanji_break {
        return Err(format!(
            "score-info.kanji_break: rust={:?} lisp={:?}",
            actual.kanji_break, exp_kanji_break
        ));
    }
    let exp_ulb = arr[2]
        .as_i64()
        .ok_or_else(|| format!("score-info[2] not int: {}", arr[2]))? as i32;
    if actual.use_length_bonus != exp_ulb {
        return Err(format!(
            "score-info.use_length_bonus: rust={} lisp={}",
            actual.use_length_bonus, exp_ulb
        ));
    }
    compare_split_info(&actual.split_info, &arr[3])
}

fn compare_split_info(actual: &KaniSplitInfo, val: &Value) -> Result<(), String> {
    match (actual, val) {
        (KaniSplitInfo::None, Value::Null) => Ok(()),
        (KaniSplitInfo::Score(n), Value::Number(m)) => {
            let exp = m
                .as_i64()
                .ok_or_else(|| format!("split-info int not i64: {}", m))? as i32;
            if *n != exp {
                Err(format!("split-info Score: rust={} lisp={}", n, exp))
            } else {
                Ok(())
            }
        }
        (KaniSplitInfo::Parts { score_mod, part_scores }, Value::Array(arr)) => {
            if arr.is_empty() {
                return Err("split-info Parts: lisp array empty".into());
            }
            let exp_score_mod = arr[0]
                .as_i64()
                .ok_or_else(|| format!("split-info[0] not int: {}", arr[0]))?
                as i32;
            if *score_mod != exp_score_mod {
                return Err(format!(
                    "split-info Parts.score_mod: rust={} lisp={}",
                    score_mod, exp_score_mod
                ));
            }
            let exp_parts: Vec<i32> = arr[1..]
                .iter()
                .map(|v| {
                    v.as_i64()
                        .map(|n| n as i32)
                        .ok_or_else(|| format!("split-info part not int: {}", v))
                })
                .collect::<Result<_, _>>()?;
            if part_scores != &exp_parts {
                return Err(format!(
                    "split-info Parts.part_scores: rust={:?} lisp={:?}",
                    part_scores, exp_parts
                ));
            }
            Ok(())
        }
        (a, e) => Err(format!(
            "split-info: rust={:?} lisp={}",
            a,
            short_plist(e)
        )),
    }
}

fn compare_kpcl(actual: (bool, bool, bool, bool), val: &Value) -> Result<(), String> {
    let arr = val
        .as_array()
        .ok_or_else(|| format!("kpcl: expected array, got {}", val))?;
    if arr.len() != 4 {
        return Err(format!("kpcl: expected 4-tuple, got {}", arr.len()));
    }
    let bools: Vec<bool> = arr
        .iter()
        .map(|v| match v {
            Value::Null => false,
            Value::Bool(b) => *b,
            _ => true,
        })
        .collect();
    let exp = (bools[0], bools[1], bools[2], bools[3]);
    if actual != exp {
        return Err(format!("kpcl: rust={:?} lisp={:?}", actual, exp));
    }
    Ok(())
}

fn compare_conj_count(
    actual: &[kaniran_core::dict::conj::ConjData],
    val: &Value,
) -> Result<(), String> {
    let arr = match val {
        Value::Null => &[][..],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("conj: expected array/null, got {}", other)),
    };
    // The compound-text branch of get-segsplit writes
    // `(word-conj-data word)` where `word` is the compound; this
    // recurses to the last word of `words` (dict.lisp:660-661). Conj
    // count and per-row seq/from/via match upstream — borrow the
    // structural comparator from calc_score_test.
    if actual.len() != arr.len() {
        return Err(format!(
            "conj: rust len={} lisp len={}",
            actual.len(),
            arr.len()
        ));
    }
    for (idx, (a, e)) in actual.iter().zip(arr.iter()).enumerate() {
        compare_conj_data(a, e).map_err(|err| format!("conj[{}]: {}", idx, err))?;
    }
    Ok(())
}

fn compare_conj_data(
    actual: &kaniran_core::dict::conj::ConjData,
    val: &Value,
) -> Result<(), String> {
    let class = captured_class(val)?;
    if class != "CONJ-DATA" {
        return Err(format!("expected CONJ-DATA, got :{}", class));
    }
    let exp_seq = parse_opt_i32(val.get("seq").unwrap_or(&Value::Null), "seq")?;
    if actual.seq != exp_seq {
        return Err(format!("seq: rust={:?} lisp={:?}", actual.seq, exp_seq));
    }
    let exp_from = parse_opt_i32(val.get("from").unwrap_or(&Value::Null), "from")?;
    if actual.from != exp_from {
        return Err(format!("from: rust={:?} lisp={:?}", actual.from, exp_from));
    }
    let exp_via = parse_opt_i32(val.get("via").unwrap_or(&Value::Null), "via")?;
    if actual.via != exp_via {
        return Err(format!("via: rust={:?} lisp={:?}", actual.via, exp_via));
    }
    // prop — None in CL when absent; field-by-field on the inner CONJ-PROP.
    let prop_val = val.get("prop").unwrap_or(&Value::Null);
    match (&actual.prop, prop_val) {
        (None, Value::Null) => {}
        (Some(a), v) if !v.is_null() => compare_conj_prop(a, v)?,
        (a, e) => {
            return Err(format!(
                "prop: rust={} lisp={}",
                if a.is_some() { "Some" } else { "None" },
                short_plist(e)
            ))
        }
    }
    // src_map — Lisp `(text . source-text)` pairs serialized as 2-arrays.
    let src_map_val = val.get("src_map").unwrap_or(&Value::Null);
    let exp_src_map: Vec<(String, String)> = match src_map_val {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr
            .iter()
            .map(|pair| {
                let pair_arr = pair
                    .as_array()
                    .ok_or_else(|| format!("src_map entry not array: {}", pair))?;
                if pair_arr.len() != 2 {
                    return Err(format!("src_map pair not 2-elem: {}", pair));
                }
                let a = pair_arr[0]
                    .as_str()
                    .ok_or_else(|| format!("src_map[0] not string: {}", pair_arr[0]))?
                    .to_string();
                let b = pair_arr[1]
                    .as_str()
                    .ok_or_else(|| format!("src_map[1] not string: {}", pair_arr[1]))?
                    .to_string();
                Ok((a, b))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("src_map: expected array/null, got {}", other)),
    };
    if actual.src_map != exp_src_map {
        return Err(format!(
            "src_map: rust={:?} lisp={:?}",
            actual.src_map, exp_src_map
        ));
    }
    Ok(())
}

fn compare_conj_prop(actual: &ConjProp, val: &Value) -> Result<(), String> {
    let class = captured_class(val)?;
    if class != "CONJ-PROP" {
        return Err(format!("expected CONJ-PROP, got :{}", class));
    }
    let exp_id = val.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if actual.id != exp_id {
        return Err(format!("conj-prop.id: rust={} lisp={}", actual.id, exp_id));
    }
    let exp_conj_id = val.get("conj_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if actual.conj_id != exp_conj_id {
        return Err(format!(
            "conj-prop.conj_id: rust={} lisp={}",
            actual.conj_id, exp_conj_id
        ));
    }
    let exp_conj_type = val.get("conj_type").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    if actual.conj_type != exp_conj_type {
        return Err(format!(
            "conj-prop.conj_type: rust={} lisp={}",
            actual.conj_type, exp_conj_type
        ));
    }
    let exp_pos = val
        .get("pos")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if actual.pos != exp_pos {
        return Err(format!(
            "conj-prop.pos: rust={:?} lisp={:?}",
            actual.pos, exp_pos
        ));
    }
    let exp_neg = parse_opt_bool(val.get("neg").unwrap_or(&Value::Null), "neg")?;
    if actual.neg != exp_neg {
        return Err(format!(
            "conj-prop.neg: rust={:?} lisp={:?}",
            actual.neg, exp_neg
        ));
    }
    let exp_fml = parse_opt_bool(val.get("fml").unwrap_or(&Value::Null), "fml")?;
    if actual.fml != exp_fml {
        return Err(format!(
            "conj-prop.fml: rust={:?} lisp={:?}",
            actual.fml, exp_fml
        ));
    }
    Ok(())
}

// Suppress unused-import warnings — CapturedKanaText / CapturedKanjiText
// are re-exported by `common::` but this runner consumes the parsed DAOs
// directly via [`parse_captured_word`] instead.
#[allow(dead_code)]
fn _unused(_a: CapturedKanaText, _b: CapturedKanjiText) {}

fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one);
}
