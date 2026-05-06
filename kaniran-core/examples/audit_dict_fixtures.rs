//! Audit `corpus/.../<pkg>/<sym>.parquet` fixtures for the async,
//! DB-backed transliterations under [`kaniran_core::dict`]. Sibling of
//! `audit_fixtures.rs` (which handles sync, pure functions); kept
//! separate so the sync harness needn't take a tokio runtime or
//! KaniranContext for handlers that don't need them.
//!
//! For each parquet row: parse `args` and `result` via `kani::sexp`,
//! dispatch the args to the Rust async fn, project the returned DAO
//! list/option to the same `(:CLASS :ID :SEQ :TEXT :ORD ...)` plist
//! shape the Lisp side captured (see
//! `ichiran-extractor/projectors.lisp`), and compare.
//!
//! Run with:
//!   cargo run --release --example audit_dict_fixtures -- corpus/extracted_wave_112
//!   cargo run --release --example audit_dict_fixtures -- corpus/extracted/dict
//!
//! Requires `ICHIRAN_CONNECTION` (or whatever
//! [`kaniran_core::conn::kani_context::KaniranContext::from_env`]
//! reads) pointing at a populated ichiran Postgres.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use arrow::array::StringArray;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use tokio::task::JoinSet;

const CONCURRENCY: usize = 16;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::conj_data_struct::ConjData;
use kaniran_core::dict::conj_prop_dao::ConjProp;
use kaniran_core::dict::find_word_conj_of::find_word_conj_of;
use kaniran_core::dict::find_word_seq::{find_word_seq, WordSeqRows};
use kaniran_core::dict::get_conj_data::{get_conj_data, FromOrConjIds};
use kaniran_core::dict::get_kana_form::get_kana_form;
use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kanji_text_dao::KanjiText;
use kaniran_core::kani::sexp::{self, Sexp};

const MISMATCH_PRINT_LIMIT: usize = 5;


// --- shared sexp helpers -----------------------------------------------

fn list_elems(s: &Sexp) -> Result<Vec<&Sexp>, String> {
    s.list_iter()
        .map(|it| it.collect())
        .ok_or_else(|| format!("expected proper list, got {}", s))
}

fn expect_one(expected: &Sexp) -> Result<&Sexp, String> {
    let elems = list_elems(expected)?;
    if elems.len() != 1 {
        return Err(format!("expected 1-element list (multi-val wrap), got {}", expected));
    }
    Ok(elems[0])
}

fn parse_int(s: &Sexp) -> Result<i32, String> {
    s.as_i64()
        .ok_or_else(|| format!("not an int: {}", s))
        .map(|n| n as i32)
}


// --- projected DAO row + compare ---------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct DaoRow {
    class: String,
    id: i32,
    seq: i32,
    text: String,
    ord: i32,
}

/// Walk a `(:KEY value :KEY value ...)` plist into a `DaoRow`.
fn parse_dao_plist(s: &Sexp) -> Result<DaoRow, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("plist has odd element count: {}", s));
    }
    let mut class = None;
    let mut id = None;
    let mut seq = None;
    let mut text = None;
    let mut ord = None;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("plist key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => class = Some(
                v.as_keyword()
                    .ok_or_else(|| format!(":CLASS value not keyword: {}", v))?
                    .to_string(),
            ),
            "ID"   => id   = Some(parse_int(v)?),
            "SEQ"  => seq  = Some(parse_int(v)?),
            "TEXT" => text = Some(
                v.as_str()
                    .ok_or_else(|| format!(":TEXT value not string: {}", v))?
                    .to_string(),
            ),
            "ORD"  => ord  = Some(parse_int(v)?),
            other => return Err(format!("unknown plist key: :{}", other)),
        }
    }
    Ok(DaoRow {
        class: class.ok_or(":CLASS missing")?,
        id:    id.ok_or(":ID missing")?,
        seq:   seq.ok_or(":SEQ missing")?,
        text:  text.ok_or(":TEXT missing")?,
        ord:   ord.ok_or(":ORD missing")?,
    })
}

fn dao_from_kana(r: &KanaText) -> DaoRow {
    DaoRow {
        class: "KANA-TEXT".into(),
        id: r.id, seq: r.seq, text: r.text.clone(), ord: r.ord,
    }
}

fn dao_from_kanji(r: &KanjiText) -> DaoRow {
    DaoRow {
        class: "KANJI-TEXT".into(),
        id: r.id, seq: r.seq, text: r.text.clone(), ord: r.ord,
    }
}

fn project_word_seq(rows: &WordSeqRows) -> Vec<DaoRow> {
    match rows {
        WordSeqRows::Kana(v)  => v.iter().map(dao_from_kana).collect(),
        WordSeqRows::Kanji(v) => v.iter().map(dao_from_kanji).collect(),
    }
}

/// Compare projected actual rows to the captured Lisp result.
/// Order-insensitive (sort by id) — `(union ... :key #'id)` and
/// `select-dao` without ORDER BY both have implementation-defined
/// ordering per the CL spec / SQL spec; comparing as multisets
/// avoids false negatives that don't reflect a behavioral diff.
fn compare_dao_lists(mut actual: Vec<DaoRow>, expected: &Sexp) -> Result<(), String> {
    let exp_elems = list_elems(expected)?;
    let mut expected_rows: Vec<DaoRow> = exp_elems.iter()
        .map(|e| parse_dao_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!(
            "row count: rust={} lisp={}\n  rust: {:?}\n  lisp: {:?}",
            actual.len(), expected_rows.len(), actual, expected_rows,
        ));
    }
    actual.sort_by_key(|r| (r.id, r.class.clone()));
    expected_rows.sort_by_key(|r| (r.id, r.class.clone()));
    for (i, (a, e)) in actual.iter().zip(&expected_rows).enumerate() {
        if a != e {
            return Err(format!("row {}: rust={:?} lisp={:?}", i, a, e));
        }
    }
    Ok(())
}


// --- per-FQN handlers --------------------------------------------------

async fn audit_find_word_seq(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("args empty (need at least word)".into()); }
    let word = argv[0].as_str().ok_or("arg 0 not string")?;
    let seqs: Vec<i32> = argv[1..].iter()
        .map(|s| parse_int(s))
        .collect::<Result<_, _>>()?;
    let actual = find_word_seq(ctx, word, &seqs).await
        .map_err(|e| format!("find_word_seq query: {}", e))?;
    compare_dao_lists(project_word_seq(&actual), expect_one(expected)?)
}

async fn audit_find_word_conj_of(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("args empty (need at least word)".into()); }
    let word = argv[0].as_str().ok_or("arg 0 not string")?;
    let seqs: Vec<i32> = argv[1..].iter()
        .map(|s| parse_int(s))
        .collect::<Result<_, _>>()?;
    let actual = find_word_conj_of(ctx, word, &seqs).await
        .map_err(|e| format!("find_word_conj_of query: {}", e))?;
    compare_dao_lists(project_word_seq(&actual), expect_one(expected)?)
}

// --- get-conj-data: DB-backed conj-data list audit -----------------------

fn parse_bool_or_dbnull(s: &Sexp) -> Result<Option<bool>, String> {
    if s.is_nil() { return Ok(Some(false)); }
    if s.is_t() { return Ok(Some(true)); }
    if s.as_keyword() == Some("NULL") { return Ok(None); }
    Err(format!("not T / NIL / :NULL: {}", s))
}

fn parse_int_or_nil(s: &Sexp) -> Result<Option<i32>, String> {
    if s.is_nil() { return Ok(None); }
    Ok(Some(s.as_i64().ok_or_else(|| format!("not int/nil: {}", s))? as i32))
}

fn parse_conj_prop_plist(s: &Sexp) -> Result<ConjProp, String> {
    let elems = list_elems(s)?;
    let mut id = None;
    let mut conj_id = None;
    let mut conj_type = None;
    let mut pos = None;
    let mut neg = None;
    let mut fml = None;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("conj-prop key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {} // checked by caller
            "ID" => id = Some(parse_int(v)?),
            "CONJ-ID" => conj_id = Some(parse_int(v)?),
            "CONJ-TYPE" => conj_type = Some(parse_int(v)?),
            "POS" => pos = Some(v.as_str().ok_or(":POS not string")?.to_string()),
            "NEG" => neg = Some(parse_bool_or_dbnull(v)?),
            "FML" => fml = Some(parse_bool_or_dbnull(v)?),
            other => return Err(format!("unknown conj-prop key: :{}", other)),
        }
    }
    Ok(ConjProp {
        id: id.ok_or(":ID missing")?,
        conj_id: conj_id.ok_or(":CONJ-ID missing")?,
        conj_type: conj_type.ok_or(":CONJ-TYPE missing")?,
        pos: pos.ok_or(":POS missing")?,
        neg: neg.ok_or(":NEG missing")?,
        fml: fml.ok_or(":FML missing")?,
    })
}

fn parse_conj_data_plist(s: &Sexp) -> Result<ConjData, String> {
    let elems = list_elems(s)?;
    let mut seq = None;
    let mut from = None;
    let mut via = None;
    let mut prop: Option<Option<ConjProp>> = None;
    let mut src_map: Option<Vec<(String, String)>> = None;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("conj-data key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {}
            "SEQ"  => seq = Some(parse_int_or_nil(v)?),
            "FROM" => from = Some(parse_int_or_nil(v)?),
            "VIA"  => via = Some(parse_int_or_nil(v)?),
            "PROP" => prop = Some(if v.is_nil() { None } else { Some(parse_conj_prop_plist(v)?) }),
            "SRC-MAP" => {
                let pairs = if v.is_nil() {
                    Vec::new()
                } else {
                    list_elems(v)?.into_iter()
                        .map(|p| {
                            let pv = list_elems(p)?;
                            if pv.len() != 2 {
                                return Err(format!("src-map pair want 2: {}", p));
                            }
                            Ok((
                                pv[0].as_str().ok_or("src-map[0] not str")?.to_string(),
                                pv[1].as_str().ok_or("src-map[1] not str")?.to_string(),
                            ))
                        })
                        .collect::<Result<_, String>>()?
                };
                src_map = Some(pairs);
            }
            other => return Err(format!("unknown conj-data key: :{}", other)),
        }
    }
    Ok(ConjData {
        seq:  seq.ok_or(":SEQ missing")?,
        from: from.ok_or(":FROM missing")?,
        via:  via.ok_or(":VIA missing")?,
        prop: prop.ok_or(":PROP missing")?,
        src_map: src_map.ok_or(":SRC-MAP missing")?,
    })
}

fn conj_prop_eq(a: &ConjProp, b: &ConjProp) -> bool {
    a.id == b.id && a.conj_id == b.conj_id && a.conj_type == b.conj_type
        && a.pos == b.pos && a.neg == b.neg && a.fml == b.fml
}

fn conj_data_eq(a: &ConjData, b: &ConjData) -> bool {
    let prop_eq = match (&a.prop, &b.prop) {
        (None, None) => true,
        (Some(p), Some(q)) => conj_prop_eq(p, q),
        _ => false,
    };
    // src_map element order is implementation-defined: neither the Lisp
    // (postmodern's `(query (:select ...))`) nor the Rust port emits an
    // ORDER BY on conj_source_reading, so the two databases — captured
    // on .103, audited locally — return the same rows in different
    // physical orderings. Compare as a multiset (sort both sides by
    // (text, source_text)).
    let mut a_src = a.src_map.clone();
    let mut b_src = b.src_map.clone();
    a_src.sort();
    b_src.sort();
    a.seq == b.seq && a.from == b.from && a.via == b.via
        && prop_eq && a_src == b_src
}

/// Decide which `FromOrConjIds` variant matches the upstream
/// `from/conj-ids` argument shape (NIL / `:ROOT` / integer / list).
fn parse_from_or_conj_ids(s: &Sexp) -> Result<FromOrConjIds, String> {
    if s.is_nil() { return Ok(FromOrConjIds::All); }
    if s.as_keyword() == Some("ROOT") { return Ok(FromOrConjIds::Root); }
    if let Some(n) = s.as_i64() { return Ok(FromOrConjIds::From(n as i32)); }
    if s.is_list() {
        let elems = list_elems(s)?;
        let ids: Vec<i32> = elems.iter().map(|e| parse_int(e)).collect::<Result<_, _>>()?;
        return Ok(FromOrConjIds::ConjIds(ids));
    }
    Err(format!("can't classify from/conj-ids: {}", s))
}

async fn audit_get_conj_data(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("get-conj-data needs at least seq".into()); }
    let seq = parse_int(argv[0])?;
    let from_or = if argv.len() >= 2 {
        parse_from_or_conj_ids(argv[1])?
    } else {
        FromOrConjIds::All
    };
    // texts: NIL / single-string / list-of-strings
    let mut texts_owned: Vec<String> = Vec::new();
    if argv.len() >= 3 {
        let t = argv[2];
        if t.is_nil() {
            // empty
        } else if let Some(s) = t.as_str() {
            texts_owned.push(s.to_string());
        } else if t.is_list() {
            for e in list_elems(t)? {
                texts_owned.push(e.as_str().ok_or("texts list elem not string")?.to_string());
            }
        } else {
            return Err(format!("texts not nil/string/list: {}", t));
        }
    }
    let texts: Vec<&str> = texts_owned.iter().map(String::as_str).collect();

    let actual = get_conj_data(ctx, seq, from_or, &texts).await
        .map_err(|e| format!("get_conj_data query: {}", e))?;

    let inner = expect_one(expected)?;
    let exp_elems = if inner.is_nil() { Vec::new() } else { list_elems(inner)? };
    let mut expected_rows: Vec<ConjData> = exp_elems.iter()
        .map(|e| parse_conj_data_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!("conj-data count: rust={} lisp={}", actual.len(), expected_rows.len()));
    }
    let mut actual_sorted = actual;
    let key = |c: &ConjData| (
        c.seq.unwrap_or(0), c.from.unwrap_or(0), c.via.unwrap_or(0),
        c.prop.as_ref().map(|p| p.id).unwrap_or(0),
    );
    actual_sorted.sort_by_key(key);
    expected_rows.sort_by_key(key);
    for (i, (a, e)) in actual_sorted.iter().zip(&expected_rows).enumerate() {
        if !conj_data_eq(a, e) {
            return Err(format!("row {}: rust seq={:?} from={:?} via={:?} prop_id={:?}\n         lisp seq={:?} from={:?} via={:?} prop_id={:?}",
                i, a.seq, a.from, a.via, a.prop.as_ref().map(|p| p.id),
                e.seq, e.from, e.via, e.prop.as_ref().map(|p| p.id)));
        }
    }
    Ok(())
}


async fn audit_get_kana_form(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() < 2 { return Err("args want at least (seq text)".into()); }
    let seq = parse_int(argv[0])?;
    let text = argv[1].as_str().ok_or("arg 1 not string")?;
    // :CONJ keyword is optional; always passed as a keyword tag in the
    // captures (e.g. (:CONJ :ROOT)). The Rust port just toggles whether
    // the row's runtime conjugations slot gets set — the projected output
    // we compare against does NOT include that slot, so we ignore the
    // value here. Pass None; behavior under the projection is identical.
    let actual = get_kana_form(ctx, seq, text, None).await
        .map_err(|e| format!("get_kana_form query: {}", e))?;

    // Single-DAO-or-nil is captured as ((<plist or NIL>)).
    let inner = expect_one(expected)?;
    let actual_row = actual.as_ref().map(dao_from_kana);
    match (actual_row, inner.is_nil()) {
        (None, true) => Ok(()),
        (Some(a), false) => {
            let e = parse_dao_plist(inner)?;
            if a == e { Ok(()) } else {
                Err(format!("row: rust={:?} lisp={:?}", a, e))
            }
        }
        (Some(a), true)  => Err(format!("rust returned row, lisp returned nil\n  rust: {:?}", a)),
        (None,    false) => Err(format!("rust returned nil, lisp returned a row\n  lisp: {}", inner)),
    }
}


// --- driver ------------------------------------------------------------

#[derive(Default)]
struct Totals {
    pass: usize,
    fail: usize,
    fns_clean: usize,
    fns_with_failures: usize,
}

fn count_parquet_rows(path: &Path) -> Result<usize, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open: {}", e))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("parquet: {}", e))?;
    Ok(reader.metadata().file_metadata().num_rows() as usize)
}

fn format_count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(c);
    }
    out.chars().rev().collect()
}

fn format_duration(seconds: f64) -> String {
    let total = seconds as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 { format!("{}h{:02}m{:02}s", h, m, s) }
    else if m > 0 { format!("{}m{:02}s", m, s) }
    else { format!("{:.1}s", seconds) }
}

fn discover_parquets(arg: &str) -> Vec<PathBuf> {
    let p = Path::new(arg);
    if p.is_file() { return vec![p.to_path_buf()]; }
    let mut out = Vec::new();
    if p.is_dir() {
        for entry in std::fs::read_dir(p).expect("read_dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                out.extend(discover_parquets(path.to_str().expect("utf-8 path")));
            } else if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn fqn_from_metadata(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open: {}", e))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("parquet: {}", e))?;
    if let Some(kv) = reader.metadata().file_metadata().key_value_metadata() {
        for entry in kv {
            if entry.key == "ichiran_extractor_fqn" {
                if let Some(v) = entry.value.clone() { return Ok(v); }
            }
        }
    }
    // Fallback: infer from <pkg>/<sym>.parquet — survives a duckdb
    // dedupe pass (`COPY ... TO 'out.parquet' (FORMAT PARQUET)`) which
    // drops the source's KV metadata. Mirrors the package set the
    // tracer uses; expand if other packages start producing fixtures.
    let stem = path.file_stem().and_then(|s| s.to_str()).ok_or("no file stem")?;
    let pkg = path.parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .ok_or("no parent dir")?;
    let sym = stem.to_uppercase().replace('_', "-");
    let pkg_prefix = match pkg {
        "characters" | "dict" | "conn" | "kanji" | "numbers" | "romanize" =>
            format!("ICHIRAN/{}::", pkg.to_uppercase()),
        "core" => "ICHIRAN::".into(),
        other => return Err(format!("can't infer FQN from path; unknown package dir {:?}", other)),
    };
    Ok(format!("{}{}", pkg_prefix, sym))
}

async fn audit_file(
    idx: usize, total: usize, path: &Path,
    ctx: &Arc<KaniranContext>, totals: &mut Totals,
) {
    let fqn = match fqn_from_metadata(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[{}/{}] {}: skip — {}", idx, total, path.display(), e);
            return;
        }
    };

    // Pre-count rows so progress lines have a denominator + ETA.
    let total_rows = match count_parquet_rows(path) {
        Ok(n) => n,
        Err(e) => { eprintln!("  count rows failed: {}", e); 0 }
    };

    println!("[{}/{}] {} → {}  ({} rows)", idx, total, path.display(), fqn, format_count(total_rows));

    let file = std::fs::File::open(path).expect("open parquet");
    let reader = ParquetRecordBatchReaderBuilder::try_new(file).expect("build")
        .build().expect("reader");
    let mut n_pass = 0usize;
    let mut n_fail = 0usize;
    let mut first_failures: Vec<String> = Vec::new();
    let t0 = Instant::now();
    let mut last_progress = Instant::now();
    let progress_interval = std::time::Duration::from_secs(5);

    // Fan out CONCURRENCY in-flight audits via JoinSet. Required because
    // each row issues a Postgres round-trip; serial dispatch over the
    // SSH tunnel pegs at ~100 rows/s, which is hours for million-row
    // fixtures. Each task owns its KaniranContext clone (cheap — the
    // PgPool is internally Arc-shared).
    let mut set: JoinSet<(String, Result<(), String>)> = JoinSet::new();
    let tally = |args_str: String, res: Result<(), String>,
                     n_pass: &mut usize, n_fail: &mut usize,
                     first_failures: &mut Vec<String>| {
        match res {
            Ok(()) => *n_pass += 1,
            Err(msg) => {
                *n_fail += 1;
                if first_failures.len() < MISMATCH_PRINT_LIMIT {
                    first_failures.push(format!("args={} | {}", args_str, msg));
                }
            }
        }
    };

    let maybe_print_progress = |
        last: &mut Instant, n_pass: usize, n_fail: usize, t0: &Instant,
    | {
        if last.elapsed() < progress_interval { return; }
        *last = Instant::now();
        let done = n_pass + n_fail;
        let elapsed = t0.elapsed().as_secs_f64();
        let rate = done as f64 / elapsed.max(1e-9);
        let pct = if total_rows > 0 { 100.0 * done as f64 / total_rows as f64 } else { 0.0 };
        let eta = if total_rows > done && rate > 0.0 {
            let secs = (total_rows - done) as f64 / rate;
            format!(", ETA {}", format_duration(secs))
        } else { String::new() };
        let fail_str = if n_fail > 0 { format!(", fail={}", n_fail) } else { String::new() };
        println!(
            "    {:>9}/{} ({:>5.1}%, {:>5.0} rows/s, {}{}{})",
            format_count(done), format_count(total_rows), pct, rate,
            format_duration(elapsed), eta, fail_str,
        );
    };

    for batch in reader {
        let batch = batch.expect("batch");
        let args_col = batch.column(0).as_any().downcast_ref::<StringArray>().expect("args utf8");
        let result_col = batch.column(1).as_any().downcast_ref::<StringArray>().expect("result utf8");
        for i in 0..batch.num_rows() {
            let args_str = args_col.value(i).to_string();
            let result_str = result_col.value(i).to_string();
            // Backpressure: never let the in-flight set exceed CONCURRENCY.
            while set.len() >= CONCURRENCY {
                if let Some(joined) = set.join_next().await {
                    let (a, r) = joined.expect("join");
                    tally(a, r, &mut n_pass, &mut n_fail, &mut first_failures);
                }
            }
            let fqn_owned = fqn.clone();
            let ctx_clone = Arc::clone(ctx);
            let args_clone = args_str.clone();
            set.spawn(async move {
                let res = audit_one(&fqn_owned, &args_clone, &result_str, &ctx_clone).await;
                (args_str, res)
            });
            maybe_print_progress(&mut last_progress, n_pass, n_fail, &t0);
        }
    }
    // Drain remainder.
    while let Some(joined) = set.join_next().await {
        let (a, r) = joined.expect("join");
        tally(a, r, &mut n_pass, &mut n_fail, &mut first_failures);
        maybe_print_progress(&mut last_progress, n_pass, n_fail, &t0);
    }

    let elapsed = t0.elapsed().as_secs_f64();
    let done = n_pass + n_fail;
    let pct = if done > 0 { 100.0 * n_pass as f64 / done as f64 } else { 0.0 };
    let tag = if n_fail == 0 { "PASS" } else { "FAIL" };
    println!(
        "  {} {:48} pass={:>7}  fail={:>7}  ({:>6.2}%, {})",
        tag, fqn, n_pass, n_fail, pct, format_duration(elapsed),
    );
    for (i, f) in first_failures.iter().enumerate() {
        println!("    [{}] {}", i + 1, f);
    }
    totals.pass += n_pass;
    totals.fail += n_fail;
    if n_fail == 0 { totals.fns_clean += 1; } else { totals.fns_with_failures += 1; }
}

async fn audit_one(
    fqn: &str, args_str: &str, result_str: &str, ctx: &KaniranContext,
) -> Result<(), String> {
    let args = sexp::parse(args_str).map_err(|e| format!("parse args: {}", e))?;
    let expected = sexp::parse(result_str).map_err(|e| format!("parse result: {}", e))?;
    match fqn {
        "ICHIRAN/DICT::FIND-WORD-SEQ"     => audit_find_word_seq(ctx, &args, &expected).await,
        "ICHIRAN/DICT::FIND-WORD-CONJ-OF" => audit_find_word_conj_of(ctx, &args, &expected).await,
        "ICHIRAN/DICT::GET-KANA-FORM"     => audit_get_kana_form(ctx, &args, &expected).await,
        "ICHIRAN/DICT::GET-CONJ-DATA"     => audit_get_conj_data(ctx, &args, &expected).await,
        other => Err(format!("no handler for FQN: {}", other)),
    }
}

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1)
        .unwrap_or_else(|| "corpus/extracted/dict".to_string());
    let parquets = discover_parquets(&arg);
    if parquets.is_empty() {
        eprintln!("no .parquet files at {}", arg);
        std::process::exit(2);
    }

    let ctx = match KaniranContext::from_env().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("audit_dict_fixtures: failed to build KaniranContext: {}", e);
            std::process::exit(1);
        }
    };

    let mut totals = Totals::default();
    println!("=== auditing {} parquet file(s) ===\n", parquets.len());
    for (i, path) in parquets.iter().enumerate() {
        audit_file(i + 1, parquets.len(), path, &ctx, &mut totals).await;
    }
    println!(
        "\n=== summary: pass={} fail={} | clean fns={} failing fns={} ===",
        totals.pass, totals.fail, totals.fns_clean, totals.fns_with_failures,
    );
    if totals.fail > 0 { std::process::exit(1); }
}
