//! Time just `kani::sexp::parse` over the first N rows of every
//! parquet — no audit logic, no Rust transliteration call. Isolates
//! the S-expression parser's throughput from per-row regex-compile
//! costs that dominate audit_fixtures for NORMALIZE / SIMPLIFY-NGRAMS
//! / SPLIT-BY-REGEX.
//!
//! Run:
//!   cargo run --release --example bench_parse -- [rows-per-fn]
//!     default rows-per-fn = 1000

use std::path::Path;

use arrow::array::StringArray;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use kaniran_core::kani::sexp;

fn main() {
    let dir = "corpus/extracted/characters";
    let limit: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1000);

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("parquet"))
        .collect();
    entries.sort_by_key(|e| e.path());

    println!("=== parser-only benchmark, {} rows per fn ===\n", limit);
    println!(
        "  {:46}  {:>8}  {:>8}  {:>10}  {:>12}",
        "FQN", "rows", "time", "rate", "args bytes"
    );
    println!(
        "  {:46}  {:>8}  {:>8}  {:>10}  {:>12}",
        "---", "----", "----", "----", "----------"
    );

    let mut grand_total_rows = 0usize;
    let mut grand_total_bytes = 0usize;
    let mut grand_total_secs = 0.0f64;

    for entry in &entries {
        let path = entry.path();
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();

        // Phase 1: load `limit` rows into memory (NOT timed).
        let file = std::fs::File::open(&path).expect("open");
        let builder = ParquetRecordBatchReaderBuilder::try_new(file).expect("builder");
        let reader = builder.build().expect("reader");

        let mut samples: Vec<(String, String)> = Vec::with_capacity(limit);
        let mut bytes = 0usize;
        'rowloop: for batch in reader {
            let batch = batch.expect("batch");
            let args = batch.column_by_name("args").unwrap()
                .as_any().downcast_ref::<StringArray>().unwrap();
            let result = batch.column_by_name("result").unwrap()
                .as_any().downcast_ref::<StringArray>().unwrap();
            for i in 0..batch.num_rows() {
                if samples.len() >= limit { break 'rowloop; }
                let a = args.value(i).to_string();
                let r = result.value(i).to_string();
                bytes += a.len() + r.len();
                samples.push((a, r));
            }
        }

        // Phase 2: pure parse — timed only across these calls.
        let t0 = std::time::Instant::now();
        for (a, r) in &samples {
            let _ = sexp::parse(a).unwrap_or_else(|e| panic!("parse args: {}\n{}", e, a));
            let _ = sexp::parse(r).unwrap_or_else(|e| panic!("parse result: {}\n{}", e, r));
        }
        let elapsed = t0.elapsed().as_secs_f64();

        let rows = samples.len();
        let parse_rate_rows = rows as f64 / elapsed.max(1e-9);
        let mb_per_s = (bytes as f64 / 1e6) / elapsed.max(1e-9);
        println!(
            "  {:46}  {:>8}  {:>7.3}s  {:>9.0}/s  {:>12}",
            normalize_path_to_fqn(Path::new(&stem)),
            format_count(rows),
            elapsed,
            parse_rate_rows,
            format_count(bytes),
        );
        let _ = mb_per_s; // surfaced in totals
        grand_total_rows += rows;
        grand_total_bytes += bytes;
        grand_total_secs += elapsed;
    }

    println!();
    println!(
        "  {:46}  {:>8}  {:>7.3}s  {:>9.0}/s  {:>12}",
        "TOTAL (all fns)",
        format_count(grand_total_rows),
        grand_total_secs,
        grand_total_rows as f64 / grand_total_secs.max(1e-9),
        format_count(grand_total_bytes),
    );
    println!();
    println!(
        "  per-row mean: {:.0} bytes (args+result) — {:.1} MB/s parse throughput",
        grand_total_bytes as f64 / grand_total_rows.max(1) as f64,
        (grand_total_bytes as f64 / 1e6) / grand_total_secs.max(1e-9),
    );
}

fn normalize_path_to_fqn(p: &Path) -> String {
    let stem = p.file_stem().unwrap().to_string_lossy();
    // Reverse fqn_to_path: characters/normalize.parquet → ICHIRAN/CHARACTERS:NORMALIZE
    // The dir gives us the package (assumes we're in characters/).
    let upper = stem.to_uppercase().replace('_', "-");
    format!("ICHIRAN/CHARACTERS:{}", upper)
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
