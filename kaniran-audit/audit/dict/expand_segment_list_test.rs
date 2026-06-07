//! Manual fixture-replay runner for `ICHIRAN/DICT:EXPAND-SEGMENT-LIST`.
//! Source under test: `src/dict/expand_segment_list.rs`.
//!
//! Run with:
//!   cargo run --release --bin expand_segment_list_test -- \
//!       --path corpus/extracted_chunk_b_segmentation_2026_05_14/dict/expand_segment_list.parquet
//!
//! Expands each segment with its `get-segsplit` (compound split), then
//! sorts the combined list by descending score, mutating the
//! segment-list in place. Args are `[<segment-list>]` (captured
//! post-call, so already expanded); the result is the new segments
//! list.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::conj::ConjData;
use kaniran_core::dict::path::expand_segment_list;
use kaniran_core::dict::split::segsplit::get_segsplit;
use kaniran_core::dict::kani_word::KaniWordDispatchEnum;
use kaniran_core::dict::path::SegmentList;
use kaniran_core::dict::scoring::score::{
    KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment,
};

use common::{
    captured_class, parse_captured_word, parse_conj_list, parse_int_list, parse_kpcl,
    parse_opt_i32, parse_score_info, parse_string_list, single_result, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:EXPAND-SEGMENT-LIST";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!(
            "expected 1 arg (segment-list), got {}",
            row.args.len()
        ));
    }

    // Parse the captured (post-state) segment-list with full per-segment
    // fields including the info plist. This is the "expected" output
    // we want Rust's expand_segment_list to reproduce.
    let captured_post = parse_segment_list_full(&row.args[0])
        .map_err(|err| format!("arg 0: {}", err))?;

    // Sanity: result[0] equals args[0].segments by construction
    // (`setf` returns the value stored). Length cross-check catches
    // truncation / projector divergence.
    let result_segments = match single_result(&row.result)? {
        Value::Null => &[][..],
        Value::Array(arr) => arr.as_slice(),
        other => return Err(format!("result[0]: expected array / null, got {}", other)),
    };
    if result_segments.len() != captured_post.segments.len() {
        return Err(format!(
            "result vs args.segments length: result={} args={}",
            result_segments.len(),
            captured_post.segments.len()
        ));
    }

    // Reconstruct pre-call segment-list. dict.lisp:1183-1187 loops
    // `for segment in segments for segsplit = (get-segsplit segment)
    //  collect segment when segsplit collect segsplit and do (incf matches)`.
    // So post = stable-sort([s, ss(s)?, …], by score desc). Filtering
    // out the ss(s) entries recovers the pre-call segments list (still
    // sorted by score desc, which is what cull-segments produces
    // upstream — `dict.lisp:1027`). The filter target is identified by
    // re-running get_segsplit on each segment and matching its output
    // against a compound in post-state.
    let added_mask = mark_segsplit_added(ctx, &captured_post.segments).await?;
    let pre_segments: Vec<Segment> = captured_post
        .segments
        .iter()
        .zip(added_mask.iter())
        .filter_map(|(s, added)| if !*added { Some(s.clone()) } else { None })
        .collect();
    let dropped = added_mask.iter().filter(|x| **x).count();
    let pre_matches = captured_post
        .matches
        .checked_sub(dropped)
        .ok_or_else(|| {
            format!(
                "pre-call matches underflow: post.matches={} dropped={}",
                captured_post.matches, dropped
            )
        })?;
    let mut pre_state = SegmentList {
        segments: pre_segments,
        start: captured_post.start,
        end: captured_post.end,
        top: None,
        matches: pre_matches,
    };

    expand_segment_list(ctx, &mut pre_state)
        .await
        .map_err(|err| format!("expand_segment_list: {}", err))?;

    compare_segment_list_all(&pre_state, &captured_post)
}

// ----- segsplit-added marking ---------------------------------------------

/// For each segment in `segments`, run `get_segsplit`. When it returns
/// `Some(ss)`, locate the first unmarked compound segment in `segments`
/// whose `(start, end, score, text, kana, compound.primary.seq)` matches
/// `ss` and mark it as a segsplit-added entry that expand-segment-list
/// would have inserted. The returned `bool` vector is parallel to
/// `segments`.
async fn mark_segsplit_added(
    ctx: &KaniranContext,
    segments: &[Segment],
) -> Result<Vec<bool>, String> {
    let mut added = vec![false; segments.len()];
    for (i, seg) in segments.iter().enumerate() {
        // get_segsplit gates on simple-text — compound segments (any
        // pre-existing compound or a segsplit insertion) short-circuit
        // to None, so iterating compounds too is cheap and harmless.
        let candidate = match get_segsplit(ctx, seg).await {
            Ok(c) => c,
            Err(err) => {
                return Err(format!("get_segsplit @ segments[{}]: {}", i, err))
            }
        };
        let Some(predicted) = candidate else { continue };
        let pred_kana = compound_kana(&predicted.word)?;
        let pred_primary_seq = compound_primary_seq(&predicted.word)?;
        let pred_text = predicted
            .text
            .as_deref()
            .ok_or_else(|| format!("predicted segsplit @ {} missing text", i))?;
        let mut matched = false;
        for (j, cand) in segments.iter().enumerate() {
            if added[j] {
                continue;
            }
            if cand.start != predicted.start
                || cand.end != predicted.end
                || cand.score != predicted.score
            {
                continue;
            }
            if cand.text.as_deref() != Some(pred_text) {
                continue;
            }
            // Compound-segment fields. get_segsplit always wraps in a
            // CompoundText; pre-existing simple-text rows therefore never
            // match here.
            let cand_kana = match compound_kana(&cand.word) {
                Ok(k) => k,
                Err(_) => continue,
            };
            if cand_kana != pred_kana {
                continue;
            }
            let cand_primary_seq = match compound_primary_seq(&cand.word) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if cand_primary_seq != pred_primary_seq {
                continue;
            }
            added[j] = true;
            matched = true;
            break;
        }
        if !matched {
            return Err(format!(
                "segments[{}] produces a segsplit (primary-seq={}, score={:?}, text={:?}) but no matching compound found in post-state",
                i, pred_primary_seq, predicted.score, pred_text
            ));
        }
    }
    Ok(added)
}

fn compound_kana(w: &KaniWordDispatchEnum) -> Result<&str, String> {
    match w {
        KaniWordDispatchEnum::Compound(c) => Ok(&c.kana),
        _ => Err("not a CompoundText".into()),
    }
}

fn compound_primary_seq(w: &KaniWordDispatchEnum) -> Result<i32, String> {
    match w {
        KaniWordDispatchEnum::Compound(c) => match c.primary.as_ref() {
            KaniWordDispatchEnum::Kanji(k) => Ok(k.seq),
            KaniWordDispatchEnum::Kana(k) => Ok(k.seq),
            KaniWordDispatchEnum::Proxy(p) => match p.source.as_ref() {
                kaniran_core::dict::kani_word::KaniSimpleTextDispatchEnum::Kanji(k) => Ok(k.seq),
                kaniran_core::dict::kani_word::KaniSimpleTextDispatchEnum::Kana(k) => Ok(k.seq),
                kaniran_core::dict::kani_word::KaniSimpleTextDispatchEnum::Proxy(_) => {
                    Err("nested proxy primary".into())
                }
            },
            _ => Err("compound primary not simple-text".into()),
        },
        _ => Err("not a CompoundText".into()),
    }
}

// ----- captured-SegmentList / Segment parsers (full: includes info) -------

fn parse_segment_list_full(value: &Value) -> Result<SegmentList, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT-LIST" {
        return Err(format!("expected SEGMENT-LIST class, got :{}", class));
    }
    let segments: Vec<Segment> = match value.get("segments") {
        Some(Value::Array(arr)) => arr
            .iter()
            .map(parse_segment_full)
            .collect::<Result<_, _>>()
            .map_err(|err| format!("segments: {}", err))?,
        Some(Value::Null) | None => Vec::new(),
        Some(other) => {
            return Err(format!("segments: expected array/null, got {}", other))
        }
    };
    let start = require_field(value, "start")?
        .as_i64()
        .ok_or_else(|| "start not int".to_string())? as usize;
    let end = require_field(value, "end")?
        .as_i64()
        .ok_or_else(|| "end not int".to_string())? as usize;
    let matches = require_field(value, "matches")?
        .as_i64()
        .ok_or_else(|| "matches not int".to_string())? as usize;
    Ok(SegmentList {
        segments,
        start,
        end,
        top: None,
        matches,
    })
}

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

// ----- comparators ---------------------------------------------------------

fn compare_segment_list_all(actual: &SegmentList, expected: &SegmentList) -> Result<(), String> {
    if actual.start != expected.start {
        return Err(format!(
            "start: rust={} lisp={}",
            actual.start, expected.start
        ));
    }
    if actual.end != expected.end {
        return Err(format!("end: rust={} lisp={}", actual.end, expected.end));
    }
    if actual.matches != expected.matches {
        return Err(format!(
            "matches: rust={} lisp={}",
            actual.matches, expected.matches
        ));
    }
    // `top` is transient; expand-segment-list does not touch it.
    if actual.segments.len() != expected.segments.len() {
        return Err(format!(
            "segments len: rust={} lisp={}",
            actual.segments.len(),
            expected.segments.len()
        ));
    }
    for (i, (a, e)) in actual.segments.iter().zip(expected.segments.iter()).enumerate() {
        compare_segment_all(a, e).map_err(|err| format!("segments[{}]: {}", i, err))?;
    }
    Ok(())
}

fn compare_segment_all(actual: &Segment, expected: &Segment) -> Result<(), String> {
    if actual.start != expected.start {
        return Err(format!("start: rust={} lisp={}", actual.start, expected.start));
    }
    if actual.end != expected.end {
        return Err(format!("end: rust={} lisp={}", actual.end, expected.end));
    }
    if actual.score != expected.score {
        return Err(format!(
            "score: rust={:?} lisp={:?}",
            actual.score, expected.score
        ));
    }
    if actual.text != expected.text {
        return Err(format!(
            "text: rust={:?} lisp={:?}",
            actual.text, expected.text
        ));
    }
    compare_words(&actual.word, &expected.word)?;
    match (&actual.info, &expected.info) {
        (None, None) => Ok(()),
        (Some(a), Some(e)) => compare_info(a, e),
        (a, e) => Err(format!(
            "info: rust={} lisp={}",
            if a.is_some() { "Some" } else { "None" },
            if e.is_some() { "Some" } else { "None" }
        )),
    }
}

// Word equality via Debug-format string compare. Every field type
// inside `KaniWordDispatchEnum` derives `Debug` over primitives,
// `String`, `Option`, `Vec`, and nested `Box<KaniWordDispatchEnum>`
// only — no `HashMap` / `HashSet` whose Debug iteration order is
// nondeterministic. Mirrors get_seg_initial_test::compare_words.
fn compare_words(
    actual: &KaniWordDispatchEnum,
    expected: &KaniWordDispatchEnum,
) -> Result<(), String> {
    let a = format!("{:?}", actual);
    let e = format!("{:?}", expected);
    if a == e {
        Ok(())
    } else {
        Err(format!("word: rust={} lisp={}", a, e))
    }
}

// ----- info plist parsing + comparison (mirror of get_seg_initial_test) ---

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

fn compare_info(actual: &KaniSegmentInfo, expected: &KaniSegmentInfo) -> Result<(), String> {
    // posi: order-insensitive (get-non-arch-posi is a DB query and
    // unordered upstream).
    let mut a = actual.posi.clone();
    let mut e = expected.posi.clone();
    a.sort();
    e.sort();
    if a != e {
        return Err(format!(
            "posi: rust={:?} lisp={:?}",
            actual.posi, expected.posi
        ));
    }
    if actual.seq_set != expected.seq_set {
        return Err(format!(
            "seq-set: rust={:?} lisp={:?}",
            actual.seq_set, expected.seq_set
        ));
    }
    if actual.conj.len() != expected.conj.len() {
        return Err(format!(
            "conj len: rust={} lisp={}",
            actual.conj.len(),
            expected.conj.len()
        ));
    }
    for (i, (a, e)) in actual.conj.iter().zip(expected.conj.iter()).enumerate() {
        compare_conj_data(a, e).map_err(|err| format!("conj[{}]: {}", i, err))?;
    }
    if actual.common != expected.common {
        return Err(format!(
            "common: rust={:?} lisp={:?}",
            actual.common, expected.common
        ));
    }
    compare_score_info(&actual.score_info, &expected.score_info)?;
    if actual.kpcl != expected.kpcl {
        return Err(format!(
            "kpcl: rust={:?} lisp={:?}",
            actual.kpcl, expected.kpcl
        ));
    }
    Ok(())
}

fn compare_conj_data(actual: &ConjData, expected: &ConjData) -> Result<(), String> {
    if actual.seq != expected.seq {
        return Err(format!("seq: rust={:?} lisp={:?}", actual.seq, expected.seq));
    }
    if actual.from != expected.from {
        return Err(format!("from: rust={:?} lisp={:?}", actual.from, expected.from));
    }
    if actual.via != expected.via {
        return Err(format!("via: rust={:?} lisp={:?}", actual.via, expected.via));
    }
    match (&actual.prop, &expected.prop) {
        (None, None) => (),
        (Some(a), Some(e)) => {
            if a.id != e.id
                || a.conj_id != e.conj_id
                || a.conj_type != e.conj_type
                || a.pos != e.pos
                || a.neg != e.neg
                || a.fml != e.fml
            {
                return Err(format!("prop: rust={:?} lisp={:?}", a, e));
            }
        }
        (a, e) => {
            return Err(format!(
                "prop: rust={} lisp={}",
                if a.is_some() { "Some" } else { "None" },
                if e.is_some() { "Some" } else { "None" }
            ))
        }
    }
    if actual.src_map != expected.src_map {
        return Err(format!(
            "src_map: rust={:?} lisp={:?}",
            actual.src_map, expected.src_map
        ));
    }
    Ok(())
}

fn compare_score_info(actual: &KaniScoreInfo, expected: &KaniScoreInfo) -> Result<(), String> {
    if actual.prop_score != expected.prop_score {
        return Err(format!(
            "score-info.prop_score: rust={} lisp={}",
            actual.prop_score, expected.prop_score
        ));
    }
    if actual.kanji_break != expected.kanji_break {
        return Err(format!(
            "score-info.kanji_break: rust={:?} lisp={:?}",
            actual.kanji_break, expected.kanji_break
        ));
    }
    if actual.use_length_bonus != expected.use_length_bonus {
        return Err(format!(
            "score-info.use_length_bonus: rust={} lisp={}",
            actual.use_length_bonus, expected.use_length_bonus
        ));
    }
    match (&actual.split_info, &expected.split_info) {
        (KaniSplitInfo::None, KaniSplitInfo::None) => Ok(()),
        (KaniSplitInfo::Score(a), KaniSplitInfo::Score(e)) if a == e => Ok(()),
        (
            KaniSplitInfo::Parts { score_mod: ams, part_scores: aps },
            KaniSplitInfo::Parts { score_mod: ems, part_scores: eps },
        ) if ams == ems && aps == eps => Ok(()),
        (a, e) => Err(format!(
            "score-info.split_info: rust={:?} lisp={:?}",
            a, e
        )),
    }
}

// Local copy of `audit/common/mod.rs`'s private `require_field` — kept
// in-binary per gen_score_test L483's "audit-binary independence" note.
fn require_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field {} on: {}", key, value))
}

#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
