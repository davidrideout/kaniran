//! Loads a kaniran database from source: runs the complete data-loader
//! chain (JMdict + kanjidic + kanji stats) end to end against a fresh
//! schema, in the order of `ichiran/maintenance:full-init`.
//!
//! Run:
//!   cargo run --release -p kaniran-loader --bin full_e2e_load -- \
//!       --db-url postgres:///ichiran_full_e2e \
//!       --jmdict-path bookkeeping/e2e/fixtures/JMdict_e_2026-02-22.xml \
//!       --kanjidic-path bookkeeping/e2e/fixtures/kanjidic2_2015-03-17.xml
//!
//! The data loaders are async and run their `sqlx` queries against the
//! pool, so `main` is synchronous: [`KaniranContext::pool_only_from_url`]
//! builds a cache-empty context whose pool carries its own runtime, and
//! [`KaniranContext::block_on`] drives each loader on that runtime. A
//! fresh schema has empty dictionary tables, so the cache populators that
//! `from_url` runs can't run here — this is the build path that fills
//! those tables.

use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_loader::custom::load::load_custom_data;
use kaniran_loader::dict::load::readings::load_best_readings;
use kaniran_loader::dict::load::jmdict::load_jmdict;
use kaniran_loader::dict::load::jmdict::recalc_entry_stats_all;
use kaniran_loader::kanji::loaders::load_kanji_stats;
use kaniran_loader::kanji::loaders::load_kanjidic;
use kaniran_loader::maintenance::add_errata::add_errata;

#[derive(Parser)]
struct Args {
    /// Postgres URL of the target DB.
    #[arg(long)]
    db_url: String,

    /// Path to the JMdict_e XML dump.
    #[arg(long)]
    jmdict_path: PathBuf,

    /// Path to a kanjidic2.xml dump.
    #[arg(long)]
    kanjidic_path: PathBuf,

    /// Skip load_extras (errata + secondary conjugations).
    #[arg(long)]
    skip_extras: bool,

    /// Skip load_kanjidic + load_kanji_stats.
    #[arg(long)]
    skip_kanji: bool,

    /// Resume mid-pipeline: skip load_jmdict entirely (it would
    /// TRUNCATE the tables), assume `load_conjugations` and
    /// `load_secondary_conjugations` already ran, and pick up at
    /// `load_custom_data` → `add_errata` → `recalc_entry_stats_all` →
    /// `ANALYZE`, then proceed with kanjidic. Use after a crash inside
    /// `load_extras` once the missing input file is fixed.
    #[arg(long)]
    resume_from_custom_data: bool,

    /// Run only `load_best_readings` against the existing DB and exit.
    /// Use after the rest of the chain already ran but missed this step
    /// (kaniran wired it in late; pre-fix runs left every
    /// `kanji_text.best_kana` / `kana_text.best_kanji` as NULL).
    #[arg(long)]
    just_best_readings: bool,

    /// Run only `load_kanji_stats` against the existing DB and exit.
    /// Use after `--just-best-readings` back-filled `kanji_text.best_kana`
    /// on a DB whose original load ran the chain in the wrong order —
    /// `load_kanji_stats` reads `best_kana` via `get_kanji_words`, so a
    /// pre-fix run got total=0 on every kanji and wrote `stat_common=0`
    /// everywhere.
    #[arg(long)]
    just_kanji_stats: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // pool_only_from_url skips the JMdict-backed cache populators
    // (build_no_conj_data / build_counter_cache / build_suffix_caches);
    // those query tables that are empty on a fresh DB. The loaders are
    // async, so ctx.block_on runs each on the pool's own runtime.
    let ctx = KaniranContext::pool_only_from_url(&args.db_url)?;

    let t0 = Instant::now();
    if args.just_best_readings {
        println!("e2e: load_best_readings (reset=true)");
        ctx.block_on(load_best_readings(&ctx, true))?;
        println!("e2e: load_best_readings done in {:.1}s", t0.elapsed().as_secs_f64());
        return Ok(());
    }
    if args.just_kanji_stats {
        println!("e2e: load_kanji_stats");
        ctx.block_on(load_kanji_stats(&ctx))?;
        println!("e2e: load_kanji_stats done in {:.1}s", t0.elapsed().as_secs_f64());
        return Ok(());
    }
    if args.resume_from_custom_data {
        // Pick up where load_extras crashed mid-stream. Skips
        // load_jmdict (would TRUNCATE everything), load_conjugations,
        // and load_secondary_conjugations (already in the DB).
        println!("e2e: resume — load_custom_data");
        ctx.block_on(load_custom_data(&ctx, &[], true))?;
        println!("e2e: resume — add_errata");
        ctx.block_on(add_errata(&ctx))?;
        println!("e2e: resume — recalc_entry_stats_all");
        ctx.block_on(recalc_entry_stats_all(&ctx))?;
        println!("e2e: resume — load_best_readings");
        ctx.block_on(load_best_readings(&ctx, true))?;
        println!("e2e: resume — ANALYZE");
        ctx.block_on(sqlx::query("ANALYZE").execute(ctx.pool.as_ref().expect("postgres pool")))?;
        println!("e2e: resume done in {:.1}s", t0.elapsed().as_secs_f64());
    } else {
        println!("e2e: load_jmdict from {}", args.jmdict_path.display());
        ctx.block_on(load_jmdict(&ctx, &args.jmdict_path, !args.skip_extras))?;
        println!("e2e: load_jmdict done in {:.1}s", t0.elapsed().as_secs_f64());
        // ichiran.lisp:34 — (load-best-readings :reset t) runs between
        // load-jmdict and load-kanjidic. Skip when --skip-extras since
        // it depends on the populated conjugation tables.
        if !args.skip_extras {
            let t = Instant::now();
            println!("e2e: load_best_readings");
            ctx.block_on(load_best_readings(&ctx, true))?;
            println!("e2e: load_best_readings done in {:.1}s", t.elapsed().as_secs_f64());
        }
    }

    if !args.skip_kanji {
        let t1 = Instant::now();
        println!("e2e: load_kanjidic from {}", args.kanjidic_path.display());
        ctx.block_on(load_kanjidic(&ctx, &args.kanjidic_path))?;
        println!("e2e: load_kanjidic done in {:.1}s", t1.elapsed().as_secs_f64());

        let t2 = Instant::now();
        println!("e2e: load_kanji_stats");
        ctx.block_on(load_kanji_stats(&ctx))?;
        println!("e2e: load_kanji_stats done in {:.1}s", t2.elapsed().as_secs_f64());
    }

    println!("e2e: total {:.1}s", t0.elapsed().as_secs_f64());
    Ok(())
}
