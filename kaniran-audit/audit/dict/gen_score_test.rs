//! Manual fixture-replay runner for `ICHIRAN/DICT:GEN-SCORE`.
//! Source under test: `src/dict/gen_score.rs`.
//!
//! Run with:
//!   cargo run --release --bin gen_score_test -- \
//!       --path corpus/extracted_chunk_b_segmentation_2026_05_14/dict/gen_score.parquet
//!
//! Captured args shape (per `chunk_b_segmentation` corpus):
//!   `[<segment>, ":FINAL", <bool|null>, ":KANJI-BREAK", <int-list|null>]`
//!
//! Captured result shape:
//!   `[<mutated-segment>]` — the same segment with `score` and `info`
//!   populated by the recursive `calc-score` call. Every other slot
//!   (start, end, word, text) is preserved by gen-score's setf and
//!   passes through to the result. The audit validates ALL captured
//!   slots: score / info as the function's outputs, and
//!   start / end / word / text as passthrough invariants
//!   (a divergence would indicate the Rust port accidentally mutated
//!   a slot gen-score is supposed to leave alone).
//!
//! Streaming async runner because the parquet is 2.25M rows — the
//! default `run_async` would OOM during its in-memory group-by-args
//! dedup pass.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::conj_data::ConjData;
use kaniran_core::dict::calc_score::gen_score;
use kaniran_core::dict::kani::KaniWordDispatchEnum;
use kaniran_core::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
use kaniran_core::dict::counters::dispatchers::seq as word_seq;
use kaniran_core::dict::counters::dispatchers::text as word_text;
use kaniran_core::dict::word_info::WordInfoSeq;

use common::{
    captured_class, parse_captured_segment, parse_conj_list, parse_int_list, parse_kpcl,
    parse_opt_i32, parse_score_info, parse_string_list, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GEN-SCORE";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.is_empty() {
        return Err("gen-score args empty".into());
    }
    // The tracer projects args POST-call (see `feedback_extract_post_call_mutation_risk`),
    // so the captured segment already has the `score` / `info` slots
    // mutated by `gen-score` itself. That doesn't bias the audit: the
    // ported `gen_score` unconditionally overwrites both slots before
    // returning, so the captured values flow through as-is and the
    // comparison below checks the function's own re-computation
    // against the recorded result segment (the same mutated instance).
    let mut segment = parse_captured_segment(&row.args[0])
        .map_err(|e| format!("arg 0 (segment): {}", e))?;
    let (final_, kanji_break) = walk_keywords(&row.args[1..])?;

    gen_score(ctx, &mut segment, final_, &kanji_break)
        .await
        .map_err(|e| format!("gen_score: {}", e))?;

    if row.result.len() != 1 {
        return Err(format!("expected 1 result, got {}", row.result.len()));
    }
    let expected_segment = &row.result[0];
    let class = captured_class(expected_segment)?;
    if class != "SEGMENT" {
        return Err(format!("result not SEGMENT, got :{}", class));
    }

    // dict.lisp:986-987 — gen-score mutates `score` and `info` only.
    // The other four slots (`start`, `end`, `word`, `text`) are
    // passthrough invariants; verifying them catches any Rust-side
    // bug that accidentally clobbers them.

    // --- unknown-slot guard ---
    // Reject any segment slot the projector emits that the audit
    // doesn't explicitly handle. Catches upstream defstruct changes
    // (new slots silently passing through unchecked).
    if let Some(obj) = expected_segment.as_object() {
        for key in obj.keys() {
            match key.as_str() {
                "_meta" | "start" | "end" | "word" | "score" | "info" | "text" | "top" => {}
                other => {
                    return Err(format!(
                        "unhandled segment slot: {} (audit drift — extend gen_score_test)",
                        other
                    ));
                }
            }
        }
    }

    // --- passthrough slots (start, end, word, text, and top when captured) ---
    let expected_passthrough = parse_captured_segment(expected_segment)
        .map_err(|e| format!("result segment: {}", e))?;
    compare_passthrough_slots(&segment, &expected_passthrough)?;
    // Deep word equality. `compare_word_identity` above checks only
    // variant + seq + text; a deep field-by-field walk would require a
    // hand-rolled comparator on every word subclass (5 variants, each
    // recursive via PROXY.source / COMPOUND.primary+words+score_base).
    // Cheaper + stricter alternative: the captured args[0].word and
    // result[0].word are the same object (gen-score does not assign
    // segment-word), and the Rust port borrows segment.word
    // immutably, so JSON-equality between args[0].word and
    // result[0].word implies parse(args[0].word) == parse(result[0].word)
    // == Rust segment.word post-call. A mismatch surfaces either
    // upstream tracer drift or a Rust port mistakenly mutating word.
    let args_word = row.args[0]
        .get("word")
        .ok_or_else(|| "args[0] missing word".to_string())?;
    let result_word = expected_segment
        .get("word")
        .ok_or_else(|| "result[0] missing word".to_string())?;
    if !json_deep_eq(args_word, result_word) {
        return Err(format!(
            "word deep-equality: args[0].word != result[0].word \
             (first divergence: {})",
            json_first_diff(args_word, result_word).unwrap_or_else(|| "<unknown>".into())
        ));
    }
    // `top` is emitted by the JSON projector
    // (`ichiran-extractor/projectors_json.lisp:158-166` structure-object
    // arm, verified via SBCL probe), but the 2026-05-14 chunk_b parquet
    // pre-dates that and omits the key. Defensive: validate only when
    // present, so re-extractions catch any future drift without
    // breaking the current parquet.
    if let Some(top_val) = expected_segment.get("top") {
        if !top_val.is_null() {
            return Err(format!(
                "top: lisp captured non-null ({}) but gen-score is called on \
                 fresh segments whose top is always nil — projector / capture drift",
                top_val
            ));
        }
        if segment.top.is_some() {
            return Err("top: rust is Some(_) but lisp is null".into());
        }
    }

    // --- score ---
    let expected_score = match expected_segment.get("score").unwrap_or(&Value::Null) {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_i64()
                .ok_or_else(|| format!("expected score not i64: {}", n))?
                as i32,
        ),
        other => return Err(format!("expected score: {}", other)),
    };
    if segment.score != expected_score {
        return Err(format!(
            "score: rust={:?} lisp={:?}",
            segment.score, expected_score
        ));
    }

    // --- info (deep) ---
    let expected_info_val = expected_segment.get("info").unwrap_or(&Value::Null);
    let expected_info_opt = match expected_info_val {
        Value::Null => None,
        other => Some(parse_info_plist(other)?),
    };
    match (&segment.info, &expected_info_opt) {
        (None, None) => Ok(()),
        (Some(a), Some(e)) => compare_info(a, e),
        (a, e) => Err(format!(
            "info: rust={} lisp={}",
            if a.is_some() { "Some" } else { "None" },
            if e.is_some() { "Some" } else { "None" },
        )),
    }
}

/// Validate the four slots that `gen-score` does NOT touch
/// (`dict.lisp:985-988` only `setf`s `segment-score` / `segment-info`).
/// The Rust port mutates the segment in place, so post-call the
/// passthrough slots should be byte-identical to the captured result.
fn compare_passthrough_slots(actual: &Segment, expected: &Segment) -> Result<(), String> {
    if actual.start != expected.start {
        return Err(format!(
            "start: rust={} lisp={}",
            actual.start, expected.start
        ));
    }
    if actual.end != expected.end {
        return Err(format!("end: rust={} lisp={}", actual.end, expected.end));
    }
    if actual.text != expected.text {
        return Err(format!(
            "text: rust={:?} lisp={:?}",
            actual.text, expected.text
        ));
    }
    compare_word_identity(&actual.word, &expected.word)
}

/// Word equality by variant tag + `seq` + `text` — the three observable
/// bits that uniquely identify a captured word reading. Full structural
/// equality on the enum is intentionally not used: `KaniWordDispatchEnum`
/// has no `PartialEq`, and slot-for-slot comparison would compare
/// transient fields (e.g. `state.hintedp`) that the projector strips
/// from non-input positions.
fn compare_word_identity(
    actual: &KaniWordDispatchEnum,
    expected: &KaniWordDispatchEnum,
) -> Result<(), String> {
    let actual_tag = word_variant_tag(actual);
    let expected_tag = word_variant_tag(expected);
    if actual_tag != expected_tag {
        return Err(format!(
            "word variant: rust={} lisp={}",
            actual_tag, expected_tag
        ));
    }
    let actual_seq = word_seq(actual);
    let expected_seq = word_seq(expected);
    if !word_seqs_equal(&actual_seq, &expected_seq) {
        return Err(format!(
            "word seq: rust={:?} lisp={:?}",
            actual_seq, expected_seq
        ));
    }
    let actual_text = word_text(actual);
    let expected_text = word_text(expected);
    if actual_text != expected_text {
        return Err(format!(
            "word text: rust={:?} lisp={:?}",
            actual_text, expected_text
        ));
    }
    Ok(())
}

fn word_variant_tag(w: &KaniWordDispatchEnum) -> &'static str {
    match w {
        KaniWordDispatchEnum::Kanji(_) => "Kanji",
        KaniWordDispatchEnum::Kana(_) => "Kana",
        KaniWordDispatchEnum::Proxy(_) => "Proxy",
        KaniWordDispatchEnum::Compound(_) => "Compound",
        KaniWordDispatchEnum::Counter(_) => "Counter",
    }
}

/// Deep JSON-value equality. Used to validate the entire captured
/// word subtree round-trip (every nested slot in KANA/KANJI/PROXY/
/// COMPOUND/COUNTER) without needing a hand-rolled comparator per
/// subclass. Objects are compared key-set then per-key recursively;
/// arrays positionally; scalars by `Value::eq`.
fn json_deep_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => {
            if ma.len() != mb.len() {
                return false;
            }
            for (k, va) in ma {
                match mb.get(k) {
                    Some(vb) => {
                        if !json_deep_eq(va, vb) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        }
        (Value::Array(xa), Value::Array(xb)) => {
            xa.len() == xb.len() && xa.iter().zip(xb).all(|(x, y)| json_deep_eq(x, y))
        }
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

/// Find the first divergent path between two JSON values, for
/// diagnostic output. Returns a dotted path like
/// `primary.source.hintedp` or `words[2].best_kana`.
fn json_first_diff(a: &Value, b: &Value) -> Option<String> {
    json_first_diff_at(a, b, String::new())
}

fn json_first_diff_at(a: &Value, b: &Value, path: String) -> Option<String> {
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => {
            for k in ma.keys().chain(mb.keys()) {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", path, k)
                };
                match (ma.get(k), mb.get(k)) {
                    (Some(va), Some(vb)) => {
                        if let Some(d) = json_first_diff_at(va, vb, p) {
                            return Some(d);
                        }
                    }
                    (Some(_), None) => return Some(format!("{} (only in args)", p)),
                    (None, Some(_)) => return Some(format!("{} (only in result)", p)),
                    (None, None) => {}
                }
            }
            None
        }
        (Value::Array(xa), Value::Array(xb)) => {
            if xa.len() != xb.len() {
                return Some(format!(
                    "{} (len {} vs {})",
                    if path.is_empty() { "<root>" } else { &path },
                    xa.len(),
                    xb.len()
                ));
            }
            for (i, (x, y)) in xa.iter().zip(xb).enumerate() {
                let p = format!("{}[{}]", path, i);
                if let Some(d) = json_first_diff_at(x, y, p) {
                    return Some(d);
                }
            }
            None
        }
        (x, y) if x == y => None,
        (x, y) => Some(format!(
            "{} (args={} result={})",
            if path.is_empty() { "<root>" } else { &path },
            x,
            y
        )),
    }
}

fn word_seqs_equal(a: &Option<WordInfoSeq>, b: &Option<WordInfoSeq>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(WordInfoSeq::Single(x)), Some(WordInfoSeq::Single(y))) => x == y,
        (Some(WordInfoSeq::Multi(x)), Some(WordInfoSeq::Multi(y))) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|(xa, xb)| word_seqs_equal(xa, xb))
        }
        _ => false,
    }
}

fn walk_keywords(tail: &[Value]) -> Result<(bool, Vec<usize>), String> {
    let mut i = 0;
    let mut final_ = false;
    let mut kanji_break: Vec<usize> = Vec::new();
    while i < tail.len() {
        let key = tail[i]
            .as_str()
            .ok_or_else(|| format!("keyword at {} not string: {}", i, tail[i]))?;
        if i + 1 >= tail.len() {
            return Err(format!("keyword {} missing value", key));
        }
        let v = &tail[i + 1];
        match key {
            ":FINAL" => {
                final_ = match v {
                    Value::Null => false,
                    Value::Bool(b) => *b,
                    other => return Err(format!(":FINAL: expected bool/null, got {}", other)),
                };
            }
            ":KANJI-BREAK" => {
                kanji_break = match v {
                    Value::Null => Vec::new(),
                    Value::Array(arr) => arr
                        .iter()
                        .map(|n| {
                            n.as_u64()
                                .map(|n| n as usize)
                                .ok_or_else(|| format!(":KANJI-BREAK entry: {}", n))
                        })
                        .collect::<Result<_, _>>()?,
                    other => {
                        return Err(format!(":KANJI-BREAK: expected array/null, got {}", other))
                    }
                };
            }
            other => return Err(format!("gen-score: unexpected keyword {}", other)),
        }
        i += 2;
    }
    Ok((final_, kanji_break))
}

// ----- info plist parsing + comparison (shared shape with calc-score
// audit; kept inline to preserve audit-binary independence) -----

/// Captured plists from the compound + skip-word branch carry only
/// `:CONJ` — upstream's `(setf (getf nil :conj) X)` at
/// `dict.lisp:785-786` synthesizes a one-key plist when the inner
/// `calc-score` returned just `0`. The Rust src side mirrors this by
/// defaulting the other five fields to zero/empty, so missing keys
/// here resolve to the same zero/empty values rather than erroring.
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
    Ok(KaniSegmentInfo { posi, seq_set, conj, common, score_info, kpcl })
}

fn compare_info(actual: &KaniSegmentInfo, expected: &KaniSegmentInfo) -> Result<(), String> {
    // posi: order-insensitive (get-non-arch-posi DB query is unordered).
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


#[tokio::main]
async fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one).await;
}
