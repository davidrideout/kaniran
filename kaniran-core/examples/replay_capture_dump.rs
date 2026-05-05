//! Replay a `/extract` JSONL response dump from `ichiran-extractor`
//! through `kani::sexp::parse`, asserting every `args` and `result`
//! string round-trips. One-shot validation tool — not a unit test.
//!
//! Usage:
//!   cargo run --release --example replay_capture_dump -- <path.jsonl>
//!
//! The file format is one full `/extract` HTTP-style response per line:
//!   {"ok":true,"result":{"captures":[{...},...],"skipped":N}}
//! plus an optional leading `{"ok":true,"result":<install-count>}` line
//! (the install op response). Anything not matching the captures shape
//! is skipped silently.

use std::collections::BTreeMap;
use std::env;
use std::io::{BufRead, BufReader};

use kaniran_core::kani::sexp;
use serde_json::Value;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: replay_capture_dump <path.jsonl>");
        std::process::exit(2);
    });

    let f = std::fs::File::open(&path).unwrap_or_else(|e| {
        eprintln!("open {}: {}", path, e);
        std::process::exit(1);
    });

    let mut total_args = 0usize;
    let mut total_results = 0usize;
    let mut errors: Vec<(String, String, String, String)> = Vec::new(); // (fn, kind, src, msg)
    let mut per_fn: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_fn_err: BTreeMap<String, usize> = BTreeMap::new();
    let mut max_args_len = 0usize;
    let mut max_result_len = 0usize;

    for (lineno, line) in BufReader::new(f).lines().enumerate() {
        let line = line.expect("io");
        if line.is_empty() { continue; }

        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("line {}: bad JSON: {}", lineno + 1, e);
                continue;
            }
        };

        if v.get("ok") != Some(&Value::Bool(true)) {
            eprintln!("line {}: response not ok: {}", lineno + 1, v);
            continue;
        }
        let captures = match v.pointer("/result/captures").and_then(|c| c.as_array()) {
            Some(a) => a,
            None => continue, // install/ping/etc. response — skip
        };

        for cap in captures {
            let fqn = cap.get("fn").and_then(|x| x.as_str()).unwrap_or("?").to_string();
            let args_src = cap.get("args").and_then(|x| x.as_str()).unwrap_or("");
            let result_src = cap.get("result").and_then(|x| x.as_str()).unwrap_or("");

            *per_fn.entry(fqn.clone()).or_default() += 1;
            max_args_len = max_args_len.max(args_src.len());
            max_result_len = max_result_len.max(result_src.len());

            total_args += 1;
            if let Err(e) = sexp::parse(args_src) {
                errors.push((fqn.clone(), "args".into(), args_src.to_string(), e.to_string()));
                *per_fn_err.entry(fqn.clone()).or_default() += 1;
            }
            total_results += 1;
            if let Err(e) = sexp::parse(result_src) {
                errors.push((fqn.clone(), "result".into(), result_src.to_string(), e.to_string()));
                *per_fn_err.entry(fqn).or_default() += 1;
            }
        }
    }

    let total = total_args + total_results;
    let n_err = errors.len();
    let n_ok = total - n_err;

    println!("=== replay summary ===");
    println!("file:          {}", path);
    println!("total strings: {}  ({} args + {} results)",
             total, total_args, total_results);
    println!("parsed OK:     {}  ({:.3}%)",
             n_ok, 100.0 * n_ok as f64 / total.max(1) as f64);
    println!("errors:        {}", n_err);
    println!("max args len:  {} bytes", max_args_len);
    println!("max result len:{} bytes", max_result_len);
    println!();
    println!("per-fn capture counts:");
    for (fqn, n) in &per_fn {
        let errs = per_fn_err.get(fqn).copied().unwrap_or(0);
        let tag = if errs > 0 { format!(" [{} ERR]", errs) } else { String::new() };
        println!("  {:<48} {:>6}{}", fqn, n, tag);
    }

    if !errors.is_empty() {
        println!();
        println!("=== first {} parse errors ===", errors.len().min(10));
        for (fqn, kind, src, msg) in errors.iter().take(10) {
            let snippet = if src.len() > 200 { format!("{}…", &src[..200]) } else { src.clone() };
            println!("  fn={}  field={}", fqn, kind);
            println!("    src: {}", snippet);
            println!("    err: {}", msg);
        }
        std::process::exit(1);
    }
}
