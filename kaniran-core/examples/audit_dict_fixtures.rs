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
use kaniran_core::dict::find_word::{find_word, FindWordRows};
use kaniran_core::dict::find_word_as_hiragana::find_word_as_hiragana;
use kaniran_core::dict::find_word_conj_of::find_word_conj_of;
use kaniran_core::dict::find_word_seq::{find_word_seq, WordSeqRows};
use kaniran_core::dict::get_conj_data::{get_conj_data, FromOrConjIds};
use kaniran_core::dict::get_kana_form::get_kana_form;
use kaniran_core::dict::get_kana_forms::get_kana_forms;
use kaniran_core::dict::get_kana_forms_star_::get_kana_forms_star_;
use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kani_word::KaniSimpleTextDispatchEnum;
use kaniran_core::dict::kanji_text_dao::KanjiText;
use kaniran_core::dict::proxy_text_class::ProxyText;
use kaniran_core::dict::simple_text_class::WordConjugations;
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

/// Like [`expect_one`], but tolerates a trailing secondary value at
/// position 1 (postmodern's `query-dao` / `select-dao` returns the
/// row list as the primary value and the row count as the secondary;
/// `find-word` propagates both because its body is a DB call's tail
/// position, while the cache-path's `loop collect` produces only the
/// primary). The trailing slot is non-load-bearing for every audit
/// that opts into this — comparison runs against the rows in
/// position 0.
fn expect_first_of_one_or_two(expected: &Sexp) -> Result<&Sexp, String> {
    let elems = list_elems(expected)?;
    if elems.is_empty() || elems.len() > 2 {
        return Err(format!(
            "expected 1- or 2-element list (multi-val wrap), got {}", expected,
        ));
    }
    Ok(elems[0])
}

fn parse_int(s: &Sexp) -> Result<i32, String> {
    s.as_i64()
        .ok_or_else(|| format!("not an int: {}", s))
        .map(|n| n as i32)
}


// --- projected DAO row + compare ---------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct DaoRow {
    class: String,
    /// `None` when the captured plist had `:ID NIL`. Upstream cache-path
    /// rows carry NIL because `kana-text` / `kanji-text` declare `id`
    /// without `:initarg :id` (`dict.lisp:87,129`); `find-word`'s
    /// `(apply 'make-instance recipe)` therefore leaves the slot unbound
    /// when serving from `*substring-hash*`. The Rust port always hits
    /// the DB and so always has an integer id; comparison treats Lisp's
    /// NIL as a wildcard against any Rust id (see `dao_row_eq`).
    id: Option<i32>,
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
    let mut id: Option<Option<i32>> = None;
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
            "ID"   => id   = Some(if v.is_nil() { None } else { Some(parse_int(v)?) }),
            "SEQ"  => seq  = Some(parse_int(v)?),
            "TEXT" => text = Some(
                v.as_str()
                    .ok_or_else(|| format!(":TEXT value not string: {}", v))?
                    .to_string(),
            ),
            "ORD"  => ord  = Some(parse_int(v)?),
            // Other column slots and the inherited SIMPLE-TEXT
            // session-only slots are projected by the post-2026-05-06
            // projector (CONJUGATIONS, HINTEDP, COMMON, COMMON-TAGS,
            // CONJUGATE-P, NOKANJI, BEST-KANA / BEST-KANJI). The lite
            // DaoRow comparison only needs the four columns above; drop
            // the rest silently so older audit handlers keep passing
            // against re-captured parquets.
            _ => {}
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
        id: Some(r.id), seq: r.seq, text: r.text.clone(), ord: r.ord,
    }
}

fn dao_from_kanji(r: &KanjiText) -> DaoRow {
    DaoRow {
        class: "KANJI-TEXT".into(),
        id: Some(r.id), seq: r.seq, text: r.text.clone(), ord: r.ord,
    }
}

fn project_word_seq(rows: &WordSeqRows) -> Vec<DaoRow> {
    match rows {
        WordSeqRows::Kana(v)  => v.iter().map(dao_from_kana).collect(),
        WordSeqRows::Kanji(v) => v.iter().map(dao_from_kanji).collect(),
    }
}

/// `actual == expected` on the four required columns, with id treated
/// as a wildcard whenever EITHER side is `None` (NIL on the Lisp side
/// for cache-path rows; no Rust producer currently emits None, but
/// symmetric is cheaper than asymmetric).
fn dao_row_eq(a: &DaoRow, e: &DaoRow) -> bool {
    a.class == e.class && a.seq == e.seq && a.text == e.text && a.ord == e.ord
        && match (a.id, e.id) {
            (Some(x), Some(y)) => x == y,
            _ => true,
        }
}

/// Compare projected actual rows to the captured Lisp result.
/// Order-insensitive — `(union ... :key #'id)` and `select-dao`
/// without ORDER BY both have implementation-defined ordering per
/// the CL spec / SQL spec; comparing as multisets avoids false
/// negatives that don't reflect a behavioral diff. Sort by
/// `(seq, text, ord, class)` rather than `id` because cache-path
/// captures (`find-word`) carry `:ID NIL` and would otherwise all
/// collapse to a single sort key.
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
    let key = |r: &DaoRow| (r.seq, r.text.clone(), r.ord, r.class.clone());
    actual.sort_by_key(key);
    expected_rows.sort_by_key(key);
    for (i, (a, e)) in actual.iter().zip(&expected_rows).enumerate() {
        if !dao_row_eq(a, e) {
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

// --- find-word: same DaoRow shape as find-word-seq -----------------------

fn project_find_word_rows(rows: &FindWordRows) -> Vec<DaoRow> {
    match rows {
        FindWordRows::Kana(v)  => v.iter().map(dao_from_kana).collect(),
        FindWordRows::Kanji(v) => v.iter().map(dao_from_kanji).collect(),
    }
}

async fn audit_find_word(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("find-word args empty".into()); }
    let word = argv[0].as_str().ok_or("arg 0 not string")?;
    let mut root_only = false;
    let mut i = 1;
    while i < argv.len() {
        let key = argv[i].as_keyword()
            .ok_or_else(|| format!("expected keyword at idx {}: {}", i, argv[i]))?;
        if i + 1 >= argv.len() {
            return Err(format!("keyword :{} missing value", key));
        }
        let v = argv[i + 1];
        match key {
            "ROOT-ONLY" => root_only = v.is_t(),
            other => return Err(format!("find-word: unknown keyword :{}", other)),
        }
        i += 2;
    }
    let actual = find_word(ctx, word, root_only).await
        .map_err(|e| format!("find_word query: {}", e))?;
    compare_dao_lists(project_find_word_rows(&actual), expect_first_of_one_or_two(expected)?)
}

// --- find-word-as-hiragana: proxy-text rows comparison ------------------

#[derive(Debug, PartialEq, Eq)]
struct ProxyDao {
    text: String,
    kana: String,
    source: DaoRow,
}

fn parse_proxy_plist(s: &Sexp) -> Result<ProxyDao, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("proxy plist odd length: {}", s));
    }
    let mut text = None;
    let mut kana = None;
    let mut source = None;
    let mut class_ok = false;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("proxy key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {
                if v.as_keyword() != Some("PROXY-TEXT") {
                    return Err(format!(":CLASS not :PROXY-TEXT: {}", v));
                }
                class_ok = true;
            }
            "TEXT" => text = Some(v.as_str().ok_or(":TEXT not string")?.to_string()),
            "KANA" => kana = Some(v.as_str().ok_or(":KANA not string")?.to_string()),
            "SOURCE" => source = Some(parse_dao_plist(v)?),
            // Inherited simple-text runtime slots — `find-word-as-hiragana`
            // never sets them on the freshly-built proxy (default
            // `Default::default()`), so the projector emits them as NIL /
            // false; nothing to assert against.
            "CONJUGATIONS" | "HINTEDP" => {}
            other => return Err(format!("unknown proxy key: :{}", other)),
        }
    }
    if !class_ok { return Err(":CLASS missing on proxy-text".into()); }
    Ok(ProxyDao {
        text: text.ok_or(":TEXT missing")?,
        kana: kana.ok_or(":KANA missing")?,
        source: source.ok_or(":SOURCE missing")?,
    })
}

fn dao_from_simple(s: &KaniSimpleTextDispatchEnum) -> Result<DaoRow, String> {
    match s {
        KaniSimpleTextDispatchEnum::Kana(k)  => Ok(dao_from_kana(k)),
        KaniSimpleTextDispatchEnum::Kanji(k) => Ok(dao_from_kanji(k)),
        // find-word never returns a proxy-text — its source is always
        // a fresh kana-text or kanji-text DAO row from the table query.
        // A nested proxy at this level would mean either the upstream
        // semantics drifted or this audit is replaying a non-tatoeba
        // capture; surface it instead of silently flattening.
        KaniSimpleTextDispatchEnum::Proxy(_) =>
            Err("unexpected nested proxy under find-word-as-hiragana".into()),
    }
}

fn proxy_dao_from(p: &ProxyText) -> Result<ProxyDao, String> {
    Ok(ProxyDao {
        text: p.text.clone(),
        kana: p.kana.clone(),
        source: dao_from_simple(&p.source)?,
    })
}

fn compare_proxy_lists(
    mut actual: Vec<ProxyDao>, expected: &Sexp,
) -> Result<(), String> {
    let exp_elems = if expected.is_nil() { Vec::new() } else { list_elems(expected)? };
    let mut expected_rows: Vec<ProxyDao> = exp_elems.iter()
        .map(|e| parse_proxy_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!(
            "row count: rust={} lisp={}\n  rust: {:?}\n  lisp: {:?}",
            actual.len(), expected_rows.len(), actual, expected_rows,
        ));
    }
    actual.sort_by_key(|r| (r.source.id, r.source.class.clone()));
    expected_rows.sort_by_key(|r| (r.source.id, r.source.class.clone()));
    for (i, (a, e)) in actual.iter().zip(&expected_rows).enumerate() {
        if a != e {
            return Err(format!("row {}: rust={:?} lisp={:?}", i, a, e));
        }
    }
    Ok(())
}

async fn audit_find_word_as_hiragana(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("find-word-as-hiragana args empty".into()); }
    let str_ = argv[0].as_str().ok_or("arg 0 not string")?;
    let mut exclude: Vec<i32> = Vec::new();
    let mut finder_present_non_nil = false;
    let mut i = 1;
    while i < argv.len() {
        let key = argv[i].as_keyword()
            .ok_or_else(|| format!("expected keyword at idx {}: {}", i, argv[i]))?;
        if i + 1 >= argv.len() {
            return Err(format!("keyword :{} missing value", key));
        }
        let v = argv[i + 1];
        match key {
            "EXCLUDE" => {
                if !v.is_nil() {
                    for e in list_elems(v)? {
                        exclude.push(parse_int(e)?);
                    }
                }
            }
            // The captured fixture format is keyword=keyword: `:FINDER
            // <value>`. Every tatoeba capture has `:FINDER NIL`
            // (or-as-hiragana isn't on the segmenter's path), so the
            // audit always passes `None` as the Rust finder. A non-nil
            // value would imply a closure projection the audit harness
            // can't replay — surface it as a divergence rather than
            // silently coerce.
            "FINDER" => finder_present_non_nil = !v.is_nil(),
            other => return Err(format!("find-word-as-hiragana: unknown keyword :{}", other)),
        }
        i += 2;
    }
    if finder_present_non_nil {
        return Err(":FINDER non-nil — audit harness has no replay strategy for closures".into());
    }
    let actual = find_word_as_hiragana(ctx, str_, &exclude, None).await
        .map_err(|e| format!("find_word_as_hiragana query: {}", e))?;
    let actual_proxies: Vec<ProxyDao> = actual.iter()
        .map(proxy_dao_from)
        .collect::<Result<_, _>>()?;
    compare_proxy_lists(actual_proxies, expect_one(expected)?)
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


// --- get-kana-forms / get-kana-forms* DAO + conjugations comparison -----

#[derive(Debug, PartialEq, Eq)]
struct KanaTextWithConj {
    id: i32,
    seq: i32,
    text: String,
    ord: i32,
    conjugations: Option<WordConjCmp>,
}

/// Audit-only echo of [`WordConjugations`] tolerant of NIL / :ROOT /
/// (id ...) shapes coming out of the projected plist.
#[derive(Debug, PartialEq, Eq, Clone)]
enum WordConjCmp {
    Root,
    Ids(Vec<i32>),
}

fn parse_conjugations(s: &Sexp) -> Result<Option<WordConjCmp>, String> {
    if s.is_nil() { return Ok(None); }
    if s.as_keyword() == Some("ROOT") { return Ok(Some(WordConjCmp::Root)); }
    if s.is_list() {
        let elems = list_elems(s)?;
        let ids: Vec<i32> = elems.iter().map(|e| parse_int(e)).collect::<Result<_, _>>()?;
        return Ok(Some(WordConjCmp::Ids(ids)));
    }
    Err(format!("conjugations: expected NIL/:ROOT/(int...), got {}", s))
}

fn parse_kana_text_plist(s: &Sexp) -> Result<KanaTextWithConj, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("kana-text plist odd length: {}", s));
    }
    let mut id = None;
    let mut seq = None;
    let mut text = None;
    let mut ord = None;
    let mut conjugations = None;
    let mut conjugations_seen = false;
    let mut class_ok = false;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("kana-text key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {
                if v.as_keyword() != Some("KANA-TEXT") {
                    return Err(format!(":CLASS not :KANA-TEXT: {}", v));
                }
                class_ok = true;
            }
            "ID"   => id   = Some(parse_int(v)?),
            "SEQ"  => seq  = Some(parse_int(v)?),
            "TEXT" => text = Some(v.as_str().ok_or(":TEXT not string")?.to_string()),
            "ORD"  => ord  = Some(parse_int(v)?),
            "CONJUGATIONS" => {
                conjugations = parse_conjugations(v)?;
                conjugations_seen = true;
            }
            // Other column slots and HINTEDP are projected but not
            // load-bearing for these audits — drop them.
            "COMMON" | "COMMON-TAGS" | "CONJUGATE-P" | "NOKANJI"
                | "BEST-KANJI" | "HINTEDP" => {}
            other => return Err(format!("unknown kana-text key: :{}", other)),
        }
    }
    if !class_ok { return Err(":CLASS missing on kana-text".into()); }
    if !conjugations_seen {
        return Err("captured kana-text missing :CONJUGATIONS — re-extract under projector patch".into());
    }
    Ok(KanaTextWithConj {
        id: id.ok_or(":ID missing")?,
        seq: seq.ok_or(":SEQ missing")?,
        text: text.ok_or(":TEXT missing")?,
        ord: ord.ok_or(":ORD missing")?,
        conjugations,
    })
}

fn project_kana_text_with_conj(rows: &[KanaText]) -> Vec<KanaTextWithConj> {
    rows.iter()
        .map(|r| KanaTextWithConj {
            id: r.id,
            seq: r.seq,
            text: r.text.clone(),
            ord: r.ord,
            conjugations: r.state.conjugations.as_ref().map(|c| match c {
                WordConjugations::Root => WordConjCmp::Root,
                WordConjugations::Ids(v) => WordConjCmp::Ids(v.clone()),
            }),
        })
        .collect()
}

fn compare_kana_text_with_conj(
    mut actual: Vec<KanaTextWithConj>, expected: &Sexp,
) -> Result<(), String> {
    let exp_elems = if expected.is_nil() { Vec::new() } else { list_elems(expected)? };
    let mut expected_rows: Vec<KanaTextWithConj> = exp_elems.iter()
        .map(|e| parse_kana_text_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!(
            "row count: rust={} lisp={}\n  rust ids: {:?}\n  lisp ids: {:?}",
            actual.len(), expected_rows.len(),
            actual.iter().map(|r| r.id).collect::<Vec<_>>(),
            expected_rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        ));
    }
    actual.sort_by_key(|r| r.id);
    expected_rows.sort_by_key(|r| r.id);
    // Ids inside WordConjCmp::Ids may differ in element order across
    // runs (loop collect order over conj-data list — postmodern's
    // implicit ORDER BY isn't stable across the .103 capture vs the
    // local replay). Compare as a multiset.
    for (i, (a, e)) in actual.iter().zip(&expected_rows).enumerate() {
        if a.id != e.id || a.seq != e.seq || a.text != e.text || a.ord != e.ord {
            return Err(format!("row {}: rust={:?} lisp={:?}", i, a, e));
        }
        let conj_eq = match (&a.conjugations, &e.conjugations) {
            (None, None) => true,
            (Some(WordConjCmp::Root), Some(WordConjCmp::Root)) => true,
            (Some(WordConjCmp::Ids(av)), Some(WordConjCmp::Ids(ev))) => {
                let mut a_sorted = av.clone();
                let mut e_sorted = ev.clone();
                a_sorted.sort();
                e_sorted.sort();
                a_sorted == e_sorted
            }
            _ => false,
        };
        if !conj_eq {
            return Err(format!(
                "row {} (id={}) conjugations: rust={:?} lisp={:?}",
                i, a.id, a.conjugations, e.conjugations,
            ));
        }
    }
    Ok(())
}

async fn audit_get_kana_forms_star(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 { return Err(format!("get-kana-forms* wants 1 arg, got {}", argv.len())); }
    let seq = parse_int(argv[0])?;
    let actual = get_kana_forms_star_(ctx, seq).await
        .map_err(|e| format!("get_kana_forms_star query: {}", e))?;
    compare_kana_text_with_conj(project_kana_text_with_conj(&actual), expect_one(expected)?)
}

async fn audit_get_kana_forms(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 { return Err(format!("get-kana-forms wants 1 arg, got {}", argv.len())); }
    let seq = parse_int(argv[0])?;
    let actual = get_kana_forms(ctx, seq).await
        .map_err(|e| format!("get_kana_forms query: {}", e))?;
    compare_kana_text_with_conj(project_kana_text_with_conj(&actual), expect_one(expected)?)
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

    if SYNC_HARNESS_FQNS.contains(&fqn.as_str()) {
        println!("[{}/{}] {} → {}  SKIP (handled by audit_fixtures)", idx, total, path.display(), fqn);
        return;
    }

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

/// FQNs deliberately handled by the sync harness (`audit_fixtures`) —
/// the async dispatcher recognises them so a mixed parquet directory
/// doesn't drown the report in spurious failures. Run both binaries
/// over the same dir for full coverage.
const SYNC_HARNESS_FQNS: &[&str] = &[
    "ICHIRAN/DICT::SKIP-BY-CONJ-DATA",
    "ICHIRAN/DICT::GET-KANA-FORMS-CONJ-DATA-FILTER",
    "ICHIRAN/DICT::TEST-CONJ-PROP",
    "ICHIRAN/DICT::CONJ-DATA-PROP",
    "ICHIRAN/DICT::MAKE-CONJ-DATA",
    "ICHIRAN/DICT::NO-CONJ-DATA",
];

async fn audit_one(
    fqn: &str, args_str: &str, result_str: &str, ctx: &KaniranContext,
) -> Result<(), String> {
    let args = sexp::parse(args_str).map_err(|e| format!("parse args: {}", e))?;
    let expected = sexp::parse(result_str).map_err(|e| format!("parse result: {}", e))?;
    match fqn {
        "ICHIRAN/DICT:FIND-WORD"              => audit_find_word(ctx, &args, &expected).await,
        "ICHIRAN/DICT:FIND-WORD-AS-HIRAGANA"  => audit_find_word_as_hiragana(ctx, &args, &expected).await,
        "ICHIRAN/DICT::FIND-WORD-SEQ"         => audit_find_word_seq(ctx, &args, &expected).await,
        "ICHIRAN/DICT::FIND-WORD-CONJ-OF"     => audit_find_word_conj_of(ctx, &args, &expected).await,
        "ICHIRAN/DICT::GET-KANA-FORM"         => audit_get_kana_form(ctx, &args, &expected).await,
        "ICHIRAN/DICT::GET-KANA-FORMS"        => audit_get_kana_forms(ctx, &args, &expected).await,
        "ICHIRAN/DICT::GET-KANA-FORMS*"       => audit_get_kana_forms_star(ctx, &args, &expected).await,
        "ICHIRAN/DICT::GET-CONJ-DATA"         => audit_get_conj_data(ctx, &args, &expected).await,
        other if SYNC_HARNESS_FQNS.contains(&other) => {
            Err(format!("__SYNC_HARNESS__:{}", other))
        }
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
