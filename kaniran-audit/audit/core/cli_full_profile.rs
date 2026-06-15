//! Profiling / speed driver for the e2e cli_full pipeline on the rkyv
//! backend (MEM_PLAN performance pass). Runs the same JSON reproduction
//! as `cli_full_test` (`full_json` mirrored from there — keep in sync)
//! with no expected-output comparison, under a pprof sampling guard.
//! `--concurrency 1` (default) is the documented single-thread baseline;
//! higher values fan sentences across worker threads to measure engine
//! throughput scaling without the audit harness's per-row overhead.
//!
//! Run with:
//!   DATABASE_URL=memory://corpus/kaniran_ichiran_latest_2026_06_10.rkyv \
//!   cargo run --profile profiling -p kaniran-audit --features rkyv \
//!     --bin cli_full_profile -- \
//!     --path corpus/cli_full_ichiran_latest_2026_06_09.parquet \
//!     --limit 20000 --flamegraph /tmp/cli_full_profile_20k.svg

#[path = "../common/mod.rs"]
mod common;

// Allocator diagnostic for the perf pass: the profile shows ~34% of
// CPU in allocator page management under macOS system malloc.
#[cfg(not(feature = "dhat-heap"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Allocation profiling (feature dhat-heap): dhat::Alloc records every
// allocation with its backtrace while a dhat::Profiler is alive. The
// profiler brackets only the run loop, so startup (archive load, index
// build, populators) stays out of the numbers; outside the profiler's
// lifetime dhat forwards to the system allocator unrecorded. Expect a
// large slowdown — throughput numbers from a dhat run are meaningless.
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static GLOBAL: dhat::Alloc = dhat::Alloc;

#[cfg(not(feature = "dhat-heap"))]
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use arrow::array::{Array, StringArray};
use clap::Parser;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ProjectionMask;
use serde_json::{Number, Value};

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::core::kani_romanize_method::KaniRomanizeMethod;
use kaniran_core::core::methods::hepburn_traditional;
use kaniran_core::core::methods::RomanizationMethod;
use kaniran_core::core::romanize::{romanize_star_, RomanizeStarSegment};
use kaniran_core::dict::word_info_str::word_info_gloss_json;

#[derive(Parser)]
struct Args {
    /// cli_full corpus parquet; only the `args` column (the sentence) is read.
    #[arg(long)]
    path: std::path::PathBuf,
    /// How many sentences to run (from the start of the parquet).
    #[arg(long, default_value_t = 20_000)]
    limit: usize,
    /// Where to write the flamegraph SVG.
    #[arg(long, default_value = "/tmp/cli_full_profile.svg")]
    flamegraph: String,
    /// pprof sampling frequency (Hz).
    #[arg(long, default_value_t = 250)]
    frequency: i32,
    /// Where to write the dhat heap profile (feature dhat-heap only;
    /// view at https://nnethercote.github.io/dh_view/dh_view.html).
    #[arg(long, default_value = "/tmp/cli_full_profile_dhat.json")]
    dhat_out: String,
    /// Worker threads to fan sentences across (1 = sequential, the
    /// documented single-thread baseline). Each thread pulls the next
    /// sentence via a shared atomic counter; the reported rate is the
    /// aggregate across threads.
    #[arg(long, default_value_t = 1)]
    concurrency: usize,
}

/// Cumulative wall-clock split between the two pipeline halves.
#[derive(Default)]
struct PhaseSplit {
    romanize: Duration,
    gloss: Duration,
}

/// Mirror of `cli_full_test::full_json` with per-phase timers; the
/// produced JSON is returned so the optimizer can't drop the work.
fn full_json(
    ctx: &KaniranContext,
    text: &str,
    split: &mut PhaseSplit,
) -> Result<Value, kaniran_core::conn::KaniDbError> {
    let method = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
        hepburn_traditional(),
    ));
    let romanize_start = Instant::now();
    let segments = romanize_star_(ctx, text, method, Some(5), |_, _| ())?;
    split.romanize += romanize_start.elapsed();

    let gloss_start = Instant::now();
    let mut top = Vec::with_capacity(segments.len());
    for segment in &segments {
        match segment {
            RomanizeStarSegment::Misc(misc) => top.push(Value::String(misc.clone())),
            RomanizeStarSegment::Word(alternatives) => {
                let mut alts = Vec::with_capacity(alternatives.len());
                for (words, score) in alternatives {
                    let mut word_jsons = Vec::with_capacity(words.len());
                    for (romaji, word_info, _prop) in words {
                        let gloss = word_info_gloss_json(ctx, word_info, false)?;
                        // Move `gloss` instead of re-serializing it through
                        // json!'s to_value (mirrors cli_full_test — keep in sync).
                        word_jsons.push(Value::Array(vec![
                            Value::String(romaji.clone()),
                            gloss,
                            Value::Array(Vec::new()),
                        ]));
                    }
                    alts.push(Value::Array(vec![
                        Value::Array(word_jsons),
                        Value::Number(Number::from(*score)),
                    ]));
                }
                top.push(Value::Array(alts));
            }
        }
    }
    split.gloss += gloss_start.elapsed();
    Ok(Value::Array(top))
}

/// Stream the first `limit` sentences out of the parquet, reading only
/// the `args` column (the `result` column is multi-KB per row and
/// unused here).
fn load_sentences(path: &std::path::Path, limit: usize) -> Vec<String> {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|err| panic!("open {:?}: {}", path, err));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap_or_else(|err| panic!("parquet builder {:?}: {}", path, err));
    let args_leaf = builder
        .parquet_schema()
        .columns()
        .iter()
        .position(|column| column.name() == "args")
        .expect("args column in parquet schema");
    let mask = ProjectionMask::leaves(builder.parquet_schema(), [args_leaf]);
    let reader = builder
        .with_projection(mask)
        .build()
        .expect("build projected reader");

    let mut sentences = Vec::with_capacity(limit);
    'batches: for batch in reader {
        let batch = batch.expect("batch");
        let args_col = batch
            .column_by_name("args")
            .expect("args column")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("args is StringArray");
        for row_idx in 0..batch.num_rows() {
            let args: Vec<Value> =
                serde_json::from_str(args_col.value(row_idx)).expect("args JSON");
            let sentence = args
                .first()
                .and_then(|value| value.as_str())
                .expect("sentence arg");
            sentences.push(sentence.to_string());
            if sentences.len() >= limit {
                break 'batches;
            }
        }
    }
    sentences
}

fn fmt_dur(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let rest = secs % 60;
    if hours > 0 {
        format!("{}h{:02}m{:02}s", hours, mins, rest)
    } else if mins > 0 {
        format!("{}m{:02}s", mins, rest)
    } else {
        format!("{}s", rest)
    }
}

/// Print a top-N table of self/total sample counts per symbol from the
/// pprof report. `frames[0]` is the leaf (innermost) frame. The SVG is
/// the authority; this is the terminal-readable summary.
#[cfg(not(feature = "dhat-heap"))]
fn print_top_symbols(report: &pprof::Report, top_n: usize) {
    let mut self_counts: HashMap<String, isize> = HashMap::new();
    let mut total_counts: HashMap<String, isize> = HashMap::new();
    let mut total_samples: isize = 0;
    for (frames, count) in &report.data {
        total_samples += count;
        if let Some(leaf) = frames.frames.first().and_then(|inlined| inlined.first()) {
            *self_counts.entry(leaf.name()).or_default() += count;
        }
        let mut seen: Vec<String> = Vec::new();
        for inlined in &frames.frames {
            for symbol in inlined {
                let name = symbol.name();
                if !seen.contains(&name) {
                    seen.push(name);
                }
            }
        }
        for name in seen {
            *total_counts.entry(name).or_default() += count;
        }
    }
    let mut rows: Vec<(&String, &isize)> = self_counts.iter().collect();
    rows.sort_by(|left, right| right.1.cmp(left.1));
    eprintln!("\ntop {} symbols by self samples (total {} samples):", top_n, total_samples);
    eprintln!("{:>7} {:>6}  {:>7} {:>6}  symbol", "self", "self%", "total", "tot%");
    for (name, self_count) in rows.into_iter().take(top_n) {
        let total_count = total_counts.get(name).copied().unwrap_or(0);
        let mut shown = name.clone();
        if shown.len() > 120 {
            shown.truncate(120);
            shown.push('…');
        }
        eprintln!(
            "{:>7} {:>5.1}%  {:>7} {:>5.1}%  {}",
            self_count,
            100.0 * *self_count as f64 / total_samples.max(1) as f64,
            total_count,
            100.0 * total_count as f64 / total_samples.max(1) as f64,
            shown,
        );
    }
}

fn main() {
    let args = Args::parse();

    let load_start = Instant::now();
    let sentences = load_sentences(&args.path, args.limit);
    eprintln!(
        "loaded {} sentences in {:.1}s",
        sentences.len(),
        load_start.elapsed().as_secs_f64()
    );

    let ctx_start = Instant::now();
    let ctx = common::setup_ctx();
    eprintln!("ctx ready in {:.1}s", ctx_start.elapsed().as_secs_f64());

    #[cfg(not(feature = "dhat-heap"))]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(args.frequency)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .expect("pprof guard");
    #[cfg(feature = "dhat-heap")]
    let dhat_profiler = dhat::Profiler::builder()
        .file_name(std::path::PathBuf::from(&args.dhat_out))
        .build();

    let total = sentences.len();
    let next = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let errors_atomic = AtomicUsize::new(0);
    let concurrency = args.concurrency.max(1);
    eprintln!("running {} sentences across {} thread(s)", total, concurrency);

    let run_start = Instant::now();
    let split = std::thread::scope(|scope| {
        // Progress monitor: aggregate rate every ~5s until drained.
        let monitor = scope.spawn(|| {
            let mut last_tick = run_start;
            let mut last_done: usize = 0;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let done = completed.load(Ordering::Relaxed);
                let now = Instant::now();
                if now.duration_since(last_tick).as_secs() >= 5 || done >= total {
                    let elapsed = now.duration_since(run_start).as_secs_f64();
                    let recent_rate = (done - last_done) as f64
                        / now.duration_since(last_tick).as_secs_f64().max(1e-6);
                    let avg_rate = done as f64 / elapsed.max(1e-6);
                    let eta = if avg_rate > 0.0 {
                        ((total - done) as f64 / avg_rate) as u64
                    } else {
                        0
                    };
                    eprintln!(
                        "[cli_full_profile] {}/{} ({:.1}%), errors={}, recent {:.1}/s avg {:.1}/s, elapsed {}, eta {}",
                        done,
                        total,
                        100.0 * done as f64 / total.max(1) as f64,
                        errors_atomic.load(Ordering::Relaxed),
                        recent_rate,
                        avg_rate,
                        fmt_dur(elapsed as u64),
                        fmt_dur(eta),
                    );
                    last_tick = now;
                    last_done = done;
                }
                if done >= total {
                    break;
                }
            }
        });

        // Workers: each pulls the next sentence index until the list is
        // drained, accumulating its own phase split.
        let workers: Vec<_> = (0..concurrency)
            .map(|_| {
                scope.spawn(|| {
                    let mut split = PhaseSplit::default();
                    loop {
                        let idx = next.fetch_add(1, Ordering::Relaxed);
                        if idx >= total {
                            break;
                        }
                        match full_json(&ctx, &sentences[idx], &mut split) {
                            Ok(value) => {
                                std::hint::black_box(&value);
                            }
                            Err(err) => {
                                let seen = errors_atomic.fetch_add(1, Ordering::Relaxed);
                                if seen < 10 {
                                    eprintln!(
                                        "ERROR [row {}] {:?}: {}",
                                        idx + 1,
                                        &sentences[idx],
                                        err
                                    );
                                }
                            }
                        }
                        completed.fetch_add(1, Ordering::Relaxed);
                    }
                    split
                })
            })
            .collect();

        let mut total_split = PhaseSplit::default();
        for worker in workers {
            let worker_split = worker.join().expect("worker panicked");
            total_split.romanize += worker_split.romanize;
            total_split.gloss += worker_split.gloss;
        }
        monitor.join().expect("monitor panicked");
        total_split
    });
    let wall = run_start.elapsed();
    let errors = errors_atomic.load(Ordering::Relaxed);

    #[cfg(feature = "dhat-heap")]
    {
        // Drop writes the JSON and prints total bytes/blocks to stderr.
        drop(dhat_profiler);
        eprintln!("dhat heap profile: {}", args.dhat_out);
    }
    #[cfg(not(feature = "dhat-heap"))]
    {
        let report = guard.report().build().expect("pprof report");
        let svg = std::fs::File::create(&args.flamegraph).expect("create flamegraph file");
        report.flamegraph(svg).expect("write flamegraph");
        eprintln!("flamegraph: {}", args.flamegraph);
        print_top_symbols(&report, 30);
    }

    eprintln!(
        "\ndone: {} sentences in {:.1}s ({:.1}/s aggregate, {} thread(s)), errors={}",
        total,
        wall.as_secs_f64(),
        total as f64 / wall.as_secs_f64().max(1e-6),
        concurrency,
        errors,
    );
    // Phase times are summed across worker threads, so report each as a
    // share of total measured phase CPU (romanize + gloss) rather than of
    // wall — that ratio is thread-count-independent (~81/18).
    let phase_cpu = (split.romanize + split.gloss).as_secs_f64().max(1e-6);
    eprintln!(
        "phase split (summed CPU over {} thread(s)): romanize_star_ {:.1}s ({:.1}%), gloss+assembly {:.1}s ({:.1}%)",
        concurrency,
        split.romanize.as_secs_f64(),
        100.0 * split.romanize.as_secs_f64() / phase_cpu,
        split.gloss.as_secs_f64(),
        100.0 * split.gloss.as_secs_f64() / phase_cpu,
    );
}
