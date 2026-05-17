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
use kaniran_core::dict::conj_data_struct::ConjData;
use kaniran_core::dict::conj_prop_dao::ConjProp;
use kaniran_core::dict::counter_age_class::CounterAge;
use kaniran_core::dict::counter_days_kun_class::CounterDaysKun;
use kaniran_core::dict::counter_days_on_class::CounterDaysOn;
use kaniran_core::dict::counter_halfhour_class::CounterHalfhour;
use kaniran_core::dict::counter_hifumi_class::CounterHifumi;
use kaniran_core::dict::counter_months_class::CounterMonths;
use kaniran_core::dict::counter_people_class::CounterPeople;
use kaniran_core::dict::counter_text_class::{
    Common, Counter, CounterSource, CounterText, DigitOp, DigitOptEntry, DigitOptKey,
};
use kaniran_core::dict::counter_tsu_class::CounterTsu;
use kaniran_core::dict::counter_wari_class::CounterWari;
use kaniran_core::dict::entry_dao::Entry;
use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kani_suffix_kind::SuffixKind;
use kaniran_core::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::kanji_text_dao::KanjiText;
use kaniran_core::dict::number_text_class::NumberText;
use kaniran_core::dict::proxy_text_class::ProxyText;
use kaniran_core::dict::segment_list_struct::SegmentList;
use kaniran_core::dict::segment_struct::{KaniScoreInfo, KaniSplitInfo, Segment};
use kaniran_core::dict::simple_text_class::{SimpleText, WordConjugations};
use kaniran_core::dict::synergy_struct::Synergy;
use kaniran_core::dict::top_array_item_struct::PathElement;
use kaniran_core::dict::word_info_class::{WordInfo, WordInfoKana, WordInfoSeq, WordInfoType};


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

    let total_rows = builder.metadata().file_metadata().num_rows() as usize;
    let reader = builder.build().expect("build reader");
    let mut rows: Vec<CapturedRow> = Vec::with_capacity(total_rows);
    let mut row_seq: usize = 0;
    let mut skipped: usize = 0;
    let start = std::time::Instant::now();
    let mut last_tick = start;
    eprintln!("loader: parsing {} rows from {:?}", total_rows, path);

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
        let now = std::time::Instant::now();
        if now.duration_since(last_tick).as_secs() >= 5 {
            let elapsed = now.duration_since(start).as_secs_f64();
            let rate = row_seq as f64 / elapsed.max(1e-6);
            let pct = if total_rows > 0 {
                100.0 * row_seq as f64 / total_rows as f64
            } else {
                0.0
            };
            let eta = if rate > 0.0 && total_rows > row_seq {
                (total_rows - row_seq) as f64 / rate
            } else {
                0.0
            };
            eprintln!(
                "loader: {}/{} rows ({:.1}%), {:.0}/s, elapsed {}, eta {}",
                row_seq,
                total_rows,
                pct,
                rate,
                fmt_dur(elapsed as u64),
                fmt_dur(eta as u64),
            );
            last_tick = now;
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    eprintln!(
        "loader: parsed {} rows in {} ({:.0}/s)",
        row_seq,
        fmt_dur(elapsed as u64),
        row_seq as f64 / elapsed.max(1e-6),
    );
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
/// Bounded concurrency. The default sqlx pool is `max_connections=10`;
/// keep one connection in reserve for non-query work.
const ASYNC_CONCURRENCY: usize = 8;

pub async fn run_async<F>(expected_fqn: &str, audit_one: F) -> !
where
    F: AsyncFn(&KaniranContext, &CapturedRow) -> Result<(), String>,
{
    use futures::stream::StreamExt;

    let path = parse_path_arg();
    let file = load_parquet(&path);
    let groups = group_by_args(file.rows);
    let total = groups.len();

    let ctx = setup_ctx().await;
    let ctx_ref: &KaniranContext = &ctx;
    let audit_one = &audit_one;

    let stream = futures::stream::iter(groups.into_iter().enumerate().map(
        |(idx, group)| async move {
            let size = group.len();
            let mut last_err: Option<String> = None;
            let mut group_ok = false;
            for row in &group {
                match audit_one(ctx_ref, row).await {
                    Ok(()) => { group_ok = true; break; }
                    Err(err) => { last_err = Some(err); }
                }
            }
            let outcome = if group_ok { Ok(()) } else { Err(last_err.unwrap_or_default()) };
            (idx, size, outcome)
        },
    ))
    .buffer_unordered(ASYNC_CONCURRENCY);

    let mut pass: usize = 0;
    let mut fail: usize = 0;
    let mut first_failures: Vec<String> = Vec::new();
    let mut progress = Progress::new(expected_fqn, total);
    let mut done: usize = 0;
    let mut stream = Box::pin(stream);
    while let Some((idx, size, outcome)) = stream.next().await {
        record_outcome(outcome, idx, size, &mut pass, &mut fail, &mut first_failures);
        done += 1;
        progress.tick(done, pass, fail);
    }
    report_and_exit(expected_fqn, pass, fail, &first_failures)
}

/// Streaming async runner: reads parquet batches lazily, parses each
/// batch into [`CapturedRow`]s, and runs `audit_one` per row with
/// bounded concurrency. Skips the [`group_by_args`] dedup-by-args
/// pass — every row is treated as an independent unit, so this is
/// suitable for deterministic functions on multi-million-row corpora
/// where buffering the whole parquet would OOM.
///
/// Each captured row is checked exactly once against the produced
/// output (no equivalence classes). For nondeterministic captures
/// (e.g. `GET-SPLIT`), use [`run_async`] instead.
pub async fn run_async_streaming<F>(expected_fqn: &str, audit_one: F) -> !
where
    F: AsyncFn(&KaniranContext, &CapturedRow) -> Result<(), String>,
{
    use futures::stream::StreamExt;

    let path = parse_path_arg();
    let file = std::fs::File::open(&path)
        .unwrap_or_else(|err| panic!("open {:?}: {}", path, err));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap_or_else(|err| panic!("parquet builder {:?}: {}", path, err));
    let total_rows = builder.metadata().file_metadata().num_rows() as usize;
    eprintln!("streaming: {} rows from {:?}", total_rows, path);
    let reader = builder.build().expect("build reader");

    let ctx = setup_ctx().await;
    let ctx_ref: &KaniranContext = &ctx;
    let audit_one = &audit_one;

    let mut pass: usize = 0;
    let mut fail: usize = 0;
    let mut skipped: usize = 0;
    let mut first_failures: Vec<String> = Vec::new();
    let mut progress = Progress::new(expected_fqn, total_rows);
    let mut done: usize = 0;
    let mut row_seq: usize = 0;

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

        let mut rows: Vec<CapturedRow> = Vec::with_capacity(batch.num_rows());
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

        let outcomes: Vec<(usize, Result<(), String>)> = futures::stream::iter(
            rows.iter().enumerate().map(|(local_idx, row)| async move {
                let outcome = audit_one(ctx_ref, row).await;
                (local_idx, outcome)
            }),
        )
        .buffer_unordered(ASYNC_CONCURRENCY)
        .collect()
        .await;

        let batch_done_start = done;
        for (local_idx, outcome) in outcomes {
            let global_idx = batch_done_start + local_idx;
            match outcome {
                Ok(()) => pass += 1,
                Err(err) => {
                    fail += 1;
                    if first_failures.len() < MAX_FIRST_FAILURES {
                        first_failures.push(format!("[row {}] {}", global_idx + 1, err));
                    }
                }
            }
        }
        done += rows.len();
        progress.tick(done, pass, fail);
    }

    if skipped > 0 {
        eprintln!("loader: {} row(s) skipped (malformed JSON from extractor)", skipped);
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
    let total = rows.len();
    eprintln!("grouping: {} rows by args", total);
    let start = std::time::Instant::now();
    let mut last_tick = start;
    let mut by_args: HashMap<String, Vec<CapturedRow>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for (idx, row) in rows.into_iter().enumerate() {
        let key = serde_json::to_string(&row.args).expect("args serialize");
        match by_args.get_mut(&key) {
            Some(v) => v.push(row),
            None => {
                order.push(key.clone());
                by_args.insert(key, vec![row]);
            }
        }
        let now = std::time::Instant::now();
        if now.duration_since(last_tick).as_secs() >= 5 {
            let elapsed = now.duration_since(start).as_secs_f64();
            let done = idx + 1;
            let rate = done as f64 / elapsed.max(1e-6);
            let pct = 100.0 * done as f64 / total.max(1) as f64;
            let eta = if rate > 0.0 && total > done {
                (total - done) as f64 / rate
            } else {
                0.0
            };
            eprintln!(
                "grouping: {}/{} rows ({:.1}%), {} unique args, {:.0}/s, elapsed {}, eta {}",
                done,
                total,
                pct,
                order.len(),
                rate,
                fmt_dur(elapsed as u64),
                fmt_dur(eta as u64),
            );
            last_tick = now;
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    eprintln!(
        "grouping: {} rows -> {} groups in {} ({:.0}/s)",
        total,
        order.len(),
        fmt_dur(elapsed as u64),
        total as f64 / elapsed.max(1e-6),
    );
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
    /// `kana-text.id` primary key. Present as an integer when the
    /// captured row came from the database (DAO load); present as
    /// JSON `null` when the captured row was synthesized at runtime
    /// (counter-text sources, proxy-text inner readings, transient
    /// segmenter assemblies) and was never persisted. No audited
    /// impl path reads `id`, so `None` flows through to the produced
    /// `KanaText` as `0` without affecting outcomes.
    pub id: Option<i32>,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    #[serde(deserialize_with = "deserialize_common")]
    pub common: Option<i32>,
    pub common_tags: String,
    #[serde(deserialize_with = "deserialize_null_as_false")]
    pub conjugate_p: bool,
    #[serde(deserialize_with = "deserialize_null_as_false")]
    pub nokanji: bool,
    #[serde(deserialize_with = "deserialize_optional_string")]
    pub best_kanji: Option<String>,
    /// `simple-text.conjugations` slot — emitted on every row since
    /// the projector stopped omitting the slot 2026-05-12. Pre-
    /// 2026-05-12 parquets are no longer accepted (this struct's
    /// `deny_unknown_fields` + missing-field-is-error would reject
    /// them); re-extract those corpora against the current projector.
    #[serde(deserialize_with = "deserialize_conjugations")]
    pub conjugations: Option<WordConjugations>,
    /// `simple-text.hintedp` slot. Load-bearing for the `:around
    /// get-kana` method — proxy-text built by abbr-suffix patterns
    /// (`dict-grammar.lisp:574`) is hintedp=T at runtime and bypasses
    /// the hint branch. Emitted on every row since 2026-05-13; older
    /// parquets must be re-extracted.
    #[serde(deserialize_with = "deserialize_null_as_false")]
    pub hintedp: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedKanjiText {
    #[serde(rename = "_meta")]
    _meta: serde::de::IgnoredAny,
    /// `kanji-text.id` PK — see [`CapturedKanaText::id`] for the
    /// runtime-synthesized null-id case.
    pub id: Option<i32>,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    #[serde(deserialize_with = "deserialize_common")]
    pub common: Option<i32>,
    pub common_tags: String,
    #[serde(deserialize_with = "deserialize_null_as_false")]
    pub conjugate_p: bool,
    #[serde(deserialize_with = "deserialize_null_as_false")]
    pub nokanji: bool,
    #[serde(deserialize_with = "deserialize_optional_string")]
    pub best_kana: Option<String>,
    /// `simple-text.conjugations` slot — see [`CapturedKanaText::conjugations`].
    #[serde(deserialize_with = "deserialize_conjugations")]
    pub conjugations: Option<WordConjugations>,
    /// `simple-text.hintedp` slot — see [`CapturedKanaText::hintedp`].
    #[serde(deserialize_with = "deserialize_null_as_false")]
    pub hintedp: bool,
}

impl CapturedKanaText {
    pub fn matches(&self, actual: &KanaText) -> bool {
        // Synthesized (non-DB) rows carry `id: None`; in that case
        // the audit doesn't compare ids (the runtime object has no
        // meaningful id to match against). DB-loaded rows compare
        // strictly.
        let id_ok = match self.id {
            Some(captured_id) => captured_id == actual.id,
            None => true,
        };
        id_ok
            && self.seq == actual.seq
            && self.text == actual.text
            && self.ord == actual.ord
            && self.common == actual.common
            && self.common_tags == actual.common_tags
            && self.conjugate_p == actual.conjugate_p
            && self.nokanji == actual.nokanji
            && self.best_kanji == actual.best_kanji
            && self.conjugations == actual.state.conjugations
            && self.hintedp == actual.state.hintedp
    }

    pub fn into_dao(self) -> KanaText {
        KanaText {
            // Synthesized rows have no id (upstream `make-instance`
            // path silently drops `:id` because the slot has no
            // `:initarg`). 0 is a sentinel; no audited code path
            // reads it.
            id: self.id.unwrap_or(0),
            seq: self.seq,
            text: self.text,
            ord: self.ord,
            common: self.common,
            common_tags: self.common_tags,
            conjugate_p: self.conjugate_p,
            nokanji: self.nokanji,
            best_kanji: self.best_kanji,
            state: SimpleText {
                conjugations: self.conjugations,
                hintedp: self.hintedp,
            },
        }
    }
}

impl CapturedKanjiText {
    pub fn matches(&self, actual: &KanjiText) -> bool {
        // See [`CapturedKanaText::matches`] for the None-id semantics.
        let id_ok = match self.id {
            Some(captured_id) => captured_id == actual.id,
            None => true,
        };
        id_ok
            && self.seq == actual.seq
            && self.text == actual.text
            && self.ord == actual.ord
            && self.common == actual.common
            && self.common_tags == actual.common_tags
            && self.conjugate_p == actual.conjugate_p
            && self.nokanji == actual.nokanji
            && self.best_kana == actual.best_kana
            && self.conjugations == actual.state.conjugations
            && self.hintedp == actual.state.hintedp
    }

    pub fn into_dao(self) -> KanjiText {
        KanjiText {
            // Synthesized rows have no id; see [`CapturedKanaText::into_dao`].
            id: self.id.unwrap_or(0),
            seq: self.seq,
            text: self.text,
            ord: self.ord,
            common: self.common,
            common_tags: self.common_tags,
            conjugate_p: self.conjugate_p,
            nokanji: self.nokanji,
            best_kana: self.best_kana,
            state: SimpleText {
                conjugations: self.conjugations,
                hintedp: self.hintedp,
            },
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
    let source_value = require_field(value, "source")?;
    let source = parse_captured_simple_text(source_value)?;
    Ok(ProxyText {
        text: require_string(value, "text")?,
        kana: require_string(value, "kana")?,
        source: Box::new(source),
        state: SimpleText {
            // proxy-text inherits both simple-text slots
            // (`dict.lisp:550 (defclass proxy-text (simple-text))`).
            // The projector emits both on every row; strict reads.
            hintedp: require_bool(value, "hintedp")?,
            conjugations: require_conjugations(value, "conjugations")?,
        },
    })
}

/// Full word-polymorphism dispatcher: KANA-TEXT, KANJI-TEXT, PROXY-TEXT,
/// COMPOUND-TEXT, and the 11 counter-text subclasses. ENTRY is **not**
/// in the dispatch — no upstream callsite routes an entry through
/// polymorphic dispatch; locally-typed Entry callers invoke
/// `Entry::get_text(ctx)` / `Entry::get_kana(ctx)` directly.
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

/// Read an ENTRY-shaped capture into an [`Entry`]. Used only by
/// runners that consume entry rows directly (none in the current
/// audit set). The projector's `*omit-slots*` blocks `content`
/// (JMdict XML, kilobytes per row, no audit consumer reads it);
/// every other slot is strict-required.
#[allow(dead_code)]
fn parse_captured_entry(value: &Value) -> Result<Entry, String> {
    Ok(Entry {
        seq: require_i32(value, "seq")?,
        // `content` is intentionally omitted by the projector;
        // empty-string is the audit-side substitute and no consumer
        // reads it. The omission is the single documented exception
        // to the capture-everything rule.
        content: String::new(),
        root_p: require_bool(value, "root_p")?,
        n_kanji: require_i32(value, "n_kanji")?,
        n_kana: require_i32(value, "n_kana")?,
        primary_nokanji: require_bool(value, "primary_nokanji")?,
    })
}

fn parse_captured_compound_text(value: &Value) -> Result<CompoundText, String> {
    let primary_value = require_field(value, "primary")?;
    let primary = Box::new(parse_captured_word(primary_value)?);
    let words_value = require_field(value, "words")?;
    let words_arr = words_value
        .as_array()
        .ok_or_else(|| format!("words: expected array, got {}", words_value))?;
    let mut words = Vec::with_capacity(words_arr.len());
    for w in words_arr {
        words.push(parse_captured_word(w)?);
    }
    let score_base = match require_field(value, "score_base")? {
        Value::Null => None,
        v => Some(Box::new(parse_captured_word(v)?)),
    };
    let score_mod = parse_captured_score_mod(require_field(value, "score_mod")?)?;
    Ok(CompoundText {
        text: require_string(value, "text")?,
        kana: require_string(value, "kana")?,
        primary,
        words,
        score_base,
        score_mod,
    })
}

/// Read the `score-mod` slot. Upstream `adjoin-word` writes either an
/// integer literal or a `(constantly N)` closure at first adjoin
/// (`dict.lisp:642-645`); subsequent adjoins wrap-or-cons into a flat
/// list whose elements are themselves either integers or
/// `(constantly N)` closures (`dict.lisp:648-651`). Mirror that:
/// - JSON integer → [`ScoreMod::Single`].
/// - JSON array of integers → [`ScoreMod::Stack`] of `Single`
///   elements.
/// - JSON `null` → [`ScoreMod::Single(0)`] (the `:around` method's
///   `(or score-mod 0)` fallback at `dict.lisp:639`).
///
/// ## Audit blackout: four suffixes are unreachable today
///
/// Compounds produced by `suffix-kudasai` (`dict-grammar.lisp:404`),
/// `suffix-sou` (`dict-grammar.lisp:448`), `suffix-desu`
/// (`dict-grammar.lisp:516`), and `suffix-desho`
/// (`dict-grammar.lisp:532`) carry a `(constantly N)` closure in
/// their `score-mod` slot. The JSON projector at
/// `ichiran-extractor/projectors_json.lisp` does not yet have a clause
/// for `function`-typed slot values, so captures of those compounds
/// either skip (projector elides the slot) or land here as a non-numeric
/// JSON value. The parser errors out below with `score_mod: function
/// closures not yet representable in capture JSON …` to surface the
/// gap loudly rather than collapsing a function to an integer.
///
/// Fixing this end-to-end requires:
/// 1. Teach the projector to emit `{"kind":"constantly","value":N}`
///    for `(functionp slot)` values — match on the closure's source
///    form to recover the literal `N`, or fall back to
///    `(funcall slot 0)` (which is well-defined for `constantly` and
///    arbitrary for other closures — protect with a
///    `(if (and (functionp slot) (constantly-form-p slot)) …)` guard
///    once an introspection helper exists).
/// 2. Extend the match below with an `Value::Object` arm matching
///    `{"kind":"constantly", "value": N}` → [`ScoreMod::Constant`].
fn parse_captured_score_mod(v: &Value) -> Result<ScoreMod, String> {
    match v {
        Value::Null => Ok(ScoreMod::Single(0)),
        Value::Number(n) => {
            let i = n.as_i64().ok_or_else(|| format!("score_mod not i64: {}", n))?;
            Ok(ScoreMod::Single(i))
        }
        Value::Array(arr) => {
            let mut items = Vec::with_capacity(arr.len());
            for item in arr {
                let i = item.as_i64().ok_or_else(|| {
                    format!(
                        "score_mod entry not i64 (likely a (constantly N) closure from \
                         suffix-kudasai/sou/desu/desho — projector clause for function-\
                         typed slots is missing; see parse_captured_score_mod doc): {}",
                        item
                    )
                })?;
                items.push(ScoreMod::Single(i));
            }
            Ok(ScoreMod::Stack(items))
        }
        other => Err(format!(
            "score_mod: function closures not yet representable in capture JSON — got \
             {} (expected int / array of int / null; see parse_captured_score_mod doc \
             for the four-suffix audit blackout)",
            other
        )),
    }
}

fn parse_captured_counter(value: &Value, class: &str) -> Result<Counter, String> {
    let source = match require_field(value, "source")? {
        Value::Null => None,
        s => match parse_captured_simple_text(s)? {
            KaniSimpleTextDispatchEnum::Kanji(k) => Some(CounterSource::Kanji(k)),
            KaniSimpleTextDispatchEnum::Kana(k) => Some(CounterSource::Kana(k)),
            // counter-text source is a kanji or kana, never proxy upstream.
            KaniSimpleTextDispatchEnum::Proxy(_) => None,
        },
    };
    let common = parse_counter_common(require_field(value, "common")?)?;
    let base = CounterText {
        text: require_string(value, "text")?,
        kana: require_string(value, "kana")?,
        number_text: require_string(value, "number_text")?,
        number: require_u64(value, "number")?,
        source,
        ordinalp: require_bool(value, "ordinalp")?,
        suffix: require_optional_string(value, "suffix")?,
        accepts_suffixes: parse_accepts_suffixes(require_field(value, "accepts_suffixes")?)?,
        suffix_descriptions: require_string_list(value, "suffix_descriptions")?,
        digit_opts: parse_digit_opts(require_field(value, "digit_opts")?)?,
        common,
        allowed: require_i32_list(value, "allowed")?,
        foreign: require_bool(value, "foreign")?,
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
        "COUNTER-HIFUMI" => Counter::Hifumi(CounterHifumi {
            base,
            // Load-bearing for COUNTER-HIFUMI get-kana: keys on
            // `(find value digit-set)`. Empty would silently push
            // every capture into the call-next-method fallback path.
            digit_set: require_i32_list(value, "digit_set")?,
        }),
        other => return Err(format!("unknown counter subclass: :{}", other)),
    })
}

/// Read `accepts_suffixes` — a JSON array of `:KW` keyword strings
/// like `":KAN"`, `":CHUU"`, `":KANGO"`. The Lisp slot value comes
/// from the populator's `:accepts` initarg
/// (`dict-counters.lisp:256`); only Kan / Kango / Chuu are defined
/// in the `*counter-suffixes*` registry. Unknown tags surface as
/// an error rather than silent drop so a future upstream addition
/// triggers a port update. `null` is the empty-list wire shape.
fn parse_accepts_suffixes(v: &Value) -> Result<Vec<SuffixKind>, String> {
    let arr = match v {
        Value::Null => return Ok(Vec::new()),
        Value::Array(arr) => arr,
        other => return Err(format!("accepts_suffixes: expected array / null, got {}", other)),
    };
    arr.iter()
        .map(|item| match item.as_str() {
            Some(":KAN") => Ok(SuffixKind::Kan),
            Some(":KANGO") => Ok(SuffixKind::Kango),
            Some(":CHUU") => Ok(SuffixKind::Chuu),
            Some(other) => Err(format!("unknown accepts tag: {}", other)),
            None => Err(format!("accepts entry not string: {}", item)),
        })
        .collect()
}

/// Read `digit_opts` — a JSON array of `[<key>, <op>, <op>, ...]`
/// inner arrays. The key head is either an integer (→
/// [`DigitOptKey::Digit`]) or the literal `":OFF"` (→ [`DigitOptKey::Off`]);
/// rest elements are strings (`":G"` / `":R"` / `":H"` / `":C"` →
/// closed [`DigitOp`] variants, or any other string → a literal
/// kana replacement via [`DigitOp::Replace`]). Mirrors the Lisp
/// `:digit-opts '((4 "よ") (10 :g))` shape captured by the
/// projector.
fn parse_digit_opts(v: &Value) -> Result<Vec<DigitOptEntry>, String> {
    let arr = match v {
        Value::Null => return Ok(Vec::new()),
        Value::Array(arr) => arr,
        other => return Err(format!("digit_opts: expected array / null, got {}", other)),
    };
    arr.iter()
        .map(|entry| {
            let inner = entry
                .as_array()
                .ok_or_else(|| format!("digit_opts entry not array: {}", entry))?;
            let mut iter = inner.iter();
            let head = iter
                .next()
                .ok_or_else(|| "digit_opts entry empty".to_string())?;
            let key = match head {
                Value::String(s) if s == ":OFF" => DigitOptKey::Off,
                Value::Number(n) => {
                    let i = n.as_i64().ok_or_else(|| {
                        format!("digit_opts head not int: {}", n)
                    })?;
                    DigitOptKey::Digit(i as i32)
                }
                other => {
                    return Err(format!("digit_opts head not :OFF/int: {}", other))
                }
            };
            let ops: Result<Vec<DigitOp>, String> = iter
                .map(|op| match op {
                    Value::String(s) => Ok(match s.as_str() {
                        ":G" => DigitOp::Geminate,
                        ":R" => DigitOp::Rendaku,
                        ":H" => DigitOp::Handakuten,
                        ":C" => DigitOp::Counter,
                        other => DigitOp::Replace(other.to_string()),
                    }),
                    other => Err(format!("digit_opts op not string: {}", other)),
                })
                .collect();
            Ok(DigitOptEntry { key, ops: ops? })
        })
        .collect()
}

/// Read a JSON array of integers; null / missing → empty Vec.
fn get_i32_list(value: &Value, key: &str) -> Vec<i32> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64().map(|n| n as i32))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_counter_common(v: &Value) -> Result<Common, String> {
    match v {
        // CL `nil` flattens to JSON `null` — the upstream `common`
        // slot's default. Maps to [`Common::Inherit`].
        Value::Null => Ok(Common::Inherit),
        Value::String(s) if s == ":NULL" => Ok(Common::Null),
        Value::Number(n) => {
            let i = n
                .as_i64()
                .ok_or_else(|| format!("common: not i64-representable: {}", n))?;
            Ok(Common::Score(i as i32))
        }
        other => Err(format!("common: expected null / number / \":NULL\", got {}", other)),
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


// --- strict field readers --------------------------------------------------
//
// Required-field counterparts of the `get_*` helpers above. Use these in
// `parse_captured_*` paths where the projector's contract guarantees the
// slot is emitted on every row. Missing key, wrong JSON type, or null
// (where null is not a valid wire value) returns a descriptive error
// instead of silently defaulting — matching the strict-field policy on
// `CapturedKanaText` / `CapturedKanjiText`.

fn require_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing field: {}", key))
}

fn require_string(value: &Value, key: &str) -> Result<String, String> {
    match require_field(value, key)? {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!("{}: expected string, got {}", key, other)),
    }
}

/// CL-truthy read of a "boolean" slot. The upstream `dao-class`
/// boolean slots (`:col-type boolean`) round-trip through select-dao
/// as `t`/`nil` and project as JSON `true`/`null`. But slots set
/// only at make-instance time can receive any truthy CL value —
/// e.g. `counter-text.foreign` is bound to `(find seq *counter-foreign*)`
/// which returns the seq integer when found, not `t`. Every reader
/// upstream uses these slots as predicates (`(when (counter-foreign obj) ...)`),
/// so any non-nil JSON value (number, string, true, array) collapses
/// to `true` here. `null` and `false` are `false`.
fn require_bool(value: &Value, key: &str) -> Result<bool, String> {
    match require_field(value, key)? {
        Value::Null => Ok(false),
        Value::Bool(b) => Ok(*b),
        _ => Ok(true),
    }
}

fn require_i32(value: &Value, key: &str) -> Result<i32, String> {
    match require_field(value, key)? {
        Value::Number(n) => {
            let i = n.as_i64().ok_or_else(|| format!("{}: not i64-representable: {}", key, n))?;
            Ok(i as i32)
        }
        other => Err(format!("{}: expected number, got {}", key, other)),
    }
}

fn require_u64(value: &Value, key: &str) -> Result<u64, String> {
    match require_field(value, key)? {
        Value::Number(n) => n.as_u64().ok_or_else(|| format!("{}: not u64-representable: {}", key, n)),
        other => Err(format!("{}: expected non-negative number, got {}", key, other)),
    }
}

fn require_i32_list(value: &Value, key: &str) -> Result<Vec<i32>, String> {
    let arr = match require_field(value, key)? {
        Value::Array(arr) => arr,
        Value::Null => return Ok(Vec::new()),
        other => return Err(format!("{}: expected array / null, got {}", key, other)),
    };
    arr.iter()
        .map(|v| match v {
            Value::Number(n) => {
                let i = n.as_i64().ok_or_else(|| format!("{}: entry not i64-representable: {}", key, n))?;
                Ok(i as i32)
            }
            other => Err(format!("{}: entry expected number, got {}", key, other)),
        })
        .collect()
}

fn require_string_list(value: &Value, key: &str) -> Result<Vec<String>, String> {
    let arr = match require_field(value, key)? {
        Value::Array(arr) => arr,
        Value::Null => return Ok(Vec::new()),
        other => return Err(format!("{}: expected array / null, got {}", key, other)),
    };
    arr.iter()
        .map(|v| match v {
            Value::String(s) => Ok(s.clone()),
            other => Err(format!("{}: entry expected string, got {}", key, other)),
        })
        .collect()
}

fn require_optional_string(value: &Value, key: &str) -> Result<Option<String>, String> {
    match require_field(value, key)? {
        Value::Null => Ok(None),
        Value::String(s) if s == ":NULL" => Ok(None),
        Value::String(s) => Ok(Some(s.clone())),
        other => Err(format!("{}: expected string / null / \":NULL\", got {}", key, other)),
    }
}

/// `simple-text.conjugations` slot — same wire shape as
/// [`deserialize_conjugations`] but for hand-written parsers
/// (proxy-text, where slot reads are direct JSON probes).
fn require_conjugations(value: &Value, key: &str) -> Result<Option<WordConjugations>, String> {
    match require_field(value, key)? {
        Value::Null => Ok(None),
        Value::String(s) if s == ":ROOT" => Ok(Some(WordConjugations::Root)),
        Value::String(other) => Err(format!("{}: unexpected string: {}", key, other)),
        Value::Array(arr) => {
            let mut ids = Vec::with_capacity(arr.len());
            for v in arr {
                let i = v.as_i64().ok_or_else(|| {
                    format!("{}: conj-id entry not i64-representable: {}", key, v)
                })?;
                ids.push(i as i32);
            }
            Ok(Some(WordConjugations::Ids(ids)))
        }
        other => Err(format!("{}: expected null / \":ROOT\" / int-array, got {}", key, other)),
    }
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

/// Short bisectable description of a captured word for failure
/// messages. Format: `<CLASS> seq=<seq> text=<excerpt>` with the
/// text excerpt truncated to ~24 chars so a divergent row in a
/// 100K-group run can be located without re-running the parquet.
/// Counter and compound carry no single `text` of their own so
/// surface only class + seq; entry surfaces only class + seq for
/// the same reason (no `text` slot upstream).
pub fn describe_word(word: &KaniWordDispatchEnum) -> String {
    fn excerpt(s: &str) -> String {
        let max_chars = 24;
        let chars: Vec<char> = s.chars().collect();
        if chars.len() <= max_chars {
            s.to_string()
        } else {
            let head: String = chars.iter().take(max_chars).collect();
            format!("{}\u{2026}", head)
        }
    }
    match word {
        KaniWordDispatchEnum::Kana(k) => {
            format!("KANA-TEXT seq={} text={:?}", k.seq, excerpt(&k.text))
        }
        KaniWordDispatchEnum::Kanji(k) => {
            format!("KANJI-TEXT seq={} text={:?}", k.seq, excerpt(&k.text))
        }
        KaniWordDispatchEnum::Proxy(p) => {
            format!("PROXY-TEXT text={:?} kana={:?}", excerpt(&p.text), excerpt(&p.kana))
        }
        KaniWordDispatchEnum::Compound(c) => {
            format!("COMPOUND-TEXT text={:?} kana={:?}", excerpt(&c.text), excerpt(&c.kana))
        }
        KaniWordDispatchEnum::Counter(c) => {
            let base = c.base();
            format!("COUNTER number={} text={:?}", base.number, excerpt(&base.text))
        }
    }
}

/// Detect the trailing `{"_meta":{"context":{"disable_hints":<bool>}}}`
/// element appended by `flatten-args-json-with-hint-state` (the
/// projector wired for `GET-KANA`, `GET-HINT`, `TRUE-KANA` in
/// `ichiran-extractor/projectors_json.lisp`). Returns the binding
/// value when the trailing element matches the meta-context shape.
/// Runners that consume hint-cluster parquets pass the returned
/// value into the impl's `disable_hints: bool` parameter so the
/// replay follows the same `:around` branch the upstream took at
/// capture; on `None`, those runners surface a hard error (the
/// meta element is required, never optional).
///
/// JSON shapes seen from the upstream projector:
/// - `true` ← Lisp `*disable-hints* = T` (set by the `:around`
///   method before invoking get-hint).
/// - `null` ← Lisp `*disable-hints* = NIL` (default; the projector
///   emits `(if disable-hints t nil)`, and `nil` flattens to JSON
///   `null` per the general projector contract). Equivalent to
///   `false` for the downstream impl.
/// - `false` ← would arise if the projector ever switched to an
///   explicit boolean encoder; tolerated for forward compatibility.
pub fn extract_disable_hints_meta(value: &Value) -> Option<bool> {
    match value.pointer("/_meta/context/disable_hints") {
        Some(Value::Bool(b)) => Some(*b),
        Some(Value::Null) => Some(false),
        _ => None,
    }
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

/// Project the `simple-text.conjugations` slot back into Rust. Wire
/// shape: `null` (slot is `nil`), `":ROOT"` (the symbol `:root`), or a
/// JSON array of integers (list of conjugation row ids).
fn deserialize_conjugations<'de, D>(deserializer: D) -> Result<Option<WordConjugations>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(None),
        Value::String(ref s) if s == ":ROOT" => Ok(Some(WordConjugations::Root)),
        Value::Array(arr) => {
            let mut ids = Vec::with_capacity(arr.len());
            for v in &arr {
                let id = v.as_i64().ok_or_else(|| {
                    serde::de::Error::custom(format!("conj-id not int: {}", v))
                })?;
                ids.push(id as i32);
            }
            Ok(Some(WordConjugations::Ids(ids)))
        }
        other => Err(serde::de::Error::custom(format!(
            "conjugations: expected null / \":ROOT\" / int-array, got {}",
            other
        ))),
    }
}


// --- segment / segment-list / synergy / path-element parsers ----------------

/// Parse a captured SEGMENT JSON into a [`Segment`]. The projector
/// captures every slot the segment-list / find-best-path machinery
/// produces (`start`, `end`, `word`, `score`, `info`, `text`) plus the
/// nested word; `top` is transient inside find-best-path and is not
/// captured. The `info` plist is captured but the audit runners that
/// consume segments (word_info_from_segment / fill_segment_path) do
/// not read it, so it deserializes to [`None`] here without loss.
pub fn parse_captured_segment(value: &Value) -> Result<Segment, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT" {
        return Err(format!("expected SEGMENT class, got :{}", class));
    }
    let start = require_i32(value, "start")? as usize;
    let end = require_i32(value, "end")? as usize;
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
    Ok(Segment {
        start,
        end,
        word,
        score,
        info: None,
        top: None,
        text,
    })
}

/// Parse a captured SEGMENT-LIST JSON into a [`SegmentList`]. As with
/// [`parse_captured_segment`], `top` is transient and not captured;
/// it deserializes to [`None`] without loss.
pub fn parse_captured_segment_list(value: &Value) -> Result<SegmentList, String> {
    let class = captured_class(value)?;
    if class != "SEGMENT-LIST" {
        return Err(format!("expected SEGMENT-LIST class, got :{}", class));
    }
    let segments_value = require_field(value, "segments")?;
    let segments_arr = segments_value
        .as_array()
        .ok_or_else(|| format!("segments: expected array, got {}", segments_value))?;
    let mut segments = Vec::with_capacity(segments_arr.len());
    for s in segments_arr {
        segments.push(parse_captured_segment(s)?);
    }
    let start = require_i32(value, "start")? as usize;
    let end = require_i32(value, "end")? as usize;
    let matches = require_i32(value, "matches")? as usize;
    Ok(SegmentList {
        segments,
        start,
        end,
        top: None,
        matches,
    })
}

/// Parse a captured SYNERGY JSON into a [`Synergy`]. `description` and
/// `connector` are `Option<String>` — the upstream `defstruct synergy`
/// has no `:initform` on either slot, and the wi-path corpus contains
/// rows where one or both are nil.
pub fn parse_captured_synergy(value: &Value) -> Result<Synergy, String> {
    let class = captured_class(value)?;
    if class != "SYNERGY" {
        return Err(format!("expected SYNERGY class, got :{}", class));
    }
    Ok(Synergy {
        description: require_optional_string(value, "description")?,
        connector: require_optional_string(value, "connector")?,
        score: require_i32(value, "score")?,
        start: require_i32(value, "start")? as usize,
        end: require_i32(value, "end")? as usize,
    })
}

/// Parse a captured path-element JSON (SEGMENT-LIST or SYNERGY) — the
/// element type the `find-best-path` payload list carries (per
/// `slot_types.csv` on `top-array-item.payload`).
pub fn parse_captured_path_element(value: &Value) -> Result<PathElement, String> {
    match captured_class(value)? {
        "SEGMENT-LIST" => Ok(PathElement::SegmentList(parse_captured_segment_list(value)?)),
        "SYNERGY" => Ok(PathElement::Synergy(parse_captured_synergy(value)?)),
        other => Err(format!("unknown path-element class: :{}", other)),
    }
}


/// Parse a captured `seq` JSON value into [`Option<WordInfoSeq>`].
/// `null` → `None`; integer → `Some(Single(_))`; array → `Some(Multi(_))`
/// where each element is recursively parsed (preserving nested arrays
/// from compound-text children and `null` entries from sourceless-counter
/// children).
fn parse_captured_seq(value: &Value) -> Result<Option<WordInfoSeq>, String> {
    match value {
        Value::Null => Ok(None),
        Value::Number(n) => Ok(Some(WordInfoSeq::Single(
            n.as_i64().ok_or_else(|| format!("seq not i64: {}", n))? as i32,
        ))),
        Value::Array(arr) => {
            let mut v = Vec::with_capacity(arr.len());
            for entry in arr {
                v.push(parse_captured_seq(entry)?);
            }
            Ok(Some(WordInfoSeq::Multi(v)))
        }
        other => Err(format!("seq: expected int / array / null, got {}", other)),
    }
}

/// Parse a captured `kana` JSON value into [`Option<WordInfoKana>`].
/// `null` → `None`; string → `Some(Single(_))`; array → `Some(Multi(_))`
/// where each element is recursively parsed (preserving nested arrays
/// and `null` entries when an upstream child has no kana).
fn parse_captured_kana(value: &Value) -> Result<Option<WordInfoKana>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(WordInfoKana::Single(s.clone()))),
        Value::Array(arr) => {
            let mut v = Vec::with_capacity(arr.len());
            for entry in arr {
                v.push(parse_captured_kana(entry)?);
            }
            Ok(Some(WordInfoKana::Multi(v)))
        }
        other => Err(format!("kana: expected string / array / null, got {}", other)),
    }
}


// --- word-info comparator --------------------------------------------------

/// Compare a Rust-produced [`WordInfo`] against a captured WORD-INFO
/// JSON value. Returns `Ok(())` on byte-identical match across every
/// captured slot, or `Err` with a one-line description of the first
/// mismatching field.
///
/// Field shape decisions:
/// - `kind` ← `":KANJI"` / `":KANA"` / `":GAP"` → [`WordInfoType`].
/// - `kana` ← JSON string → [`WordInfoKana::Single`]; JSON array of
///   strings → [`WordInfoKana::Multi`]; `null` → [`None`].
/// - `seq` ← JSON int → [`WordInfoSeq::Single`]; JSON array of ints
///   → [`WordInfoSeq::Multi`] (flat; matches the seq dispatcher's
///   compound flattening); `null` → [`None`].
/// - `conjugations` ← projector shape from [`require_conjugations`].
/// - `primary` / `alternative` ← null → `false`, true → `true`.
/// - `counter` ← null → [`None`]; 2-element array `[string, bool|null]`
///   → [`Some((string, bool))`].
/// - `components` ← null → empty Vec; array → recursive compare.
pub fn compare_captured_word_info(actual: &WordInfo, captured: &Value) -> Result<(), String> {
    let class = captured_class(captured)?;
    if class != "WORD-INFO" {
        return Err(format!("expected WORD-INFO class, got :{}", class));
    }
    // kind
    let cap_kind = require_string(captured, "type")?;
    let expected_kind = match cap_kind.as_str() {
        ":KANJI" => WordInfoType::Kanji,
        ":KANA" => WordInfoType::Kana,
        ":GAP" => WordInfoType::Gap,
        other => return Err(format!("unknown type: {}", other)),
    };
    if actual.kind != expected_kind {
        return Err(format!("kind: rust={:?} lisp={:?}", actual.kind, expected_kind));
    }
    // text
    let cap_text = require_string(captured, "text")?;
    if actual.text != cap_text {
        return Err(format!("text: rust={:?} lisp={:?}", actual.text, cap_text));
    }
    // true_text
    let cap_true_text = require_optional_string(captured, "true_text")?;
    if actual.true_text != cap_true_text {
        return Err(format!(
            "true_text: rust={:?} lisp={:?}",
            actual.true_text, cap_true_text
        ));
    }
    // kana
    let cap_kana = parse_captured_kana(require_field(captured, "kana")?)?;
    if actual.kana != cap_kana {
        return Err(format!("kana: rust={:?} lisp={:?}", actual.kana, cap_kana));
    }
    // seq
    let cap_seq = parse_captured_seq(require_field(captured, "seq")?)?;
    if actual.seq != cap_seq {
        return Err(format!("seq: rust={:?} lisp={:?}", actual.seq, cap_seq));
    }
    // conjugations
    let cap_conj = require_conjugations(captured, "conjugations")?;
    if actual.conjugations != cap_conj {
        return Err(format!(
            "conjugations: rust={:?} lisp={:?}",
            actual.conjugations, cap_conj
        ));
    }
    // score — Lisp slot can hold nil or int; null → None
    let cap_score = match require_field(captured, "score")? {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_i64().ok_or_else(|| format!("score not i64: {}", n))? as i32,
        ),
        other => return Err(format!("score: expected int / null, got {}", other)),
    };
    if actual.score != cap_score {
        return Err(format!("score: rust={:?} lisp={:?}", actual.score, cap_score));
    }
    // alternative — null → false, true → true
    let cap_alternative = match require_field(captured, "alternative")? {
        Value::Null => false,
        Value::Bool(b) => *b,
        other => return Err(format!("alternative: expected bool / null, got {}", other)),
    };
    if actual.alternative != cap_alternative {
        return Err(format!(
            "alternative: rust={} lisp={}",
            actual.alternative, cap_alternative
        ));
    }
    // primary — null → false (the upstream projects `(slot-value obj 'primary)`,
    // which is t/nil; nil flattens to JSON null).
    let cap_primary = match require_field(captured, "primary")? {
        Value::Null => false,
        Value::Bool(b) => *b,
        other => return Err(format!("primary: expected bool / null, got {}", other)),
    };
    if actual.primary != cap_primary {
        return Err(format!(
            "primary: rust={} lisp={}",
            actual.primary, cap_primary
        ));
    }
    // start / end
    let cap_start = match require_field(captured, "start")? {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_u64()
                .ok_or_else(|| format!("start not u64: {}", n))? as usize,
        ),
        other => return Err(format!("start: expected int / null, got {}", other)),
    };
    if actual.start != cap_start {
        return Err(format!("start: rust={:?} lisp={:?}", actual.start, cap_start));
    }
    let cap_end = match require_field(captured, "end")? {
        Value::Null => None,
        Value::Number(n) => Some(
            n.as_u64()
                .ok_or_else(|| format!("end not u64: {}", n))? as usize,
        ),
        other => return Err(format!("end: expected int / null, got {}", other)),
    };
    if actual.end != cap_end {
        return Err(format!("end: rust={:?} lisp={:?}", actual.end, cap_end));
    }
    // counter — null, or [string, bool|null]
    let cap_counter = match require_field(captured, "counter")? {
        Value::Null => None,
        Value::Array(arr) => {
            if arr.len() != 2 {
                return Err(format!("counter: expected 2-element array, got {} elements", arr.len()));
            }
            let s = arr[0]
                .as_str()
                .ok_or_else(|| format!("counter[0] not string: {}", arr[0]))?;
            let ordinalp = match &arr[1] {
                Value::Null => false,
                Value::Bool(b) => *b,
                other => return Err(format!("counter[1]: expected bool / null, got {}", other)),
            };
            Some((s.to_string(), ordinalp))
        }
        other => return Err(format!("counter: expected array / null, got {}", other)),
    };
    if actual.counter != cap_counter {
        return Err(format!(
            "counter: rust={:?} lisp={:?}",
            actual.counter, cap_counter
        ));
    }
    // skipped
    let cap_skipped = require_i32(captured, "skipped")?;
    if actual.skipped != cap_skipped {
        return Err(format!(
            "skipped: rust={} lisp={}",
            actual.skipped, cap_skipped
        ));
    }
    // components — null → empty; array → recurse pairwise.
    let cap_components = match require_field(captured, "components")? {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr.iter().collect::<Vec<_>>(),
        other => return Err(format!("components: expected array / null, got {}", other)),
    };
    if actual.components.len() != cap_components.len() {
        return Err(format!(
            "components count: rust={} lisp={}",
            actual.components.len(),
            cap_components.len()
        ));
    }
    for (idx, (a_child, c_child)) in actual.components.iter().zip(&cap_components).enumerate() {
        compare_captured_word_info(a_child, c_child)
            .map_err(|err| format!("components[{}]: {}", idx, err))?;
    }
    Ok(())
}


// --- shared score-/conj-data parsers ---------------------------------------
//
// These were previously duplicated across
// `audit/dict/calc_score_test.rs`, `gen_score_test.rs`, and
// `kanji_break_penalty_test.rs`. Consolidating here keeps the
// projector-bug workaround on `parse_opt_bool` in one place and prevents
// drift between three slightly-different copies.

/// Truncate a JSON value's `Display` form for use inside error messages.
/// Keeps the first 200 Unicode scalar values and appends `…` past that.
pub fn short_plist(v: &Value) -> String {
    let s = v.to_string();
    let max_chars = 200;
    if s.chars().count() <= max_chars {
        s
    } else {
        let head: String = s.chars().take(max_chars).collect();
        format!("{}…", head)
    }
}

pub fn parse_string_list(v: &Value, field: &str) -> Result<Vec<String>, String> {
    match v {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr
            .iter()
            .map(|item| {
                item.as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| format!("{}: entry not string: {}", field, item))
            })
            .collect(),
        other => Err(format!("{}: expected array/null, got {}", field, other)),
    }
}

pub fn parse_int_list(v: &Value, field: &str) -> Result<Vec<i32>, String> {
    match v {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr
            .iter()
            .map(|item| {
                item.as_i64()
                    .map(|n| n as i32)
                    .ok_or_else(|| format!("{}: entry not int: {}", field, item))
            })
            .collect(),
        other => Err(format!("{}: expected array/null, got {}", field, other)),
    }
}

pub fn parse_opt_i32(v: &Value, field: &str) -> Result<Option<i32>, String> {
    match v {
        Value::Null => Ok(None),
        Value::String(s) if s == ":NULL" => Ok(None),
        Value::Number(n) => Ok(Some(
            n.as_i64()
                .ok_or_else(|| format!("{}: not i64: {}", field, n))? as i32,
        )),
        other => Err(format!("{}: expected int/null, got {}", field, other)),
    }
}

/// Workaround for a **capture-side bug** in
/// `ichiran-extractor/projectors_json.lisp`:
///
/// The projector emits Lisp `nil` as JSON `null` universally (no
/// awareness of `:col-type boolean` vs `(or db-null boolean)` slot
/// metadata). So PG `false` — decoded by postmodern as Lisp `nil` —
/// arrives on the audit side as JSON `null`, indistinguishable from a
/// generic absent value. PG `NULL` does survive intact as Lisp `:null`
/// → JSON `":NULL"`.
///
/// Mapping below:
/// - JSON `null` → `Some(false)` (assumed PG `false`; lossy)
/// - JSON `":NULL"` → `None`
/// - JSON `true` / `false` → `Some(_)`
///
/// Cost: the audit cannot distinguish a genuine `Some(false)` from
/// `None` Rust-side mismatch on `neg` / `fml`. The projector needs to
/// special-case DAO slots typed as boolean and emit JSON `false` for
/// Lisp `nil`; see `feedback_projector_boolean_emission_bug` in
/// memory. Until that lands and the affected parquets are re-extracted,
/// this asymmetric mapping is the only way to keep the audit clean
/// (without it ~582K rows fail across calc-score + gen-score).
pub fn parse_opt_bool(v: &Value, field: &str) -> Result<Option<bool>, String> {
    match v {
        Value::Null => Ok(Some(false)),
        Value::String(s) if s == ":NULL" => Ok(None),
        Value::Bool(b) => Ok(Some(*b)),
        other => Err(format!("{}: expected bool/null/\":NULL\", got {}", field, other)),
    }
}

pub fn parse_conj_prop(value: &Value) -> Result<ConjProp, String> {
    let class = captured_class(value)?;
    if class != "CONJ-PROP" {
        return Err(format!("expected CONJ-PROP, got :{}", class));
    }
    Ok(ConjProp {
        id: value.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        conj_id: value.get("conj_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        conj_type: value.get("conj_type").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        pos: value.get("pos").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        neg: parse_opt_bool(value.get("neg").unwrap_or(&Value::Null), "neg")?,
        fml: parse_opt_bool(value.get("fml").unwrap_or(&Value::Null), "fml")?,
    })
}

pub fn parse_conj_data(value: &Value) -> Result<ConjData, String> {
    let class = captured_class(value)?;
    if class != "CONJ-DATA" {
        return Err(format!("expected CONJ-DATA, got :{}", class));
    }
    let seq = parse_opt_i32(value.get("seq").unwrap_or(&Value::Null), "conj-data.seq")?;
    let from = parse_opt_i32(value.get("from").unwrap_or(&Value::Null), "conj-data.from")?;
    let via = parse_opt_i32(value.get("via").unwrap_or(&Value::Null), "conj-data.via")?;
    let prop = match value.get("prop").unwrap_or(&Value::Null) {
        Value::Null => None,
        prop_val => Some(parse_conj_prop(prop_val)?),
    };
    let src_map: Vec<(String, String)> = match value.get("src_map").unwrap_or(&Value::Null) {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr
            .iter()
            .map(|pair| {
                let pa = pair
                    .as_array()
                    .ok_or_else(|| format!("src_map entry not array: {}", pair))?;
                if pa.len() != 2 {
                    return Err(format!("src_map pair not 2-elem: {}", pair));
                }
                let a = pa[0]
                    .as_str()
                    .ok_or_else(|| format!("src_map[0] not string: {}", pa[0]))?
                    .to_string();
                let b = pa[1]
                    .as_str()
                    .ok_or_else(|| format!("src_map[1] not string: {}", pa[1]))?
                    .to_string();
                Ok((a, b))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("src_map: expected array/null, got {}", other)),
    };
    Ok(ConjData { seq, from, via, prop, src_map })
}

pub fn parse_conj_list(v: &Value) -> Result<Vec<ConjData>, String> {
    match v {
        Value::Null => Ok(Vec::new()),
        Value::Array(arr) => arr.iter().map(parse_conj_data).collect(),
        other => Err(format!("conj: expected array/null, got {}", other)),
    }
}

/// Parses the four-element `(:SCORE-INFO …)` tuple at
/// `dict.lisp:976-980`: `(prop-score kanji-break use-length-bonus
/// split-info)`. `split-info` accepts the three upstream shapes —
/// `nil` → [`KaniSplitInfo::None`], integer → [`KaniSplitInfo::Score`],
/// `(score-mod . part-scores)` array → [`KaniSplitInfo::Parts`].
pub fn parse_score_info(v: &Value) -> Result<KaniScoreInfo, String> {
    let arr = v
        .as_array()
        .ok_or_else(|| format!("score-info: expected array, got {}", v))?;
    if arr.len() != 4 {
        return Err(format!("score-info: expected 4-tuple, got {}", arr.len()));
    }
    let prop_score = arr[0]
        .as_i64()
        .ok_or_else(|| format!("score-info[0] not int: {}", arr[0]))? as i32;
    let kanji_break: Vec<usize> = match &arr[1] {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr
            .iter()
            .map(|item| {
                item.as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| format!("score-info[1] entry not uint: {}", item))
            })
            .collect::<Result<_, _>>()?,
        other => return Err(format!("score-info[1]: expected array/null, got {}", other)),
    };
    let use_length_bonus = arr[2]
        .as_i64()
        .ok_or_else(|| format!("score-info[2] not int: {}", arr[2]))? as i32;
    let split_info = match &arr[3] {
        Value::Null => KaniSplitInfo::None,
        Value::Number(n) => KaniSplitInfo::Score(
            n.as_i64()
                .ok_or_else(|| format!("score-info[3] not i64: {}", n))? as i32,
        ),
        Value::Array(parts) => {
            if parts.is_empty() {
                return Err("score-info[3]: empty Parts array".into());
            }
            let score_mod = parts[0]
                .as_i64()
                .ok_or_else(|| format!("score-info[3][0] not int: {}", parts[0]))?
                as i32;
            let part_scores: Vec<i32> = parts[1..]
                .iter()
                .map(|item| {
                    item.as_i64()
                        .map(|n| n as i32)
                        .ok_or_else(|| format!("score-info[3] part not int: {}", item))
                })
                .collect::<Result<_, _>>()?;
            KaniSplitInfo::Parts { score_mod, part_scores }
        }
        other => return Err(format!("score-info[3]: unexpected: {}", other)),
    };
    Ok(KaniScoreInfo { prop_score, kanji_break, use_length_bonus, split_info })
}

/// Parses the `(:KPCL …)` 4-tuple. Upstream stores the raw truthy
/// values from `(or kanji-p katakana-p)` / `(and common-p pronoun-p)` /
/// `(intersection seq-set *copulae*)` etc., not always `t` — any
/// non-nil JSON value collapses to `true` here to match the Lisp
/// predicate semantics every downstream consumer applies.
pub fn parse_kpcl(v: &Value) -> Result<(bool, bool, bool, bool), String> {
    let arr = v
        .as_array()
        .ok_or_else(|| format!("kpcl: expected array, got {}", v))?;
    if arr.len() != 4 {
        return Err(format!("kpcl: expected 4-tuple, got {}", arr.len()));
    }
    let mut bools = [false; 4];
    for (i, entry) in arr.iter().enumerate() {
        bools[i] = match entry {
            Value::Null => false,
            Value::Bool(b) => *b,
            _ => true,
        };
    }
    Ok((bools[0], bools[1], bools[2], bools[3]))
}
