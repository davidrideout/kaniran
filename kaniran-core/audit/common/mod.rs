//! Shared helpers for fixture-replay audit binaries.
//!
//! Each per-FQN audit binary includes this via:
//!
//! ```ignore
//! #[path = "../common/mod.rs"]
//! mod common;
//! ```
//!
//! Each runner uses only a subset of the helpers (the simple-text /
//! DAO machinery doesn't apply to e.g. `normalize_test`), so dead-code
//! warnings are silenced module-wide.
//!
//! Provides:
//! - [`load_parquet`]: reads a `corpus/<corpus_tag>/<fqn>.parquet`
//!   captured by the JSON projector. Each row has `args` and `result`
//!   text columns holding one JSON value apiece (per the schema
//!   documented below). The FQN is implied by the binary identity and
//!   the parquet filename — no metadata assertion.
//! - [`parse_path_arg`]: read `--path <file>` from `std::env::args`.
//! - Sentinel-aware serde deserializers and captured-DAO mirrors
//!   ([`CapturedKanaText`], [`CapturedKanjiText`]) that consume the
//!   wire format and produce real production DAOs ready to feed back
//!   into the function under test.
//!
//! JSON value encoding (matches `:ICHI-PROJECTORS-JSON` on .103):
//! - `null` ← Lisp NIL (false / empty list / unset / missing)
//! - `true` ← Lisp T
//! - `":FOO"` ← Lisp keyword `:FOO`
//! - `":NULL"` ← Lisp `:NULL` (DB null, distinct from NIL)
//! - DAO / struct / class → `{"_meta": {"class": "KANA-TEXT"}, "id": ..., ...}`
//! - `#\char` → `{"_meta": {"char": <codepoint>}}` (unused for get-split)
//! - `(a . b)` cons → `{"_meta": {"cons": [a, b]}}` (unused for get-split)
//! - proper list → JSON array

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow::array::StringArray;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::Deserialize;
use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::compound_text_class::{CompoundText, ScoreMod};
use kaniran_core::dict::counter_age_class::CounterAge;
use kaniran_core::dict::counter_days_kun_class::CounterDaysKun;
use kaniran_core::dict::counter_days_on_class::CounterDaysOn;
use kaniran_core::dict::counter_halfhour_class::CounterHalfhour;
use kaniran_core::dict::counter_hifumi_class::CounterHifumi;
use kaniran_core::dict::counter_months_class::CounterMonths;
use kaniran_core::dict::counter_people_class::CounterPeople;
use kaniran_core::dict::counter_text_class::{Common, Counter, CounterSource, CounterText};
use kaniran_core::dict::counter_tsu_class::CounterTsu;
use kaniran_core::dict::counter_wari_class::CounterWari;
use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::kanji_text_dao::KanjiText;
use kaniran_core::dict::number_text_class::NumberText;
use kaniran_core::dict::proxy_text_class::ProxyText;
use kaniran_core::dict::simple_text_class::SimpleText;


// --- captured-row envelope --------------------------------------------------

#[derive(Debug)]
pub struct CapturedRow {
    pub args: Vec<Value>,
    pub result: Vec<Value>,
}

/// Loaded parquet content: one [`CapturedRow`] per row. The FQN the
/// binary expects is supplied separately by the runner — it's already
/// implied by the binary identity and the parquet filename.
#[derive(Debug)]
pub struct CapturedFile {
    pub rows: Vec<CapturedRow>,
}

pub fn load_parquet(path: &Path) -> CapturedFile {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|err| panic!("open {:?}: {}", path, err));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap_or_else(|err| panic!("parquet builder {:?}: {}", path, err));

    let reader = builder.build().expect("build reader");
    let mut rows: Vec<CapturedRow> = Vec::new();
    let mut row_seq: usize = 0;
    let mut skipped: usize = 0;

    for batch in reader {
        let batch = batch.expect("batch");
        let args_col = batch
            .column_by_name("args")
            .expect("args column")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("args is StringArray");
        let result_col = batch
            .column_by_name("result")
            .expect("result column")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("result is StringArray");
        for row_idx in 0..batch.num_rows() {
            row_seq += 1;
            let args_json = args_col.value(row_idx);
            let result_json = result_col.value(row_idx);
            let args: Vec<Value> = match serde_json::from_str(args_json) {
                Ok(v) => v,
                Err(err) => {
                    eprintln!("skip row {}: args parse: {}", row_seq, err);
                    skipped += 1;
                    continue;
                }
            };
            let result: Vec<Value> = match serde_json::from_str(result_json) {
                Ok(v) => v,
                Err(err) => {
                    eprintln!("skip row {}: result parse: {}", row_seq, err);
                    skipped += 1;
                    continue;
                }
            };
            rows.push(CapturedRow { args, result });
        }
    }

    if skipped > 0 {
        eprintln!("loader: {} row(s) skipped (malformed JSON from extractor)", skipped);
    }
    CapturedFile { rows }
}


// --- CLI ----------------------------------------------------------------

/// Parse `--path <file>` from process argv. Panics with a usage hint
/// if missing or malformed.
pub fn parse_path_arg() -> PathBuf {
    let argv: Vec<String> = std::env::args().collect();
    let usage = "usage: --path <corpus/<tag>/<fqn>.parquet>";
    let mut idx = 1;
    while idx < argv.len() {
        if argv[idx] == "--path" {
            return PathBuf::from(
                argv.get(idx + 1).unwrap_or_else(|| panic!("{}", usage)),
            );
        }
        idx += 1;
    }
    panic!("{}", usage);
}


// --- KaniranContext setup ---------------------------------------------

pub async fn setup_ctx() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        .await
        .expect("KaniranContext::from_env failed (DATABASE_URL or kaniran.toml not configured?)")
}


// --- runner harness --------------------------------------------------------

const MAX_FIRST_FAILURES: usize = 10;

/// Sync fixture-replay driver: parses `--path`, loads the parquet, then
/// invokes `audit_one(row)`. Rows are grouped by their `args` JSON so
/// non-deterministic captures (same args, multiple distinct results)
/// pass when Rust's output matches any captured result for those args.
/// Exits 0 if every group passes, 1 otherwise, printing the standard
/// summary line and up to [`MAX_FIRST_FAILURES`] failures.
pub fn run_sync<F>(expected_fqn: &str, audit_one: F) -> !
where
    F: Fn(&CapturedRow) -> Result<(), String>,
{
    let path = parse_path_arg();
    let file = load_parquet(&path);
    let groups = group_by_args(file.rows);
    let total = groups.len();

    let mut pass: usize = 0;
    let mut fail: usize = 0;
    let mut first_failures: Vec<String> = Vec::new();
    let mut progress = Progress::new(expected_fqn, total);
    for (idx, group) in groups.iter().enumerate() {
        let outcome = run_group(group, |row| audit_one(row));
        record_outcome(outcome, idx, group.len(), &mut pass, &mut fail, &mut first_failures);
        progress.tick(idx + 1, pass, fail);
    }
    report_and_exit(expected_fqn, pass, fail, &first_failures)
}

/// Async flavor: same as [`run_sync`] but builds a [`KaniranContext`] via
/// [`setup_ctx`] and passes it as the first argument to `audit_one`.
pub async fn run_async<F>(expected_fqn: &str, audit_one: F) -> !
where
    F: AsyncFn(&KaniranContext, &CapturedRow) -> Result<(), String>,
{
    let path = parse_path_arg();
    let file = load_parquet(&path);
    let groups = group_by_args(file.rows);
    let total = groups.len();

    let ctx = setup_ctx().await;

    let mut pass: usize = 0;
    let mut fail: usize = 0;
    let mut first_failures: Vec<String> = Vec::new();
    let mut progress = Progress::new(expected_fqn, total);
    for (idx, group) in groups.iter().enumerate() {
        let mut last_err: Option<String> = None;
        let mut group_ok = false;
        for row in group {
            match audit_one(&ctx, row).await {
                Ok(()) => { group_ok = true; break; }
                Err(err) => { last_err = Some(err); }
            }
        }
        let outcome = if group_ok { Ok(()) } else { Err(last_err.unwrap_or_default()) };
        record_outcome(outcome, idx, group.len(), &mut pass, &mut fail, &mut first_failures);
        progress.tick(idx + 1, pass, fail);
    }
    report_and_exit(expected_fqn, pass, fail, &first_failures)
}

struct Progress {
    fqn: String,
    total: usize,
    start: std::time::Instant,
    last: std::time::Instant,
    last_idx: usize,
}

impl Progress {
    fn new(fqn: &str, total: usize) -> Self {
        let now = std::time::Instant::now();
        Self {
            fqn: fqn.to_string(),
            total,
            start: now,
            last: now,
            last_idx: 0,
        }
    }

    fn tick(&mut self, current: usize, pass: usize, fail: usize) {
        let now = std::time::Instant::now();
        let since_last = now.duration_since(self.last);
        if since_last.as_secs() < 5 && current < self.total {
            return;
        }
        let elapsed = now.duration_since(self.start).as_secs_f64();
        let recent_rate = (current - self.last_idx) as f64 / since_last.as_secs_f64().max(1e-6);
        let avg_rate = current as f64 / elapsed.max(1e-6);
        let remaining = self.total.saturating_sub(current);
        let eta_secs = if avg_rate > 0.0 { (remaining as f64 / avg_rate) as u64 } else { 0 };
        eprintln!(
            "[{}] {}/{} groups ({:.1}%), pass={} fail={}, recent {:.0}/s avg {:.0}/s, elapsed {}, eta {}",
            self.fqn,
            current,
            self.total,
            100.0 * current as f64 / self.total.max(1) as f64,
            pass,
            fail,
            recent_rate,
            avg_rate,
            fmt_dur(elapsed as u64),
            fmt_dur(eta_secs),
        );
        self.last = now;
        self.last_idx = current;
    }
}

fn fmt_dur(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 { format!("{}h{:02}m{:02}s", h, m, s) }
    else if m > 0 { format!("{}m{:02}s", m, s) }
    else { format!("{}s", s) }
}

// Equivalence-class grouping: rows with identical `args` JSON form one
// group. A group passes when Rust matches any of its captured results.
fn group_by_args(rows: Vec<CapturedRow>) -> Vec<Vec<CapturedRow>> {
    use std::collections::HashMap;
    let mut by_args: HashMap<String, Vec<CapturedRow>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for row in rows {
        let key = serde_json::to_string(&row.args).expect("args serialize");
        match by_args.get_mut(&key) {
            Some(v) => v.push(row),
            None => {
                order.push(key.clone());
                by_args.insert(key, vec![row]);
            }
        }
    }
    order.into_iter().map(|k| by_args.remove(&k).unwrap()).collect()
}

fn run_group<F>(group: &[CapturedRow], mut audit_one: F) -> Result<(), String>
where
    F: FnMut(&CapturedRow) -> Result<(), String>,
{
    let mut last_err: Option<String> = None;
    for row in group {
        match audit_one(row) {
            Ok(()) => return Ok(()),
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err.unwrap_or_default())
}

fn record_outcome(
    outcome: Result<(), String>,
    group_idx: usize,
    group_size: usize,
    pass: &mut usize,
    fail: &mut usize,
    first_failures: &mut Vec<String>,
) {
    match outcome {
        Ok(()) => *pass += group_size,
        Err(err) => {
            *fail += group_size;
            if first_failures.len() < MAX_FIRST_FAILURES {
                first_failures.push(format!(
                    "[group {} (rows ×{})] {}",
                    group_idx + 1,
                    group_size,
                    err,
                ));
            }
        }
    }
}

fn report_and_exit(fqn: &str, pass: usize, fail: usize, first_failures: &[String]) -> ! {
    println!(
        "{} : pass={}, fail={}, total={}",
        fqn,
        pass,
        fail,
        pass + fail
    );
    for failure in first_failures {
        println!("  {}", failure);
    }
    std::process::exit(if fail == 0 { 0 } else { 1 });
}


// --- captured DAO mirrors --------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedKanaText {
    #[serde(rename = "_meta")]
    _meta: serde::de::IgnoredAny,
    pub id: Option<i32>,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    #[serde(deserialize_with = "deserialize_common")]
    pub common: Option<i32>,
    pub common_tags: String,
    #[serde(default, deserialize_with = "deserialize_null_as_false")]
    pub conjugate_p: bool,
    #[serde(default, deserialize_with = "deserialize_null_as_false")]
    pub nokanji: bool,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub best_kanji: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedKanjiText {
    #[serde(rename = "_meta")]
    _meta: serde::de::IgnoredAny,
    pub id: Option<i32>,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    #[serde(deserialize_with = "deserialize_common")]
    pub common: Option<i32>,
    pub common_tags: String,
    #[serde(default, deserialize_with = "deserialize_null_as_false")]
    pub conjugate_p: bool,
    #[serde(default, deserialize_with = "deserialize_null_as_false")]
    pub nokanji: bool,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub best_kana: Option<String>,
}

impl CapturedKanaText {
    pub fn matches(&self, actual: &KanaText) -> bool {
        let id_ok = self.id.map_or(true, |captured_id| actual.id == captured_id);
        id_ok
            && self.seq == actual.seq
            && self.text == actual.text
            && self.ord == actual.ord
            && self.common == actual.common
            && self.common_tags == actual.common_tags
            && self.conjugate_p == actual.conjugate_p
            && self.nokanji == actual.nokanji
            && self.best_kanji == actual.best_kanji
    }

    pub fn into_dao(self) -> KanaText {
        KanaText {
            id: self.id.unwrap_or(0),
            seq: self.seq,
            text: self.text,
            ord: self.ord,
            common: self.common,
            common_tags: self.common_tags,
            conjugate_p: self.conjugate_p,
            nokanji: self.nokanji,
            best_kanji: self.best_kanji,
            state: SimpleText::default(),
        }
    }
}

impl CapturedKanjiText {
    pub fn matches(&self, actual: &KanjiText) -> bool {
        let id_ok = self.id.map_or(true, |captured_id| actual.id == captured_id);
        id_ok
            && self.seq == actual.seq
            && self.text == actual.text
            && self.ord == actual.ord
            && self.common == actual.common
            && self.common_tags == actual.common_tags
            && self.conjugate_p == actual.conjugate_p
            && self.nokanji == actual.nokanji
            && self.best_kana == actual.best_kana
    }

    pub fn into_dao(self) -> KanjiText {
        KanjiText {
            id: self.id.unwrap_or(0),
            seq: self.seq,
            text: self.text,
            ord: self.ord,
            common: self.common,
            common_tags: self.common_tags,
            conjugate_p: self.conjugate_p,
            nokanji: self.nokanji,
            best_kana: self.best_kana,
            state: SimpleText::default(),
        }
    }
}


// --- input-side dispatchers -----------------------------------------------

/// Read `_meta.class` off a captured DAO/struct/class envelope.
pub fn captured_class(value: &Value) -> Result<&str, String> {
    value
        .pointer("/_meta/class")
        .and_then(|c| c.as_str())
        .ok_or_else(|| format!("missing _meta.class on: {}", value))
}

pub fn parse_captured_simple_text(value: &Value) -> Result<KaniSimpleTextDispatchEnum, String> {
    let class = captured_class(value)?;
    match class {
        "KANA-TEXT" => {
            let captured: CapturedKanaText = serde_json::from_value(value.clone())
                .map_err(|err| format!("kana-text parse: {}", err))?;
            Ok(KaniSimpleTextDispatchEnum::Kana(captured.into_dao()))
        }
        "KANJI-TEXT" => {
            let captured: CapturedKanjiText = serde_json::from_value(value.clone())
                .map_err(|err| format!("kanji-text parse: {}", err))?;
            Ok(KaniSimpleTextDispatchEnum::Kanji(captured.into_dao()))
        }
        "PROXY-TEXT" => Ok(KaniSimpleTextDispatchEnum::Proxy(parse_captured_proxy_text(value)?)),
        other => Err(format!("unsupported simple-text class: :{}", other)),
    }
}

pub fn parse_captured_proxy_text(value: &Value) -> Result<ProxyText, String> {
    let source_value = value
        .get("source")
        .ok_or_else(|| "proxy-text missing source".to_string())?;
    let source = parse_captured_simple_text(source_value)?;
    Ok(ProxyText {
        text: get_string(value, "text"),
        kana: get_string(value, "kana"),
        source: Box::new(source),
        state: SimpleText::default(),
    })
}

/// Full word-polymorphism dispatcher: KANA-TEXT, KANJI-TEXT, PROXY-TEXT,
/// COMPOUND-TEXT, plus the 11 counter-text subclasses.
pub fn parse_captured_word(value: &Value) -> Result<KaniWordDispatchEnum, String> {
    let class = captured_class(value)?;
    match class {
        "KANA-TEXT" | "KANJI-TEXT" | "PROXY-TEXT" => {
            Ok(match parse_captured_simple_text(value)? {
                KaniSimpleTextDispatchEnum::Kana(k) => KaniWordDispatchEnum::Kana(k),
                KaniSimpleTextDispatchEnum::Kanji(k) => KaniWordDispatchEnum::Kanji(k),
                KaniSimpleTextDispatchEnum::Proxy(p) => KaniWordDispatchEnum::Proxy(p),
            })
        }
        "COMPOUND-TEXT" => Ok(KaniWordDispatchEnum::Compound(parse_captured_compound_text(value)?)),
        c if c.starts_with("COUNTER-") || c == "NUMBER-TEXT" => {
            Ok(KaniWordDispatchEnum::Counter(parse_captured_counter(value, c)?))
        }
        other => Err(format!("unknown word class: :{}", other)),
    }
}

fn parse_captured_compound_text(value: &Value) -> Result<CompoundText, String> {
    let primary_value = value
        .get("primary")
        .ok_or_else(|| "compound-text missing primary".to_string())?;
    let primary = Box::new(parse_captured_word(primary_value)?);
    let mut words = Vec::new();
    if let Some(words_value) = value.get("words") {
        if let Some(arr) = words_value.as_array() {
            for w in arr {
                words.push(parse_captured_word(w)?);
            }
        }
    }
    Ok(CompoundText {
        text: get_string(value, "text"),
        kana: get_string(value, "kana"),
        primary,
        words,
        score_base: None,
        score_mod: ScoreMod::Single(0),
    })
}

fn parse_captured_counter(value: &Value, class: &str) -> Result<Counter, String> {
    let source = match value.get("source") {
        Some(s) if !s.is_null() => match parse_captured_simple_text(s)? {
            KaniSimpleTextDispatchEnum::Kanji(k) => Some(CounterSource::Kanji(k)),
            KaniSimpleTextDispatchEnum::Kana(k) => Some(CounterSource::Kana(k)),
            // counter-text source is a kanji or kana, never proxy upstream.
            KaniSimpleTextDispatchEnum::Proxy(_) => None,
        },
        _ => None,
    };
    let common = parse_counter_common(value.get("common"));
    let base = CounterText {
        text: get_string(value, "text"),
        kana: get_string(value, "kana"),
        number_text: get_string(value, "number_text"),
        number: value.get("number").and_then(|v| v.as_u64()).unwrap_or(0),
        source,
        ordinalp: get_bool(value, "ordinalp"),
        suffix: get_optional_string(value, "suffix"),
        accepts_suffixes: Vec::new(),
        suffix_descriptions: get_string_list(value, "suffix_descriptions"),
        digit_opts: Vec::new(),
        common,
        allowed: Vec::new(),
        foreign: get_bool(value, "foreign"),
    };
    Ok(match class {
        "COUNTER-TEXT" => Counter::Base(base),
        "NUMBER-TEXT" => Counter::NumberText(NumberText(base)),
        "COUNTER-TSU" => Counter::Tsu(CounterTsu(base)),
        "COUNTER-AGE" => Counter::Age(CounterAge(base)),
        "COUNTER-DAYS-KUN" => Counter::DaysKun(CounterDaysKun(base)),
        "COUNTER-DAYS-ON" => Counter::DaysOn(CounterDaysOn(base)),
        "COUNTER-HALFHOUR" => Counter::Halfhour(CounterHalfhour(base)),
        "COUNTER-MONTHS" => Counter::Months(CounterMonths(base)),
        "COUNTER-PEOPLE" => Counter::People(CounterPeople(base)),
        "COUNTER-WARI" => Counter::Wari(CounterWari(base)),
        "COUNTER-HIFUMI" => Counter::Hifumi(CounterHifumi { base, digit_set: Vec::new() }),
        other => return Err(format!("unknown counter subclass: :{}", other)),
    })
}

fn parse_counter_common(v: Option<&Value>) -> Common {
    match v {
        None | Some(Value::Null) => Common::Inherit,
        Some(Value::String(s)) if s == ":NULL" => Common::Null,
        Some(Value::Number(n)) => Common::Score(n.as_i64().unwrap_or(0) as i32),
        _ => Common::Inherit,
    }
}

pub fn get_string(value: &Value, key: &str) -> String {
    value.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default()
}

pub fn get_bool(value: &Value, key: &str) -> bool {
    value.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

pub fn get_optional_string(value: &Value, key: &str) -> Option<String> {
    match value.get(key) {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s == ":NULL" => None,
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

pub fn get_string_list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default()
}

/// Read an integer field; missing/null/non-int → 0. Matches the
/// `.unwrap_or(0) as i32` shape that several runners reimplement inline.
pub fn get_i32(value: &Value, key: &str) -> i32 {
    value.get(key).and_then(|v| v.as_i64()).unwrap_or(0) as i32
}

/// Read an unsigned integer field; missing/null/non-int → 0.
pub fn get_usize(value: &Value, key: &str) -> usize {
    value.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as usize
}

/// Assert that `result` carries exactly one captured value and return it.
/// Most encapsulated fns project as a single-element `(values v)` list;
/// this is the standard shape-check for those runners.
pub fn single_result(result: &[Value]) -> Result<&Value, String> {
    if result.len() != 1 {
        return Err(format!("expected 1 result value, got {}", result.len()));
    }
    Ok(&result[0])
}


// --- custom serde deserializers --------------------------------------------

fn deserialize_common<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(None),
        Value::String(ref s) if s == ":NULL" => Ok(None),
        Value::Number(ref n) => Ok(Some(
            n.as_i64()
                .ok_or_else(|| serde::de::Error::custom(format!("common not int: {}", n)))?
                as i32,
        )),
        other => Err(serde::de::Error::custom(format!(
            "common: expected int / null / \":NULL\", got {}",
            other
        ))),
    }
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s == ":NULL" => Ok(None),
        Value::String(s) => Ok(Some(s)),
        other => Err(serde::de::Error::custom(format!(
            "expected string / null / \":NULL\", got {}",
            other
        ))),
    }
}

fn deserialize_null_as_false<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<bool>::deserialize(deserializer)?;
    Ok(value.unwrap_or(false))
}
