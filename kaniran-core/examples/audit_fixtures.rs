//! Audit `corpus/extracted/<pkg>/<sym>.parquet` fixtures against the
//! Rust transliterations. For each row, parse `args` and `result` via
//! `kani::sexp`, dispatch the args to the Rust function, compare the
//! produced value to the captured Lisp result.
//!
//! Run with:
//!   cargo run --release --example audit_fixtures
//!
//! Pass a path or directory:
//!   cargo run --release --example audit_fixtures -- corpus/extracted/characters
//!
//! Reports per-FQN pass / fail / skip counts + the first N mismatches.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use arrow::array::StringArray;
use fancy_regex::Regex as FancyRegex;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use kaniran_core::characters::as_hiragana::as_hiragana;
use kaniran_core::characters::as_katakana::as_katakana;
use kaniran_core::characters::basic_split::{basic_split, SegmentKind};
use kaniran_core::characters::char_class_type::CharClass;
use kaniran_core::characters::kanji_prefix::kanji_prefix;
use kaniran_core::characters::mora_length::mora_length;
use kaniran_core::characters::normalize::normalize;
use kaniran_core::characters::sequential_kanji_positions::sequential_kanji_positions;
use kaniran_core::characters::simplify_ngrams::simplify_ngrams;
use kaniran_core::characters::split_by_regex::split_by_regex;
use kaniran_core::characters::test_word::test_word;
use kaniran_core::characters::to_normal_char::{to_normal_char, NormalizationContext};
use kaniran_core::kani::sexp::{self, Sexp};

const MISMATCH_PRINT_LIMIT: usize = 5;
type Handler = fn(&Sexp, &Sexp) -> Result<(), String>;

fn handlers() -> BTreeMap<&'static str, Handler> {
    let mut m: BTreeMap<&'static str, Handler> = BTreeMap::new();
    m.insert("ICHIRAN/CHARACTERS:NORMALIZE",                 audit_normalize);
    m.insert("ICHIRAN/CHARACTERS:BASIC-SPLIT",               audit_basic_split);
    m.insert("ICHIRAN/CHARACTERS:AS-HIRAGANA",               audit_as_hiragana);
    m.insert("ICHIRAN/CHARACTERS:AS-KATAKANA",               audit_as_katakana);
    m.insert("ICHIRAN/CHARACTERS:MORA-LENGTH",               audit_mora_length);
    m.insert("ICHIRAN/CHARACTERS:KANJI-PREFIX",              audit_kanji_prefix);
    m.insert("ICHIRAN/CHARACTERS:SEQUENTIAL-KANJI-POSITIONS",audit_sequential_kanji_positions);
    m.insert("ICHIRAN/CHARACTERS:TO-NORMAL-CHAR",            audit_to_normal_char);
    m.insert("ICHIRAN/CHARACTERS:SIMPLIFY-NGRAMS",           audit_simplify_ngrams);
    m.insert("ICHIRAN/CHARACTERS:SPLIT-BY-REGEX",            audit_split_by_regex);
    m.insert("ICHIRAN/CHARACTERS:TEST-WORD",                 audit_test_word);
    m
}


// --- shared helpers ------------------------------------------------------

fn list_elems(s: &Sexp) -> Result<Vec<&Sexp>, String> {
    s.list_iter()
        .map(|it| it.collect())
        .ok_or_else(|| format!("expected proper list, got {}", s))
}

fn expect_one(expected: &Sexp) -> Result<&Sexp, String> {
    let elems = list_elems(expected)?;
    if elems.len() != 1 {
        return Err(format!("expected 1-element list, got {}", expected));
    }
    Ok(elems[0])
}

fn parse_keyword_arg(args: &[&Sexp], i: usize, key: &str) -> Result<Option<String>, String> {
    if args.len() <= i { return Ok(None); }
    let k = args[i].as_keyword()
        .ok_or_else(|| format!("arg {} not a keyword: {}", i, args[i]))?;
    if k != key {
        return Err(format!("expected :{}, got :{}", key, k));
    }
    if args.len() <= i + 1 {
        return Err(format!("missing value after :{}", key));
    }
    let v = args[i + 1].as_keyword()
        .ok_or_else(|| format!("value after :{} not a keyword: {}", key, args[i + 1]))?;
    Ok(Some(v.to_string()))
}

fn parse_norm_context(name: Option<&str>) -> Result<NormalizationContext, String> {
    Ok(match name {
        None | Some("DEFAULT") => NormalizationContext::Default,
        Some("KANA") => NormalizationContext::Kana,
        Some(other) => return Err(format!("unknown NormalizationContext: :{}", other)),
    })
}

fn parse_char_class(name: &str) -> Result<CharClass, String> {
    Ok(match name {
        "KATAKANA" => CharClass::Katakana,
        "KATAKANA-UNIQ" => CharClass::KatakanaUniq,
        "HIRAGANA" => CharClass::Hiragana,
        "KANJI" => CharClass::Kanji,
        "KANJI-CHAR" => CharClass::KanjiChar,
        "KANA" => CharClass::Kana,
        "TRADITIONAL" => CharClass::Traditional,
        "NONWORD" => CharClass::Nonword,
        "NUMBER" => CharClass::Number,
        other => return Err(format!("unknown CharClass: :{}", other)),
    })
}

/// `(:CHAR n)` → char.
fn parse_char_tag(s: &Sexp) -> Result<char, String> {
    let elems = list_elems(s).map_err(|e| format!("(:CHAR n) shape: {}", e))?;
    if elems.len() != 2 { return Err(format!("(:CHAR n) wants 2 elements, got {}", s)); }
    if elems[0].as_keyword() != Some("CHAR") {
        return Err(format!("(:CHAR n) car: {}", elems[0]));
    }
    let cp = elems[1].as_i64().ok_or_else(|| format!("(:CHAR n) cdr: {}", elems[1]))? as u32;
    char::from_u32(cp).ok_or_else(|| format!("invalid codepoint: {}", cp))
}


// --- per-FQN handlers ----------------------------------------------------

fn audit_normalize(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let ctx = parse_norm_context(parse_keyword_arg(&argv, 1, "CONTEXT")?.as_deref())?;
    let actual = normalize(s, ctx);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_as_hiragana(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = as_hiragana(s);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_as_katakana(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = as_katakana(s);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_mora_length(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = mora_length(s) as i64;
    let exp = expect_one(expected)?.as_i64().ok_or("expected[0] not int")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual, exp))
    }
}

fn audit_kanji_prefix(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = kanji_prefix(s);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_sequential_kanji_positions(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let offset = argv.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
    let actual: Vec<i64> = sequential_kanji_positions(s, offset).into_iter().map(|n| n as i64).collect();
    // expected: ((p1 p2 ...))  — outer 1-element list, inner list-of-ints (or NIL)
    let inner = expect_one(expected)?;
    let exp: Vec<i64> = if inner.is_nil() {
        Vec::new()
    } else {
        list_elems(inner)?.iter()
            .map(|s| s.as_i64().ok_or_else(|| format!("position not int: {}", s)))
            .collect::<Result<_, _>>()?
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_to_normal_char(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let c = parse_char_tag(argv[0])?;
    let ctx = parse_norm_context(parse_keyword_arg(&argv, 1, "CONTEXT")?.as_deref())?;
    let actual = to_normal_char(c, ctx);
    // expected: (NIL) or ((:CHAR n))
    let inner = expect_one(expected)?;
    let exp = if inner.is_nil() {
        None
    } else {
        Some(parse_char_tag(inner)?)
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_simplify_ngrams(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    // arg 1 is a flat plist of (k1 v1 k2 v2 ...) — pair them up.
    let plist = list_elems(argv[1])?;
    if plist.len() % 2 != 0 {
        return Err(format!("plist has odd length: {}", plist.len()));
    }
    let mut map: Vec<(String, String)> = Vec::with_capacity(plist.len() / 2);
    for pair in plist.chunks(2) {
        let k = pair[0].as_str().ok_or_else(|| format!("plist key not string: {}", pair[0]))?;
        let v = pair[1].as_str().ok_or_else(|| format!("plist value not string: {}", pair[1]))?;
        map.push((k.to_string(), v.to_string()));
    }
    let actual = simplify_ngrams(s, &map);
    // expected: (cleaned T)  — multiple values; first is the string, second is bool.
    let exp_elems = list_elems(expected)?;
    let exp_str = exp_elems.first().and_then(|v| v.as_str())
        .ok_or("expected[0] not string")?;
    if actual == exp_str { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp_str))
    }
}

fn audit_split_by_regex(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let pattern = argv[0].as_str().ok_or("arg 0 not regex string")?;
    let s = argv[1].as_str().ok_or("arg 1 not string")?;
    let re = FancyRegex::new(pattern)
        .map_err(|e| format!("regex compile failed: {}", e))?;
    let actual: Vec<String> = split_by_regex(&re, s);
    let inner = expect_one(expected)?;
    let exp: Vec<String> = if inner.is_nil() {
        Vec::new()
    } else {
        list_elems(inner)?.iter()
            .map(|v| v.as_str().map(String::from).ok_or_else(|| format!("token not string: {}", v)))
            .collect::<Result<_, _>>()?
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_test_word(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let kw = argv[1].as_keyword().ok_or("arg 1 not keyword")?;
    let class = parse_char_class(kw)?;
    let actual = test_word(s, class);
    // Lisp `test-word` calls cl-ppcre:scan which returns 4 values:
    // (start end groups names) on match, or NIL on no match. The
    // multiple-value-list capture is therefore either `(NIL)` (no match)
    // or `(start end #() #())` (match). Compare just the truthiness of
    // the first element. The Rust port collapsed to `bool` ahead of
    // CONVENTIONS §4.1 — see TODO upstream.
    let exp_elems = list_elems(expected)?;
    let exp_truthy = match exp_elems.first() {
        Some(first) => !first.is_nil(),
        None => return Err("expected list is empty".into()),
    };
    if actual == exp_truthy { Ok(()) } else {
        Err(format!("\n  rust bool: {}\n  lisp:      {}", actual, expected))
    }
}

fn audit_basic_split(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = basic_split(s);
    // expected: (((:WORD . "x") (:MISC . ".") ...))  — outer 1-list, inner list-of-cons
    let inner = expect_one(expected)?;
    let pairs = if inner.is_nil() { Vec::new() } else { list_elems(inner)? };
    if actual.len() != pairs.len() {
        return Err(format!("len mismatch: rust {} vs lisp {}", actual.len(), pairs.len()));
    }
    for (i, (rust_pair, lisp_pair)) in actual.iter().zip(pairs.iter()).enumerate() {
        let (lcar, lcdr) = lisp_pair.as_cons()
            .ok_or_else(|| format!("pair {} not cons: {}", i, lisp_pair))?;
        let lkind = lcar.as_keyword()
            .ok_or_else(|| format!("pair {} car not keyword: {}", i, lcar))?;
        let lstr = lcdr.as_str()
            .ok_or_else(|| format!("pair {} cdr not string: {}", i, lcdr))?;
        let rust_kind = match rust_pair.0 { SegmentKind::Word => "WORD", SegmentKind::Misc => "MISC" };
        if rust_kind != lkind || rust_pair.1 != lstr {
            return Err(format!(
                "\n  pair {}:\n    rust: ({} . {:?})\n    lisp: (:{} . {:?})",
                i, rust_kind, rust_pair.1, lkind, lstr
            ));
        }
    }
    Ok(())
}


// --- driver --------------------------------------------------------------

fn audit_file(
    idx: usize,
    of: usize,
    path: &Path,
    h: &BTreeMap<&str, Handler>,
    totals: &mut Totals,
) {
    println!("--- file {}/{} : {}", idx, of, path.display());
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("open {:?}: {}", path, e));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap_or_else(|e| panic!("parquet builder {:?}: {}", path, e));
    let metadata = builder.metadata().clone();
    let kv = metadata
        .file_metadata()
        .key_value_metadata()
        .cloned()
        .unwrap_or_default();
    let fqn = kv.iter()
        .find(|kv| kv.key == "ichiran_extractor_fqn")
        .and_then(|kv| kv.value.clone())
        .unwrap_or_else(|| panic!("no ichiran_extractor_fqn in {:?}", path));

    let handler = match h.get(fqn.as_str()) {
        Some(h) => *h,
        None => {
            println!("  SKIP {}  (no handler registered)", fqn);
            totals.skipped += 1;
            return;
        }
    };

    let total_rows = metadata.file_metadata().num_rows() as usize;
    let reader = builder.build().expect("build reader");
    let mut n_pass = 0;
    let mut n_fail = 0;
    let mut n_done = 0usize;
    let mut last_progress_at = 0usize;
    let progress_every: usize = (total_rows / 10).max(50_000).min(200_000);
    let mut first_failures: Vec<String> = Vec::new();
    let t0 = std::time::Instant::now();
    println!("  > {:48}  total={}", fqn, format_count(total_rows));

    for batch in reader {
        let batch = batch.expect("batch");
        let args_col = batch.column_by_name("args").expect("args column")
            .as_any().downcast_ref::<StringArray>().expect("args is StringArray");
        let result_col = batch.column_by_name("result").expect("result column")
            .as_any().downcast_ref::<StringArray>().expect("result is StringArray");
        for i in 0..batch.num_rows() {
            let args_src = args_col.value(i);
            let result_src = result_col.value(i);
            let args = match sexp::parse(args_src) {
                Ok(v) => v,
                Err(e) => {
                    n_fail += 1;
                    if first_failures.len() < MISMATCH_PRINT_LIMIT {
                        first_failures.push(format!("[parse-args] {}\n  args: {}", e, args_src));
                    }
                    continue;
                }
            };
            let expected = match sexp::parse(result_src) {
                Ok(v) => v,
                Err(e) => {
                    n_fail += 1;
                    if first_failures.len() < MISMATCH_PRINT_LIMIT {
                        first_failures.push(format!("[parse-result] {}\n  result: {}", e, result_src));
                    }
                    continue;
                }
            };
            match handler(&args, &expected) {
                Ok(()) => n_pass += 1,
                Err(e) => {
                    n_fail += 1;
                    if first_failures.len() < MISMATCH_PRINT_LIMIT {
                        first_failures.push(format!("{}\n  args: {}\n  err:  {}",
                            args_src, args_src, e));
                    }
                }
            }
            n_done += 1;
            if n_done - last_progress_at >= progress_every {
                last_progress_at = n_done;
                let elapsed = t0.elapsed().as_secs_f64();
                let rate = n_done as f64 / elapsed.max(1e-9);
                let pct = if total_rows > 0 { 100.0 * n_done as f64 / total_rows as f64 } else { 0.0 };
                let eta = if rate > 0.0 && total_rows > n_done {
                    format_duration((total_rows - n_done) as f64 / rate)
                } else { "?".into() };
                println!(
                    "    [progress] total={}  current={} ({:.1}%)  rate={}/s  elapsed={}  ETA={}  fail={}",
                    format_count(total_rows),
                    format_count(n_done), pct,
                    format_count(rate as usize),
                    format_duration(elapsed),
                    eta,
                    n_fail,
                );
            }
        }
    }

    let elapsed = t0.elapsed().as_secs_f64();
    let total = n_pass + n_fail;
    let rate = total as f64 / elapsed.max(1e-9);
    let pct = if total > 0 { 100.0 * n_pass as f64 / total as f64 } else { 0.0 };
    let tag = if n_fail == 0 { "PASS" } else { "FAIL" };
    println!(
        "  {} {:48} pass={:>7}  fail={:>7}  ({:>6.2}%, {:>6.0} rows/s, {:>5.2}s)",
        tag, fqn, n_pass, n_fail, pct, rate, elapsed,
    );
    if !first_failures.is_empty() {
        for (i, f) in first_failures.iter().enumerate() {
            println!("    [{}] {}", i + 1, f);
        }
    }

    totals.pass += n_pass;
    totals.fail += n_fail;
    if n_fail == 0 { totals.fns_clean += 1; } else { totals.fns_with_failures += 1; }
}

#[derive(Default)]
struct Totals {
    pass: usize,
    fail: usize,
    skipped: usize,
    fns_clean: usize,
    fns_with_failures: usize,
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
    if h > 0 {
        format!("{}h{:02}m{:02}s", h, m, s)
    } else if m > 0 {
        format!("{}m{:02}s", m, s)
    } else {
        format!("{:.1}s", seconds)
    }
}

fn discover_parquets(arg: &str) -> Vec<PathBuf> {
    let p = Path::new(arg);
    if p.is_file() {
        return vec![p.to_path_buf()];
    }
    let mut out = Vec::new();
    if p.is_dir() {
        for entry in std::fs::read_dir(p).expect("read_dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn main() {
    let arg = std::env::args().nth(1)
        .unwrap_or_else(|| "corpus/extracted/characters".to_string());
    let parquets = discover_parquets(&arg);
    if parquets.is_empty() {
        eprintln!("no .parquet files at {}", arg);
        std::process::exit(2);
    }

    let h = handlers();
    let mut totals = Totals::default();
    let t0 = std::time::Instant::now();

    println!("=== auditing {} parquet file(s) ===\n", parquets.len());
    for (i, path) in parquets.iter().enumerate() {
        audit_file(i + 1, parquets.len(), path, &h, &mut totals);
        let so_far = totals.pass + totals.fail;
        let elapsed = t0.elapsed().as_secs_f64();
        let rate = so_far as f64 / elapsed.max(1e-9);
        println!(
            "    [overall] files={}/{}  rows={}  pass={}  fail={}  rate={}/s  elapsed={}\n",
            i + 1, parquets.len(),
            format_count(so_far),
            format_count(totals.pass),
            format_count(totals.fail),
            format_count(rate as usize),
            format_duration(elapsed),
        );
    }

    let elapsed = t0.elapsed().as_secs_f64();
    let total_rows = totals.pass + totals.fail;
    println!();
    println!("=== summary ===");
    println!("  parquet files:    {}", parquets.len());
    println!("  fns clean:        {}", totals.fns_clean);
    println!("  fns w/ failures:  {}", totals.fns_with_failures);
    println!("  fns skipped:      {}", totals.skipped);
    println!("  rows audited:     {}", total_rows);
    println!("  rows passed:      {} ({:.3}%)",
             totals.pass,
             100.0 * totals.pass as f64 / total_rows.max(1) as f64);
    println!("  rows failed:      {}", totals.fail);
    println!("  wall clock:       {:.1}s ({:.0} rows/s)",
             elapsed, total_rows as f64 / elapsed.max(1e-9));

    if totals.fail > 0 {
        std::process::exit(1);
    }
}
