//! Manual fixture-replay runner for `ICHIRAN/DICT:FIND-WORD-SUFFIX`.
//! Source under test: `src/dict/find_word_suffix.rs`.
//!
//! Run with:
//!   cargo run --release --bin find_word_suffix_test -- \
//!       --path corpus/extracted_find_word_layer_2026_05_21/dict/find_word_suffix.parquet
//!
//! Replays captured words through `find_word_suffix` and compares the
//! returned compound-text rows against the Lisp result.
//!
//! Replay sets `ctx.suffix_map_temp = None`, routing through
//! `get_suffixes(ctx, word)`; the captured driver instead read the
//! pre-bound `*suffix-map-temp*`, but both yield the same suffix
//! triples for a given word — the `(> offset 0)` guard leaves exactly
//! the proper suffixes of `word`. The same equivalence applies at
//! each recursion level.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::text_classes::{CompoundText, ScoreMod};
use kaniran_core::dict::grammar::suffix::resolve::find_word_suffix;
use kaniran_core::dict::dao::KanaText;
use kaniran_core::dict::kani_word::KaniWordDispatchEnum;
use kaniran_core::dict::dao::KanjiText;
use kaniran_core::dict::text_classes::ProxyText;

use common::{parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:FIND-WORD-SUFFIX";

fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 3 {
        return Err(format!("expected 3 args, got {}", row.args.len()));
    }
    let word = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 (word) not string: {}", row.args[0]))?;
    // arg 1 is the literal `:MATCHES` keyword marker — assert it
    // structurally to catch projector-shape drift, then ignore.
    match row.args[1].as_str() {
        Some(":MATCHES") => {}
        other => {
            return Err(format!(
                "arg 1 expected \":MATCHES\" keyword marker, got {:?}",
                other
            ))
        }
    }
    let matches = parse_matches(&row.args[2])?;

    let actual = find_word_suffix(ctx, word, &matches)
        
        .map_err(|e| format!("find_word_suffix: {}", e))?;

    let expected = parse_expected(unwrap_result(&row.result)?)?;

    let actual_fp = canonical_fingerprints_word(&actual);
    let expected_fp = canonical_fingerprints_word(&expected);

    if actual_fp != expected_fp {
        return Err(format!(
            "result mismatch on word={:?} matches.len={}:\n  rust ={:?}\n  lisp ={:?}",
            word,
            matches.len(),
            actual_fp,
            expected_fp,
        ));
    }
    Ok(())
}

fn parse_matches(value: &Value) -> Result<Vec<KaniWordDispatchEnum>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for (idx, item) in arr.iter().enumerate() {
                out.push(
                    parse_captured_word(item)
                        .map_err(|e| format!("matches[{}]: {}", idx, e))?,
                );
            }
            Ok(out)
        }
        other => Err(format!("matches: expected array / null, got {}", other)),
    }
}

fn unwrap_result(result: &[Value]) -> Result<&Value, String> {
    if result.is_empty() {
        return Err("expected at least 1 result value, got 0".into());
    }
    Ok(&result[0])
}

fn parse_expected(value: &Value) -> Result<Vec<KaniWordDispatchEnum>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for (idx, item) in arr.iter().enumerate() {
                out.push(
                    parse_captured_word(item)
                        .map_err(|e| format!("compound {}: {}", idx, e))?,
                );
            }
            Ok(out)
        }
        other => Err(format!("result: expected array or null, got {}", other)),
    }
}

fn canonical_fingerprints_word(words: &[KaniWordDispatchEnum]) -> Vec<String> {
    let mut fps: Vec<String> = words.iter().map(fp_word).collect();
    fps.sort();
    fps
}

// Structural fingerprints — id is omitted everywhere. See module
// docstring for the synthesized-DAO rationale.

fn fp_word(w: &KaniWordDispatchEnum) -> String {
    match w {
        KaniWordDispatchEnum::Kana(k) => fp_kana(k),
        KaniWordDispatchEnum::Kanji(k) => fp_kanji(k),
        KaniWordDispatchEnum::Compound(c) => fp_compound(c),
        KaniWordDispatchEnum::Proxy(p) => fp_proxy(p),
        // Counter rows don't arise from find-word-suffix at the
        // upstream callsites observed in this corpus (the suffix
        // dispatch produces compound-text + proxy-text only).
        // Keep a structural fingerprint so an unexpected counter
        // landing here surfaces as a mismatch rather than a panic.
        KaniWordDispatchEnum::Counter(c) => {
            let b = c.base();
            format!("COUNTER{{text={:?},kana={:?},number={}}}", b.text, b.kana, b.number)
        }
    }
}

fn fp_proxy(p: &ProxyText) -> String {
    format!(
        "PROXY{{text={:?},kana={:?},source={},hintedp={},conjugations={:?}}}",
        p.text,
        p.kana,
        // proxy-text wraps a simple-text source (KANA/KANJI/PROXY).
        // Recurse so source-side mismatches surface in the fingerprint.
        fp_simple_text(&p.source),
        p.state.hintedp,
        p.state.conjugations,
    )
}

fn fp_simple_text(s: &kaniran_core::dict::kani_word::KaniSimpleTextDispatchEnum) -> String {
    use kaniran_core::dict::kani_word::KaniSimpleTextDispatchEnum;
    match s {
        KaniSimpleTextDispatchEnum::Kana(k) => fp_kana(k),
        KaniSimpleTextDispatchEnum::Kanji(k) => fp_kanji(k),
        KaniSimpleTextDispatchEnum::Proxy(p) => fp_proxy(p),
    }
}

fn fp_kana(k: &KanaText) -> String {
    format!(
        "KANA{{seq={},text={:?},ord={},common={:?},common_tags={:?},conjugate_p={},nokanji={},best_kanji={:?},conjugations={:?},hintedp={}}}",
        k.seq,
        k.text,
        k.ord,
        k.common,
        k.common_tags,
        k.conjugate_p,
        k.nokanji,
        k.best_kanji,
        k.state.conjugations,
        k.state.hintedp,
    )
}

fn fp_kanji(k: &KanjiText) -> String {
    format!(
        "KANJI{{seq={},text={:?},ord={},common={:?},common_tags={:?},conjugate_p={},nokanji={},best_kana={:?},conjugations={:?},hintedp={}}}",
        k.seq,
        k.text,
        k.ord,
        k.common,
        k.common_tags,
        k.conjugate_p,
        k.nokanji,
        k.best_kana,
        k.state.conjugations,
        k.state.hintedp,
    )
}

fn fp_compound(c: &CompoundText) -> String {
    let words_fp: Vec<String> = c.words.iter().map(fp_word).collect();
    let score_base_fp = c.score_base.as_deref().map(fp_word);
    format!(
        "COMPOUND{{text={:?},kana={:?},primary={},words=[{}],score_base={:?},score_mod={}}}",
        c.text,
        c.kana,
        fp_word(&c.primary),
        words_fp.join(","),
        score_base_fp,
        fp_score_mod(&c.score_mod),
    )
}

fn fp_score_mod(s: &ScoreMod) -> String {
    match s {
        ScoreMod::Single(n) => format!("S({})", n),
        ScoreMod::Stack(items) => {
            let inner: Vec<String> = items.iter().map(fp_score_mod).collect();
            format!("Stack[{}]", inner.join(","))
        }
        // Constant is unreachable from this audit corpus (the four
        // function-closure suffixes parse_captured_score_mod
        // documents) but include it for completeness so a future
        // capture surfacing it produces a structural mismatch instead
        // of a panic.
        other => format!("{:?}", other),
    }
}

fn main() {
    common::run_async_streaming(EXPECTED_FQN, audit_one);
}
