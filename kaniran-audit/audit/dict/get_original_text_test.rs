//! Manual fixture-replay runner for `ICHIRAN/DICT:GET-ORIGINAL-TEXT`.
//! Source under test: `src/dict/get_original_text.rs`.
//!
//! Run with:
//!   cargo run --bin get_original_text_test -- \
//!       --path corpus/<corpus_tag>/dict/get_original_text.parquet
//!
//! Replays a captured reading (plus conj-data) through
//! `get_original_text` and compares the returned kana-text/kanji-text
//! rows against the Lisp result.

#[path = "../common/mod.rs"]
mod common;

use serde::Deserialize;
use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::conj::ConjData;
use kaniran_core::dict::dao::ConjProp;
use kaniran_core::dict::accessors::get_original_text;
use kaniran_core::dict::dao::KanaText;
use kaniran_core::dict::kani_word::KaniSimpleTextDispatchEnum;
use kaniran_core::dict::dao::KanjiText;

use common::{
    captured_class, parse_captured_simple_text, CapturedKanaText, CapturedKanjiText, CapturedRow,
};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:GET-ORIGINAL-TEXT";


async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 3 {
        return Err(format!("expected 3 args, got {}", row.args.len()));
    }
    let reading = parse_captured_simple_text(&row.args[0])?;

    let keyword = row.args[1]
        .as_str()
        .ok_or_else(|| format!("arg[1] not keyword: {}", row.args[1]))?;
    if keyword != ":CONJ-DATA" {
        return Err(format!("unexpected keyword at arg[1]: {}", keyword));
    }

    // value: null → Lisp NIL → Option::None (per get_original_text.rs:17,
    // "None ≡ keyword absent ≡ Lisp NIL"; Lisp's `(or conj-data ...)`
    // also collapses an empty list to nil-fallback, but the captured
    // corpus only ever passes non-empty arrays here).
    let conj_data_owned: Option<Vec<ConjData>> = match &row.args[2] {
        Value::Null => None,
        Value::Array(items) => {
            let parsed: Result<Vec<_>, String> =
                items.iter().map(parse_captured_conj_data).collect();
            Some(parsed?)
        }
        other => {
            return Err(format!(
                "conj-data value: expected null/array, got {}",
                other
            ))
        }
    };

    let actual = get_original_text(ctx, &reading, conj_data_owned.as_deref())
        .await
        .map_err(|err| format!("get_original_text query: {}", err))?;

    let expected = expected_list(&row.result)?;
    compare(actual, expected)
}

fn expected_list(result: &[Value]) -> Result<Vec<&Value>, String> {
    if result.is_empty() {
        return Err("result envelope empty".into());
    }
    match &result[0] {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => Ok(items.iter().collect()),
        other => Err(format!("expected list/null at result[0], got {}", other)),
    }
}

fn compare(
    actual: Vec<KaniSimpleTextDispatchEnum>,
    expected: Vec<&Value>,
) -> Result<(), String> {
    let mut actual_kana: Vec<KanaText> = Vec::new();
    let mut actual_kanji: Vec<KanjiText> = Vec::new();
    for item in actual {
        match item {
            KaniSimpleTextDispatchEnum::Kana(k) => actual_kana.push(k),
            KaniSimpleTextDispatchEnum::Kanji(k) => actual_kanji.push(k),
            KaniSimpleTextDispatchEnum::Proxy(_) => {
                return Err("rust returned proxy-text — impossible for get-original-text".into())
            }
        }
    }

    let mut expected_kana: Vec<CapturedKanaText> = Vec::new();
    let mut expected_kanji: Vec<CapturedKanjiText> = Vec::new();
    for item in expected {
        let class = captured_class(item)?;
        match class {
            "KANA-TEXT" => expected_kana.push(
                serde_json::from_value(item.clone())
                    .map_err(|err| format!("kana-text parse: {}", err))?,
            ),
            "KANJI-TEXT" => expected_kanji.push(
                serde_json::from_value(item.clone())
                    .map_err(|err| format!("kanji-text parse: {}", err))?,
            ),
            other => return Err(format!("unsupported row class: :{}", other)),
        }
    }

    compare_kana(actual_kana, expected_kana)?;
    compare_kanji(actual_kanji, expected_kanji)?;
    Ok(())
}

fn compare_kana(
    mut actual: Vec<KanaText>,
    mut expected: Vec<CapturedKanaText>,
) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "kana row count: rust={} lisp={}",
            actual.len(),
            expected.len()
        ));
    }
    actual.sort_by(|a, b| (a.seq, &a.text, a.ord, a.id).cmp(&(b.seq, &b.text, b.ord, b.id)));
    expected.sort_by(|a, b| (a.seq, &a.text, a.ord, a.id).cmp(&(b.seq, &b.text, b.ord, b.id)));
    for (idx, (a, c)) in actual.iter().zip(&expected).enumerate() {
        if !c.matches(a) {
            return Err(format!("kana row {}: rust={:?} lisp={:?}", idx, a, c));
        }
    }
    Ok(())
}

fn compare_kanji(
    mut actual: Vec<KanjiText>,
    mut expected: Vec<CapturedKanjiText>,
) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "kanji row count: rust={} lisp={}",
            actual.len(),
            expected.len()
        ));
    }
    actual.sort_by(|a, b| (a.seq, &a.text, a.ord, a.id).cmp(&(b.seq, &b.text, b.ord, b.id)));
    expected.sort_by(|a, b| (a.seq, &a.text, a.ord, a.id).cmp(&(b.seq, &b.text, b.ord, b.id)));
    for (idx, (a, c)) in actual.iter().zip(&expected).enumerate() {
        if !c.matches(a) {
            return Err(format!("kanji row {}: rust={:?} lisp={:?}", idx, a, c));
        }
    }
    Ok(())
}


// --- conj-data input mirror ------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturedConjProp {
    #[serde(rename = "_meta")]
    _meta: serde::de::IgnoredAny,
    id: i32,
    conj_id: i32,
    conj_type: i32,
    pos: String,
    #[serde(deserialize_with = "deserialize_optional_bool")]
    neg: Option<bool>,
    #[serde(deserialize_with = "deserialize_optional_bool")]
    fml: Option<bool>,
}

impl CapturedConjProp {
    fn into_dao(self) -> ConjProp {
        ConjProp {
            id: self.id,
            conj_id: self.conj_id,
            conj_type: self.conj_type,
            pos: self.pos,
            neg: self.neg,
            fml: self.fml,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturedConjData {
    #[serde(rename = "_meta")]
    _meta: serde::de::IgnoredAny,
    seq: Option<i32>,
    from: Option<i32>,
    via: Option<i32>,
    prop: Option<CapturedConjProp>,
    src_map: Vec<(String, String)>,
}

fn parse_captured_conj_data(value: &Value) -> Result<ConjData, String> {
    let class = captured_class(value)?;
    if class != "CONJ-DATA" {
        return Err(format!("expected CONJ-DATA, got :{}", class));
    }
    let captured: CapturedConjData = serde_json::from_value(value.clone())
        .map_err(|err| format!("conj-data parse: {}", err))?;
    Ok(ConjData {
        seq: captured.seq,
        from: captured.from,
        via: captured.via,
        prop: captured.prop.map(|p| p.into_dao()),
        src_map: captured.src_map,
    })
}

/// `(or db-null boolean)` — projects as `null` (nil), `":NULL"`
/// (db-null sentinel), or a bool. All non-bool inputs collapse to
/// `None`; the underlying DAO field is `Option<bool>` and treats
/// "absent" and "DB NULL" identically.
fn deserialize_optional_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(None),
        Value::String(ref s) if s == ":NULL" => Ok(None),
        Value::Bool(b) => Ok(Some(b)),
        other => Err(serde::de::Error::custom(format!(
            "expected bool / null / \":NULL\", got {}",
            other
        ))),
    }
}


#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
