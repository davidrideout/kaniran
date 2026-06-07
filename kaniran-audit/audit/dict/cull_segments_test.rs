//! Manual fixture-replay runner for `ICHIRAN/DICT:CULL-SEGMENTS`.
//! Source under test: `src/dict/cull_segments.rs`.
//!
//! Run with:
//!   cargo run --release --bin cull_segments_test -- \
//!       --path corpus/<corpus_tag>/dict/cull_segments.parquet
//!
//! Sorts a list of segments (by `common` then score) and keeps the
//! best sub-list. Args are `[[segment, ...]]`; the one result value is
//! the culled list.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::scoring::score::cull_segments;
use kaniran_core::dict::kani_word::KaniWordDispatchEnum;
use kaniran_core::dict::scoring::score::{
    KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment,
};

use common::{
    captured_class, parse_captured_word, parse_opt_i32, single_result, CapturedRow,
};

fn require_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field: {}", key))
}

const EXPECTED_FQN: &str = "ICHIRAN/DICT:CULL-SEGMENTS";

/// Parse a captured SEGMENT into an [`Segment`] with `info.common`
/// populated. The full info plist is not required — only `:common`
/// participates in the sort key — but parsing the whole struct keeps
/// the audit forward-compatible if cull-segments ever consults
/// another info key. Other plist keys parse into the default zero
/// values of [`KaniScoreInfo`]; downstream code under test does not
/// read them.
fn parse_segment(value: &Value) -> Result<Segment, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT" {
        return Err(format!("expected SEGMENT class, got :{}", class));
    }
    let start = require_field(value, "start")?
        .as_i64()
        .ok_or_else(|| "start: expected int".to_string())? as usize;
    let end = require_field(value, "end")?
        .as_i64()
        .ok_or_else(|| "end: expected int".to_string())? as usize;
    let word = parse_captured_word(require_field(value, "word")?)?;
    let score = match require_field(value, "score")? {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("score not i64: {}", n))? as i32,
        ),
        other => return Err(format!("score: expected int / null, got {}", other)),
    };
    let info = match require_field(value, "info")? {
        Value::Null => None,
        Value::Array(plist) => Some(parse_info_plist(&plist)?),
        other => return Err(format!("info: expected plist array / null, got {}", other)),
    };
    Ok(Segment {
        start,
        end,
        word,
        score,
        info,
        top: None,
        text: None,
    })
}

/// Parse `:common` and `:seq-set` out of the info plist. `:common` is
/// the function's primary sort key; `:seq-set` is the identity
/// tiebreaker for compound-text segments (which carry `word.seq =
/// None`). Other plist keys land in the zero defaults of
/// [`KaniSegmentInfo`] — cull-segments never reads them. Unknown keys
/// are skipped for forward-compat.
fn parse_info_plist(plist: &[Value]) -> Result<KaniSegmentInfo, String> {
    if plist.len() % 2 != 0 {
        return Err(format!("info plist: odd length {}", plist.len()));
    }
    let mut common: Option<i32> = None;
    let mut seq_set: Vec<i32> = Vec::new();
    let mut i = 0;
    while i < plist.len() {
        let key = plist[i]
            .as_str()
            .ok_or_else(|| format!("info plist key at {} not string: {}", i, plist[i]))?;
        let val = &plist[i + 1];
        match key {
            ":COMMON" => common = parse_opt_i32(val, "info.common")?,
            ":SEQ-SET" => {
                seq_set = match val {
                    Value::Null => Vec::new(),
                    Value::Array(arr) => arr
                        .iter()
                        .map(|v| {
                            v.as_i64()
                                .ok_or_else(|| format!("seq_set entry not int: {}", v))
                                .map(|n| n as i32)
                        })
                        .collect::<Result<_, _>>()?,
                    other => return Err(format!("seq_set: expected array/null, got {}", other)),
                };
            }
            _ => {}
        }
        i += 2;
    }
    Ok(KaniSegmentInfo {
        posi: Vec::new(),
        seq_set,
        conj: Vec::new(),
        common,
        score_info: KaniScoreInfo {
            prop_score: 0,
            kanji_break: Vec::new(),
            use_length_bonus: 0,
            split_info: KaniSplitInfo::None,
        },
        kpcl: (false, false, false, false),
    })
}

fn word_seq(word: &KaniWordDispatchEnum) -> Option<i32> {
    match word {
        KaniWordDispatchEnum::Kana(k) => Some(k.seq),
        KaniWordDispatchEnum::Kanji(k) => Some(k.seq),
        KaniWordDispatchEnum::Proxy(_) => None,
        KaniWordDispatchEnum::Compound(_) => None,
        KaniWordDispatchEnum::Counter(_) => None,
    }
}

fn word_text(word: &KaniWordDispatchEnum) -> &str {
    match word {
        KaniWordDispatchEnum::Kana(k) => &k.text,
        KaniWordDispatchEnum::Kanji(k) => &k.text,
        KaniWordDispatchEnum::Proxy(p) => &p.text,
        KaniWordDispatchEnum::Compound(c) => &c.text,
        KaniWordDispatchEnum::Counter(c) => &c.base().text,
    }
}

#[derive(PartialEq, Eq, Debug)]
struct Signature<'a> {
    seq: Option<i32>,
    text: &'a str,
    score: Option<i32>,
    common: Option<i32>,
    seq_set: &'a [i32],
}

fn signature(seg: &Segment) -> Signature<'_> {
    let info = seg.info.as_ref();
    Signature {
        seq: word_seq(&seg.word),
        text: word_text(&seg.word),
        score: seg.score,
        common: info.and_then(|i| i.common),
        seq_set: info.map(|i| i.seq_set.as_slice()).unwrap_or(&[]),
    }
}

fn parse_segment_list(value: &Value) -> Result<Vec<Segment>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr.iter().map(parse_segment).collect(),
        other => Err(format!("segment list: expected array / null, got {}", other)),
    }
}

fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let input = parse_segment_list(&row.args[0])?;
    let expected = parse_segment_list(single_result(&row.result)?)?;

    let actual = cull_segments(input);

    let actual_sig: Vec<_> = actual.iter().map(signature).collect();
    let expected_sig: Vec<_> = expected.iter().map(signature).collect();

    if actual_sig == expected_sig {
        Ok(())
    } else {
        Err(format!(
            "\n  rust ({} segs): {:?}\n  lisp ({} segs): {:?}",
            actual_sig.len(),
            actual_sig,
            expected_sig.len(),
            expected_sig,
        ))
    }
}

fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
