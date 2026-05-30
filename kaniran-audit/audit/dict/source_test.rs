//! Manual fixture-replay runner for `ICHIRAN/DICT:SOURCE`.
//! Source under test: `src/dict/source.rs`.
//!
//! Run with:
//!   cargo run --bin source_test -- \
//!       --path corpus/<corpus_tag>/dict/source.parquet
//!
//! Source identity: (variant-class-tag, seq, text). Lisp's
//! `(source obj)` returns whatever the slot holds — a kanji-text /
//! kana-text row (or nil) for both counter-text and proxy-text. The
//! counter-vs-proxy distinction is at the input, not the output, so
//! equality compares row identity. Variants without a `source` slot
//! upstream (kana, kanji, compound) are skipped.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kani::KaniSimpleTextDispatchEnum;
use kaniran_core::dict::kanji_text_dao::KanjiText;
use kaniran_core::dict::source::{source, SourceRef};

use common::{
    captured_class, parse_captured_word, single_result, CapturedKanaText, CapturedKanjiText,
    CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:SOURCE";

enum SimpleLeaf<'a> {
    Kana(&'a KanaText),
    Kanji(&'a KanjiText),
}


fn audit_one(row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 1 {
        return Err(format!("expected 1 arg, got {}", row.args.len()));
    }
    let class = captured_class(&row.args[0])?;
    if matches!(
        class,
        "CONJUGATION" | "SENSE" | "SENSE-PROP" | "RESTRICTED-READINGS" | "ENTRY" | "CONJ-SOURCE-READING"
    ) {
        return Ok(());
    }
    // No `source` upstream method on these classes; Rust dispatcher
    // returns None and Lisp would error. Either way the row carries no
    // value to compare, so skip.
    if matches!(class, "KANA-TEXT" | "KANJI-TEXT" | "COMPOUND-TEXT") {
        return Ok(());
    }

    let word = parse_captured_word(&row.args[0])?;
    let actual = source(&word);

    let expected_value = unwrap_proxy(single_result(&row.result)?)?;

    match (actual.as_ref().map(source_leaf), expected_value) {
        (None, None) => Ok(()),
        (Some(_), None) => Err(format!(
            "rust=Some lisp=null\n  rust: {:?}",
            actual.as_ref().map(source_leaf_dbg)
        )),
        (None, Some(v)) => Err(format!("rust=None lisp=Some\n  lisp: {}", v)),
        (Some(leaf), Some(v)) => compare_leaf(&leaf, v),
    }
}

fn source_leaf<'a>(s: &SourceRef<'a>) -> SimpleLeaf<'a> {
    match s {
        SourceRef::CounterKana(k) => SimpleLeaf::Kana(k),
        SourceRef::CounterKanji(k) => SimpleLeaf::Kanji(k),
        SourceRef::ProxySimple(s) => simple_leaf(s),
    }
}

fn simple_leaf<'a>(s: &'a KaniSimpleTextDispatchEnum) -> SimpleLeaf<'a> {
    match s {
        KaniSimpleTextDispatchEnum::Kana(k) => SimpleLeaf::Kana(k),
        KaniSimpleTextDispatchEnum::Kanji(k) => SimpleLeaf::Kanji(k),
        KaniSimpleTextDispatchEnum::Proxy(p) => simple_leaf(&p.source),
    }
}

fn source_leaf_dbg(s: &SourceRef<'_>) -> String {
    match source_leaf(s) {
        SimpleLeaf::Kana(k) => format!("{:?}", k),
        SimpleLeaf::Kanji(k) => format!("{:?}", k),
    }
}

// Recurse on proxy: its source slot may itself be a proxy whose leaf is
// the actual row.
fn unwrap_proxy(v: &Value) -> Result<Option<&Value>, String> {
    if v.is_null() {
        return Ok(None);
    }
    let class = captured_class(v)?;
    if class == "PROXY-TEXT" {
        let inner = v
            .get("source")
            .ok_or_else(|| "proxy result missing source".to_string())?;
        return unwrap_proxy(inner);
    }
    Ok(Some(v))
}

fn compare_leaf(actual: &SimpleLeaf<'_>, expected: &Value) -> Result<(), String> {
    let class = captured_class(expected)?;
    match (actual, class) {
        (SimpleLeaf::Kana(k), "KANA-TEXT") => {
            let captured: CapturedKanaText = serde_json::from_value(expected.clone())
                .map_err(|err| format!("kana-text parse: {}", err))?;
            if captured.matches(k) {
                Ok(())
            } else {
                Err(format!("\n  rust: {:?}\n  lisp: {:?}", k, captured))
            }
        }
        (SimpleLeaf::Kanji(k), "KANJI-TEXT") => {
            let captured: CapturedKanjiText = serde_json::from_value(expected.clone())
                .map_err(|err| format!("kanji-text parse: {}", err))?;
            if captured.matches(k) {
                Ok(())
            } else {
                Err(format!("\n  rust: {:?}\n  lisp: {:?}", k, captured))
            }
        }
        (SimpleLeaf::Kana(k), other) => Err(format!(
            "class mismatch: rust=KANA-TEXT lisp=:{}\n  rust: {:?}",
            other, k
        )),
        (SimpleLeaf::Kanji(k), other) => Err(format!(
            "class mismatch: rust=KANJI-TEXT lisp=:{}\n  rust: {:?}",
            other, k
        )),
    }
}


fn main() {
    common::run_sync(EXPECTED_FQN, audit_one);
}
